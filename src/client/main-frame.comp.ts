import { Terminal } from '@xterm/xterm';
import { WebglAddon } from '@xterm/addon-webgl';

import type { Dimensions, NaturalNumber } from '../shared/natural-number.ts';

import { channel } from './channel.ts';
import type { Commut } from './commut.ts';
import { welcomePanel } from './welcome.comp.ts';


const te = new TextEncoder();
const td = new TextDecoder();

export const mainFrame = ({
  smallInit,
  onCommutReady,
  emitBell,
  emitCopy,
  onWidthChange,
  emitResizeSend,
  onResizeReceive,
  onVk,
  onVkComp,
  onFocusBtnClick,
}: {
  readonly smallInit: () => boolean;
  readonly onCommutReady: (listen: (commut: Commut) => void) => void;
  readonly emitBell: () => void;
  readonly emitCopy: (text: string) => void;
  readonly onWidthChange: (listen: (isSmall: boolean) => void) => void;
  readonly emitResizeSend: (dimensions: Dimensions) => void;
  readonly onResizeReceive: (listen: (dimensions: Dimensions) => void) => void;
  readonly onVk: (listen: (v: string) => void) => void;
  readonly onVkComp: (listen: (v: string) => void) => void;
  readonly onFocusBtnClick: (listen: () => void) => void;
}): Readonly<HTMLDivElement> => {
  const { emit: emitWidthMain, on: onWidthMain } = channel<boolean>();
  const { emit: emitWidthWelc, on: onWidthWelc } = channel<boolean>();

  onWidthChange(isSmall => {
    emitWidthMain(isSmall);
    emitWidthWelc(isSmall);
  });

  const it = document.createElement('div');
  it.className = 'size-fit' as Uno;

  onCommutReady(({ emitSend, onReceive }) => {
    const small = smallInit();

    const term = new Terminal({
      cols: small ? 40 : 100,
      rows: small ? 16 : 30,
      macOptionIsMeta: true,
      scrollback: 0,
      fontFamily: 'Hack Nerd Font',
      theme: {
        foreground: '#E1E3E4',
        background: '#2A2F38',
        cursor: '#E1E3E4',
        cursorAccent: '#2A2F38',
        selectionBackground: '#3D4455',
        black: '#2A2F38',
        red: '#FF6578',
        green: '#9DD274',
        yellow: '#EACB64',
        blue: '#F69C5E',
        magenta: '#BA9CF3',
        cyan: '#72CCE8',
        white: '#E1E3E4',
        brightBlack: '#828A9A',
        brightRed: '#FF6578',
        brightGreen: '#9DD274',
        brightYellow: '#EACB64',
        brightBlue: '#F69C5E',
        brightMagenta: '#BA9CF3',
        brightCyan: '#72CCE8',
        brightWhite: '#E1E3E4',
      },
    });

    term.loadAddon(new WebglAddon());

    term.onData(str => {
      emitSend(te.encode(str));
    });

    onReceive(data => {
      term.write(data);
    });

    onVk(v => term.input(v));
    onVkComp(v => term.input(v));

    onFocusBtnClick(() => term.focus());

    term.onBell(() => {
      emitBell();
    });

    term.parser.registerOscHandler(52, (data: string) => {
      const [command, encoded] = data.split(';');
      if (!encoded || command !== 'c') return false;

      emitCopy(td.decode(Uint8Array.fromBase64(encoded)));
      return true;
    });

    onWidthMain(isSmall => {
      const cols = (isSmall ? 40 : 100) as NaturalNumber;
      const rows = (isSmall ? 16 : 30) as NaturalNumber;
      emitResizeSend({ cols, rows });
    });

    onResizeReceive(({ cols, rows }) => term.resize(cols, rows));

    const commutPanel = document.createElement('div');
    commutPanel.className = 'rounded size-fit overflow-hidden' as Uno;

    term.open(commutPanel);

    it.replaceChildren(commutPanel);

    term.focus();
  });

  it.replaceChildren(welcomePanel({
    smallInit,
    onWidthChange: onWidthWelc,
  }));

  return it;
};
