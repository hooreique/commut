import { connectBtn } from './connect-btn.comp.ts';
import { disconnBtn } from './disconn-btn.comp.ts';


export const header = ({
  emitConnBtnClick,
  emitDisconnBtnClick,
  onBell,
  onHealthyChange,
}: {
  readonly emitConnBtnClick: () => void;
  readonly emitDisconnBtnClick: () => void;
  readonly onBell: (listen: () => void) => void;
  readonly onHealthyChange: (listen: (healthy: boolean) => void) => void;
}): Readonly<HTMLDivElement> => {
  const bellEl = document.createElement('div');
  bellEl.className = 'text-sm' as Uno;

  onBell((() => {
    let i = 0;
    return () => {
      i++;
      bellEl.innerText = '🚨';
      setTimeout(() => {
        i--;
        if (i === 0) bellEl.innerText = '';
      }, 1_000);
    };
  })());

  const healthIndEl = document.createElement('div');
  healthIndEl.className = 'w-4 h-4 rounded-full bg-[#FF6578]' as Uno;

  const connectBtnEl = connectBtn({ emitConnBtnClick });
  const disconnBtnEl = disconnBtn({ emitDisconnBtnClick });

  const it = document.createElement('div');
  it.className = 'flex justify-between items-center' as Uno;

  onHealthyChange(healthy => {
    it.replaceChildren(
      healthy ? disconnBtnEl : connectBtnEl,
      bellEl,
      healthIndEl);

    healthIndEl.className = healthy
      ?
      'w-4 h-4 rounded-full bg-[#9DD274]' as Uno
      :
      'w-4 h-4 rounded-full bg-[#FF6578]' as Uno;

    if (!healthy) connectBtnEl.focus();
  });

  it.replaceChildren(
    connectBtnEl,
    bellEl,
    healthIndEl);

  return it;
};
