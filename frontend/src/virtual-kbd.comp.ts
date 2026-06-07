import { seVkLayout } from './se-vk-layout.comp.ts';
import { vkCommonRows } from './vk-common-rows.comp.ts';
import type { VkLayout, VkLayoutContext } from './vk-layout.ts';


const VK_LAYOUTS: readonly VkLayout[] = Object.freeze([
  {
    id: 'se',
    label: 'se',
    renderDynamic: seVkLayout,
  },
]);

export const virtualKbd = (context: VkLayoutContext): Readonly<HTMLDivElement> => {
  const activeLayout = VK_LAYOUTS[0];

  const it = document.createElement('div');
  it.className = 'grid gap-4' as Uno;

  const vkLayoutNav = document.createElement('div');
  vkLayoutNav.className = 'flex justify-center gap-2' as Uno;

  vkLayoutNav.replaceChildren(...VK_LAYOUTS.map(layout => {
    const layoutBtn = document.createElement('button');
    layoutBtn.type = 'button';
    layoutBtn.className = layout.id === activeLayout.id
      ? 'inline-block p-0 border-none cursor-pointer hover:underline font-bold' as Uno
      : 'inline-block p-0 border-none cursor-pointer hover:underline' as Uno;
    layoutBtn.innerText = `[${layout.label}]`;

    return layoutBtn;
  }));

  const vkLayoutPanel = document.createElement('div');
  vkLayoutPanel.className = 'grid gap-2' as Uno;

  const vkLayoutDynamic = document.createElement('div');
  vkLayoutDynamic.className = 'grid gap-4' as Uno;
  vkLayoutDynamic.replaceChildren(activeLayout.renderDynamic(context));

  const vkLayoutStatic = document.createElement('div');
  vkLayoutStatic.className = 'grid min-w-0 justify-self-center box-border gap-2 px-1 justify-items-stretch w-[min(360px,calc(100vw-1rem))] min-[960px]:w-[min(28rem,calc(100vw-1rem))]' as Uno;
  vkLayoutStatic.replaceChildren(...vkCommonRows(context));

  vkLayoutPanel.replaceChildren(vkLayoutStatic, vkLayoutDynamic);

  it.replaceChildren(vkLayoutNav, vkLayoutPanel);

  return it;
};
