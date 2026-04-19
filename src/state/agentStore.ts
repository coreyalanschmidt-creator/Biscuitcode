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

interface AgentState {
  /** Whether the chat panel is in agent mode (auto-continues on tool calls). */
  agentMode: boolean;
  /** Active conversation id — set by ChatPanel when a conversation is created. */
  conversationId: string | null;
  /** Ordered list of tool-call cards for the current session. */
  cards: ToolCallCard[];

  // Actions
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
}

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
}));
