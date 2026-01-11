import console from 'node:console';

import { webcrypto } from 'node:crypto';

import { Hono } from 'hono';

import type { Codec } from './codec.ts';


const te = new TextEncoder();

const UP: Readonly<Uint8Array<ArrayBufferLike>> = te.encode('client -> server');
const DN: Readonly<Uint8Array<ArrayBufferLike>> = te.encode('server -> client');

export const createSalt = (
  checkpoints: Map<string, CryptoKey>,
  crypts: Map<string, { readonly en: Codec; readonly de: Codec }>,
): Hono => {
  const app = new Hono();

  app.post(({ req, text }) => req.text()
    .then(str => str.split('.'))
    .then(([id, pub]) => Promise.all([
      webcrypto.getRandomValues(new Uint8Array(32)),
      Promise.resolve(checkpoints.get(id) ?? Promise.reject({ message: 'checkpoint not found' }))
        .then(pri => webcrypto.subtle.importKey('spki', Buffer.from(pub, 'base64'), { name: 'ECDH', namedCurve: 'P-256' }, false, [])
          .then(pub => webcrypto.subtle.deriveBits({ name: 'ECDH', public: pub }, pri, 256)))
        .then(buf => webcrypto.subtle.importKey('raw', buf, 'HKDF', false, ['deriveKey'])),
    ])
      .then(([salt, key]) => Promise.all([
        webcrypto.subtle.deriveKey({ name: 'HKDF', hash: 'SHA-256', salt, info: UP }, key, { name: 'AES-GCM', length: 128 }, false, ['decrypt']),
        webcrypto.subtle.deriveKey({ name: 'HKDF', hash: 'SHA-256', salt, info: DN }, key, { name: 'AES-GCM', length: 128 }, false, ['encrypt']),
      ])
        .then(([upKey, dnKey]) => {
          crypts.set(id, {
            en: (iv, data) => webcrypto.subtle.encrypt({ name: 'AES-GCM', iv }, dnKey, data),
            de: (iv, data) => webcrypto.subtle.decrypt({ name: 'AES-GCM', iv }, upKey, data),
          });
          console.debug('crypts:', crypts);
          checkpoints.delete(id);
          console.debug('checkpoints:', checkpoints);
        })
        .then(() => text(id + '.' + Buffer.from(salt).toString('base64')))))
    .catch(({ message }) => text(message, 500)));

  return app;
};
