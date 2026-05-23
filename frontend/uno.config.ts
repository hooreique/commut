import { readFile } from 'node:fs/promises';
import { URL } from 'node:url';

import { defineConfig, extractorSplit } from 'unocss';


export default defineConfig({
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
    getCSS: () => Promise.all([
      Promise.all([
        'node_modules/@unocss/reset/sanitize/sanitize.css',
        'node_modules/@unocss/reset/sanitize/forms.css',
        'node_modules/@unocss/reset/sanitize/assets.css',
        'node_modules/@unocss/reset/sanitize/typography.css',
        'node_modules/@unocss/reset/sanitize/reduce-motion.css',
        'node_modules/@unocss/reset/sanitize/system-ui.css',
        'node_modules/@unocss/reset/sanitize/ui-monospace.css',
      ]
        .map(name => Promise.resolve(new URL(name, import.meta.url))
          .then(url => readFile(url))
          .then(buf => buf.toString())
          .then(str => `/* ${name} */\n\n${str}`)))
        .then(arr => arr.join('\n'))
        .then(str => `\n${str}`),
      Promise.resolve(new URL('src/base.css', import.meta.url))
        .then(url => readFile(url))
        .then(buf => buf.toString())
        .then(str => `\n/* src/base.css */\n${str}`),
    ])
      .then(arr => arr.join('')),
  }],
});
