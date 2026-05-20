import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';


const digestMetaPattern = /<meta\b(?=[^>]*\bname="digest")(?=[^>]*\bcontent="unavailable")[^>]*>/u;
const projectRoot = fileURLToPath(new URL('../', import.meta.url));
const packageJsonPath = join(projectRoot, 'package.json');

const digest = (content: string): string => createHash('sha256')
  .update(content)
  .digest('base64');

const readPackageVersion = (): Promise<string> => readFile(packageJsonPath, 'utf8')
  .then(content => {
    const packageJson = JSON.parse(content) as { readonly version?: string };
    return packageJson.version ?? '0.0.0';
  });

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

const embedDigest = (html: string, version: string): string => {
  if (!digestMetaPattern.test(html)) {
    throw { message: 'page template is missing <meta name="digest" content="unavailable" />' };
  }

  return html.replace(digestMetaPattern, `<meta name="digest" content="${version} ${digest(html)}" />`);
};

Promise.all([
  readStdin(),
  readPackageVersion(),
])
  .then(([html, version]) => {
    process.stdout.write(embedDigest(html, version));
  });
