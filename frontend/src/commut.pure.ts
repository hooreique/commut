export type Commut = {
  readonly emitSend: (data: Uint8Array<ArrayBuffer>) => void;
  readonly onReceive: (consume: (data: Uint8Array<ArrayBuffer>) => void) => void;
};
