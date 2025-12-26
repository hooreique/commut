import { inner } from './inner.comp.ts';


export const outer = ({ encpri, fetch, smallInit, onWidthChange }: {
  readonly encpri: string;
  readonly fetch: Fetch;
  readonly smallInit: () => boolean;
  readonly onWidthChange: (listen: (isSmall: boolean) => void) => void;
}): Readonly<HTMLDivElement> => {
  const it = document.createElement('div');

  it.replaceChildren(inner({
    encpri,
    fetch,
    smallInit,
    onWidthChange,
  }));

  return it;
};
