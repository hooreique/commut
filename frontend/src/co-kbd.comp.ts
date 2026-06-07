import type { CoKey } from './co-kbd.pure.ts';
import { showKeyPop } from './key-pop.comp.ts';
import { CO_KEY_ROWS } from './co-kbd.pure.ts';


const LONG_PRESS_MS = 200;
const CO_KBD_GRID_COLUMNS = 20;
const CO_KBD_KEY_COLUMNS = 2;

const coBtn = ({ coKey, emitVk }: {
  readonly coKey: CoKey;
  readonly emitVk: (v: string) => void;
}): Readonly<HTMLButtonElement> => {
  const it = document.createElement('button');
  it.type = 'button';
  it.className = 'relative grid h-11 w-full max-w-9 min-w-0 touch-none select-none place-items-center justify-self-center overflow-hidden rounded border border-gray-500 bg-[#303641] p-0 text-center cursor-pointer hover:border-gray-400 active:bg-[#3D4455]' as Uno;

  if (coKey.face !== undefined) {
    const labelEl = document.createElement('kbd');
    labelEl.className = 'text-lg leading-none font-inherit' as Uno;
    labelEl.innerText = coKey.face;

    it.replaceChildren(labelEl);
  } else {
    const upperEl = document.createElement('span');
    upperEl.className = 'absolute top-1 text-[0.65rem] leading-none text-[#9DA6B8]' as Uno;
    upperEl.innerText = coKey.upper;

    const lowerEl = document.createElement('kbd');
    lowerEl.className = 'pt-2 text-lg leading-none font-inherit' as Uno;
    lowerEl.innerText = coKey.lower;

    it.replaceChildren(upperEl, lowerEl);
  }

  let timer: number | undefined;
  let emitted = false;
  let closePop: (() => void) | undefined;

  const closeActivePop = (): void => {
    closePop?.();
    closePop = undefined;
  };

  const showPop = (label: string): void => {
    closeActivePop();
    closePop = showKeyPop(it, label);
  };

  const clearTimer = (): void => {
    if (timer === undefined) return;
    window.clearTimeout(timer);
    timer = undefined;
  };

  it.addEventListener('pointerdown', ev => {
    if (ev.pointerType === 'mouse' && ev.button !== 0) return;

    ev.preventDefault();
    emitted = false;
    showPop(coKey.lower);
    it.setPointerCapture(ev.pointerId);

    timer = window.setTimeout(() => {
      timer = undefined;
      emitted = true;
      showPop(coKey.upper);
      emitVk(coKey.upper);
    }, LONG_PRESS_MS);
  });

  it.addEventListener('pointerup', ev => {
    ev.preventDefault();
    clearTimer();
    if (it.hasPointerCapture(ev.pointerId)) {
      it.releasePointerCapture(ev.pointerId);
    }

    if (!emitted) {
      emitVk(coKey.lower);
    }
    closeActivePop();
  });

  it.addEventListener('pointercancel', () => {
    clearTimer();
    closeActivePop();
  });
  it.addEventListener('lostpointercapture', clearTimer);

  return it;
};

const coRow = ({ keys, emitVk }: {
  readonly keys: readonly CoKey[];
  readonly emitVk: (v: string) => void;
}): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');
  it.className = 'grid grid-cols-20 w-full gap-1 justify-items-center' as Uno;

  const startColumn = Math.floor((CO_KBD_GRID_COLUMNS - keys.length * CO_KBD_KEY_COLUMNS) / 2) + 1;
  it.replaceChildren(...keys.map((coKey, index) => {
    const btn = coBtn({ coKey, emitVk });
    btn.style.gridColumn = index === 0
      ? `${startColumn} / span ${CO_KBD_KEY_COLUMNS}`
      : `span ${CO_KBD_KEY_COLUMNS}`;

    return btn;
  }));

  return it;
};

export const coKbd = ({ emitVk }: {
  readonly emitVk: (v: string) => void;
}): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');
  it.className = 'grid w-full max-w-[min(28rem,calc(100vw-1rem))] justify-self-center box-border gap-2 overflow-hidden px-1 pb-1 justify-items-stretch' as Uno;

  it.replaceChildren(...CO_KEY_ROWS.map(row => coRow({
    keys: row.keys,
    emitVk,
  })));

  return it;
};
