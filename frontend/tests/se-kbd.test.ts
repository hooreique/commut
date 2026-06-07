import assert from 'node:assert/strict';
import test from 'node:test';

import { SE_KEY_ROWS } from '../src/se-kbd.pure.ts';


const keyLabel = (name: string): string => name.startsWith('_') ? name.substring(1) : name;

const lowerLabels = (rowIndex: number): readonly string[] =>
  SE_KEY_ROWS[rowIndex].keys.map(key => keyLabel(key.lower));

const upperLabels = (rowIndex: number): readonly string[] =>
  SE_KEY_ROWS[rowIndex].keys.map(key => {
    if (key.flushOnLongPress === true) return '·';
    if (key.upper === undefined) return '';
    return keyLabel(key.upper);
  });

test('se keyboard lower labels match se-layout.txt', () => {
  assert.deepEqual(lowerLabels(0), ['ㅆ', 'ㅎ', 'ㄴ', 'ㅇ', 'ㄹ', 'ㅅ', 'ㅂ', 'ㄱ', 'ㅁ', 'ㅌ']);
  assert.deepEqual(lowerLabels(1), ['ㅝ', 'ㅜ', 'ㅓ', 'ㅏ', 'ㅣ', 'ㅕ', 'ㅡ', 'ㅗ', 'ㅐ']);
  assert.deepEqual(lowerLabels(2), ['ㅎ', 'ㅂ', 'ㄷ', 'ㄹ', 'ㅇ', 'ㄱ', 'ㄴ', 'ㅁ', 'ㅅ', 'ㅈ']);
});

test('se keyboard long-press labels match upper keys', () => {
  assert.deepEqual(upperLabels(0), ['ㄲ', 'ㅈ', 'ㅍ', 'ㄷ', 'ㄺ', 'ㅊ', 'ㅄ', 'ㄶ', 'ㄻ', 'ㅋ']);
  assert.deepEqual(upperLabels(1), ['ㅘ', 'ㅠ', 'ㅔ', 'ㅑ', 'ㅞ', 'ㅖ', 'ㅙ', 'ㅛ', 'ㅒ']);
  assert.deepEqual(upperLabels(2), ['ㅋ', 'ㅃ', 'ㄸ', 'ㅌ', '·', 'ㄲ', 'ㅊ', 'ㅍ', 'ㅆ', 'ㅉ']);
});

test('se keyboard flushes on long-pressing initial ieung', () => {
  const choIeung = SE_KEY_ROWS[2].keys[4];

  assert.equal(choIeung.lower, 'ㅇ');
  assert.equal(choIeung.upper, undefined);
  assert.equal(choIeung.flushOnLongPress, true);
});
