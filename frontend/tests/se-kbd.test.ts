import assert from 'node:assert/strict';
import test from 'node:test';

import { SE_KEY_ROWS } from '../src/se-kbd.pure.ts';


const keyLabel = (name: string): string => name.startsWith('_') ? name.substring(1) : name;

const lowerLabels = (rowIndex: number): readonly string[] =>
  SE_KEY_ROWS[rowIndex].keys.map(key => keyLabel(key.lower));

const upperLabels = (rowIndex: number): readonly string[] =>
  SE_KEY_ROWS[rowIndex].keys.map(key => {
    if (key.spaceOnLongPress === true) return '·';
    if (key.upper === undefined) return '';
    return keyLabel(key.upper);
  });

test('se keyboard lower labels match se-layout.txt', () => {
  assert.deepEqual(lowerLabels(0), ['ㅆ', 'ㅎ', 'ㄴ', 'ㅇ', 'ㄹ', 'ㅅ', 'ㅂ', 'ㄱ', 'ㅁ', 'ㄷ']);
  assert.deepEqual(lowerLabels(1), ['ㅙ', 'ㅜ', 'ㅡ', 'ㅗ', 'ㅣ', 'ㅏ', 'ㅓ', 'ㅐ', 'ㅔ']);
  assert.deepEqual(lowerLabels(2), ['ㄹ', 'ㅅ', 'ㄷ', 'ㅁ', 'ㄴ', 'ㅇ', 'ㄱ', 'ㅈ', 'ㅂ', 'ㅎ']);
});

test('se keyboard long-press labels match upper keys', () => {
  assert.deepEqual(upperLabels(0), ['ㄲ', 'ㅈ', 'ㄶ', 'ㅄ', 'ㅍ', 'ㅌ', 'ㄺ', 'ㄻ', 'ㅊ', 'ㅋ']);
  assert.deepEqual(upperLabels(1), ['ㅝ', 'ㅠ', 'ㅢ', 'ㅛ', 'ㅘ', 'ㅑ', 'ㅕ', 'ㅒ', 'ㅖ']);
  assert.deepEqual(upperLabels(2), ['ㅌ', 'ㅆ', 'ㄸ', 'ㅍ', 'ㅊ', '·', 'ㄲ', 'ㅉ', 'ㅃ', 'ㅋ']);
});

test('se keyboard emits space on long-pressing initial ieung', () => {
  const choIeung = SE_KEY_ROWS[2].keys[5];

  assert.equal(choIeung.lower, 'ㅇ');
  assert.equal(choIeung.upper, undefined);
  assert.equal(choIeung.spaceOnLongPress, true);
});
