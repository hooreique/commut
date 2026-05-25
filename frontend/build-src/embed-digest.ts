import assert from 'node:assert/strict';
import test from 'node:test';

/**
 * embed-digest reads one rendered HTML page from stdin and writes the same page with a digest meta tag.
 *
 * Digest spec:
 * - Exactly one unavailable digest meta tag must exist: <meta name="digest" content="unavailable" />.
 * - The name and content attributes may appear in either order.
 * - The unavailable digest meta tag is replaced with "<version> <sha256-base64-html-digest>".
 */

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

const main = (): Promise<void> => Promise.all([
  readStdin(),
  readPackageVersion(),
])
  .then(([html, version]) => {
    process.stdout.write(embedDigest(html, version));
  });

const inTest = process.env.NODE_TEST_CONTEXT !== undefined;

if (import.meta.main && !inTest) {
  main();
}

if (inTest) {
  const digestForTest = (content: string): string => createHash('sha256')
    .update(content)
    .digest('base64');

  const assertThrowsMessage = (fn: () => unknown, message: string): void => {
    assert.throws(fn, (error: unknown) => (
      typeof error === 'object'
      && error !== null
      && 'message' in error
      && error.message === message
    ));
  };

  test('embedDigest replaces unavailable digest meta with versioned digest', () => {
    const html = '<html><head><meta name="digest" content="unavailable" /></head></html>';

    assert.equal(
      embedDigest(html, '1.2.3'),
      `<html><head><meta name="digest" content="1.2.3 ${digestForTest(html)}" /></head></html>`,
    );
  });

  test('embedDigest accepts digest meta attributes in any order', () => {
    const html = '<html><head><meta content="unavailable" name="digest" /></head></html>';

    assert.equal(
      embedDigest(html, '1.2.3'),
      `<html><head><meta name="digest" content="1.2.3 ${digestForTest(html)}" /></head></html>`,
    );
  });

  test('embedDigest preserves unrelated meta tags', () => {
    const html = [
      '<html><head>',
      '<meta charset="utf-8" />',
      '<meta name="digest" content="unavailable" />',
      '<meta name="viewport" content="width=device-width" />',
      '</head></html>',
    ].join('');

    assert.equal(
      embedDigest(html, '1.2.3'),
      [
        '<html><head>',
        '<meta charset="utf-8" />',
        `<meta name="digest" content="1.2.3 ${digestForTest(html)}" />`,
        '<meta name="viewport" content="width=device-width" />',
        '</head></html>',
      ].join(''),
    );
  });

  test('embedDigest rejects pages without the unavailable digest meta', () => {
    assertThrowsMessage(
      () => embedDigest('<html><head></head></html>', '1.2.3'),
      'page template is missing <meta name="digest" content="unavailable" />',
    );
  });
}
