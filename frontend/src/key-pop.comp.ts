const POP_HIDE_MS = 120;

export const showKeyPop = (source: HTMLElement, label: string): (() => void) => {
  let closed = false;
  const it = document.createElement('div');
  it.className = 'fixed z-50 pointer-events-none grid min-w-12 h-14 -translate-x-1/2 -translate-y-full place-items-center rounded bg-[#E1E3E4] px-3 text-2xl font-bold text-[#24272E] shadow-xl' as Uno;
  it.innerText = label;

  const place = (): void => {
    const rect = source.getBoundingClientRect();
    it.style.left = `${rect.left + rect.width / 2}px`;
    it.style.top = `${rect.top - 8}px`;
  };

  document.body.appendChild(it);
  place();

  const onScroll = (): void => place();
  window.addEventListener('scroll', onScroll, true);
  window.addEventListener('resize', onScroll);

  return () => {
    if (closed) return;
    closed = true;
    window.removeEventListener('scroll', onScroll, true);
    window.removeEventListener('resize', onScroll);
    window.setTimeout(() => it.remove(), POP_HIDE_MS);
  };
};
