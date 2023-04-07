import { Buffer } from './node_compat';

export type MultiMap = Record<string, string[]>;
export type Encoding = 'utf8' | 'base64';

export class Bytes {
  #buffer;
  #len;

  // TODO: private
  constructor(buffer: Uint8Array, len: number) {
    this.#buffer = buffer;
    this.#len = len;
  }

  static withCapacity(capacity: number): Bytes {
    return new Bytes(new Uint8Array(capacity ?? 0), 0);
  }

  static fromArray(array: number[]): Bytes {
    return new Bytes(new Uint8Array(array), array.length);
  }

  toArray(): number[] {
    return Array.from(this.data);
  }

  get len(): number {
    return this.#len;
  }

  get capacity(): number {
    return this.#buffer.byteLength;
  }

  get data(): Uint8Array {
    return this.#buffer.subarray(0, this.len);
  }

  get uninitData(): Uint8Array {
    return this.#buffer.subarray(this.len);
  }

  reserve(additional: number) {
    const want = this.len + additional;
    if (this.capacity >= want) {
      return;
    }

    // resize exponentially, copy old data into new buffer
    const newCapacity = Math.max(this.capacity * 2, want);
    const newBuffer = new Uint8Array(newCapacity);
    newBuffer.set(this.data, 0);

    this.#buffer = newBuffer;
  }

  extend(buffer: ArrayBuffer) {
    this.reserve(buffer.byteLength);
    this.#buffer.set(new Uint8Array(buffer), this.len);
    this.#len += buffer.byteLength;
  }

  decode(encoding: Encoding = 'utf8'): string {
    // TODO: again support for TypedArrays in Javy
    const buffer = this.#buffer.buffer.slice(0, this.len);

    if (encoding === 'utf8') {
      return __ffi.unstable.bytes_to_utf8(buffer);
    } else if (encoding === 'base64') {
      return __ffi.unstable.bytes_to_base64(buffer);
    }

    throw new Error(`encoding "${encoding}" not implemented`);
  }

  // TODO: support other encodings, currently this is always utf-8
  static encode(string: string, encoding: Encoding = 'utf8'): Bytes {
    let buffer;
    if (encoding === 'utf8') {
      buffer = __ffi.unstable.utf8_to_bytes(string);
    } else if (encoding === 'base64') {
      buffer = __ffi.unstable.base64_to_bytes(string);
    } else {
      throw new Error(`encoding "${encoding}" not implemented`);
    }

    return new Bytes(new Uint8Array(buffer), buffer.byteLength);
  }
}

export class ByteStream {
  #handle: number;
  constructor(handle: number) {
    this.#handle = handle;
  }

  public readToEnd(): Bytes {
    const buffer = Bytes.withCapacity(8192);
    // TODO: support for TypedArrays in Javy - without them we have to read into a plain ArrayBuffer (which cannot be a subarray)
    // and then copy that data into our final buffer.
    //
    // If Javy supported TypedArrays (they are supported in quickjs, just not exposed in Javy), we could directly pass a subarray
    // to the `stream_read` call and we'd only need one buffer.
    const readBuffer = new ArrayBuffer(8192);

    while (true) {
      const count = __ffi.unstable.stream_read(this.#handle, readBuffer);
      if (count === 0) {
        break;
      }

      buffer.extend(readBuffer.slice(0, count));
    }

    return buffer;
  }

  public close() {
    __ffi.unstable.stream_close(this.#handle);
  }
}

export function jsonReplacerMapValue(key: any, value: any): any {
  // TODO: this is how node Buffer gets serialized - do we want that?
  // to keep in line with our core convention, this should be some kind of `$MapValue::Buffer` and only transformed to the NodeJS buffer for the sake of tests
  if (Buffer.isBuffer(value)) {
    return { type: 'Buffer', data: value.inner.toArray() };
  }

  return value;
}
export function jsonReviverMapValue(key: any, value: any): any {
  if (typeof value === 'object' && value !== null) {
    if (value['type'] === 'Buffer' && Array.isArray(value['data'])) {
      return Buffer.from(value['data']);
    }
  }

  // TODO: revive streams
  return value;
}
export type JsonReplacer = (this: any, key: string, value: any) => any;
export type JsonReviver = (this: any, key: string, value: any) => any;
export function messageExchange(message: unknown, replacer: JsonReplacer | undefined = undefined, reviver: JsonReviver | undefined = undefined) {
  const response = __ffi.unstable.message_exchange(
    JSON.stringify(message, replacer)
  );
  return JSON.parse(response, reviver);
}

export function ensureMultimap(map: unknown, lowercaseKeys: boolean = false): MultiMap {
  const result: MultiMap = {};

  if (typeof map !== 'object' || map === null) {
    return result;
  }

  for (let [key, value] of Object.entries(map)) {
    if (lowercaseKeys) {
      key = key.toLowerCase();
    }

    if (!Array.isArray(value)) {
      value = [value];
    }

    result[key] = value.filter((v: any) => v !== undefined && v !== null).map((v: any) => v.toString());
  }

  return result;
}
