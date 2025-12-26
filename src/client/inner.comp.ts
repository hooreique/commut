import type { Dimensions } from '../shared/natural-number.ts';

import type { Conn } from './connect.ts';
import type { Commut } from './commut.ts';
import { channel } from './channel.ts';
import { enter } from './enter.ts';
import { connect } from './connect.ts';
import { withTrace } from './traceable-fetch.ts';
import { reader, writer, writeResize } from './protocol.ts';
import type { VirtualKbdPartial } from './virtual-kbd.ts';
import { toAz } from './virtual-kbd.ts';
import { rack } from './rack.comp.ts';


export const inner = ({ encpri, fetch, smallInit, onWidthChange }: {
  readonly encpri: string;
  readonly fetch: Fetch;
  readonly smallInit: () => boolean;
  readonly onWidthChange: (listen: (isSmall: boolean) => void) => void;
}): Readonly<HTMLDivElement> => {
  const { emit: emitConnBtnClick, on: onConnBtnClick } = channel<void>();
  const { emit: emitDisconnBtnClick, on: onDisconnBtnClick } = channel<void>();
  const { emit: emitWsOpen, on: onWsOpen } = channel<Conn>();
  const { emit: emitWsMsg, on: onWsMsg } = channel<ArrayBuffer>();
  const { emit: emitWsClose, on: onWsClose } = channel<void>();
  const { emit: emitCommutReady, on: onCommutReady } = channel<Commut>();
  const { emit: emitHealthyChange, on: onHealthyChange } = channel<boolean>();
  const { emit: emitCopy, on: onCopy } = channel<string>();
  const { emit: emitResizeSend, on: onResizeSend } = channel<Dimensions>();
  const { emit: emitResizeReceive, on: onResizeReceive } = channel<Dimensions>();
  const { emit: emitVkPartial, on: onVkPartial } = channel<VirtualKbdPartial>();
  const { emit: emitVk, on: onVk } = channel<string>();

  onWsOpen(({ send, close, encrypt, decrypt }) => {
    const read = reader(decrypt);
    const write = writer(encrypt);

    emitCommutReady({
      emitSend: (() => {
        let mutex = Promise.resolve();

        return data => {
          mutex = Promise.all([write(data), mutex])
            .then(([data]) => send(data))
            .catch(console.error);
        }
      })(),
      onReceive: consume => {
        let mutex = Promise.resolve();

        onWsMsg(data => {
          mutex = Promise.all([read(data), mutex])
            .then(([incoming]) => {
              if (incoming.lead === 0) {
                consume(incoming.data);
              } else if (incoming.lead === 1) {
                emitResizeReceive(incoming.dimensions);
              } else {
                console.debug('[ws.onMessage] lead:', incoming.v);
              }
            })
            .catch(console.error);
        });
      },
    });

    onResizeSend(dimensions => send(writeResize(dimensions)));

    onDisconnBtnClick(() => {
      close();
    });

    emitHealthyChange(true);
  });

  onWsClose(() => {
    emitHealthyChange(false);
  });

  const dialogEl = document.createElement('dialog');
  dialogEl.className = 'p-0 size-fit rounded border-none bg-[#424B5B] z-20 text-inherit' as Uno;

  onConnBtnClick(() => {
    const connModalEl = document.createElement('div');
    connModalEl.className = 'mx-4 my-3 text-2xl' as Uno;

    const inputEl = document.createElement('input');
    inputEl.name = 'code';
    inputEl.type = 'password';
    inputEl.autofocus = true;
    inputEl.autocomplete = 'off';
    inputEl.size = 6;
    inputEl.maxLength = 6;
    inputEl.className = 'w-40 border-none focus:outline-none' as Uno;

    inputEl.addEventListener('keydown', ev => {
      if (ev.key === 'Enter') {
        ev.preventDefault();
        const passphrase = (ev.currentTarget as HTMLInputElement).value;
        enter({ passphrase, encpri })
          .then(pri => connect({
            pri,
            fetch: withTrace(fetch),
            smallInit,
            emitWsOpen,
            emitWsMsg,
            emitWsClose,
          }))
          .catch(console.error)
          .finally(() => dialogEl.close());
      }
    });

    connModalEl.replaceChildren(inputEl);

    dialogEl.replaceChildren(connModalEl);

    dialogEl.showModal();
  });

  onCopy(text => {
    const copy = () => navigator.clipboard.writeText(text)
      .then(() => dialogEl.close())
      .catch(console.error);

    const copyModalEl = document.createElement('div');
    copyModalEl.className = 'm-4 grid gap-2' as Uno;

    const menuEl = document.createElement('div');
    menuEl.className = 'flex justify-between' as Uno;

    const closeBtnEl = document.createElement('button');
    closeBtnEl.className = 'cursor-pointer hover:underline p-0 border-none' as Uno;
    closeBtnEl.innerText = '[esc] to close';
    closeBtnEl.addEventListener('click', () => dialogEl.close());

    const copyBtnEl = document.createElement('button');
    copyBtnEl.className = 'cursor-pointer hover:underline p-0 border-none focus:outline-none' as Uno;
    copyBtnEl.autofocus = true;
    copyBtnEl.innerText = '[␣] to copy';
    copyBtnEl.addEventListener('click', copy);

    menuEl.replaceChildren(closeBtnEl, copyBtnEl);

    const previewEl = document.createElement('pre');
    previewEl.className = 'm-0 w-80 h-24 overflow-scroll font-inherit' as Uno;

    const textEl = document.createElement('code');
    textEl.className = 'font-inherit' as Uno;
    textEl.innerText = text.substring(0, 200);

    previewEl.replaceChildren(textEl);

    if (text.length > 200) {
      const ellEl = document.createElement('span');
      ellEl.className = 'opacity-35' as Uno;
      ellEl.innerText = '...';
      previewEl.appendChild(ellEl);
    }

    copyModalEl.replaceChildren(menuEl, previewEl);

    dialogEl.replaceChildren(copyModalEl);

    dialogEl.showModal();
  });

  onVkPartial(({ v, label }) => {
    const prefix = document.createElement('span');
    prefix.innerText = `${label} + `;

    const input = document.createElement('input');
    input.autofocus = true;
    input.type = 'text';
    input.maxLength = 1;
    input.pattern = '[a-zA-Z]';
    input.autocomplete = 'off';
    input.placeholder = 'Listening…';
    input.size = 10;
    input.className = 'p-0 border-none focus:outline-none' as Uno;

    input.addEventListener('input', ({ data }: InputEvent) => {
      if (!data) throw { message: '[VkModal] input data is empty' };

      emitVk(v(toAz(data)));

      dialogEl.close();
    });

    const vkModal = document.createElement('div');
    vkModal.className = 'm-4 w-xs text-center' as Uno;

    vkModal.replaceChildren(prefix, input);

    dialogEl.replaceChildren(vkModal);

    dialogEl.showModal();
  });

  const it = document.createElement('div');
  it.className = 'size-fit mx-auto' as Uno;

  it.replaceChildren(
    rack({
      emitConnBtnClick,
      emitDisconnBtnClick,
      onHealthyChange,
      smallInit,
      onCommutReady,
      emitCopy,
      onWidthChange,
      emitResizeSend,
      onResizeReceive,
      emitVkPartial,
      onVk,
    }),
    dialogEl);

  return it;
};
