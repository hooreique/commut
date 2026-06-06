import { Terminal } from '@xterm/xterm';
import { WebglAddon } from '@xterm/addon-webgl';
import { createSe, type Jamo as SeJamo } from 'libse';

import type { Dimensions, NaturalNumber } from './natural-number.pure.ts';

import { channel } from './channel.pure.ts';
import type { Commut } from './commut.pure.ts';
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
  onSeJamo,
  onSeSpace,
  emitTermFocusChange,
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
  readonly onSeJamo: (listen: (jamo: SeJamo) => void) => void;
  readonly onSeSpace: (listen: () => void) => void;
  readonly emitTermFocusChange: (focused: boolean) => void;
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

    let preedit = '';

    const emitInput = (str: string): void => {
      if (str.length === 0) return;
      emitSend(te.encode(str));
    };

    const commutPanel = document.createElement('div');
    commutPanel.className = 'relative rounded size-fit overflow-hidden' as Uno;

    const preeditEl = document.createElement('div');
    preeditEl.className = 'pointer-events-none absolute z-10 whitespace-pre rounded-sm bg-[#E1E3E420] px-0.5 text-[#E1E3E4] opacity-70 ring-1 ring-[#E1E3E440]' as Uno;
    preeditEl.hidden = true;

    const updatePreeditPosition = (): void => {
      if (preedit.length === 0) return;

      const screenEl =
        commutPanel.querySelector<HTMLElement>('.xterm-screen')
        ?? commutPanel.querySelector<HTMLElement>('.xterm-rows')
        ?? commutPanel;
      const panelRect = commutPanel.getBoundingClientRect();
      const screenRect = screenEl.getBoundingClientRect();
      const cellWidth = screenRect.width / term.cols;
      const cellHeight = screenRect.height / term.rows;
      const x = Math.min(term.buffer.active.cursorX, Math.max(term.cols - 1, 0));
      const y = Math.min(term.buffer.active.cursorY, Math.max(term.rows - 1, 0));
      const screenStyle = getComputedStyle(screenEl);

      preeditEl.style.left = `${screenRect.left - panelRect.left + x * cellWidth}px`;
      preeditEl.style.top = `${screenRect.top - panelRect.top + y * cellHeight}px`;
      preeditEl.style.minWidth = `${cellWidth}px`;
      preeditEl.style.height = `${cellHeight}px`;
      preeditEl.style.lineHeight = `${cellHeight}px`;
      preeditEl.style.fontFamily = screenStyle.fontFamily;
      preeditEl.style.fontSize = screenStyle.fontSize;
    };

    const setPreedit = (next: string): void => {
      preedit = next;
      preeditEl.innerText = next;
      preeditEl.hidden = next.length === 0;
      requestAnimationFrame(updatePreeditPosition);
    };

    const se = createSe(outbound => {
      if (outbound.flushed) {
        setPreedit('');
        emitInput(outbound.character);
        return;
      }

      setPreedit(outbound.preedit);
    });

    const flushSe = (): void => se.flush();

    const handleInput = (str: string): void => {
      if (str === '\x7f' && se.backspace()) {
        return;
      }

      flushSe();
      term.input(str);
    };

    term.onData(str => {
      if (str === '\x7f' && se.backspace()) return;
      flushSe();
      emitInput(str);
    });

    onReceive(data => {
      term.write(data, updatePreeditPosition);
    });

    onVk(handleInput);
    onVkComp(handleInput);
    onSeJamo(jamo => {
      se.inbound(jamo);
    });
    onSeSpace(() => {
      flushSe();
      emitInput(' ');
    });

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

    term.onCursorMove(updatePreeditPosition);
    term.onRender(() => updatePreeditPosition());
    term.onWriteParsed(updatePreeditPosition);

    onResizeReceive(({ cols, rows }) => {
      term.resize(cols, rows);
      updatePreeditPosition();
    });

    term.open(commutPanel);
    commutPanel.appendChild(preeditEl);

    const termTextarea = term.textarea;
    if (termTextarea !== undefined) {
      termTextarea.setAttribute('inputmode', 'url');
      termTextarea.setAttribute('enterkeyhint', 'enter');
      termTextarea.setAttribute('autocapitalize', 'none');
      termTextarea.setAttribute('autocomplete', 'off');
      termTextarea.setAttribute('autocorrect', 'off');
      termTextarea.spellcheck = false;
      termTextarea.addEventListener('focus', () => emitTermFocusChange(true));
      termTextarea.addEventListener('blur', () => emitTermFocusChange(false));
      emitTermFocusChange(document.activeElement === termTextarea);
    } else {
      emitTermFocusChange(false);
    }

    it.replaceChildren(commutPanel);

    term.focus();
    emitTermFocusChange(termTextarea !== undefined && document.activeElement === termTextarea);
  });

  it.replaceChildren(welcomePanel({
    smallInit,
    onWidthChange: onWidthWelc,
  }));

  return it;
};
