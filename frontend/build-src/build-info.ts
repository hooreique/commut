import assert from 'node:assert/strict';
import test from 'node:test';

/**
 * build-info writes metadata for the dist assets that the backend can serve or prefetch.
 *
 * Build info spec:
 * - The version comes from frontend package.json, falling back to "0.0.0".
 * - Only files whose names end with one of the requested asset extensions are included.
 * - Included files are converted to "/dist/<filename>" URLs and sorted.
 * - The digest is an 8-character sha256 base64url digest of the sorted asset URL list.
 */

import { createHash } from 'node:crypto';
import {
  readFile,
  readdir,
} from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

type BuildInfo = {
  readonly version: string;
  readonly digest: string;
  readonly assets: readonly string[];
};

type BuildInfoTarget = {
  readonly distDir: string;
  readonly assetExtensions: readonly string[];
};

const projectRoot = fileURLToPath(new URL('../', import.meta.url));
const packageJsonPath = join(projectRoot, 'package.json');

const digest = (content: string): string => createHash('sha256')
  .update(content)
  .digest('base64url')
  .slice(0, 8);

const distUrl = (filename: string): string => `/dist/${filename}`;

const buildInfoFromFilenames = (
  version: string,
  filenames: readonly string[],
  assetExtensions: readonly string[],
): BuildInfo => {
  const assets = filenames
    .filter(filename => assetExtensions.some(extension => filename.endsWith(extension)))
    .map(distUrl)
    .sort();

  return {
    version,
    digest: digest(assets.join('\n')),
    assets,
  };
};

const readPackageVersion = (): Promise<string> => readFile(packageJsonPath, 'utf8')
  .then(content => {
    const packageJson = JSON.parse(content) as { readonly version?: string };
    return packageJson.version ?? '0.0.0';
  });

const parseBuildInfoTarget = (): BuildInfoTarget => {
  const [distDir, ...assetExtensions] = process.argv.slice(2);

  if (
    !distDir
    || assetExtensions.length === 0
    || assetExtensions.some(extension => !extension.startsWith('.'))
  ) {
    throw { message: 'usage: node build-src/build-info.ts <dist-dir> <asset-extension>...' };
  }

  return {
    distDir: resolve(projectRoot, distDir),
    assetExtensions,
  };
};

const getBuildInfo = ({ distDir, assetExtensions }: BuildInfoTarget): Promise<BuildInfo> => Promise.all([
  readdir(distDir),
  readPackageVersion(),
])
  .then(([filenames, version]) => buildInfoFromFilenames(version, filenames, assetExtensions));

const printBuildInfo = (target: BuildInfoTarget): Promise<void> => getBuildInfo(target)
  .then(buildInfo => {
    process.stdout.write(`${JSON.stringify(buildInfo, null, 2)}\n`);
  });

const main = (): Promise<void> => printBuildInfo(parseBuildInfoTarget());

const inTest = process.env.NODE_TEST_CONTEXT !== undefined;

if (import.meta.main && !inTest) {
  main();
}

if (inTest) {
  test('buildInfoFromFilenames includes requested asset extensions as sorted dist URLs', () => {
    assert.deepEqual(
      buildInfoFromFilenames('1.2.3', [
        'uno.bbb.css',
        'app.zzz.mjs.map',
        'app.aaa.mjs',
        'story.html',
        'app.d.ts',
        'xterm.ccc.css',
      ], [
        '.mjs',
        '.css',
      ]),
      {
        version: '1.2.3',
        digest: digest([
          '/dist/app.aaa.mjs',
          '/dist/uno.bbb.css',
          '/dist/xterm.ccc.css',
        ].join('\n')),
        assets: [
          '/dist/app.aaa.mjs',
          '/dist/uno.bbb.css',
          '/dist/xterm.ccc.css',
        ],
      },
    );
  });

  test('buildInfoFromFilenames changes digest when the asset list changes', () => {
    const first = buildInfoFromFilenames('1.2.3', ['app.aaa.mjs'], ['.mjs']);
    const second = buildInfoFromFilenames('1.2.3', ['app.bbb.mjs'], ['.mjs']);

    assert.notEqual(first.digest, second.digest);
  });
}
