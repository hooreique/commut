import type { Jamo } from 'libse';

import type { VirtualKbdPartial } from './virtual-kbd.pure.ts';


export type VkLayoutId = 'se' | 'co';

export type VkLayoutContext = Readonly<{
  readonly emitVk: (v: string) => void;
  readonly emitVkPartial: (partial: VirtualKbdPartial) => void;
  readonly emitSe: (jamo: Jamo) => void;
  readonly emitSeFlush: () => void;
  readonly emitFocusBtnClick: () => void;
}>;

export type VkLayout = Readonly<{
  readonly id: VkLayoutId;
  readonly label: string;
  readonly renderDynamic: (context: VkLayoutContext) => Readonly<HTMLDivElement>;
}>;
