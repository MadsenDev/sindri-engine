function asClampedBytes(
  frame: Uint8Array | number[] | ArrayBuffer
): ImageDataArray {
  if (frame instanceof Uint8Array) {
    return new Uint8ClampedArray(
      frame.buffer as ArrayBuffer,
      frame.byteOffset,
      frame.byteLength
    ) as ImageDataArray;
  }

  if (frame instanceof ArrayBuffer) {
    return new Uint8ClampedArray(frame) as ImageDataArray;
  }

  return new Uint8ClampedArray(frame) as ImageDataArray;
}

export function decodeRawFrame(
  frame: Uint8Array | number[] | ArrayBuffer,
  width: number,
  height: number
) {
  return new ImageData(asClampedBytes(frame), width, height);
}
