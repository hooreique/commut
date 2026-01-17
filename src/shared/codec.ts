export type Codec = (iv: BufferSource, data: BufferSource) => Promise<ArrayBuffer>;
