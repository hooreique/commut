import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';

/**
 * process-page reads one source HTML page from stdin and writes rendered HTML to stdout.
 *
 * Substitution spec:
 * - The substitution file is a JSON object whose keys are required placeholders like "{{ appScript }}".
 * - Each value is [distBaseName, extension], for example ["app", ".mjs"].
 * - Each placeholder resolves to exactly one hashed dist asset named "<distBaseName>.<hash><extension>".
 * - Unhashed dist assets are ignored.
 * - Rendered HTML must not contain unresolved "{{ name }}" placeholders or "/build/" asset references.
 */

import {
  readFile,
  readdir,
} from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

type PageInputs = {
  readonly substitutionsPath: string;
  readonly distPath: string;
};

type AssetSubstitution = readonly [distBaseName: string, extension: string];
type Substitutions = Readonly<Record<string, AssetSubstitution>>;

const projectRoot = fileURLToPath(new URL('../', import.meta.url));

const escapeRegExp = (value: string): string => value.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');

const distUrl = (filename: string): string => `/dist/${filename}`;

const findDistAsset = (distPath: string, distBaseName: string, extension: string): Promise<string> => {
  const pattern = new RegExp(
    `^${escapeRegExp(distBaseName)}\\.[^.]+${escapeRegExp(extension)}$`,
    'u',
  );

  return readdir(distPath)
    .then(filenames => filenames
      .filter(filename => pattern.test(filename))
      .sort())
    .then(matches => {
      if (matches.length !== 1) {
        throw { message: `expected exactly one hashed ${distBaseName}.*${extension} asset, found ${matches.length}` };
      }

      return distUrl(matches[0]);
    });
};

const replaceRequiredPlaceholder = (
  html: string,
  placeholder: string,
  value: string,
): string => {
  if (!html.includes(placeholder)) {
    throw { message: `page template is missing ${placeholder}` };
  }

  return html.split(placeholder).join(value);
};

const renderTemplate = (
  html: string,
  substitutions: ReadonlyMap<string, string>,
): string => {
  const rendered = [...substitutions]
    .reduce((acc, [placeholder, value]) => replaceRequiredPlaceholder(acc, placeholder, value), html);
  const unresolved = rendered.match(/\{\{\s*[a-zA-Z][a-zA-Z0-9]*\s*\}\}/u);

  if (unresolved) {
    throw { message: `page template has unresolved placeholder ${unresolved[0]}` };
  }

  if (rendered.includes('/build/')) {
    throw { message: 'HTML pages must reference /dist assets, not /build assets' };
  }

  return rendered;
};

const isRecord = (value: unknown): value is Readonly<Record<string, unknown>> => (
  typeof value === 'object'
  && value !== null
  && !Array.isArray(value)
);

const isAssetSubstitution = (value: unknown): value is AssetSubstitution => (
  Array.isArray(value)
  && value.length === 2
  && value.every(part => typeof part === 'string' && part.length > 0)
);

const parseSubstitutions = (content: string, substitutionsPath: string): Substitutions => {
  const parsed = JSON.parse(content) as unknown;

  if (!isRecord(parsed)) {
    throw { message: `${substitutionsPath} must contain a JSON object` };
  }

  for (const [placeholder, substitution] of Object.entries(parsed)) {
    if (!/^\{\{\s*[a-zA-Z][a-zA-Z0-9]*\s*\}\}$/u.test(placeholder)) {
      throw { message: `${substitutionsPath} has invalid placeholder ${placeholder}` };
    }

    if (!isAssetSubstitution(substitution)) {
      throw { message: `${substitutionsPath} has invalid substitution for ${placeholder}` };
    }

    const [, extension] = substitution;
    if (!extension.startsWith('.')) {
      throw { message: `${substitutionsPath} substitution for ${placeholder} must use an extension starting with "."` };
    }
  }

  return parsed as Substitutions;
};

const resolveSubstitutions = (
  substitutions: Substitutions,
  distPath: string,
): Promise<ReadonlyMap<string, string>> => Promise.all(Object.entries(substitutions)
  .map(([placeholder, substitution]) => findDistAsset(distPath, ...substitution)
    .then(asset => [
      placeholder,
      asset,
    ] as const)))
  .then(resolved => new Map(resolved));

const parsePageInputs = (): PageInputs => {
  const [substitutionsPath, distPath, ...rest] = process.argv.slice(2);

  if (!substitutionsPath || !distPath || rest.length > 0) {
    throw { message: 'usage: node build-src/process-page.ts <substitutions-json> <dist-dir> < <source-html>' };
  }

  return {
    substitutionsPath: resolve(projectRoot, substitutionsPath),
    distPath: resolve(projectRoot, distPath),
  };
};

const readStdin = (): Promise<string> => new Promise((resolve, reject) => {
  process.stdin.setEncoding('utf8');

  let content = '';

  process.stdin.on('data', chunk => {
    content += chunk;
  });
  process.stdin.on('end', () => {
    resolve(content);
  });
  process.stdin.on('error', reject);
});

const renderPage = ({ substitutionsPath, distPath }: PageInputs): Promise<string> => Promise.all([
  readStdin(),
  readFile(substitutionsPath, 'utf8'),
])
  .then(([html, substitutionsContent]) => resolveSubstitutions(
    parseSubstitutions(substitutionsContent, substitutionsPath),
    distPath,
  ).then(substitutions => renderTemplate(html, substitutions)));

const main = (): Promise<void> => renderPage(parsePageInputs())
  .then(html => {
    process.stdout.write(html);
  });

const inTest = process.env.NODE_TEST_CONTEXT !== undefined;

if (import.meta.main && !inTest) {
  main();
}

if (inTest) {
  const testTempRoot = resolve(projectRoot, 'build-test-temp/process-page');

  const assertThrowsMessage = (fn: () => unknown, message: string): void => {
    assert.throws(fn, (error: unknown) => (
      isRecord(error)
      && error.message === message
    ));
  };

  const assertRejectsMessage = (fn: () => Promise<unknown>, message: string): Promise<void> => assert.rejects(
    fn,
    (error: unknown) => (
      isRecord(error)
      && error.message === message
    ),
  );

  const withTempDir = (fn: (dir: string) => Promise<void>): Promise<void> => mkdir(testTempRoot, { recursive: true })
    .then(() => mkdtemp(join(testTempRoot, 'case-')))
    .then(dir => fn(dir)
      .finally(() => rm(dir, { recursive: true, force: true })));

  const writeEmptyFiles = (dir: string, filenames: readonly string[]): Promise<void> =>
    Promise.all(filenames.map(filename => writeFile(join(dir, filename), '')))
      .then(() => undefined);

  test('renderTemplate replaces required placeholders', () => {
    assert.equal(
      renderTemplate(
        '<script type="module" src="{{ appScript }}"></script><link href="{{ appStyle }}" rel="stylesheet" />',
        new Map([
          ['{{ appScript }}', '/dist/app.abc123.mjs'],
          ['{{ appStyle }}', '/dist/uno.def456.css'],
        ]),
      ),
      '<script type="module" src="/dist/app.abc123.mjs"></script><link href="/dist/uno.def456.css" rel="stylesheet" />',
    );
  });

  test('renderTemplate rejects missing required placeholders', () => {
    assertThrowsMessage(
      () => renderTemplate('<main></main>', new Map([['{{ appScript }}', '/dist/app.abc123.mjs']])),
      'page template is missing {{ appScript }}',
    );
  });

  test('renderTemplate rejects unresolved placeholders', () => {
    assertThrowsMessage(
      () => renderTemplate('<main>{{ pageTitle }}</main>', new Map()),
      'page template has unresolved placeholder {{ pageTitle }}',
    );
  });

  test('renderTemplate rejects build asset references', () => {
    assertThrowsMessage(
      () => renderTemplate('<script src="/build/app.mjs"></script>', new Map()),
      'HTML pages must reference /dist assets, not /build assets',
    );
  });

  test('parseSubstitutions accepts valid asset substitutions', () => {
    assert.deepEqual(
      parseSubstitutions('{ "{{ appScript }}": ["app", ".mjs"], "{{ appStyle }}": ["uno", ".css"] }', 'sub.json'),
      {
        '{{ appScript }}': ['app', '.mjs'],
        '{{ appStyle }}': ['uno', '.css'],
      },
    );
  });

  test('parseSubstitutions rejects invalid placeholders', () => {
    assertThrowsMessage(
      () => parseSubstitutions('{ "appScript": ["app", ".mjs"] }', 'sub.json'),
      'sub.json has invalid placeholder appScript',
    );
  });

  test('parseSubstitutions rejects invalid asset substitutions', () => {
    assertThrowsMessage(
      () => parseSubstitutions('{ "{{ appScript }}": ["app"] }', 'sub.json'),
      'sub.json has invalid substitution for {{ appScript }}',
    );
  });

  test('parseSubstitutions rejects extensions without a leading dot', () => {
    assertThrowsMessage(
      () => parseSubstitutions('{ "{{ appScript }}": ["app", "mjs"] }', 'sub.json'),
      'sub.json substitution for {{ appScript }} must use an extension starting with "."',
    );
  });

  test('findDistAsset resolves one hashed dist asset', () => withTempDir(
    dir => writeEmptyFiles(dir, [
      'app.abc123.mjs',
      'app.mjs',
      'story.abc123.mjs',
    ])
      .then(() => findDistAsset(dir, 'app', '.mjs'))
      .then(asset => {
        assert.equal(asset, '/dist/app.abc123.mjs');
      })
  ));

  test('findDistAsset ignores unhashed dist assets', () => withTempDir(
    dir => writeFile(join(dir, 'app.mjs'), '')
      .then(() => assertRejectsMessage(
        () => findDistAsset(dir, 'app', '.mjs'),
        'expected exactly one hashed app.*.mjs asset, found 0',
      ))
  ));

  test('findDistAsset rejects multiple hashed dist assets', () => withTempDir(
    dir => writeEmptyFiles(dir, [
      'app.abc123.mjs',
      'app.def456.mjs',
    ])
      .then(() => assertRejectsMessage(
        () => findDistAsset(dir, 'app', '.mjs'),
        'expected exactly one hashed app.*.mjs asset, found 2',
      ))
  ));
}
