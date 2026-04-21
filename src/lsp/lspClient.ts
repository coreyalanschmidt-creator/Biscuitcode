// src/lsp/lspClient.ts
//
// Thin LSP adapter: bridges the Rust `biscuitcode-lsp` backend (reachable via
// Tauri commands lsp_spawn / lsp_write / lsp_shutdown) to Monaco's public
// provider registration APIs (registerHoverProvider,
// registerDefinitionProvider, registerMarkerData).
//
// No `monaco-languageclient` npm package is used. The adapter handles:
//   - JSON-RPC request/response correlation (PM-01 mitigation: only resolves
//     a pending promise when frame.id === requestId AND frame.result|error
//     are present, so bare notifications are never mismatched)
//   - One LspClient instance per (language, workspaceRoot) pair; calling
//     ensureSession() for an existing session is a no-op (PM-02 mitigation)
//   - Monaco language ID lookup table covers built-in names (PM-03 mitigation)

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type * as Monaco from 'monaco-editor';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface LspMsgPayload {
  session_id: string;
  frame: LspFrame;
}

interface LspFrame {
  jsonrpc: '2.0';
  id?: number | string;
  method?: string;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
  params?: unknown;
}

// ---------------------------------------------------------------------------
// Monaco language ID ↔ LSP language string mapping (PM-03 mitigation)
// ---------------------------------------------------------------------------

// Monaco's registered language IDs differ from our Rust Language enum strings.
// This table maps Monaco IDs → the language string we pass to lsp_spawn.
const MONACO_TO_LSP_LANG: Record<string, string> = {
  rust: 'rust',
  typescript: 'typescript',
  javascript: 'typescript', // tsserver handles both
  python: 'python',
  go: 'go',
  cpp: 'cpp',
  c: 'cpp',
};

// ---------------------------------------------------------------------------
// LspClient
// ---------------------------------------------------------------------------

export class LspClient {
  private sessionId: string | null = null;
  private unlisten: UnlistenFn | null = null;
  private reqId = 1;
  // Pending requests: id → { resolve, reject, timeoutHandle }
  private pending = new Map<
    number,
    { resolve: (v: unknown) => void; reject: (e: Error) => void; timeout: ReturnType<typeof setTimeout> }
  >();

  // Notification handlers (e.g. textDocument/publishDiagnostics)
  private notifHandlers = new Map<string, (params: unknown) => void>();

  constructor(
    private readonly language: string,
    private readonly workspaceRoot: string,
    // monacoInstance reserved for future provider-registration helpers
    _monacoInstance: typeof Monaco,
  ) {}

  /**
   * Ensure an LSP session is running. Idempotent — safe to call on every
   * tab focus without spawning duplicate sessions (PM-02 mitigation).
   */
  async ensureSession(): Promise<void> {
    if (this.sessionId !== null) return;

    const sessionId = await invoke<string>('lsp_spawn', {
      request: { language: this.language, workspace_root: this.workspaceRoot },
    });
    this.sessionId = sessionId;

    // Subscribe to inbound frames from this session.
    const eventName = `lsp-msg-in-${sessionId}`;
    this.unlisten = await listen<LspMsgPayload>(eventName, ({ payload }) => {
      this._handleFrame(payload.frame);
    });
  }

  /** Send a request and await its response (or timeout after `timeoutMs`). */
  async request(method: string, params: unknown, timeoutMs = 5000): Promise<unknown> {
    if (!this.sessionId) throw new Error('LspClient: no active session');

    const id = this.reqId++;
    const frame: LspFrame = { jsonrpc: '2.0', id, method, params };
    const sessionId = this.sessionId;

    // Register the pending entry before sending, so we don't miss a
    // synchronous response (unlikely, but safe).
    const responsePromise = new Promise<unknown>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`LSP timeout for ${method} (id=${id})`));
      }, timeoutMs);
      this.pending.set(id, { resolve, reject, timeout });
    });

    // Send the frame. On write failure, reject the pending promise immediately.
    invoke<void>('lsp_write', { sessionId, frame }).catch((err: unknown) => {
      const entry = this.pending.get(id);
      if (entry) {
        this.pending.delete(id);
        clearTimeout(entry.timeout);
        entry.reject(new Error(`lsp_write failed: ${String(err)}`));
      }
    });

    return responsePromise;
  }

  /** Register a handler for an LSP notification method (e.g. publishDiagnostics). */
  onNotification(method: string, handler: (params: unknown) => void): void {
    this.notifHandlers.set(method, handler);
  }

  /** Shut down the session and clean up. */
  async shutdown(): Promise<void> {
    if (!this.sessionId) return;
    const id = this.sessionId;
    this.sessionId = null;
    this.pending.forEach(({ reject, timeout }) => {
      clearTimeout(timeout);
      reject(new Error('LspClient: session shut down'));
    });
    this.pending.clear();
    this.unlisten?.();
    this.unlisten = null;
    await invoke<void>('lsp_shutdown', { sessionId: id }).catch(() => {/* best effort */});
  }

  // ---------------------------------------------------------------------------
  // Private: inbound frame router (PM-01 mitigation)
  // ---------------------------------------------------------------------------

  private _handleFrame(frame: LspFrame): void {
    // Response: must have an id AND (result or error) — never just a notification.
    if (frame.id !== undefined && (frame.result !== undefined || frame.error !== undefined)) {
      const numId = typeof frame.id === 'number' ? frame.id : parseInt(frame.id as string, 10);
      const entry = this.pending.get(numId);
      if (entry) {
        this.pending.delete(numId);
        clearTimeout(entry.timeout);
        if (frame.error) {
          entry.reject(new Error(`LSP error ${frame.error.code}: ${frame.error.message}`));
        } else {
          entry.resolve(frame.result);
        }
      }
      return;
    }

    // Notification: has method but no id (or id is absent).
    if (frame.method && frame.id === undefined) {
      const handler = this.notifHandlers.get(frame.method);
      if (handler) handler(frame.params);
    }
  }
}

// ---------------------------------------------------------------------------
// Provider registration
// ---------------------------------------------------------------------------

/**
 * Register Monaco hover + definition providers that delegate to the LSP
 * session managed by `client`.
 *
 * Returns a disposable object; call `.dispose()` to remove the providers
 * when the workspace/language changes.
 */
export function registerLspProviders(
  monaco: typeof Monaco,
  client: LspClient,
  monacoLanguageId: string,
): Monaco.IDisposable {
  const disposables: Monaco.IDisposable[] = [];

  // --- Hover provider ---
  disposables.push(
    monaco.languages.registerHoverProvider(monacoLanguageId, {
      async provideHover(model, position): Promise<Monaco.languages.Hover | null> {
        try {
          const result = await client.request('textDocument/hover', {
            textDocument: { uri: model.uri.toString() },
            position: { line: position.lineNumber - 1, character: position.column - 1 },
          });
          if (!result) return null;
          const r = result as {
            contents?: { kind: string; value: string } | string | Array<{ kind: string; value: string } | string>;
            range?: { start: { line: number; character: number }; end: { line: number; character: number } };
          };
          const contents = normalizeLspMarkup(r.contents);
          if (contents.length === 0) return null;
          const range = r.range
            ? new monaco.Range(
                r.range.start.line + 1,
                r.range.start.character + 1,
                r.range.end.line + 1,
                r.range.end.character + 1,
              )
            : undefined;
          return { contents, range };
        } catch {
          return null;
        }
      },
    }),
  );

  // --- Go-to-definition provider ---
  disposables.push(
    monaco.languages.registerDefinitionProvider(monacoLanguageId, {
      async provideDefinition(model, position): Promise<Monaco.languages.Definition | null> {
        try {
          const result = await client.request('textDocument/definition', {
            textDocument: { uri: model.uri.toString() },
            position: { line: position.lineNumber - 1, character: position.column - 1 },
          });
          if (!result) return null;
          const locations = Array.isArray(result) ? result : [result];
          return locations
            .map((loc: { uri: string; range: { start: { line: number; character: number }; end: { line: number; character: number } } }) => ({
              uri: monaco.Uri.parse(loc.uri),
              range: new monaco.Range(
                loc.range.start.line + 1,
                loc.range.start.character + 1,
                loc.range.end.line + 1,
                loc.range.end.character + 1,
              ),
            }))
            .filter(Boolean);
        } catch {
          return null;
        }
      },
    }),
  );

  return {
    dispose() {
      disposables.forEach((d) => d.dispose());
    },
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Convert heterogeneous LSP hover content shapes to Monaco's MarkdownString[]. */
function normalizeLspMarkup(contents: unknown): Monaco.IMarkdownString[] {
  if (!contents) return [];
  const items: Array<{ kind: string; value: string } | string> = Array.isArray(contents)
    ? (contents as Array<{ kind: string; value: string } | string>)
    : [contents as { kind: string; value: string } | string];

  const result: Monaco.IMarkdownString[] = [];
  for (const item of items) {
    if (typeof item === 'string') {
      result.push({ value: item });
    } else if (item && typeof item === 'object' && 'value' in item) {
      if (item.kind === 'markdown') {
        result.push({ value: item.value, isTrusted: true });
      } else {
        result.push({ value: `\`\`\`\n${item.value}\n\`\`\`` });
      }
    }
  }
  return result;
}

// ---------------------------------------------------------------------------
// Language ID helpers (exported for tests and EditorArea use)
// ---------------------------------------------------------------------------

/** Map a Monaco language ID to the LSP language string for lsp_spawn. */
export function monacoLangToLsp(monacoId: string): string | null {
  return MONACO_TO_LSP_LANG[monacoId.toLowerCase()] ?? null;
}

/** Map a file extension to a Monaco language ID (covers common cases). */
export function extToMonacoLang(ext: string): string {
  const map: Record<string, string> = {
    rs: 'rust',
    ts: 'typescript',
    tsx: 'typescript',
    js: 'javascript',
    jsx: 'javascript',
    py: 'python',
    go: 'go',
    cpp: 'cpp',
    cc: 'cpp',
    c: 'c',
    h: 'cpp',
  };
  return map[ext.toLowerCase()] ?? 'plaintext';
}
