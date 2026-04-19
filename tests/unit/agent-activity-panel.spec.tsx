// tests/unit/agent-activity-panel.spec.tsx
//
// Phase 6a frontend unit tests.
//
// Covers:
//   1. AgentActivityPanel renders empty hint when no cards.
//   2. AgentActivityPanel renders a tool-call card and emits
//      performance.mark('tool_card_visible_<id>') on mount.
//   3. Render-gate: tool_card_render_<id> measure duration < 250ms
//      (falsifies PM-04: the mark fires in useEffect, not a batched
//      MutationObserver, so the timing is synchronous post-commit).
//   4. ChatPanel @-mention picker opens when onChange value ends with '@'
//      (falsifies PM-05: triggered in onChange, not onKeyDown).
//   5. ChatPanel @-mention picker closes on Escape, commits on Enter,
//      and inserts '@file:<path> ' token.
//   6. ChatPanel drag-and-drop inserts '@file:<path> ' token.
//   7. Agent mode toggle in ChatPanel updates agentStore.
//   8. ChatPanel dispatches startCard / appendArgsDelta / endCard into
//      agentStore when tool-call events arrive on the Tauri event channel.

/// <reference types="@testing-library/jest-dom/vitest" />
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, act, fireEvent, cleanup } from '@testing-library/react';
import { expect as jestExpect } from 'vitest';
import * as matchers from '@testing-library/jest-dom/matchers';
import React from 'react';

// Extend vitest's expect with jest-dom matchers without polluting globals.
jestExpect.extend(matchers);
import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';

// ---------- i18n bootstrap (minimal) ----------

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
          agentMode: 'Agent',
          agentModeLabel: 'Agent mode',
          agentModeTitle: 'Agent mode tooltip',
          mentionPickerLabel: 'File mention picker',
          mentionNoResults: 'No matching files',
        },
      },
    },
  },
});

// ---------- react-virtuoso mock ----------
// react-virtuoso relies on DOM layout (ResizeObserver + scroll measurements)
// which don't exist in jsdom.  Replace Virtuoso with a simple pass-through
// renderer so items are visible to getByText/getByRole queries.

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
    <div className={className}>
      {data.map((item, i) => (
        <React.Fragment key={i}>{itemContent(i, item)}</React.Fragment>
      ))}
    </div>
  ),
}));

// ---------- Tauri mocks ----------

// We must mock @tauri-apps/api before importing any component that uses it.
// vitest's module mock hoisting means vi.mock calls run before imports.

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'anthropic_key_present') return false;
    if (cmd === 'anthropic_list_models') return [];
    if (cmd === 'fs_search_files') return [];
    return null;
  }),
}));

// listen returns a stub unlisten function; the returned listener is captured
// so tests can fire synthetic events.
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

/** Helper: fire a synthetic Tauri event to all registered listeners. */
function fireEvent_tauri(channel: string, payload: unknown) {
  (_listeners[channel] ?? []).forEach((fn) => fn({ payload }));
}

// ---------- Imports (after mocks) ----------

import { AgentActivityPanel } from '../../src/components/AgentActivityPanel';
import { ChatPanel } from '../../src/components/ChatPanel';
import { useAgentStore } from '../../src/state/agentStore';

// Reset store before each test.
beforeEach(() => {
  useAgentStore.setState({
    agentMode: false,
    conversationId: null,
    cards: [],
  });
  // Clear all Tauri listeners.
  Object.keys(_listeners).forEach((k) => { _listeners[k] = []; });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

// ---------- AgentActivityPanel tests ----------

describe('AgentActivityPanel', () => {
  it('renders empty hint when there are no cards', () => {
    render(<AgentActivityPanel />);
    expect(screen.getByText('No tool calls yet.')).toBeInTheDocument();
  });

  it('renders a tool-call card when the store has one card', async () => {
    await act(async () => {
      useAgentStore.getState().startCard('call_001', 'read_file');
    });

    render(<AgentActivityPanel />);

    expect(screen.getByText('read_file')).toBeInTheDocument();
  });

  it('emits performance.mark("tool_card_visible_<id>") on card mount — falsifies PM-04', async () => {
    const markSpy = vi.spyOn(performance, 'mark');

    // Place the start mark the way ChatPanel would.
    performance.mark('tool_call_start_call_002');

    await act(async () => {
      useAgentStore.getState().startCard('call_002', 'search_code');
    });

    render(<AgentActivityPanel />);

    // The mark is emitted inside useEffect which fires synchronously after
    // the React commit in @testing-library/react's act() wrapper.
    expect(markSpy).toHaveBeenCalledWith('tool_card_visible_call_002');
    markSpy.mockRestore();
  });

  it('render-gate: tool_card_render_<id> measure duration is under 250ms', async () => {
    // Place the start mark then mount the card.  Because both events happen
    // inside the same synchronous test, the elapsed wall-clock time is << 1ms
    // which is far below the 250ms gate.  This test falsifies PM-04 by showing
    // the measure is created at all (not lost to batching) and that its
    // duration satisfies the gate.
    performance.clearMarks();
    performance.clearMeasures();

    performance.mark('tool_call_start_call_003');

    await act(async () => {
      useAgentStore.getState().startCard('call_003', 'search_code');
    });

    render(<AgentActivityPanel />);

    const measures = performance.getEntriesByName('tool_card_render_call_003', 'measure');
    expect(measures.length).toBe(1);
    expect(measures[0].duration).toBeLessThan(250);
  });

  it('shows ok status icon when card ends successfully', async () => {
    await act(async () => {
      useAgentStore.getState().startCard('call_004', 'read_file');
      useAgentStore.getState().endCard('call_004', '{"path":"src/a.ts"}', 'file contents', false);
    });

    render(<AgentActivityPanel />);

    // The ok status icon is '✓'
    expect(screen.getByText('✓')).toBeInTheDocument();
  });

  it('shows error status icon when card ends with error', async () => {
    await act(async () => {
      useAgentStore.getState().startCard('call_005', 'read_file');
      useAgentStore.getState().endCard('call_005', '', null, true);
    });

    render(<AgentActivityPanel />);

    expect(screen.getByText('✗')).toBeInTheDocument();
  });

  it('shows result text when the card is done', async () => {
    await act(async () => {
      useAgentStore.getState().startCard('call_006', 'read_file');
      useAgentStore.getState().endCard('call_006', '{"path":"x.ts"}', 'hello world', false);
    });

    render(<AgentActivityPanel />);

    expect(screen.getByText('hello world')).toBeInTheDocument();
  });
});

// ---------- ChatPanel: agent mode toggle ----------

describe('ChatPanel agent mode toggle', () => {
  it('toggles agentMode in the store when the checkbox is clicked', async () => {
    render(<ChatPanel />);
    const checkbox = screen.getByLabelText('Agent mode') as HTMLInputElement;

    expect(checkbox.checked).toBe(false);

    await act(async () => {
      fireEvent.click(checkbox);
    });

    expect(useAgentStore.getState().agentMode).toBe(true);

    await act(async () => {
      fireEvent.click(checkbox);
    });

    expect(useAgentStore.getState().agentMode).toBe(false);
  });
});

// ---------- ChatPanel: @-mention picker (falsifies PM-05) ----------

describe('ChatPanel @-mention picker', () => {
  it('opens when the textarea value ends with "@" after typing — uses onChange not onKeyDown', async () => {
    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      // Simulate the user typing "@"; onChange receives the new value.
      fireEvent.change(textarea, { target: { value: '@' } });
    });

    // The picker should now be visible.
    expect(screen.getByRole('listbox', { name: 'File mention picker' })).toBeInTheDocument();
  });

  it('opens mid-string when the last @ has no space after it', async () => {
    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      fireEvent.change(textarea, { target: { value: 'read the file @src' } });
    });

    expect(screen.getByRole('listbox', { name: 'File mention picker' })).toBeInTheDocument();
  });

  it('closes when Escape is pressed', async () => {
    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      fireEvent.change(textarea, { target: { value: '@' } });
    });

    expect(screen.getByRole('listbox', { name: 'File mention picker' })).toBeInTheDocument();

    await act(async () => {
      fireEvent.keyDown(textarea, { key: 'Escape' });
    });

    expect(screen.queryByRole('listbox', { name: 'File mention picker' })).not.toBeInTheDocument();
  });

  it('closes when a space is typed after @', async () => {
    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      fireEvent.change(textarea, { target: { value: '@ ' } });
    });

    expect(screen.queryByRole('listbox', { name: 'File mention picker' })).not.toBeInTheDocument();
  });

  it('commits the selected candidate on Enter and inserts @file:<path> token', async () => {
    // Override invoke to return a specific file list for this test.
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'fs_search_files') return ['src/alpha.ts'];
      if (cmd === 'anthropic_key_present') return false;
      if (cmd === 'anthropic_list_models') return [];
      return null;
    });

    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      fireEvent.change(textarea, { target: { value: '@' } });
    });

    // Wait for the async invoke to populate candidates.
    await act(async () => {
      // Small tick for the useEffect invoke to resolve.
      await new Promise((r) => setTimeout(r, 0));
    });

    await act(async () => {
      fireEvent.keyDown(textarea, { key: 'Enter' });
      // commitMention is async — flush microtasks so the setInput fires.
      await new Promise((r) => setTimeout(r, 0));
    });

    // The textarea should now contain the @file token.
    expect(textarea.value).toBe('@file:src/alpha.ts ');
  });
});

// ---------- ChatPanel: drag-and-drop ----------

describe('ChatPanel drag-and-drop', () => {
  it('inserts @file:<path> token when a file is dropped onto the textarea', async () => {
    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      fireEvent.dragOver(textarea, {
        dataTransfer: { dropEffect: '' },
      });
      fireEvent.drop(textarea, {
        dataTransfer: {
          getData: (type: string) => {
            if (type === 'biscuitcode/file-path') return 'src/beta.ts';
            return '';
          },
        },
      });
    });

    expect(textarea.value).toBe('@file:src/beta.ts ');
  });

  it('falls back to text/plain when biscuitcode/file-path is empty', async () => {
    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      fireEvent.drop(textarea, {
        dataTransfer: {
          getData: (type: string) => {
            if (type === 'text/plain') return 'src/gamma.ts';
            return '';
          },
        },
      });
    });

    expect(textarea.value).toBe('@file:src/gamma.ts ');
  });
});

// ---------- ChatPanel: agentStore event dispatch ----------

describe('ChatPanel agentStore event dispatch', () => {
  it('calls startCard with tool id and name on tool_call_start event', async () => {
    // We need ChatPanel to have created a conversation so it subscribes.
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'chat_create_conversation') return 'conv_test_01';
      if (cmd === 'anthropic_key_present') return true;
      if (cmd === 'anthropic_list_models') return [];
      if (cmd === 'chat_send') return null;
      return null;
    });

    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    // Type and send a message to trigger conversation creation + event subscription.
    await act(async () => {
      fireEvent.change(textarea, { target: { value: 'hello' } });
    });
    await act(async () => {
      fireEvent.keyDown(textarea, { key: 'Enter' });
    });
    // Let async invoke calls settle.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });

    // Fire a tool_call_start event on the Tauri channel.
    await act(async () => {
      fireEvent_tauri('biscuitcode:chat-event:conv_test_01', {
        type: 'tool_call_start',
        id: 'call_tcs_01',
        name: 'search_code',
      });
    });

    const card = useAgentStore.getState().cards.find((c) => c.id === 'call_tcs_01');
    expect(card).toBeDefined();
    expect(card?.name).toBe('search_code');
    expect(card?.status).toBe('running');
  });

  it('calls endCard on tool_call_end event', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'chat_create_conversation') return 'conv_test_02';
      if (cmd === 'anthropic_key_present') return true;
      if (cmd === 'anthropic_list_models') return [];
      if (cmd === 'chat_send') return null;
      return null;
    });

    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      fireEvent.change(textarea, { target: { value: 'hello' } });
      fireEvent.keyDown(textarea, { key: 'Enter' });
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });

    // First emit start.
    await act(async () => {
      fireEvent_tauri('biscuitcode:chat-event:conv_test_02', {
        type: 'tool_call_start',
        id: 'call_tce_01',
        name: 'read_file',
      });
    });

    // Then emit end with result.
    await act(async () => {
      fireEvent_tauri('biscuitcode:chat-event:conv_test_02', {
        type: 'tool_call_end',
        id: 'call_tce_01',
        args_json: '{"path":"src/a.ts"}',
        text: 'file contents here',
      });
    });

    const card = useAgentStore.getState().cards.find((c) => c.id === 'call_tce_01');
    expect(card?.status).toBe('ok');
    expect(card?.result).toBe('file contents here');
    expect(card?.argsJson).toBe('{"path":"src/a.ts"}');
  });

  it('emits performance.mark("tool_call_start_<id>") when tool_call_start arrives', async () => {
    const markSpy = vi.spyOn(performance, 'mark');

    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'chat_create_conversation') return 'conv_test_03';
      if (cmd === 'anthropic_key_present') return true;
      if (cmd === 'anthropic_list_models') return [];
      if (cmd === 'chat_send') return null;
      return null;
    });

    render(<ChatPanel />);
    const textarea = screen.getByLabelText('Chat message') as HTMLTextAreaElement;

    await act(async () => {
      fireEvent.change(textarea, { target: { value: 'hello' } });
      fireEvent.keyDown(textarea, { key: 'Enter' });
    });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });

    await act(async () => {
      fireEvent_tauri('biscuitcode:chat-event:conv_test_03', {
        type: 'tool_call_start',
        id: 'call_mark_01',
        name: 'search_code',
      });
    });

    expect(markSpy).toHaveBeenCalledWith('tool_call_start_call_mark_01');
    markSpy.mockRestore();
  });
});
