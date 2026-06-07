declare const azFlag: unique symbol;

/** `[a-zA-Z]` 를 만족하는 문자열 */
export type Az = string & { [azFlag]: never };

/** Returns the input as Az when it is exactly one ASCII letter. */
export const toAz = (str: string): Az => {
  if (str.length !== 1) throw { message: `[${str}] is too long` };
  const charCode = str.toUpperCase().charCodeAt(0);
  if (charCode < 65 || charCode >= 91) throw { message: `[${str}] out of range` };
  return str as Az;
};

export type VirtualKbdPartial = {
  readonly v: (suffix: Az) => string;
  readonly label: string;
};

export type VirtualKbd = VirtualKbdPartial | {
  readonly v: string;
  readonly label: string;
};

export const VK: Readonly<Record<string, VirtualKbd>> = {
  ESC: { v: '\x1b', label: '⎋' },
  CR: { v: '\r', label: '⏎' },
  UP: { v: '\x1b[A', label: '↑' },
  DOWN: { v: '\x1b[B', label: '↓' },
  LEFT: { v: '\x1b[D', label: '←' },
  RIGHT: { v: '\x1b[C', label: '→' },
  TAB: { v: '\t', label: '⇥' },
  HOME: { v: '\x1b[H', label: '⇱' },
  END: { v: '\x1b[F', label: '⇲' },
  PGUP: { v: '\x1b[5~', label: '⇞' },
  PGDN: { v: '\x1b[6~', label: '⇟' },
  DEL: { v: '\x1b[3~', label: '⌦' },
  SPACE: { v: ' ', label: '␣' },
  BS: { v: '\x7f', label: '⌫' },
  CTRL: {
    v: suffix => String.fromCharCode(suffix.toUpperCase().charCodeAt(0) - 64),
    label: '⌃',
  },
  META: {
    v: suffix => '\x1b' + suffix.toLowerCase(),
    label: '⌥',
  },
};
