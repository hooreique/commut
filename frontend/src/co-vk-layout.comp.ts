import { coKbd } from './co-kbd.comp.ts';
import type { VkLayoutContext } from './vk-layout.ts';


export const coVkLayout = ({ emitVk }: VkLayoutContext): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');
  it.className = 'grid gap-4' as Uno;

  it.replaceChildren(coKbd({
    emitVk,
  }));

  return it;
};
