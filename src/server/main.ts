import console from 'node:console';
import process from 'node:process';
import { readFile } from 'node:fs/promises';
import { webcrypto } from 'node:crypto';

import type { WSContext } from 'hono/ws';
import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import { createNodeWebSocket } from '@hono/node-ws';

import { spawn } from 'node-pty';

import { isNaturalNumber } from '../shared/natural-number.ts';
import { ttlSet } from './ttl-set.ts';


const app = new Hono();

app.notFound(({ text }) => text('404 Not Found', 404));

app.onError((err, { text }) => {
  console.error(err);
  return text('500 Internal Server Error', 500);
});

// '/public' 은 숨김
app.use('/favicon.ico', serveStatic({ root: './public' }));
app.use('/manifest.json', serveStatic({ root: './public' }));
app.use('/images/*', serveStatic({ root: './public' }));
app.use('/fonts/*', serveStatic({ root: './public' }));
app.use('/app/*', serveStatic({ root: './public' }));
// '/build' 는 안 숨김
app.use('/build/*', serveStatic({ root: './' }));

const nonceSet = ttlSet({
  ttl: 3_000,
  afterGone: str => console.debug(str + ' gone'),
  afterDelete: str => console.debug(str + ' deleted'),
});

app.post('/api/nonce', ({ text }) => {
  const nonce = Buffer.from(webcrypto.getRandomValues(new Uint8Array(8))).toString('base64');

  nonceSet.add(nonce);

  return text(nonce);
});

const p2d = (p: string) => {
  const arr = p.trim().split('\n');
  arr.pop();
  arr.shift();
  return Buffer.from(arr.join(''), 'base64');
};

if (!process.env.HOME) {
  throw new Error(`\$HOME cannot be empty`);
}

const pub = readFile(process.env.HOME + '/.config/commut/authorized.pub.pem')
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
  .then(pem => webcrypto.subtle.importKey('spki', p2d(pem), { name: 'ECDSA', namedCurve: 'P-256' }, false, ['verify']));

const checkpoints = new Map<string, CryptoKey>();

app.post('/api/ticket', ({ req, text }) => Promise.all([req.text().then(str => str.split('.')), pub])
  .then(([[nonce, sig], key]) => Promise.all([
    webcrypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']),
    nonceSet.has(nonce)
      ? webcrypto.subtle.verify({ name: 'ECDSA', hash: 'SHA-256' }, key, Buffer.from(sig, 'base64'), Buffer.from(nonce, 'base64'))
        .then(isValid => {
          if (isValid) nonceSet.delete(nonce);
          else throw { message: 'wrong signature' };
        })
      : Promise.reject({ message: 'nonce not found' }),
  ]))
  .then(([{ privateKey, publicKey }]) => webcrypto.subtle.exportKey('spki', publicKey).then(key => {
    const id = Buffer.from(webcrypto.getRandomValues(new Uint8Array(8))).toString('base64');
    checkpoints.set(id, privateKey);
    console.debug('checkpoints:', checkpoints);
    return text(id + '.' + Buffer.from(key).toString('base64'));
  }))
  .catch(({ message }) => text(message, 500)));

const te = new TextEncoder();
const td = new TextDecoder();

const UP: Readonly<Uint8Array<ArrayBufferLike>> = te.encode('client -> server');
const DN: Readonly<Uint8Array<ArrayBufferLike>> = te.encode('server -> client');

const crypts = new Map<string, {
  readonly en: (iv: BufferSource, data: BufferSource) => Promise<ArrayBuffer>;
  readonly de: (iv: BufferSource, data: BufferSource) => Promise<ArrayBuffer>;
}>();

app.post('/api/salt', ({ req, text }) => req.text()
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

const rand96 = () => webcrypto.getRandomValues(new Uint8Array(12));

const { injectWebSocket, upgradeWebSocket } = createNodeWebSocket({ app });

app.get('/sockets/:id', upgradeWebSocket(({ req }) => Promise.all([
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
    .then(() => spawn(`${process.env.HOME}/.nix-profile/bin/zsh`, [], {
      cols: (cols && rows) ? cols : 100,
      rows: (cols && rows) ? rows : 30,
      name: 'xterm-256color',
      cwd: process.env.HOME,
      env: {
        PATH: `${process.env.HOME}/.nix-profile/bin:/usr/bin`,
        TERM: 'xterm-256color',
        COLORTERM: 'truecolor',
        NODE_PTY: '1',
        USER: process.env.USER,
        XDG_RUNTIME_DIR: process.env.XDG_RUNTIME_DIR,
        DBUS_SESSION_BUS_ADDRESS: process.env.DBUS_SESSION_BUS_ADDRESS,
      },
    }))
    .then(pty => ({
      onOpen: (_, ws) => {
        pty.onData((() => {
          let mutex = Promise.resolve();

          return data => {
            const iv = rand96();

            mutex = Promise.all([en(iv, te.encode(data)), mutex])
              .then(([buf]) => new Uint8Array(buf))
              .then(src => {
                const cat = new Uint8Array(1 + 12 + src.length);
                cat[0] = 0;
                cat.set(iv, 1);
                cat.set(src, 13);
                return cat;
              })
              .then(cat => {
                if (WebSocket.OPEN === ws?.readyState) {
                  ws.send(cat);
                } else {
                  console.debug('could not send (reading from pty): no connection');
                }
              })
              .catch(reason => {
                console.error('[pty.onData]', reason);
              });
          };
        })());

        pty.onExit(({ exitCode, signal }) => {
          if (WebSocket.OPEN === ws?.readyState) {
            ws.close(4001, JSON.stringify({ exitCode, signal }));
          } else {
            console.debug('could not close (listening pty exit): no connection');
          }
        });
      },
      onMessage: (() => {
        let mutex = Promise.resolve();

        const handle = [
          (cat: Uint8Array<ArrayBuffer>): void => {
            mutex = Promise.all([de(cat.subarray(1, 13), cat.subarray(13)), mutex])
              .then(([buf]) => new Uint8Array(buf))
              .then(data => pty.write(td.decode(data)))
              .catch(reason => {
                console.error('[ws.onMessage]', reason);
              });
          },
          (cat: Uint8Array<ArrayBuffer>, ws: WSContext<WebSocket>): void => {
            const str = td.decode(cat.subarray(1));
            const [cols, rows, never] = str.split(',').map(Number).filter(isNaturalNumber);

            if (rows === undefined || never !== undefined) {
              console.warn(`[ws.onMessage] Bad Resize: ${str}`);
              return;
            }

            pty.resize(cols, rows);

            if (WebSocket.OPEN === ws?.readyState) {
              ws.send(cat);
            } else {
              console.debug('could not send (reading from pty): no connection');
            }
          },
        ] as const;

        return ({ data }, ws) => {
          const cat = new Uint8Array(data as ArrayBuffer);

          const messageType = cat[0];

          (handle[messageType] ?? (() => {
            console.debug('[ws.onMessage] Unknown Message Type:', messageType);
          }))(cat, ws);
        };
      })(),
      onClose: ({ code, reason }) => {
        pty.kill('SIGKILL');

        if (code === 4000 || code === 4001) {
          console.debug('connection closed with code:', code);
        } else {
          console.info('connection closed unexpectedly:', code, reason);
        }
      },
    })))));

const server = serve({
  fetch: app.fetch,
  hostname: '127.0.0.1',
  port: 3000,
}, info => {
  console.info(`Server is running on http://${info.address}:${info.port}`);
});

injectWebSocket(server);

process.on('SIGINT', () => {
  server.close();
  process.exit(0);
});

process.on('SIGTERM', () => {
  server.close(err => {
    if (err) {
      console.error(err);
      process.exit(1);
    }
    process.exit(0);
  });
});
