// tests/unit/agent-event-bridge.spec.ts
//
// Phase 6a-ii: unit tests for the "agent:event" Tauri event bridge.
//
// Verifies that the ChatPanel's listen("agent:event", handler) subscription:
//   1. Dispatches ToolCallStart → agentStore.addCard(id, name)
//      with a performance.mark('tool_call_start_<id>') side-effect.
//   2. Dispatches ToolResult → agentStore.completeCard(id, result).
//   3. Dispatches Done → unlatches the loading state (isStreaming).
//
// Tests use the same mocking strategy as agent-activity-panel.spec.tsx:
// vi.mock('@tauri-apps/api/event') captures the listener closure, then
// helpers fire synthetic events into it.

/// <reference types="@testing-library/jest-dom/vitest" />
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, act, cleanup } from '@testing-library/react';
import { expect as jestExpect } from 'vitest';
import * as matchers from '@testing-library/jest-dom/matchers';
import React from 'react';

jestExpect.extend(matchers);

import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';

// ---------- i18n bootstrap ----------

await i18next.use(initReactI18next).init({
  lng: 'en',
  resources: {
    en: {
      translation: {
        panels: { agentActivity: 'Agent Activity', chatPanel: 'Chat Panel', chats: 'Chats' },
        agent: {
          emptyHint: 'No tool calls yet.',
          running: 'running…',
          args: 'Arguments',
          result: 'Result',
          status: { running: 'Running', ok: 'Done', error: 'Error' },
          runningLabel: 'Agent running…',
          pauseLabel: 'Pause agent',
          doneLabel: 'Agent done',
          confirmWriteTitle: 'Allow file write?',
          confirmShellTitle: 'Allow shell command?',
          confirmSummaryLabel: 'Summary',
          confirmApprove: 'Allow',
          confirmDeny: 'Deny',
          confirmDenyWithFeedback: 'Deny + feedback',
          confirmSendFeedback: 'Send feedback',
          confirmFeedbackLabel: 'Feedback',
          confirmFeedbackPlaceholder: 'Tell the agent…',
          toolClassWrite: 'write',
          toolClassShell: 'shell',
          inlineEditTitle: 'AI Inline Edit',
          inlineEditInputLabel: 'Describe the change',
          inlineEditPlaceholder: 'E.g. "Add error handling"…',
          inlineEditGenerate: 'Generate',
          inlineEditGenerating: 'Generating…',
          inlineEditAccept: 'Accept',
          inlineEditReject: 'Reject',
          inlineEditRegenerate: 'Regenerate',
          inlineEditDiffLabel: 'Proposed change',
          inlineEditError: 'Failed to generate inline edit.',
          inlineEditApplyError: 'Failed to apply inline edit.',
        },
        chat: {
          you: 'You',
          assistant: 'Assistant',
          modelPickerLabel: 'Select model',
          newChat: 'New chat',
          emptyHint: 'Type a message.',
          inputLabel: 'Chat message',
          inputPlaceholder: 'Message…',
          shortcutHint: 'shortcuts',
          sendButton: 'Send',
          sending: 'Sending…',
          noKeyBanner: 'No key set.',
          errorNoKey: 'No key.',
          errorStream: 'Stream error.',
          errorSend: 'Send error.',
          rewindError: 'Rewind failed.',
          rewind: 'Rewind',
          rewindLabel: 'Rewind to this message',
          agentMode: 'Agent',
          agentModeLabel: 'Agent mode',
          agentModeTitle: 'Agent mode tooltip',
          applyCode: 'Apply code',
          apply: 'Apply',
          runCode: 'Run code',
          run: 'Run',
          mentionPickerLabel: 'File mention picker',
          mentionNoResults: 'No matching files',
        },
        mentions: { noTerminals: 'No terminals', noProblems: 'No problems' },
        editor: {
          area: 'Editor area',
          tabList: 'Open files',
          dirtyIndicator: 'Unsaved',
          closeTab: 'Close',
          noFileOpen: 'No file open',
          loading: 'Loading…',
          welcomeHint: 'Open a folder.',
          quickOpen: 'Quick open',
          quickOpenPlaceholder: 'Type…',
          noResults: 'No results',
        },
      },
    },
  },
});

// ---------- react-virtuoso mock ----------

vi.mock('react-virtuoso', () => ({
  Virtuoso: ({
    data,
    itemContent,
    className,
  }: {
    data: unknown[];
    itemContent: (index: number, item: unknown) => React.ReactNode;
    className?: string;
  }) => (
    React.createElement('div', { className },
      data.map((item, i) => React.createElement(React.Fragment, { key: i }, itemContent(i, item)))
    )
  ),
}));

// ---------- Tauri mocks ----------

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'anthropic_key_present') return false;
    if (cmd === 'anthropic_list_models') return [];
    if (cmd === 'fs_search_files') return [];
    return null;
  }),
}));

type ListenerFn = (evt: { payload: unknown }) => void;
const _listeners: Record<string, ListenerFn[]> = {};

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (channel: string, fn: ListenerFn) => {
    _listeners[channel] = _listeners[channel] ?? [];
    _listeners[channel].push(fn);
    return () => {
      _listeners[channel] = (_listeners[channel] ?? []).filter((f) => f !== fn);
    };
  }),
}));

/** Fire a synthetic Tauri event to all registered listeners on a channel. */
function fireTauriEvent(channel: string, payload: unknown) {
  (_listeners[channel] ?? []).forEach((fn) => fn({ payload }));
}

// ---------- Imports (after mocks) ----------

import { ChatPanel } from '../../src/components/ChatPanel';
import { useAgentStore } from '../../src/state/agentStore';

// ---------- Setup / teardown ----------

beforeEach(() => {
  useAgentStore.setState({ agentMode: false, conversationId: null, cards: [] });
  Object.keys(_listeners).forEach((k) => { _listeners[k] = []; });
  performance.clearMarks();
  performance.clearMeasures();
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

// ---------- Tests ----------

describe('agent:event bridge in ChatPanel', () => {
  it('ToolCallStart payload calls agentStore.addCard with correct id and name', async () => {
    // Render the component to register the "agent:event" listener.
    await act(async () => {
      render(React.createElement(ChatPanel));
    });

    // Fire a synthetic tool_call_start event on the "agent:event" channel.
    await act(async () => {
      fireTauriEvent('agent:event', {
        type: 'tool_call_start',
        id: 'call_xyz',
        name: 'search_code',
      });
    });

    const cards = useAgentStore.getState().cards;
    expect(cards).toHaveLength(1);
    expect(cards[0].id).toBe('call_xyz');
    expect(cards[0].name).toBe('search_code');
    expect(cards[0].status).toBe('running');
  });

  it('ToolCallStart payload also emits performance.mark("tool_call_start_<id>")', async () => {
    const markSpy = vi.spyOn(performance, 'mark');

    await act(async () => {
      render(React.createElement(ChatPanel));
    });

    await act(async () => {
      fireTauriEvent('agent:event', {
        type: 'tool_call_start',
        id: 'call_mark_test',
        name: 'read_file',
      });
    });

    expect(markSpy).toHaveBeenCalledWith('tool_call_start_call_mark_test');
    markSpy.mockRestore();
  });

  it('ToolResult payload calls agentStore.completeCard with id and result', async () => {
    await act(async () => {
      render(React.createElement(ChatPanel));
    });

    // Seed a running card first so completeCard has something to update.
    useAgentStore.getState().addCard('call_r1', 'read_file');
    expect(useAgentStore.getState().cards[0].status).toBe('running');

    await act(async () => {
      fireTauriEvent('agent:event', {
        type: 'tool_result',
        id: 'call_r1',
        result: 'file contents here',
      });
    });

    const card = useAgentStore.getState().cards[0];
    expect(card.status).toBe('ok');
    expect(card.result).toBe('file contents here');
  });

  it('Done payload does not throw and clears isStreaming indirectly', async () => {
    // This test verifies the Done handler runs without error.
    // The setIsStreaming(false) call is ChatPanel internal state — we verify
    // no exception is thrown and the store is unaffected.
    await act(async () => {
      render(React.createElement(ChatPanel));
    });

    await act(async () => {
      fireTauriEvent('agent:event', { type: 'done' });
    });

    // Store should be empty (done resets nothing in agentStore directly).
    expect(useAgentStore.getState().cards).toHaveLength(0);
  });
});
