import console from 'node:console';
import { webcrypto } from 'node:crypto';

import { Hono } from 'hono';
import type { NodeWebSocket } from '@hono/node-ws';

import type { Codec } from '../shared/codec.ts';
import type { Dimensions } from '../shared/natural-number.ts';
import { isNaturalNumber } from '../shared/natural-number.ts';

import { spawn } from './spawn.ts';
import { transport } from './transport.ts';


export const createSocket = (
  upgradeWebSocket: NodeWebSocket['upgradeWebSocket'],
  crypts: Map<string, { readonly en: Codec; readonly de: Codec }>,
  pub: Promise<CryptoKey>
): Hono => {
  const app = new Hono();

  app.get('/:id', upgradeWebSocket(({ req }) => Promise.all([
    req.param('id'),
    req.query('token') || Promise.reject({ message: 'token is required' }),
    (req.query('dimensions') || '100,30').split(',').map(Number).filter(isNaturalNumber),
    crypts.get(req.param('id')) ?? Promise.reject({ message: 'socket not found' }),
  ])
    .then(([id, sig, [cols, rows], { en, de }]) => pub
      .then(key => webcrypto.subtle.verify({ name: 'ECDSA', hash: 'SHA-256' }, key, Buffer.from(sig, 'base64'), Buffer.from(id, 'base64')))
      .then(isValid => {
        if (isValid) {
          crypts.delete(id);
          console.debug('crypts:', crypts);
        } else {
          throw { message: 'wrong signature' };
        }
      })
      .then(() => ({
        cols: (cols && rows) ? cols : 100,
        rows: (cols && rows) ? rows : 30,
      } as Dimensions))
      .then(spawn)
      .then(transport({ en, de })))));

  return app;
};
