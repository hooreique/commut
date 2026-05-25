import assert from 'node:assert/strict';
import test from 'node:test';

import type { Codec, CodecSource } from '../src/codec.pure.ts';
import type { Dimensions } from '../src/natural-number.pure.ts';
import { reader, writer, writeResize } from '../src/protocol.pure.ts';


const te = new TextEncoder();

const bytesOf = (source: CodecSource): readonly number[] => {
  if (source instanceof ArrayBuffer) {
    return [...new Uint8Array(source)];
  }

  return [...new Uint8Array(source.buffer, source.byteOffset, source.byteLength)];
};

test('writeResize writes lead 1 followed by cols and rows', () => {
  assert.deepEqual(
    [...writeResize({ cols: 80, rows: 24 } as Dimensions)],
    [1, ...te.encode('80,24')],
  );
});

test('reader reads resize packets and falls back for invalid dimensions', () => {
  const read = reader(() => Promise.reject({ message: 'decrypt must not run' }));

  return Promise.all([
    read(writeResize({ cols: 80, rows: 24 } as Dimensions).buffer),
    read(new Uint8Array([1, ...te.encode('0,24')]).buffer),
  ]).then(([valid, invalid]) => {
    assert.deepEqual(valid, { lead: 1, dimensions: { cols: 80, rows: 24 } });
    assert.deepEqual(invalid, { lead: 1, dimensions: { cols: 100, rows: 30 } });
  });
});

test('reader reads data packets through decrypt', () => {
  const iv = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
  const encrypted = [40, 41, 42];
  const decrypted = new Uint8Array([7, 8, 9]);
  const packet = new Uint8Array([0, ...iv, ...encrypted]);
  const decrypt: Codec = (actualIv, data) => {
    assert.deepEqual(bytesOf(actualIv), iv);
    assert.deepEqual(bytesOf(data), encrypted);
    return Promise.resolve(decrypted.buffer);
  };

  return reader(decrypt)(packet.buffer)
    .then(incoming => {
      assert.deepEqual(incoming, { lead: 0, data: decrypted });
    });
});

test('writer writes lead 0, injected IV, and encrypted payload', () => {
  const iv = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
  const data = new Uint8Array([20, 21, 22]);
  const encrypted = new Uint8Array([50, 51]);
  const encrypt: Codec = (actualIv, actualData) => {
    assert.deepEqual(bytesOf(actualIv), iv);
    assert.deepEqual(bytesOf(actualData), [...data]);
    return Promise.resolve(encrypted.buffer);
  };

  return writer(encrypt, bytes => {
    assert.equal(bytes.byteLength, 12);
    bytes.set(iv);
    return bytes;
  })(data).then(packet => {
    assert.deepEqual([...packet], [0, ...iv, ...encrypted]);
  });
});
