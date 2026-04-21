// tests/unit/lspClient.spec.ts
//
// Unit tests for src/lsp/lspClient.ts
//
// Tests are intentionally headless — no Monaco instance required for the
// protocol-level tests. Monaco-dependent tests use a minimal stub.

import { beforeEach, describe, expect, it, vi } from 'vitest';

// ---------------------------------------------------------------------------
// Mock @tauri-apps/api/core and @tauri-apps/api/event before importing the
// module under test.
// ---------------------------------------------------------------------------

const mockInvoke = vi.fn();
const mockListen = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

// ---------------------------------------------------------------------------
// Import after mocks
// ---------------------------------------------------------------------------

import { LspClient, registerLspProviders, monacoLangToLsp, extToMonacoLang } from '../../src/lsp/lspClient';

// ---------------------------------------------------------------------------
// Helpers: simulate an LspClient with a live session
// ---------------------------------------------------------------------------

/** Captured listen handler so tests can push synthetic frames. */
let capturedFrameHandler: ((event: { payload: { session_id: string; frame: unknown } }) => void) | null = null;

function makeClient() {
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === 'lsp_spawn') return Promise.resolve('sess-1');
    if (cmd === 'lsp_write') return Promise.resolve(undefined);
    if (cmd === 'lsp_shutdown') return Promise.resolve(undefined);
    return Promise.reject(new Error(`unexpected invoke: ${cmd}`));
  });

  mockListen.mockImplementation((_event: string, handler: (e: unknown) => void) => {
    capturedFrameHandler = handler as typeof capturedFrameHandler;
    return Promise.resolve(() => { capturedFrameHandler = null; });
  });

  // Minimal Monaco stub (not used for protocol tests)
  const monacoStub = {} as never;
  return new LspClient('rust', '/workspace', monacoStub);
}

function pushFrame(frame: unknown) {
  capturedFrameHandler?.({ payload: { session_id: 'sess-1', frame } });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('LspClient — session management', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedFrameHandler = null;
  });

  it('calls lsp_spawn once and subscribes to the event', async () => {
    const client = makeClient();
    await client.ensureSession();
    expect(mockInvoke).toHaveBeenCalledWith('lsp_spawn', {
      request: { language: 'rust', workspace_root: '/workspace' },
    });
    expect(mockListen).toHaveBeenCalledWith('lsp-msg-in-sess-1', expect.any(Function));
  });

  it('is idempotent — second ensureSession does NOT re-spawn (PM-02)', async () => {
    const client = makeClient();
    await client.ensureSession();
    await client.ensureSession();
    // lsp_spawn must be called exactly once
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });
});

describe('LspClient — request/response correlation (PM-01)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedFrameHandler = null;
  });

  it('resolves a pending request when the matching response arrives', async () => {
    const client = makeClient();
    await client.ensureSession();

    // Trigger request; don't await yet
    let writeCallArgs: unknown[] | null = null;
    mockInvoke.mockImplementation((cmd: string, args: unknown) => {
      if (cmd === 'lsp_write') { writeCallArgs = [cmd, args]; return Promise.resolve(undefined); }
      return Promise.resolve(undefined);
    });

    const responsePromise = client.request('textDocument/hover', { textDocument: { uri: 'file:///foo.rs' }, position: { line: 0, character: 0 } }, 500);

    // Pump the microtask queue so lsp_write fires
    await Promise.resolve();

    // Simulate the LSP server responding (id=1, which is the first request)
    pushFrame({ jsonrpc: '2.0', id: 1, result: { contents: { kind: 'markdown', value: '**i32**' } } });

    const result = await responsePromise;
    expect((result as { contents: { value: string } }).contents.value).toContain('i32');
    expect(writeCallArgs).not.toBeNull();
  });

  it('does NOT resolve on a notification with no id (PM-01: notification guard)', async () => {
    const client = makeClient();
    await client.ensureSession();

    const hoverPromise = client.request('textDocument/hover', {}, 200);
    await Promise.resolve();

    // Push a notification (textDocument/publishDiagnostics — has method but NO id)
    pushFrame({ jsonrpc: '2.0', method: 'textDocument/publishDiagnostics', params: { diagnostics: [] } });

    // The promise should time out, not resolve from the notification
    await expect(hoverPromise).rejects.toThrow('LSP timeout');
  });

  it('rejects on LSP error response', async () => {
    const client = makeClient();
    await client.ensureSession();

    const requestPromise = client.request('textDocument/hover', {}, 500);
    await Promise.resolve();

    pushFrame({ jsonrpc: '2.0', id: 1, error: { code: -32601, message: 'Method not found' } });

    await expect(requestPromise).rejects.toThrow('Method not found');
  });

  it('dispatches notifications to registered handlers', async () => {
    const client = makeClient();
    await client.ensureSession();

    const diagHandler = vi.fn();
    client.onNotification('textDocument/publishDiagnostics', diagHandler);

    pushFrame({
      jsonrpc: '2.0',
      method: 'textDocument/publishDiagnostics',
      params: { uri: 'file:///foo.rs', diagnostics: [{ message: 'unused var' }] },
    });

    expect(diagHandler).toHaveBeenCalledWith(
      expect.objectContaining({ uri: 'file:///foo.rs' }),
    );
  });
});

describe('LspClient — shutdown', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedFrameHandler = null;
  });

  it('calls lsp_shutdown and clears session', async () => {
    const client = makeClient();
    await client.ensureSession();

    // After shutdown, pending requests should be rejected
    const reqPromise = client.request('textDocument/definition', {}, 500);
    await client.shutdown();

    await expect(reqPromise).rejects.toThrow('shut down');
    expect(mockInvoke).toHaveBeenCalledWith('lsp_shutdown', { sessionId: 'sess-1' });
  });
});

// ---------------------------------------------------------------------------
// Language mapping helpers
// ---------------------------------------------------------------------------

describe('monacoLangToLsp', () => {
  it('maps known Monaco language IDs', () => {
    expect(monacoLangToLsp('rust')).toBe('rust');
    expect(monacoLangToLsp('typescript')).toBe('typescript');
    expect(monacoLangToLsp('javascript')).toBe('typescript');
    expect(monacoLangToLsp('python')).toBe('python');
    expect(monacoLangToLsp('go')).toBe('go');
    expect(monacoLangToLsp('cpp')).toBe('cpp');
    expect(monacoLangToLsp('c')).toBe('cpp');
  });

  it('returns null for unknown languages (PM-03: no silent failure)', () => {
    expect(monacoLangToLsp('cobol')).toBeNull();
    expect(monacoLangToLsp('brainfuck')).toBeNull();
  });
});

describe('extToMonacoLang', () => {
  it('maps common extensions', () => {
    expect(extToMonacoLang('rs')).toBe('rust');
    expect(extToMonacoLang('ts')).toBe('typescript');
    expect(extToMonacoLang('tsx')).toBe('typescript');
    expect(extToMonacoLang('py')).toBe('python');
    expect(extToMonacoLang('go')).toBe('go');
  });

  it('returns plaintext for unknown extensions', () => {
    expect(extToMonacoLang('xyz')).toBe('plaintext');
  });
});

// ---------------------------------------------------------------------------
// registerLspProviders — minimal stub test
// ---------------------------------------------------------------------------

describe('registerLspProviders', () => {
  it('returns a disposable object', () => {
    // Minimal Monaco stub — only the parts registerLspProviders touches
    const registrations: string[] = [];
    const monacoStub = {
      languages: {
        registerHoverProvider: vi.fn((_lang: string) => {
          registrations.push('hover');
          return { dispose: vi.fn() };
        }),
        registerDefinitionProvider: vi.fn((_lang: string) => {
          registrations.push('definition');
          return { dispose: vi.fn() };
        }),
      },
    } as unknown as typeof import('monaco-editor');

    const clientStub = { request: vi.fn() } as unknown as LspClient;

    const disposable = registerLspProviders(monacoStub, clientStub, 'rust');
    expect(registrations).toEqual(['hover', 'definition']);
    expect(typeof disposable.dispose).toBe('function');
    disposable.dispose(); // should not throw
  });
});
