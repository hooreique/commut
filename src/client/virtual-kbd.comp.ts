import type { VirtualKbd, VirtualKbdPartial } from './virtual-kbd.ts';
import { VK } from './virtual-kbd.ts';


const vkBtn = ({ vk, emitVk, emitVkPartial }: {
  readonly vk: VirtualKbd;
  readonly emitVk: (v: string) => void;
  readonly emitVkPartial: (partial: VirtualKbdPartial) => void;
}): Readonly<HTMLButtonElement> => {
  const it = document.createElement('button');
  it.className = 'inline-block min-w-[2em] px-2 py-1 rounded border border-gray-500 cursor-pointer hover:border-gray-400' as Uno;

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
    const ellEl = document.createElement('span');
    ellEl.innerText = ' + …';
    it.appendChild(ellEl);
  }

  return it;
};

const vkRow = ({ vks, emitVk, emitVkPartial }: {
  readonly vks: Readonly<VirtualKbd[]>;
  readonly emitVk: (v: string) => void;
  readonly emitVkPartial: (partial: VirtualKbdPartial) => void;
}): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');
  it.className = 'flex gap-4 justify-center' as Uno;

  it.replaceChildren(...vks.map(vk => vkBtn({ vk, emitVk, emitVkPartial })));

  return it;
};

export const virtualKbd = ({ emitVk, emitVkPartial, emitFocusBtnClick }: {
  readonly emitVk: (v: string) => void;
  readonly emitVkPartial: (partial: VirtualKbdPartial) => void;
  readonly emitFocusBtnClick: () => void;
}): Readonly<HTMLDivElement> => {
  const r1 = vkRow({
    vks: [VK.ESC, VK.LEFT, VK.DOWN, VK.UP, VK.RIGHT, VK.CR],
    emitVk,
    emitVkPartial,
  });
  const r2 = vkRow({
    vks: [VK.TAB, VK.HOME, VK.PGDN, VK.PGUP, VK.END],
    emitVk,
    emitVkPartial,
  });
  const r4 = vkRow({
    vks: [VK.CTRL, VK.META],
    emitVk,
    emitVkPartial,
  });

  const r3: Readonly<HTMLDivElement> = (({ emitFocusBtnClick }: {
    readonly emitFocusBtnClick: () => void;
  }) => {
    const focusBtnEl: Readonly<HTMLButtonElement> = (({ emitFocusBtnClick }: {
      readonly emitFocusBtnClick: () => void;
    }) => {
      const it = document.createElement('button');
      it.className = 'inline-block px-2 py-1 rounded border border-gray-500 cursor-pointer hover:border-gray-400 bg-gray-700' as Uno;
      it.innerText = 'Return Focus';

      it.addEventListener('click', () => emitFocusBtnClick());

      return it;
    })({ emitFocusBtnClick });

    const del = vkBtn({ vk: VK.DEL, emitVk, emitVkPartial });
    const bs = vkBtn({ vk: VK.BS, emitVk, emitVkPartial });

    const it = document.createElement('div');
    it.className = 'flex gap-4 justify-center' as Uno;

    it.replaceChildren(del, bs, focusBtnEl);

    return it;
  })({ emitFocusBtnClick });

  const it = document.createElement('div');
  it.className = 'grid gap-4' as Uno;

  it.replaceChildren(r1, r2, r3, r4);

  return it;
};
