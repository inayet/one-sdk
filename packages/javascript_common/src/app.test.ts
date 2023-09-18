import { jest } from '@jest/globals';

import { readFile } from 'fs/promises';
import { createRequire } from 'module';
import { WASI } from 'wasi';

import { App } from './app.js';
import { UnexpectedError, ValidationError } from './error.js';
import { FileSystem, Persistence, Network, TextCoder, Timers } from './interfaces.js';


class TestNetwork implements Network {
  fetch(input: RequestInfo, init?: RequestInit | undefined): Promise<Response> {
    throw new Error('Fetch method not implemented.');
  }
}

class TestFileSystem implements FileSystem {
  async open(path: string, options: { createNew?: boolean | undefined; create?: boolean | undefined; truncate?: boolean | undefined; append?: boolean | undefined; write?: boolean | undefined; read?: boolean | undefined; }): Promise<number> {
    return 1;
  }
  async read(handle: number, out: Uint8Array): Promise<number> {
    return 0;
  }
  write(handle: number, data: Uint8Array): Promise<number> {
    throw new Error('Write method not implemented.');
  }
  async close(handle: number): Promise<void> {
    return;
  }
}

class TestCoder implements TextCoder {
  private encoder: TextEncoder = new TextEncoder();
  private decoder: TextDecoder = new TextDecoder();

  decodeUtf8(buffer: ArrayBufferLike): string {
    return this.decoder.decode(buffer);
  }

  encodeUtf8(string: string): ArrayBuffer {
    return this.encoder.encode(string);
  }
}

class TestTimers implements Timers {
  setTimeout(callback: () => void, ms: number): number {
    return 1;
  }
  clearTimeout(handle: number): void {
  }
}

class TestPersistence implements Persistence {
  async persistMetrics(events: string[]): Promise<void> { }
  async persistDeveloperDump(events: string[]): Promise<void> { }
}

describe('App', () => {
  let app: App;
  let handleMessage: jest.SpiedFunction<(message: any) => Promise<any>>;

  beforeEach(async () => {
    app = new App({
      network: new TestNetwork(),
      fileSystem: new TestFileSystem(),
      textCoder: new TestCoder(),
      timers: new TestTimers(),
      persistence: new TestPersistence(),
    }, { metricsTimeout: 1000 });

    await app.loadCore(
      await readFile(createRequire(import.meta.url).resolve('../../assets/test-core-async.wasm'))
    );

    await app.init(new WASI({ version: 'preview1' } as any));

    handleMessage = jest.spyOn(app, 'handleMessage');
    handleMessage.mockImplementation(async (message) => {
      switch (message.kind) {
        case 'perform-input':
          const performState = (app as any).performState;

          return {
            'kind': 'ok',
            'profile_url': performState.profileUrl,
            'provider_url': performState.providerUrl,
            'map_url': performState.mapUrl,
            'usecase': performState.usecase,
            'map_input': performState.input,
            'map_parameters': performState.parameters,
            'map_security': performState.security,
          };

        case 'perform-output-result':
          (app as any).performState.result = message.result;
          return {
            kind: 'ok'
          }
      }
    });
  });

  afterEach(async () => {
    await app.destroy();
  });

  test('panicked core', async () => {
    await expect(app.perform(
      'profile',
      'provider',
      'map',
      'CORE_PERFORM_PANIC',
      null,
      {},
      {},
    )).rejects.toThrow(UnexpectedError);
  });

  test('recover from panicked core', async () => {
    try {
      await app.perform(
        '',
        '',
        '',
        'CORE_PERFORM_PANIC',
        null,
        {},
        {},
      );
    } catch (e) { }

    await app.init(new WASI({ version: 'preview1' } as any));

    const result = await app.perform(
      '',
      '',
      '',
      'CORE_PERFORM_TRUE',
      null,
      {},
      {},
    );

    expect(result).toBe(true);
  });

  test('invalid user input', async () => {
    handleMessage.mockRestore();

    await expect(app.perform(
      '',
      '',
      '',
      'CORE_PERFORM_INPUT_VALIDATION_ERROR',
      null,
      {},
      {},
    )).rejects.toThrowError(ValidationError);
  });
});