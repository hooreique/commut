import { channel } from './channel.ts';
import { outer } from './outer.comp.ts';


if (
  typeof Uint8Array.fromBase64 !== 'function'
  ||
  typeof Uint8Array.prototype.toBase64 !== 'function'
  ||
  typeof Uint8Array.prototype.toHex !== 'function'
) {
  document.body.innerText = 'browser not supported';
  throw new Error('browser not supported');
}

const targetEl = document.body.querySelector<HTMLDivElement>('& > div#app');

if (!targetEl) {
  document.body.innerText = 'body > div#app not found';
  throw new Error('body > div#app not found');
}

const { emit: emitWidthChange, on: onWidthChange } = channel<boolean>();

const mql: MediaQueryList = matchMedia("(width < 960px)");

mql.addEventListener('change', ev => {
  emitWidthChange(ev.matches);
});

const outerEl: Readonly<HTMLDivElement> = outer({
  encpri: localStorage.getItem('encpri'),
  fetch: fetch,
  smallInit: () => mql.matches,
  onWidthChange,
});

targetEl.replaceChildren(outerEl);
