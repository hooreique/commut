import console from 'node:console';
import process from 'node:process';

import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import { createNodeWebSocket } from '@hono/node-ws';

import { init } from './state.ts';
import { createNonce } from './nonce.post.ts';
import { createTicket } from './ticket.post.ts';
import { createSalt } from './salt.post.ts';
import { createSocket } from './sockets.get.ts';


const {
  nonceSet,
  checkpoints,
  pub,
  crypts,
} = init();

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

app.route('/api/nonce', createNonce(nonceSet));
app.route('/api/ticket', createTicket(nonceSet, checkpoints, pub));
app.route('/api/salt', createSalt(checkpoints, crypts));

const { injectWebSocket, upgradeWebSocket } = createNodeWebSocket({ app });

app.route('/sockets', createSocket(upgradeWebSocket, crypts, pub));

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
    } else {
      process.exit(0);
    }
  });
});
