import type { VirtualKbd, VirtualKbdPartial } from './virtual-kbd.pure.ts';
import { showKeyPop } from './key-pop.comp.ts';
import { VK } from './virtual-kbd.pure.ts';
import type { VkLayoutContext } from './vk-layout.ts';


const LONG_PRESS_MS = 200;
const VK_GRID_COLUMNS = 20;
const VK_SHIFT_KEY_COLUMNS = 2;

type VkShiftKey = Readonly<{
  readonly upper: string;
  readonly lower: string;
}>;

type VkButtonKey = Readonly<{
  readonly vk: VirtualKbd;
  readonly popLabel: string;
}>;

const VK_SHIFT_ROWS: readonly (readonly VkShiftKey[])[] = Object.freeze([
  Object.freeze([
    { upper: '~', lower: '`' },
    { upper: '{', lower: '[' },
    { upper: '}', lower: ']' },
    { upper: '|', lower: '\\' },
    { upper: ':', lower: ';' },
    { upper: '"', lower: "'" },
    { upper: '<', lower: ',' },
    { upper: '>', lower: '.' },
    { upper: '?', lower: '/' },
  ]),
  Object.freeze([
    { upper: '!', lower: '1' },
    { upper: '@', lower: '2' },
    { upper: '#', lower: '3' },
    { upper: '$', lower: '4' },
    { upper: '%', lower: '5' },
    { upper: '^', lower: '6' },
    { upper: '&', lower: '7' },
    { upper: '*', lower: '8' },
    { upper: '(', lower: '9' },
    { upper: ')', lower: '0' },
  ]),
]);

const VK_LEADING_KEYS: readonly VkButtonKey[] = Object.freeze([
  { vk: VK.ESC, popLabel: 'Escape' },
  { vk: VK.LEFT, popLabel: 'Left' },
  { vk: VK.DOWN, popLabel: 'Down' },
  { vk: VK.UP, popLabel: 'Up' },
  { vk: VK.RIGHT, popLabel: 'Right' },
  { vk: VK.SPACE, popLabel: 'Space' },
  { vk: VK.CR, popLabel: 'Return' },
  { vk: VK.TAB, popLabel: 'Tab' },
  { vk: VK.BS, popLabel: 'Backspace' },
  { vk: VK.DEL, popLabel: 'Delete' },
]);

const VK_TRAILING_KEYS: readonly VkButtonKey[] = Object.freeze([
  { vk: VK.HOME, popLabel: 'Home' },
  { vk: VK.PGDN, popLabel: 'Page Down' },
  { vk: VK.PGUP, popLabel: 'Page Up' },
  { vk: VK.END, popLabel: 'End' },
]);

const bindPressPop = (it: HTMLButtonElement, label: string): void => {
  let closePop: (() => void) | undefined;

  const closeActivePop = (): void => {
    closePop?.();
    closePop = undefined;
  };

  it.addEventListener('pointerdown', ev => {
    if (ev.pointerType === 'mouse' && ev.button !== 0) return;

    closeActivePop();
    closePop = showKeyPop(it, label);
    it.setPointerCapture(ev.pointerId);
  });

  it.addEventListener('pointerup', ev => {
    if (it.hasPointerCapture(ev.pointerId)) {
      it.releasePointerCapture(ev.pointerId);
    }
    closeActivePop();
  });

  it.addEventListener('pointercancel', closeActivePop);
  it.addEventListener('lostpointercapture', closeActivePop);
};

const vkBtn = ({ vk, popLabel, emitVk, emitVkPartial }: {
  readonly vk: VirtualKbd;
  readonly popLabel?: string;
  readonly emitVk: (v: string) => void;
  readonly emitVkPartial: (partial: VirtualKbdPartial) => void;
}): Readonly<HTMLButtonElement> => {
  const it = document.createElement('button');
  it.type = 'button';
  it.className = 'inline-block w-9 shrink-0 whitespace-nowrap px-2 py-[0.2rem] rounded border-none bg-[#303641] text-[1.7rem] leading-none cursor-pointer hover:bg-[#3D4455]' as Uno;

  it.addEventListener('click', () => {
    if (typeof vk.v === 'function') {
      emitVkPartial(vk as VirtualKbdPartial);
    } else {
      emitVk(vk.v);
    }
  });

  const label = document.createElement('kbd');
  label.innerText = vk.label;

  it.replaceChildren(label);

  if (typeof vk.v === 'function') {
    it.title = `${vk.label} + …`;
  } else if (popLabel !== undefined) {
    bindPressPop(it, popLabel);
  }

  return it;
};

const vkShiftBtn = ({ vk, emitVk }: {
  readonly vk: VkShiftKey;
  readonly emitVk: (v: string) => void;
}): Readonly<HTMLButtonElement> => {
  const it = document.createElement('button');
  it.type = 'button';
  it.className = 'relative grid h-11 w-full max-w-9 min-w-0 touch-none select-none place-items-center justify-self-center overflow-hidden rounded border border-gray-500 bg-[#303641] p-0 text-center cursor-pointer hover:border-gray-400 active:bg-[#3D4455]' as Uno;

  const upperEl = document.createElement('span');
  upperEl.className = 'absolute top-1 text-[0.65rem] leading-none text-[#9DA6B8]' as Uno;
  upperEl.innerText = vk.upper;

  const lowerEl = document.createElement('kbd');
  lowerEl.className = 'pt-2 text-lg leading-none font-inherit' as Uno;
  lowerEl.innerText = vk.lower;

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

  const clearTimer = (): void => {
    if (timer === undefined) return;
    window.clearTimeout(timer);
    timer = undefined;
  };

  it.addEventListener('pointerdown', ev => {
    if (ev.pointerType === 'mouse' && ev.button !== 0) return;

    ev.preventDefault();
    emitted = false;
    showPop(vk.lower);
    it.setPointerCapture(ev.pointerId);

    timer = window.setTimeout(() => {
      timer = undefined;
      emitted = true;
      showPop(vk.upper);
      emitVk(vk.upper);
    }, LONG_PRESS_MS);
  });

  it.addEventListener('pointerup', ev => {
    ev.preventDefault();
    clearTimer();
    if (it.hasPointerCapture(ev.pointerId)) {
      it.releasePointerCapture(ev.pointerId);
    }

    if (!emitted) {
      emitVk(vk.lower);
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

const vkShiftRow = ({ vks, emitVk }: {
  readonly vks: readonly VkShiftKey[];
  readonly emitVk: (v: string) => void;
}): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');
  it.className = 'grid grid-cols-20 w-full min-w-0 gap-1 justify-items-center' as Uno;

  const startColumn = Math.floor((VK_GRID_COLUMNS - vks.length * VK_SHIFT_KEY_COLUMNS) / 2) + 1;
  it.replaceChildren(...vks.map((vk, index) => {
    const btn = vkShiftBtn({ vk, emitVk });
    btn.style.gridColumn = index === 0
      ? `${startColumn} / span ${VK_SHIFT_KEY_COLUMNS}`
      : `span ${VK_SHIFT_KEY_COLUMNS}`;

    return btn;
  }));

  return it;
};

export const vkCommonRows = ({ emitVk, emitVkPartial, emitFocusBtnClick }: VkLayoutContext): readonly Readonly<HTMLDivElement>[] => {
  const focusBtnEl: Readonly<HTMLButtonElement> = (({ emitFocusBtnClick }: {
    readonly emitFocusBtnClick: () => void;
  }) => {
    const it = document.createElement('button');
    it.type = 'button';
    it.className = 'inline-block w-9 shrink-0 whitespace-nowrap px-2 py-[0.2rem] rounded border-none bg-[#303641] text-[1.7rem] leading-none cursor-pointer hover:bg-[#3D4455]' as Uno;
    it.title = 'Return Focus';
    it.setAttribute('aria-label', 'Return Focus');
    it.innerText = '⌖';
    bindPressPop(it, 'Return Focus');

    it.addEventListener('click', () => emitFocusBtnClick());

    return it;
  })({ emitFocusBtnClick });

  const it = document.createElement('div');
  it.className = 'box-border flex w-full max-w-full min-w-0 flex-nowrap gap-2 overflow-x-auto justify-start justify-self-stretch' as Uno;

  it.replaceChildren(
    ...VK_LEADING_KEYS
      .map(({ vk, popLabel }) => vkBtn({ vk, popLabel, emitVk, emitVkPartial })),
    focusBtnEl,
    ...[VK.CTRL, VK.META]
      .map(vk => vkBtn({ vk, emitVk, emitVkPartial })),
    ...VK_TRAILING_KEYS
      .map(({ vk, popLabel }) => vkBtn({ vk, popLabel, emitVk, emitVkPartial })));

  return [
    it,
    ...VK_SHIFT_ROWS.map(vks => vkShiftRow({ vks, emitVk })),
  ];
};
