import console from 'node:console';

import { webcrypto } from 'node:crypto';

import { Hono } from 'hono';

import type { TtlSet } from './ttl-set.ts';


export const createTicket = (
  nonceSet: TtlSet,
  checkpoints: Map<string, CryptoKey>,
  pub: Promise<CryptoKey>
): Hono => {
  const app = new Hono();

  app.post(({ req, text }) => Promise.all([req.text().then(str => str.split('.')), pub])
    .then(([[nonce, sig], key]) => Promise.all([
      webcrypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']),
      nonceSet.has(nonce)
        ?
        webcrypto.subtle.verify({ name: 'ECDSA', hash: 'SHA-256' }, key, Buffer.from(sig, 'base64'), Buffer.from(nonce, 'base64'))
          .then(isValid => {
            if (isValid) nonceSet.delete(nonce);
            else throw { message: 'wrong signature' };
          })
        :
        Promise.reject({ message: 'nonce not found' }),
    ]))
    .then(([{ privateKey, publicKey }]) => webcrypto.subtle.exportKey('spki', publicKey).then(key => {
      const id = Buffer.from(webcrypto.getRandomValues(new Uint8Array(8))).toString('base64');
      checkpoints.set(id, privateKey);
      console.debug('checkpoints:', checkpoints);
      return text(id + '.' + Buffer.from(key).toString('base64'));
    }))
    .catch(({ message }) => text(message, 500)));

  return app;
};
