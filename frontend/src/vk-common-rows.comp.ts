import type { VirtualKbd, VirtualKbdPartial } from './virtual-kbd.pure.ts';
import { VK } from './virtual-kbd.pure.ts';
import type { VkLayoutContext } from './vk-layout.ts';


const LONG_PRESS_MS = 200;
const VK_GRID_COLUMNS = 20;
const VK_SHIFT_KEY_COLUMNS = 2;

type VkShiftKey = Readonly<{
  readonly upper: string;
  readonly lower: string;
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

const vkBtn = ({ vk, emitVk, emitVkPartial }: {
  readonly vk: VirtualKbd;
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

  const clearTimer = (): void => {
    if (timer === undefined) return;
    window.clearTimeout(timer);
    timer = undefined;
  };

  it.addEventListener('pointerdown', ev => {
    if (ev.pointerType === 'mouse' && ev.button !== 0) return;

    ev.preventDefault();
    emitted = false;
    it.setPointerCapture(ev.pointerId);

    timer = window.setTimeout(() => {
      timer = undefined;
      emitted = true;
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
  });

  it.addEventListener('pointercancel', clearTimer);
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

    it.addEventListener('click', () => emitFocusBtnClick());

    return it;
  })({ emitFocusBtnClick });

  const it = document.createElement('div');
  it.className = 'box-border flex w-full max-w-full min-w-0 flex-nowrap gap-2 overflow-x-auto justify-start justify-self-stretch' as Uno;

  it.replaceChildren(
    ...[VK.ESC, VK.LEFT, VK.DOWN, VK.UP, VK.RIGHT, VK.CR, VK.TAB, VK.DEL, VK.SPACE, VK.BS]
      .map(vk => vkBtn({ vk, emitVk, emitVkPartial })),
    focusBtnEl,
    ...[VK.CTRL, VK.META, VK.HOME, VK.PGDN, VK.PGUP, VK.END]
      .map(vk => vkBtn({ vk, emitVk, emitVkPartial })));

  return [
    it,
    ...VK_SHIFT_ROWS.map(vks => vkShiftRow({ vks, emitVk })),
  ];
};
