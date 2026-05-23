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
    `^${escapeRegExp(distBaseName)}(?:\\.[^.]+)?${escapeRegExp(extension)}$`,
    'u',
  );

  return readdir(distPath)
    .then(filenames => filenames
      .filter(filename => pattern.test(filename))
      .sort())
    .then(matches => {
      if (matches.length !== 1) {
        throw { message: `expected exactly one ${distBaseName}${extension} asset, found ${matches.length}` };
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

renderPage(parsePageInputs())
  .then(html => {
    process.stdout.write(html);
  });
