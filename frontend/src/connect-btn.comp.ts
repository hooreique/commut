export const connectBtn = ({ emitConnBtnClick }: {
  readonly emitConnBtnClick: () => void;
}): Readonly<HTMLButtonElement> => {
  const lead = document.createElement('span');
  lead.className = 'underline' as Uno;
  lead.innerText = 'c';

  const full = document.createElement('span');
  full.className = 'italic' as Uno;

  full.replaceChildren(lead, 'onnect');

  const it = document.createElement('button');
  it.className = 'block p-0 cursor-pointer border-none hover:underline focus:outline-none' as Uno;
  it.autofocus = true;

  it.replaceChildren('[', full, ']');

  it.addEventListener('click', () => emitConnBtnClick());
  it.addEventListener('keydown', ev => {
    if (ev.key === 'c') {
      ev.preventDefault();
      emitConnBtnClick();
    }
  })

  return it;
};
