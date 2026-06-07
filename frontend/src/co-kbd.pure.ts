export type CoKey = Readonly<{
  lower: string;
  upper: string;
  face?: string;
}>;

export type CoKeyRow = Readonly<{
  label: string;
  keys: readonly CoKey[];
}>;

const key = (lower: string, upper: string, face?: string): CoKey => ({ lower, upper, face });

/**
 * Rows mirror frontend/co-layout.txt. The visible lower key is emitted on tap;
 * the upper key for the same column is emitted on long press.
 */
export const CO_KEY_ROWS: readonly CoKeyRow[] = Object.freeze([
  {
    label: 'top',
    keys: Object.freeze([
      key('q', 'Q', 'Q'),
      key('w', 'W', 'W'),
      key('f', 'F', 'F'),
      key('p', 'P', 'P'),
      key('g', 'G', 'G'),
      key('j', 'J', 'J'),
      key('l', 'L', 'L'),
      key('u', 'U', 'U'),
      key('y', 'Y', 'Y'),
      key('o', 'O', 'O'),
    ]),
  },
  {
    label: 'home',
    keys: Object.freeze([
      key('a', 'A', 'A'),
      key('r', 'R', 'R'),
      key('s', 'S', 'S'),
      key('t', 'T', 'T'),
      key('d', 'D', 'D'),
      key('h', 'H', 'H'),
      key('n', 'N', 'N'),
      key('e', 'E', 'E'),
      key('i', 'I', 'I'),
    ]),
  },
  {
    label: 'bottom',
    keys: Object.freeze([
      key('/', '?'),
      key('z', 'Z', 'Z'),
      key('x', 'X', 'X'),
      key('c', 'C', 'C'),
      key('v', 'V', 'V'),
      key('b', 'B', 'B'),
      key('k', 'K', 'K'),
      key('m', 'M', 'M'),
      key(',', '<'),
      key('.', '>'),
    ]),
  },
]);
