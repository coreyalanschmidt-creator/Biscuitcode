// src/state/lspStore.ts
//
// Phase 7 deliverable: lightweight store for LSP diagnostic counts
// and active language server names, consumed by StatusBar and ChatPanel
// (@problems mention).

import { create } from 'zustand';

export interface LspDiagnostic {
  path: string;
  message: string;
  severity: 1 | 2 | 3 | 4; // LSP DiagnosticSeverity: 1=Error, 2=Warn, 3=Info, 4=Hint
  line: number;
  character: number;
}

interface LspState {
  /** Map of session_id -> language string */
  activeSessions: Record<string, string>;
  /** All diagnostics across all open files */
  diagnostics: LspDiagnostic[];

  // Actions
  addSession: (sessionId: string, language: string) => void;
  removeSession: (sessionId: string) => void;
  setDiagnostics: (path: string, diags: LspDiagnostic[]) => void;
  clearDiagnostics: (path: string) => void;
}

export const useLspStore = create<LspState>((set) => ({
  activeSessions: {},
  diagnostics: [],

  addSession: (sessionId, language) =>
    set((s) => ({ activeSessions: { ...s.activeSessions, [sessionId]: language } })),

  removeSession: (sessionId) =>
    set((s) => {
      const sessions = { ...s.activeSessions };
      delete sessions[sessionId];
      return { activeSessions: sessions };
    }),

  setDiagnostics: (path, diags) =>
    set((s) => ({
      diagnostics: [
        ...s.diagnostics.filter((d) => d.path !== path),
        ...diags,
      ],
    })),

  clearDiagnostics: (path) =>
    set((s) => ({ diagnostics: s.diagnostics.filter((d) => d.path !== path) })),
}));
