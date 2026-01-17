import process from 'node:process';

import type { IPty } from 'node-pty';
import { spawn as spn } from 'node-pty';

import type { Dimensions } from '../shared/natural-number.ts';


export const spawn = ({ cols, rows }: Dimensions): IPty => spn(
  `${process.env.HOME}/.nix-profile/bin/zsh`,
  [],
  {
    cols,
    rows,
    name: 'xterm-256color',
    cwd: process.env.HOME,
    env: {
      PATH: `${process.env.HOME}/.nix-profile/bin:/usr/bin`,
      TERM: 'xterm-256color',
      COLORTERM: 'truecolor',
      NODE_PTY: '1',
      USER: process.env.USER,
      XDG_RUNTIME_DIR: process.env.XDG_RUNTIME_DIR,
      DBUS_SESSION_BUS_ADDRESS: process.env.DBUS_SESSION_BUS_ADDRESS,
    },
  });
