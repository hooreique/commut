import { fileURLToPath, URL } from 'node:url';

import { defineConfig } from 'tsdown'


export default defineConfig({
  entry: fileURLToPath(new URL('../src/app.ts', import.meta.url)),
  deps: { onlyBundle: ['@xterm/xterm', '@xterm/addon-webgl'] },
  tsconfig: fileURLToPath(new URL('../tsconfig.runtime.json', import.meta.url)),
  outDir: fileURLToPath(new URL('../dist', import.meta.url)),
  sourcemap: true,
  clean: false,
  hash: true,
  outputOptions: { entryFileNames: '[name].[hash].mjs' },
});
