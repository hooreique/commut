import type { Codec } from '../shared/codec.ts';


export type Conn = {
  readonly send: (data: Uint8Array<ArrayBuffer>) => void;
  readonly close: () => void;
  readonly encrypt: Codec;
  readonly decrypt: Codec;
};

const te = new TextEncoder();
const UP: Readonly<Uint8Array<ArrayBuffer>> = te.encode('client -> server');
const DN: Readonly<Uint8Array<ArrayBuffer>> = te.encode('server -> client');

export const connect = ({
  pri,
  fetch,
  smallInit,
  emitWsOpen,
  emitWsMsg,
  emitWsClose,
}: {
  readonly pri: ArrayBuffer;
  readonly fetch: Fetch;
  readonly smallInit: () => boolean;
  readonly emitWsOpen: (conn: Conn) => void;
  readonly emitWsMsg: (data: ArrayBuffer) => void;
  readonly emitWsClose: () => void;
}): Promise<void> => Promise.all([
  fetch('/api/nonce', { method: 'post' }).then(res => {
    if (res.ok) return res.text();
    else throw { message: 'response is not ok' };
  }),
  crypto.subtle.importKey('pkcs8', pri, { name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign']),
])
  .then(([nonce, privateKey]) => Promise.all([
    crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, privateKey, Uint8Array.fromBase64(nonce))
      .then(signature => new Uint8Array(signature).toBase64())
      .then(sig => fetch('/api/ticket', {
        method: 'post',
        body: nonce + '.' + sig,
      }).then(res => {
        if (res.ok) return res.text();
        else throw { message: 'response is not ok' };
      }))
      .then(ticket => ticket.split('.')),
    crypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']),
  ]))
  .then(([[id, pub], { privateKey, publicKey }]) => Promise.all([
    crypto.subtle.exportKey('spki', publicKey)
      .then(pub => fetch('/api/salt', {
        method: 'post',
        body: id + '.' + new Uint8Array(pub).toBase64(),
      }))
      .then(res => {
        if (res.ok) return res.text();
        else throw { message: 'response is not ok' };
      })
      .then(str => str.split('.'))
      .then(([id, salt]) => ({ id, salt: Uint8Array.fromBase64(salt) })),
    crypto.subtle.importKey('spki', Uint8Array.fromBase64(pub), { name: 'ECDH', namedCurve: 'P-256' }, false, [])
      .then(pub => crypto.subtle.deriveBits({ name: 'ECDH', public: pub }, privateKey, 256))
      .then(buf => crypto.subtle.importKey('raw', buf, 'HKDF', false, ['deriveKey'])),
  ]))
  .then(([{ id, salt }, key]) => Promise.all([
    crypto.subtle.deriveKey({ name: 'HKDF', hash: 'SHA-256', salt, info: UP }, key, { name: 'AES-GCM', length: 128 }, false, ['encrypt']),
    crypto.subtle.deriveKey({ name: 'HKDF', hash: 'SHA-256', salt, info: DN }, key, { name: 'AES-GCM', length: 128 }, false, ['decrypt']),
  ])
    .then(([upKey, dnKey]): [Codec, Codec] => [
      (iv: BufferSource, data: BufferSource) => crypto.subtle.encrypt({ name: 'AES-GCM', iv }, upKey, data),
      (iv: BufferSource, data: BufferSource) => crypto.subtle.decrypt({ name: 'AES-GCM', iv }, dnKey, data),
    ])
    .then(([encrypt, decrypt]) => crypto.subtle.importKey('pkcs8', pri, { name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign'])
      .then(privateKey => crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, privateKey, Uint8Array.fromBase64(id)))
      .then(signature => new Uint8Array(signature).toBase64())
      .then(sig => {
        const endpoint = new URL('/sockets/' + encodeURIComponent(id), location.origin);
        endpoint.searchParams.set('token', sig);
        endpoint.searchParams.set('dimensions', smallInit() ? '40,16' : '100,30');
        const ws = new WebSocket(endpoint);
        ws.binaryType = 'arraybuffer';
        return ws;
      })
      .then(ws => {
        console.debug('connection:', ws);

        ws.addEventListener('open', () => {
          emitWsOpen({
            send: data => ws.send(data),
            close: () => ws.close(4000),
            encrypt,
            decrypt,
          });
        });

        ws.addEventListener('message', ({ data }) => {
          emitWsMsg(data as ArrayBuffer);
        });

        ws.addEventListener('close', ({ code, reason }) => {
          emitWsClose();

          if (code === 4000) {
            console.debug('connection closed');
          } else if (code === 4001) {
            console.debug('connection closed by pty:', reason);
          } else {
            console.info('connection closed unexpectedly:', code, reason);
          }
        });
      })));
