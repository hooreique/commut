export const disconnBtn = ({ emitDisconnBtnClick }: {
  readonly emitDisconnBtnClick: () => void;
}): Readonly<HTMLButtonElement> => {
  const label = document.createElement('span');
  label.className = 'italic' as Uno;
  label.innerText = 'disconnect';

  const it = document.createElement('button');
  it.className = 'block p-0 cursor-pointer border-none hover:underline focus:outline-none' as Uno;

  it.replaceChildren('[', label, ']');

  it.addEventListener('click', () => emitDisconnBtnClick());

  return it;
};
