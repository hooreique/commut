import { readFile } from 'node:fs/promises';
import { URL } from 'node:url';

import { defineConfig, extractorSplit, presetWind4 } from 'unocss';


const commutFont = "'Hack Nerd Font', Menlo, Consolas, 'DejaVu Sans Mono', monospace";

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
    extract: ({ code, id }) => {
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
    },
  }],
  preflights: [{
    getCSS: () => Promise.resolve(new URL('src/fonts.css', import.meta.url))
      .then(url => readFile(url))
      .then(buf => buf.toString())
      .then(str => `\n/* src/fonts.css */\n${str}`),
  }],
});
