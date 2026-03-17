declare const azFlag: unique symbol;

/** `[a-zA-Z]` 를 만족하는 문자열 */
export type Az = string & { [azFlag]: never };

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
  ESC: { v: '\x1b', label: 'Escape' },
  CR: { v: '\r', label: 'Return' },
  UP: { v: '\x1b[A', label: '↑' },
  DOWN: { v: '\x1b[B', label: '↓' },
  LEFT: { v: '\x1b[D', label: '←' },
  RIGHT: { v: '\x1b[C', label: '→' },
  TAB: { v: '\t', label: 'Tab' },
  HOME: { v: '\x1b[H', label: 'Home' },
  END: { v: '\x1b[F', label: 'End' },
  PGUP: { v: '\x1b[5~', label: 'PgUp' },
  PGDN: { v: '\x1b[6~', label: 'PgDn' },
  DEL: { v: '\x1b[3~', label: 'Delete' },
  BS: { v: '\x7f', label: 'Backspace' },
  CTRL: {
    v: suffix => String.fromCharCode(suffix.toUpperCase().charCodeAt(0) - 64),
    label: 'Ctrl',
  },
  META: {
    v: suffix => '\x1b' + suffix.toLowerCase(),
    label: 'Meta',
  },
};
