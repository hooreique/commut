import assert from 'node:assert/strict';
import test from 'node:test';

import { isNaturalNumber } from '../src/natural-number.pure.ts';


test('isNaturalNumber accepts positive safe integers', () => {
  assert.equal(isNaturalNumber(1), true);
  assert.equal(isNaturalNumber(Number.MAX_SAFE_INTEGER), true);
});

test('isNaturalNumber rejects values outside the natural number domain', () => {
  assert.equal(isNaturalNumber(0), false);
  assert.equal(isNaturalNumber(-1), false);
  assert.equal(isNaturalNumber(1.5), false);
  assert.equal(isNaturalNumber(Number.MAX_SAFE_INTEGER + 1), false);
  assert.equal(isNaturalNumber(NaN), false);
  assert.equal(isNaturalNumber(Infinity), false);
});
