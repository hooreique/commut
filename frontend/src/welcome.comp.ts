export const welcomePanel = ({ smallInit, onWidthChange }: {
  readonly smallInit: () => boolean;
  readonly onWidthChange: (listen: (isSmall: boolean) => void) => void;
}): Readonly<HTMLDivElement> => {
  const small = 'rounded overflow-hidden bg-[#2A2F38] flex justify-center items-center w-[360px] h-[288px]' as Uno;
  const big = 'rounded overflow-hidden bg-[#2A2F38] flex justify-center items-center w-[900px] h-[540px]' as Uno;

  const it = document.createElement('div');
  it.className = smallInit() ? small : big;

  onWidthChange(isSmall => {
    it.className = isSmall ? small : big;
  });

  it.replaceChildren((() => {
    const it = document.createElement('nav');
    it.className = 'text-[#5A6477] text-center font-bold cursor-default' as Uno;

    const i1 = document.createElement('span');
    i1.className = 'text-[#828A9A] italic' as Uno;
    i1.innerText = 'Commut';
    const i2 = document.createElement('br');
    const i3 = document.createElement('span');
    i3.innerText = '-';
    const i4 = document.createElement('br');
    const i5 = document.createElement('a');
    i5.className = 'text-inherit decoration-none hover:underline' as Uno;
    i5.href = '/app/story.html';
    i5.innerText = 'Story';

    it.replaceChildren(i1, i2, i3, i4, i5);

    return it;
  })());

  return it;
};
