import { coVkLayout } from './co-vk-layout.comp.ts';
import { seVkLayout } from './se-vk-layout.comp.ts';
import { vkCommonRows } from './vk-common-rows.comp.ts';
import type { VkLayout, VkLayoutContext } from './vk-layout.ts';


const SE_LAYOUT: VkLayout = {
  id: 'se',
  label: 'se',
  renderDynamic: seVkLayout,
};

const CO_LAYOUT: VkLayout = {
  id: 'co',
  label: 'co',
  renderDynamic: coVkLayout,
};

const VK_LAYOUTS: readonly VkLayout[] = Object.freeze([CO_LAYOUT, SE_LAYOUT]);

export const virtualKbd = (context: VkLayoutContext): Readonly<HTMLDivElement> => {
  let activeLayout = CO_LAYOUT;

  const it = document.createElement('div');
  it.className = 'grid gap-4' as Uno;

  const vkLayoutNav = document.createElement('div');
  vkLayoutNav.className = 'flex justify-center gap-2' as Uno;

  const vkLayoutPanel = document.createElement('div');
  vkLayoutPanel.className = 'grid gap-2' as Uno;

  const vkLayoutDynamic = document.createElement('div');
  vkLayoutDynamic.className = 'grid gap-4' as Uno;

  const vkLayoutStatic = document.createElement('div');
  vkLayoutStatic.className = 'grid min-w-0 justify-self-center box-border gap-2 px-1 justify-items-stretch w-[min(360px,calc(100vw-1rem))] min-[960px]:w-[min(28rem,calc(100vw-1rem))]' as Uno;
  vkLayoutStatic.replaceChildren(...vkCommonRows(context));

  const renderDynamic = (): void => {
    vkLayoutDynamic.replaceChildren(activeLayout.renderDynamic(context));
  };

  const renderNav = (): void => {
    vkLayoutNav.replaceChildren(document.createTextNode('layout: '), ...VK_LAYOUTS.map(layout => {
      const active = layout.id === activeLayout.id;
      const layoutBtn = document.createElement('button');
      layoutBtn.type = 'button';
      layoutBtn.className = active
        ? 'inline-block p-0 border-none font-bold' as Uno
        : 'inline-block p-0 border-none cursor-pointer hover:underline' as Uno;

      const layoutNameEl = document.createElement('span');
      layoutNameEl.className = active ? 'italic' as Uno : '';
      layoutNameEl.innerText = layout.label;

      layoutBtn.replaceChildren(
        document.createTextNode('['),
        layoutNameEl,
        document.createTextNode(']'));
      layoutBtn.disabled = active;
      layoutBtn.setAttribute('aria-pressed', active ? 'true' : 'false');

      if (active) return layoutBtn;

      layoutBtn.addEventListener('click', () => {
        activeLayout = layout;
        renderNav();
        renderDynamic();
      });

      return layoutBtn;
    }));
  };

  renderNav();
  renderDynamic();

  vkLayoutPanel.replaceChildren(vkLayoutStatic, vkLayoutDynamic);

  it.replaceChildren(vkLayoutNav, vkLayoutPanel);

  return it;
};
