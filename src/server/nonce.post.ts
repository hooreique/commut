import { webcrypto } from 'node:crypto';

import { Hono } from 'hono';

import type { TtlSet } from './ttl-set.ts';


export const createNonce = (nonceSet: TtlSet): Hono => {
  const app = new Hono();

  app.post(({ text }) => {
    const nonce = Buffer.from(webcrypto.getRandomValues(new Uint8Array(8))).toString('base64');

    nonceSet.add(nonce);

    return text(nonce);
  });

  return app;
};
