import type { Jamo } from 'libse';

import type { Dimensions } from './natural-number.pure.ts';

import { channel } from './channel.pure.ts';
import type { Commut } from './commut.pure.ts';
import type { VirtualKbdPartial } from './virtual-kbd.pure.ts';
import { header } from './header.comp.ts';
import { mainFrame } from './main-frame.comp.ts';
import { virtualKbd } from './virtual-kbd.comp.ts';


export const rack = ({
  emitConnBtnClick,
  emitDisconnBtnClick,
  onHealthyChange,
  smallInit,
  onCommutReady,
  emitCopy,
  onWidthChange,
  emitResizeSend,
  onResizeReceive,
  emitVkPartial,
  onVk: onVkComp,
}: {
  readonly emitConnBtnClick: () => void;
  readonly emitDisconnBtnClick: () => void;
  readonly onHealthyChange: (listen: (healthy: boolean) => void) => void;
  readonly smallInit: () => boolean;
  readonly onCommutReady: (listen: (commut: Commut) => void) => void;
  readonly emitCopy: (text: string) => void;
  readonly onWidthChange: (listen: (isSmall: boolean) => void) => void;
  readonly emitResizeSend: (dimensions: Dimensions) => void;
  readonly onResizeReceive: (listen: (dimensions: Dimensions) => void) => void;
  readonly emitVkPartial: (partial: VirtualKbdPartial) => void;
  readonly onVk: (listen: (v: string) => void) => void;
}): Readonly<HTMLDivElement> => {
  const { emit: emitBell, on: onBell } = channel<void>();
  const { emit: emitVk, on: onVk } = channel<string>();
  const { emit: emitSe, on: onSe } = channel<Jamo>();
  const { emit: emitSeSpace, on: onSeSpace } = channel<void>();
  const { emit: emitFocusBtnClick, on: onFocusBtnClick } = channel<void>();
  const { emit: emitHealthyHead, on: onHealthyHead } = channel<boolean>();

  const it = document.createElement('div');
  it.className = 'size-fit py-4 grid gap-4' as Uno;

  const vkEl = virtualKbd({
    emitVk,
    emitVkPartial,
    emitSe,
    emitSeSpace,
    emitFocusBtnClick,
  });

  onHealthyChange(healthy => {
    emitHealthyHead(healthy);

    if (healthy) {
      it.appendChild(vkEl);
    } else {
      it.removeChild(vkEl);
    }
  });

  it.replaceChildren(
    header({
      emitConnBtnClick,
      emitDisconnBtnClick,
      onBell,
      onHealthyChange: onHealthyHead,
    }),
    mainFrame({
      smallInit,
      onCommutReady,
      emitBell,
      emitCopy,
      onWidthChange,
      emitResizeSend,
      onResizeReceive,
      onVk,
      onVkComp,
      onSe,
      onSeSpace,
      onFocusBtnClick,
    }));

  return it;
};
