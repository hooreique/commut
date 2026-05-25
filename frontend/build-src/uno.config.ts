import assert from 'node:assert/strict';
import test from 'node:test';

/**
 * uno.config builds the generated utility CSS used by the frontend.
 *
 * Uno extraction spec:
 * - HTML files use UnoCSS' default split extractor.
 * - Plain TypeScript files are ignored by the default extractor.
 * - Component files ending in ".comp.ts" expose utility tokens through string literals marked with "as Uno".
 */

import { readFile } from 'node:fs/promises';
import { URL } from 'node:url';

import { defineConfig, extractorSplit, presetWind4 } from 'unocss';

const commutFont = "'Hack Nerd Font', Menlo, Consolas, 'DejaVu Sans Mono', monospace";

type UnoExtractContext = {
  readonly code: string;
  readonly id?: string;
};

const extractUnoTokens = ({ code, id }: UnoExtractContext): string[] | undefined => {
  if (!id?.endsWith('.comp.ts')) return;

  const res: string[] = [];
  const regex = /'([^']+)'\s+as\s+Uno\b/g;

  let match: RegExpExecArray | null;
  while ((match = regex.exec(code)) !== null) {
    match[1]
      .split(/\s+/)
      .filter(Boolean)
      .forEach(tok => res.push(tok));
  }

  return res;
};

export default defineConfig({
  presets: [
    presetWind4(),
  ],
  theme: {
    colors: {
      canvas: '#24272E',
    },
    font: {
      sans: commutFont,
      mono: commutFont,
    },
  },
  cli: {
    entry: [{
      patterns: ['src/*.html', 'src/*.comp.ts'],
      outFile: 'build-temp/uno.css',
    }],
  },
  extractorDefault: {
    name: 'Ignore .ts',
    extract: context => context.id?.endsWith('.ts')
      ?
      undefined
      :
      (extractorSplit.extract ?? (() => undefined))(context),
  },
  extractors: [{
    name: 'Uno in .comp.ts',
    extract: extractUnoTokens,
  }],
  preflights: [{
    getCSS: () => Promise.resolve(new URL('../src/fonts.css', import.meta.url))
      .then(url => readFile(url))
      .then(buf => buf.toString())
      .then(str => `\n/* src/fonts.css */\n${str}`),
  }],
});

const inTest = process.env.NODE_TEST_CONTEXT !== undefined;

if (inTest) {
  test('extractUnoTokens extracts utility tokens from component files', () => {
    assert.deepEqual(
      extractUnoTokens({
        code: [
          "const root = 'flex gap-2' as Uno;",
          "const label = 'text-sm font-bold' as Uno;",
        ].join('\n'),
        id: 'src/status.comp.ts',
      }),
      [
        'flex',
        'gap-2',
        'text-sm',
        'font-bold',
      ],
    );
  });

  test('extractUnoTokens ignores non-component TypeScript files', () => {
    assert.equal(
      extractUnoTokens({
        code: "const root = 'flex gap-2' as Uno;",
        id: 'src/status.ts',
      }),
      undefined,
    );
  });

  test('extractUnoTokens omits empty whitespace tokens', () => {
    assert.deepEqual(
      extractUnoTokens({
        code: "const root = '  flex   gap-2  ' as Uno;",
        id: 'src/status.comp.ts',
      }),
      [
        'flex',
        'gap-2',
      ],
    );
  });
}
