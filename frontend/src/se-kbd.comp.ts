import { Jamo, type Jamo as SeJamo } from 'libse';

import type { SeJamoName, SeKey } from './se-kbd.pure.ts';
import { SE_KEY_ROWS, seKeyLabel } from './se-kbd.pure.ts';


const LONG_PRESS_MS = 200;
const POP_HIDE_MS = 120;
const SE_KBD_GRID_COLUMNS = 20;
const SE_KBD_KEY_COLUMNS = 2;

const toSeJamo = (name: SeJamoName): SeJamo => Jamo[name as keyof typeof Jamo] as SeJamo;

const showKeyPop = (source: HTMLElement, label: string): (() => void) => {
  let closed = false;
  const it = document.createElement('div');
  it.className = 'fixed z-50 pointer-events-none grid min-w-12 h-14 -translate-x-1/2 -translate-y-full place-items-center rounded bg-[#E1E3E4] px-3 text-2xl font-bold text-[#24272E] shadow-xl' as Uno;
  it.innerText = label;

  const place = (): void => {
    const rect = source.getBoundingClientRect();
    it.style.left = `${rect.left + rect.width / 2}px`;
    it.style.top = `${rect.top - 8}px`;
  };

  document.body.appendChild(it);
  place();

  const onScroll = (): void => place();
  window.addEventListener('scroll', onScroll, true);
  window.addEventListener('resize', onScroll);

  return () => {
    if (closed) return;
    closed = true;
    window.removeEventListener('scroll', onScroll, true);
    window.removeEventListener('resize', onScroll);
    window.setTimeout(() => it.remove(), POP_HIDE_MS);
  };
};

const longPressLabel = (seKey: SeKey): string =>
  seKey.upper === undefined
    ? (seKey.spaceOnLongPress === true ? '·' : '')
    : seKeyLabel(seKey.upper);

const seBtn = ({ seKey, emitSeJamo, emitSeSpace }: {
  readonly seKey: SeKey;
  readonly emitSeJamo: (jamo: SeJamo) => void;
  readonly emitSeSpace: () => void;
}): Readonly<HTMLButtonElement> => {
  const it = document.createElement('button');
  it.type = 'button';
  it.className = 'relative grid h-11 w-full max-w-9 min-w-0 touch-none select-none place-items-center justify-self-center overflow-hidden rounded border border-gray-500 bg-[#303641] p-0 text-center cursor-pointer hover:border-gray-400 active:bg-[#3D4455]' as Uno;

  const upperEl = document.createElement('span');
  upperEl.className = 'absolute top-1 text-[0.65rem] leading-none text-[#9DA6B8]' as Uno;
  upperEl.innerText = longPressLabel(seKey);

  const lowerEl = document.createElement('kbd');
  lowerEl.className = 'pt-2 text-lg leading-none font-inherit' as Uno;
  lowerEl.innerText = seKeyLabel(seKey.lower);

  it.replaceChildren(upperEl, lowerEl);

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

  const emit = (jamo: SeJamoName): void => {
    emitted = true;
    emitSeJamo(toSeJamo(jamo));
  };

  const space = (): void => {
    emitted = true;
    emitSeSpace();
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
    showPop(seKeyLabel(seKey.lower));
    it.setPointerCapture(ev.pointerId);

    const upper = seKey.upper;
    if (upper !== undefined || seKey.spaceOnLongPress === true) {
      timer = window.setTimeout(() => {
        timer = undefined;
        if (upper !== undefined) {
          showPop(seKeyLabel(upper));
          emit(upper);
          return;
        }

        showPop('Space');
        space();
      }, LONG_PRESS_MS);
    }
  });

  it.addEventListener('pointerup', ev => {
    ev.preventDefault();
    clearTimer();
    if (it.hasPointerCapture(ev.pointerId)) {
      it.releasePointerCapture(ev.pointerId);
    }

    if (!emitted) {
      emit(seKey.lower);
    }
    closeActivePop();
  });

  it.addEventListener('pointercancel', () => {
    clearTimer();
    closeActivePop();
  });

  it.addEventListener('lostpointercapture', () => {
    clearTimer();
  });

  return it;
};

const seRow = ({ keys, emitSeJamo, emitSeSpace }: {
  readonly keys: readonly SeKey[];
  readonly emitSeJamo: (jamo: SeJamo) => void;
  readonly emitSeSpace: () => void;
}): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');
  it.className = 'grid w-full gap-1 justify-items-center' as Uno;
  it.style.gridTemplateColumns = `repeat(${SE_KBD_GRID_COLUMNS}, minmax(0, 1fr))`;

  const startColumn = Math.floor((SE_KBD_GRID_COLUMNS - keys.length * SE_KBD_KEY_COLUMNS) / 2) + 1;
  it.replaceChildren(...keys.map((seKey, index) => {
    const btn = seBtn({
      seKey,
      emitSeJamo,
      emitSeSpace,
    });

    btn.style.gridColumn = index === 0
      ? `${startColumn} / span ${SE_KBD_KEY_COLUMNS}`
      : `span ${SE_KBD_KEY_COLUMNS}`;

    return btn;
  }));

  return it;
};

export const seKbd = ({ emitSeJamo, emitSeSpace, onTermFocusChange }: {
  readonly emitSeJamo: (jamo: SeJamo) => void;
  readonly emitSeSpace: () => void;
  readonly onTermFocusChange: (listen: (focused: boolean) => void) => void;
}): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');
  it.className = 'grid w-full justify-self-center box-border gap-2 overflow-hidden px-1 pb-1 justify-items-stretch' as Uno;
  it.style.maxWidth = 'min(28rem, calc(100vw - 1rem))';

  onTermFocusChange(focused => {
    it.hidden = focused;
  });

  it.replaceChildren(...SE_KEY_ROWS.map(row => seRow({
    keys: row.keys,
    emitSeJamo,
    emitSeSpace,
  })));

  return it;
};
