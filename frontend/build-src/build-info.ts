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
  .digest('hex')
  .slice(0, 8);

const distUrl = (filename: string): string => `/dist/${filename}`;

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
  readdir(distDir)
    .then(filenames => filenames
      .filter(filename => assetExtensions.some(extension => filename.endsWith(extension)))
      .map(distUrl)
      .sort()),
  readPackageVersion(),
])
  .then(([assets, version]) => ({
    version,
    digest: digest(assets.join('\n')),
    assets,
  }));

const printBuildInfo = (target: BuildInfoTarget): Promise<void> => getBuildInfo(target)
  .then(buildInfo => {
    process.stdout.write(`${JSON.stringify(buildInfo, null, 2)}\n`);
  });

printBuildInfo(parseBuildInfoTarget());
