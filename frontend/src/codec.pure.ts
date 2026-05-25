export type CodecSource = ArrayBuffer | ArrayBufferView<ArrayBuffer>;

export type Codec = (iv: CodecSource, data: CodecSource) => Promise<ArrayBuffer>;
