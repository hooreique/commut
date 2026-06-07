import { seKbd } from './se-kbd.comp.ts';
import type { VkLayoutContext } from './vk-layout.ts';


export const seVkLayout = ({ emitSe, emitSeFlush }: VkLayoutContext): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');
  it.className = 'grid gap-4' as Uno;

  it.replaceChildren(seKbd({
    emitSe,
    emitSeFlush,
  }));

  return it;
};
