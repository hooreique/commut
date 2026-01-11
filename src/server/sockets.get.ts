import console from 'node:console';
import process from 'node:process';

import { webcrypto } from 'node:crypto';

import { Hono } from 'hono';
import type { WSContext } from 'hono/ws';
import type { NodeWebSocket } from '@hono/node-ws';

import { spawn } from 'node-pty';

import { isNaturalNumber } from '../shared/natural-number.ts';

import type { Codec } from './codec.ts';


const te = new TextEncoder();
const td = new TextDecoder();

const rand96 = () => webcrypto.getRandomValues(new Uint8Array(12));

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

  return app;
};
