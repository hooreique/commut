import assert from 'node:assert/strict';
import { mkdtemp } from 'node:fs/promises';
import test from 'node:test';

/**
 * digest-css copies one CSS asset from a build-temp directory into dist with a content digest in its filename.
 *
 * CSS digest spec:
 * - The source file is "<distBaseName>.css" under the provided source directory.
 * - The dist file is written as "<distBaseName>.<sha256-base64url-prefix>.css".
 * - Existing hashed CSS files for the same dist base name are removed before writing the new file.
 * - Dist files for other base names are preserved.
 */

import { createHash } from 'node:crypto';
import {
  mkdir,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

type CssTarget = {
  readonly distBaseName: string;
  readonly sourceDir: string;
  readonly distDir: string;
};

const projectRoot = fileURLToPath(new URL('../', import.meta.url));

const digest = (content: Buffer): string => createHash('sha256')
  .update(content)
  .digest('base64url')
  .slice(0, 8);

const escapeRegExp = (value: string): string => value.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');

const removeStaleDistFiles = (distBaseName: string, distDir: string): Promise<void> => {
  const stalePattern = new RegExp(`^${escapeRegExp(distBaseName)}\\.[^.]+\\.css$`, 'u');

  return readdir(distDir)
    .then(filenames => Promise.all(filenames
      .filter(filename => stalePattern.test(filename))
      .map(filename => rm(join(distDir, filename), { force: true }))))
    .then(() => undefined);
};

const digestCss = ({ distBaseName, sourceDir, distDir }: CssTarget): Promise<void> => readFile(join(sourceDir, `${distBaseName}.css`))
  .then(raw => {
    const distName = `${distBaseName}.${digest(raw)}.css`;

    return mkdir(distDir, { recursive: true })
      .then(() => removeStaleDistFiles(distBaseName, distDir))
      .then(() => writeFile(join(distDir, distName), raw));
  });

const parseCssTarget = (): CssTarget => {
  const [distBaseName, sourceDir, distDir, ...rest] = process.argv.slice(2);

  if (!distBaseName || !sourceDir || !distDir || rest.length > 0) {
    throw { message: 'usage: node build-src/digest-css.ts <dist-base-name> <source-dir> <dist-dir>' };
  }

  return {
    distBaseName,
    sourceDir: resolve(projectRoot, sourceDir),
    distDir: resolve(projectRoot, distDir),
  };
};

const main = (): Promise<void> => digestCss(parseCssTarget());

const inTest = process.env.NODE_TEST_CONTEXT !== undefined;

if (import.meta.main && !inTest) {
  main();
}

if (inTest) {
  const testTempRoot = resolve(projectRoot, 'build-test-temp/digest-css');

  const digestForTest = (content: string): string => createHash('sha256')
    .update(Buffer.from(content))
    .digest('base64url')
    .slice(0, 8);

  const withTempDir = (fn: (dir: string) => Promise<void>): Promise<void> => mkdir(testTempRoot, { recursive: true })
    .then(() => mkdtemp(join(testTempRoot, 'case-')))
    .then(dir => fn(dir)
      .finally(() => rm(dir, { recursive: true, force: true })));

  test('digestCss writes a hashed CSS asset and removes stale files for the same base name', () => withTempDir(
    dir => {
      const sourceDir = join(dir, 'source');
      const distDir = join(dir, 'dist');
      const content = 'body { color: red; }\n';
      const distName = `uno.${digestForTest(content)}.css`;

      return Promise.all([
        mkdir(sourceDir, { recursive: true }),
        mkdir(distDir, { recursive: true }),
      ])
        .then(() => Promise.all([
          writeFile(join(sourceDir, 'uno.css'), content),
          writeFile(join(distDir, 'uno.old.css'), 'old'),
          writeFile(join(distDir, 'xterm.old.css'), 'xterm'),
        ]))
        .then(() => digestCss({ distBaseName: 'uno', sourceDir, distDir }))
        .then(() => Promise.all([
          readdir(distDir).then(filenames => filenames.sort()),
          readFile(join(distDir, distName), 'utf8'),
        ]))
        .then(([filenames, distContent]) => {
          assert.deepEqual(filenames, [
            distName,
            'xterm.old.css',
          ]);
          assert.equal(distContent, content);
        });
    }
  ));
}
