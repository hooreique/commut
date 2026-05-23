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

digestCss(parseCssTarget());
