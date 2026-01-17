import console from 'node:console';
import { webcrypto } from 'node:crypto';

import type { WSContext, WSEvents } from 'hono/ws';

import type { IPty } from 'node-pty';

import type { Codec } from '../shared/codec.ts';
import { isNaturalNumber } from '../shared/natural-number.ts';


const te = new TextEncoder();
const td = new TextDecoder();

const rand96 = () => webcrypto.getRandomValues(new Uint8Array(12));

export const transport = ({ en, de }: {
  readonly en: Codec;
  readonly de: Codec;
}) => (pty: IPty): WSEvents<WebSocket> => ({
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
});
