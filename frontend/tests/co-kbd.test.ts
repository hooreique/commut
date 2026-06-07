import assert from 'node:assert/strict';
import test from 'node:test';

import { CO_KEY_ROWS } from '../src/co-kbd.pure.ts';


const lowerLabels = (rowIndex: number): readonly string[] =>
  CO_KEY_ROWS[rowIndex].keys.map(key => key.lower);

const upperLabels = (rowIndex: number): readonly string[] =>
  CO_KEY_ROWS[rowIndex].keys.map(key => key.upper);

const faces = (rowIndex: number): readonly (string | undefined)[] =>
  CO_KEY_ROWS[rowIndex].keys.map(key => key.face);

test('co keyboard lower labels match co-layout.txt', () => {
  assert.deepEqual(lowerLabels(0), ['q', 'w', 'f', 'p', 'g', 'j', 'l', 'u', 'y', 'o']);
  assert.deepEqual(lowerLabels(1), ['a', 'r', 's', 't', 'd', 'h', 'n', 'e', 'i']);
  assert.deepEqual(lowerLabels(2), ['/', 'z', 'x', 'c', 'v', 'b', 'k', 'm', ',', '.']);
});

test('co keyboard long-press labels match upper keys', () => {
  assert.deepEqual(upperLabels(0), ['Q', 'W', 'F', 'P', 'G', 'J', 'L', 'U', 'Y', 'O']);
  assert.deepEqual(upperLabels(1), ['A', 'R', 'S', 'T', 'D', 'H', 'N', 'E', 'I']);
  assert.deepEqual(upperLabels(2), ['?', 'Z', 'X', 'C', 'V', 'B', 'K', 'M', '<', '>']);
});

test('co keyboard faces are explicit for alphabet keys only', () => {
  assert.deepEqual(faces(0), ['Q', 'W', 'F', 'P', 'G', 'J', 'L', 'U', 'Y', 'O']);
  assert.deepEqual(faces(1), ['A', 'R', 'S', 'T', 'D', 'H', 'N', 'E', 'I']);
  assert.deepEqual(faces(2), [undefined, 'Z', 'X', 'C', 'V', 'B', 'K', 'M', undefined, undefined]);
});
