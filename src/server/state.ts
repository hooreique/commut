import console from 'node:console';
import process from 'node:process';
import { readFile } from 'node:fs/promises';
import { webcrypto } from 'node:crypto';

import type { Codec } from '../shared/codec.ts';
import type { TtlSet } from './ttl-set.ts';
import { ttlSet } from './ttl-set.ts';


const p2d = (p: string) => {
  const arr = p.trim().split('\n');
  arr.pop();
  arr.shift();
  return Buffer.from(arr.join(''), 'base64');
};

export const init = (): {
  nonceSet: TtlSet;
  pub: Promise<CryptoKey>;
  checkpoints: Map<string, CryptoKey>;
  crypts: Map<string, { readonly en: Codec; readonly de: Codec }>;
} => {
  if (!process.env.HOME) {
    throw new Error(`\$HOME cannot be empty`);
  }

  return {
    nonceSet: ttlSet({
      ttl: 3_000,
      afterGone: str => console.debug(str + ' gone'),
      afterDelete: str => console.debug(str + ' deleted'),
    }),
    pub: readFile(process.env.HOME + '/.config/commut/authorized.pub.pem')
      .catch(err => {
        if (err.code === 'ENOENT') {
          console.error(`
# Hint

\`\`\`sh
mkdir -p ~/.config/commut
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 \\
  -out       ~/.config/commut/authorized.pri.pem \\
  -outpubkey ~/.config/commut/authorized.pub.pem
\`\`\`
`);
        }

        throw err;
      })
      .then(buf => buf.toString())
      .then(pem => webcrypto.subtle.importKey('spki', p2d(pem), { name: 'ECDSA', namedCurve: 'P-256' }, false, ['verify'])),
    checkpoints: new Map<string, CryptoKey>(),
    crypts: new Map<string, { readonly en: Codec; readonly de: Codec }>(),
  };
};
