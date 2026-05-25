import assert from 'node:assert/strict';
import test from 'node:test';

import { toAz } from '../src/virtual-kbd.pure.ts';


const assertThrowsMessage = (fn: () => unknown, message: string): void => {
  assert.throws(fn, (error: unknown) => (
    typeof error === 'object'
    && error !== null
    && 'message' in error
    && error.message === message
  ));
};

test('toAz accepts one ASCII letter', () => {
  assert.equal(toAz('a'), 'a');
  assert.equal(toAz('Z'), 'Z');
});

test('toAz rejects non-letter and multi-character input', () => {
  assertThrowsMessage(() => toAz('1'), '[1] out of range');
  assertThrowsMessage(() => toAz('ab'), '[ab] is too long');
});
