import type { Codec } from './codec.pure.ts';
import type { Dimensions } from './natural-number.pure.ts';
import { isNaturalNumber } from './natural-number.pure.ts';


export type Incoming = {
  readonly lead: 0;
  readonly data: Uint8Array<ArrayBuffer>;
} | {
  readonly lead: 1;
  readonly dimensions: Dimensions;
} | {
  readonly lead: -1;
  readonly v: number;
};

const td = new TextDecoder();
const te = new TextEncoder();

const readers = (decrypt: Codec): ((cat: Uint8Array<ArrayBuffer>) => Promise<Incoming>)[] => [
  cat => decrypt(cat.subarray(1, 13), cat.subarray(13))
    .then(buf => new Uint8Array(buf))
    .then(data => ({ lead: 0, data })),
  cat => {
    const [cols, rows, never] = td.decode(cat.subarray(1))
      .split(',')
      .map(Number)
      .filter(isNaturalNumber);

    return (rows !== undefined && never === undefined)
      ?
      Promise.resolve({ lead: 1, dimensions: { cols, rows } })
      :
      Promise.resolve({ lead: 1, dimensions: { cols: 100, rows: 30 } as Dimensions });
  },
];

/** Reads protocol packets into terminal data, resize events, or unknown-lead markers. */
export const reader = (decrypt: Codec) => {
  const reads = readers(decrypt);

  return (data: ArrayBuffer): Promise<Incoming> => {
    const cat = new Uint8Array(data);
    const lead = cat[0];
    return (
      reads[lead]
      ??
      (() => Promise.resolve({ lead: -1, v: lead }))
    )(cat);
  };
};

/** Writes terminal data packets with an injected random 12-byte IV source. */
export const writer = (
  encrypt: Codec,
  randomBytes: (bytes: Uint8Array<ArrayBuffer>) => Uint8Array<ArrayBuffer>,
) => (data: Uint8Array<ArrayBuffer>): Promise<Uint8Array<ArrayBuffer>> => {
  const iv = randomBytes(new Uint8Array(12));

  return encrypt(iv, data)
    .then(buf => new Uint8Array(buf))
    .then(src => {
      const cat = new Uint8Array(1 + 12 + src.length);
      cat[0] = 0;
      cat.set(iv, 1);
      cat.set(src, 13);
      return cat;
    });
};

/** Writes a resize packet as lead 1 followed by "cols,rows" UTF-8 data. */
export const writeResize = ({ cols, rows }: Dimensions): Uint8Array<ArrayBuffer> => {
  const src = te.encode([cols, rows].join(','));
  const cat = new Uint8Array(1 + src.length);
  cat[0] = 1;
  cat.set(src, 1);
  return cat;
};
