// src/state/agentStore.ts
//
// Phase 6a: Shared Zustand store for agent activity state.
//
// AgentActivityPanel reads tool-call cards from here.
// ChatPanel writes ToolCallStart/End events here as they arrive on the
// Tauri event channel.  Sharing via store (not prop-drilling) avoids
// the PM-06 failure mode where the panel can't subscribe to events it
// doesn't own the conversationId for.

import { create } from 'zustand';

export type ToolCallStatus = 'running' | 'ok' | 'error';

export interface ToolCallCard {
  /** Unique id from the provider (e.g. "call_abc123"). */
  id: string;
  name: string;
  /** Accumulated JSON argument string — partial while streaming. */
  argsJson: string;
  /** Result text set on ToolCallEnd. */
  result: string | null;
  status: ToolCallStatus;
  /** `performance.now()` at ToolCallStart. */
  startedAt: number;
  /** `performance.now()` at ToolCallEnd. */
  endedAt: number | null;
}

/** Exported as AgentStore to match Phase 6a-i plan interface requirement. */
export interface AgentStore {
  /** Whether the chat panel is in agent mode (auto-continues on tool calls). */
  agentMode: boolean;
  /** Active conversation id — set by ChatPanel when a conversation is created. */
  conversationId: string | null;
  /** Ordered list of tool-call cards for the current session. */
  cards: ToolCallCard[];

  // Actions (primary names used throughout the codebase)
  setAgentMode: (on: boolean) => void;
  setConversationId: (id: string | null) => void;
  /** Called when a ToolCallStart event arrives. */
  startCard: (id: string, name: string) => void;
  /** Append delta text to the args accumulator. */
  appendArgsDelta: (id: string, delta: string) => void;
  /** Called when a ToolCallEnd event arrives with final args + result. */
  endCard: (id: string, argsJson: string, result: string | null, error: boolean) => void;
  /** Reset cards (e.g. on new conversation). */
  clearCards: () => void;

  // Phase 6a-i plan-required aliases
  /** Alias for startCard — sets status:'running', argsJson:'', startedAt, endedAt:null. */
  addCard: (id: string, name: string) => void;
  /** Alias for appendArgsDelta. */
  updateCardArgs: (id: string, delta: string) => void;
  /** Alias for endCard with error:false. */
  completeCard: (id: string, result: string) => void;
  /** Alias for endCard with error:true and result:null. */
  errorCard: (id: string, error: string) => void;
}

// Internal alias so create<> infers the full type.
type AgentState = AgentStore;

export const useAgentStore = create<AgentState>()((set) => ({
  agentMode: false,
  conversationId: null,
  cards: [],

  setAgentMode: (on) => set({ agentMode: on }),

  setConversationId: (id) => set({ conversationId: id }),

  startCard: (id, name) =>
    set((s) => ({
      cards: [
        ...s.cards,
        {
          id,
          name,
          argsJson: '',
          result: null,
          status: 'running' as ToolCallStatus,
          startedAt: performance.now(),
          endedAt: null,
        },
      ],
    })),

  appendArgsDelta: (id, delta) =>
    set((s) => ({
      cards: s.cards.map((c) =>
        c.id === id ? { ...c, argsJson: c.argsJson + delta } : c,
      ),
    })),

  endCard: (id, argsJson, result, error) =>
    set((s) => ({
      cards: s.cards.map((c) =>
        c.id === id
          ? {
              ...c,
              argsJson,
              result,
              status: error ? ('error' as ToolCallStatus) : ('ok' as ToolCallStatus),
              endedAt: performance.now(),
            }
          : c,
      ),
    })),

  clearCards: () => set({ cards: [] }),

  // Phase 6a-i plan-required aliases — delegate to primary implementations.
  addCard: (id, name) =>
    set((s) => ({
      cards: [
        ...s.cards,
        {
          id,
          name,
          argsJson: '',
          result: null,
          status: 'running' as ToolCallStatus,
          startedAt: performance.now(),
          endedAt: null,
        },
      ],
    })),

  updateCardArgs: (id, delta) =>
    set((s) => ({
      cards: s.cards.map((c) =>
        c.id === id ? { ...c, argsJson: c.argsJson + delta } : c,
      ),
    })),

  completeCard: (id, result) =>
    set((s) => ({
      cards: s.cards.map((c) =>
        c.id === id
          ? { ...c, result, status: 'ok' as ToolCallStatus, endedAt: performance.now() }
          : c,
      ),
    })),

  errorCard: (id, error) =>
    set((s) => ({
      cards: s.cards.map((c) =>
        c.id === id
          ? { ...c, result: error, status: 'error' as ToolCallStatus, endedAt: performance.now() }
          : c,
      ),
    })),
}));
