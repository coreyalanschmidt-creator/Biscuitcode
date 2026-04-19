// src/components/ChatPanel.tsx
//
// Phase 5 deliverable: Anthropic streaming E2E chat panel.
// Phase 6a additions:
//   - Agent mode toggle (auto-continues on tool calls when ON).
//   - @-mention file picker (keyboard-driven quick selector).
//   - Drag-and-drop file-into-chat inserts @file:<path> token.
//   - Dispatches tool_call_start/end events into agentStore.
//   - performance.mark('tool_call_start_<id>') on ToolCallStart events.
//
// - react-virtuoso-virtualized message list
// - react-markdown + remark-gfm for markdown rendering
// - Model picker backed by anthropic_list_models command
// - Ctrl+L (insert selection as quote) and Ctrl+Shift+L (new chat) from Phase 2
// - Streaming tokens via `biscuitcode:chat-event:<convId>` Tauri event
// - SQLite persistence via chat_create_conversation / chat_list_messages

import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Virtuoso, VirtuosoHandle } from 'react-virtuoso';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { useAgentStore } from '../state/agentStore';
import { useLspStore } from '../state/lspStore';

// ---------- Types ----------

interface ModelInfo {
  id: string;
  display_name: string;
  legacy: boolean;
  is_reasoning_model: boolean;
}

interface ChatMessage {
  id: string;           // local client-side key
  role: 'user' | 'assistant';
  text: string;         // markdown content
  streaming: boolean;
  /** DB message_id — set after the message is persisted. */
  dbMessageId?: string;
  /** True if this assistant message performed write/shell tools (has a snapshot). */
  hasSnapshot?: boolean;
}

interface ChatEventPayload {
  type: 'text_delta' | 'thinking_delta' | 'tool_call_start' | 'tool_call_delta' |
        'tool_call_end' | 'done' | 'error';
  text?: string;
  id?: string;
  name?: string;
  args_delta?: string;
  args_json?: string;
  stop_reason?: string;
  usage?: {
    input_tokens: number;
    output_tokens: number;
    cache_read_input_tokens?: number;
  };
  code?: string;
  message?: string;
  recoverable?: boolean;
}

// ---------- @-mention picker types ----------

interface MentionCandidate {
  path: string;
  label: string;
  /** Non-file special mention type */
  specialType?: 'terminal-output' | 'problems' | 'git-diff';
  /** True if the mention source has no data and should be shown disabled */
  disabled?: boolean;
}

// Placeholder workspace/conversation IDs for Phase 5.
// Full workspace integration lands in Phase 3's UI wiring + Phase 8's onboarding.
const PHASE5_WORKSPACE_ID = 'wks_phase5_default';
const PHASE5_CONV_TITLE = 'Chat';

// ---------- Component ----------

export function ChatPanel() {
  const { t } = useTranslation();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedModel, setSelectedModel] = useState('claude-opus-4-7');
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [keyPresent, setKeyPresent] = useState<boolean | null>(null);

  // Mention picker state.
  const [mentionOpen, setMentionOpen] = useState(false);
  const [mentionQuery, setMentionQuery] = useState('');
  const [mentionIndex, setMentionIndex] = useState(0);
  const [mentionCandidates, setMentionCandidates] = useState<MentionCandidate[]>([]);

  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // agentStore actions.
  const agentMode = useAgentStore((s) => s.agentMode);
  const setAgentMode = useAgentStore((s) => s.setAgentMode);
  const setStoreConvId = useAgentStore((s) => s.setConversationId);
  const startCard = useAgentStore((s) => s.startCard);
  const appendArgsDelta = useAgentStore((s) => s.appendArgsDelta);
  const endCard = useAgentStore((s) => s.endCard);
  const clearCards = useAgentStore((s) => s.clearCards);

  // lspStore for @problems mention data availability check.
  const lspDiagnostics = useLspStore((s) => s.diagnostics);

  // Track whether any terminals are open (for @terminal-output availability).
  const [hasTerminals, setHasTerminals] = useState(false);
  useEffect(() => {
    const handler = (e: Event) => {
      const { open } = (e as CustomEvent).detail ?? {};
      setHasTerminals(Boolean(open));
    };
    window.addEventListener('biscuitcode:terminal-count', handler);
    return () => window.removeEventListener('biscuitcode:terminal-count', handler);
  }, []);

  // Load models and check key on mount.
  useEffect(() => {
    (async () => {
      try {
        const present = await invoke<boolean>('anthropic_key_present');
        setKeyPresent(present);
      } catch {
        setKeyPresent(false);
      }

      try {
        const ms = await invoke<ModelInfo[]>('anthropic_list_models');
        setModels(ms);
        if (ms.length > 0 && !ms.find(m => m.id === selectedModel)) {
          setSelectedModel(ms[0].id);
        }
      } catch {
        // Silently ignore — model picker will show empty.
      }
    })();
  // selectedModel intentionally omitted — we only want this on mount.
  }, []);

  // Sync conversationId into agentStore so AgentActivityPanel can reference it.
  useEffect(() => {
    setStoreConvId(conversationId);
  }, [conversationId, setStoreConvId]);

  // Ensure a conversation exists.
  const ensureConversation = useCallback(async (): Promise<string> => {
    if (conversationId) return conversationId;
    const id = await invoke<string>('chat_create_conversation', {
      workspaceId: PHASE5_WORKSPACE_ID,
      title: PHASE5_CONV_TITLE,
      model: selectedModel,
    });
    setConversationId(id);
    return id;
  }, [conversationId, selectedModel]);

  // ---------- Mention picker ----------

  // Refresh candidates from workspace file list + special mentions whenever query changes.
  useEffect(() => {
    if (!mentionOpen) return;
    let cancelled = false;

    // Build special non-file mention candidates (always shown first when query is empty
    // or matches the keyword).
    const buildSpecials = (): MentionCandidate[] => {
      const specials: MentionCandidate[] = [];
      const q = mentionQuery.toLowerCase();
      const matchesOrEmpty = (kw: string) => !q || kw.startsWith(q);

      if (matchesOrEmpty('terminal-output')) {
        specials.push({
          path: '@terminal-output',
          label: '@terminal-output',
          specialType: 'terminal-output',
          disabled: !hasTerminals,
        });
      }
      if (matchesOrEmpty('problems')) {
        specials.push({
          path: '@problems',
          label: '@problems',
          specialType: 'problems',
          disabled: lspDiagnostics.length === 0,
        });
      }
      if (matchesOrEmpty('git-diff')) {
        // git-diff is only useful when a workspace is open.
        // Check via window global set by WorkspaceRoot context (EditorStore).
        const hasWorkspace = Boolean(
          (window as Window & { __BISCUIT_WORKSPACE_ROOT__?: string }).__BISCUIT_WORKSPACE_ROOT__
        );
        specials.push({
          path: '@git-diff',
          label: '@git-diff',
          specialType: 'git-diff',
          disabled: !hasWorkspace,
        });
      }
      return specials;
    };

    (async () => {
      const specials = buildSpecials();

      try {
        const files = await invoke<string[]>('fs_search_files', {
          query: mentionQuery,
          limit: 20,
        });
        if (!cancelled) {
          const fileCandidates = files.map((p) => ({
            path: p,
            label: p.split('/').pop() ?? p,
            disabled: false as boolean,
          }));
          const all = [...specials, ...fileCandidates];
          setMentionCandidates(all);
          // Start index on the first non-disabled candidate.
          const firstEnabled = all.findIndex((c) => !c.disabled);
          setMentionIndex(firstEnabled === -1 ? specials.length : firstEnabled);
        }
      } catch {
        if (!cancelled) {
          setMentionCandidates(specials);
          const firstEnabled = specials.findIndex((c) => !c.disabled);
          setMentionIndex(firstEnabled === -1 ? 0 : firstEnabled);
        }
      }
    })();
    return () => { cancelled = true; };
  }, [mentionOpen, mentionQuery, hasTerminals, lspDiagnostics.length]);

  /** Insert a mention token at the current @ position.
   *
   * Special mentions (@terminal-output, @problems, @git-diff) fetch their
   * content inline so the LLM receives the actual data in the user message.
   * File mentions use the @file:<path> token convention.
   */
  const commitMention = useCallback(async (candidate: MentionCandidate) => {
    const { path, specialType, disabled } = candidate;
    if (disabled) return;

    let token: string;

    if (specialType === 'terminal-output') {
      // Request visible terminal buffer from TerminalPanel.
      const event = new CustomEvent('biscuitcode:get-terminal-output', {});
      window.dispatchEvent(event);
      // TerminalPanel responds synchronously via a side-channel.
      const output = (window as Window & { __BISCUIT_TERMINAL_OUTPUT__?: string }).__BISCUIT_TERMINAL_OUTPUT__ ?? '';
      token = `\`\`\`terminal-output\n${output}\n\`\`\``;

    } else if (specialType === 'problems') {
      const diagLines = lspDiagnostics
        .slice(0, 50) // cap at 50 to avoid huge prompts
        .map((d) => `${d.path}:${d.line} — ${d.message}`)
        .join('\n');
      token = `\`\`\`problems\n${diagLines}\n\`\`\``;

    } else if (specialType === 'git-diff') {
      try {
        const diff = await invoke<string>('git_diff_all');
        token = diff ? `\`\`\`diff\n${diff}\n\`\`\`` : '(no git changes)';
      } catch {
        token = '(git diff unavailable)';
      }

    } else {
      token = `@file:${path}`;
    }

    setInput((prev) => {
      const atIdx = prev.lastIndexOf('@');
      if (atIdx === -1) return `${prev}${token} `;
      return `${prev.slice(0, atIdx)}${token} `;
    });
    setMentionOpen(false);
    setMentionQuery('');
    textareaRef.current?.focus();
  }, [lspDiagnostics]);

  // ---------- Send ----------

  const handleSend = useCallback(async () => {
    const text = input.trim();
    if (!text || isStreaming) return;

    setInput('');
    setIsStreaming(true);

    // Optimistically add the user message.
    const userMsgId = `user-${Date.now()}`;
    setMessages(prev => [
      ...prev,
      { id: userMsgId, role: 'user', text, streaming: false },
    ]);

    // Add a placeholder for the assistant reply.
    const assistantMsgId = `asst-${Date.now()}`;
    setMessages(prev => [
      ...prev,
      { id: assistantMsgId, role: 'assistant', text: '', streaming: true },
    ]);

    // Scroll to bottom.
    virtuosoRef.current?.scrollToIndex({ index: 'LAST', behavior: 'smooth' });

    let convId: string;
    try {
      convId = await ensureConversation();
    } catch {
      setMessages(prev =>
        prev.map(m =>
          m.id === assistantMsgId
            ? { ...m, text: t('chat.errorNoKey'), streaming: false }
            : m,
        ),
      );
      setIsStreaming(false);
      return;
    }

    // Subscribe to streaming events.
    const eventChannel = `biscuitcode:chat-event:${convId}`;
    if (unlistenRef.current) {
      unlistenRef.current();
    }
    unlistenRef.current = await listen<ChatEventPayload>(eventChannel, (evt) => {
      const payload = evt.payload;

      if (payload.type === 'text_delta' && payload.text) {
        setMessages(prev =>
          prev.map(m =>
            m.id === assistantMsgId
              ? { ...m, text: m.text + payload.text }
              : m,
          ),
        );
        virtuosoRef.current?.scrollToIndex({ index: 'LAST', behavior: 'auto' });

      } else if (payload.type === 'tool_call_start' && payload.id && payload.name) {
        // Emit the render-gate start mark.
        performance.mark(`tool_call_start_${payload.id}`);
        startCard(payload.id, payload.name);

      } else if (payload.type === 'tool_call_delta' && payload.id && payload.args_delta) {
        appendArgsDelta(payload.id, payload.args_delta);

      } else if (payload.type === 'tool_call_end' && payload.id) {
        endCard(
          payload.id,
          payload.args_json ?? '',
          payload.text ?? null,
          false,
        );

      } else if (payload.type === 'done') {
        setMessages(prev =>
          prev.map(m =>
            m.id === assistantMsgId ? { ...m, streaming: false } : m,
          ),
        );
        setIsStreaming(false);
        if (unlistenRef.current) {
          unlistenRef.current();
          unlistenRef.current = null;
        }

      } else if (payload.type === 'error') {
        // If error comes back on a tool call in flight, mark it as errored.
        if (payload.id) {
          endCard(payload.id, '', payload.message ?? null, true);
        }
        setMessages(prev =>
          prev.map(m =>
            m.id === assistantMsgId
              ? {
                  ...m,
                  text: m.text || t('chat.errorStream', { code: payload.code }),
                  streaming: false,
                }
              : m,
          ),
        );
        setIsStreaming(false);
        if (unlistenRef.current) {
          unlistenRef.current();
          unlistenRef.current = null;
        }
      }
    });

    // Issue the command.
    try {
      await invoke('chat_send', {
        req: {
          conversation_id: convId,
          workspace_id: PHASE5_WORKSPACE_ID,
          model: selectedModel,
          text,
          system: null,
          parent_message_id: null,
          agent_mode: agentMode,
        },
      });
    } catch (e) {
      const errMsg = typeof e === 'string' ? e : t('chat.errorSend');
      setMessages(prev =>
        prev.map(m =>
          m.id === assistantMsgId
            ? { ...m, text: errMsg, streaming: false }
            : m,
        ),
      );
      setIsStreaming(false);
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    }
  }, [input, isStreaming, ensureConversation, selectedModel, t, agentMode,
      startCard, appendArgsDelta, endCard]);

  // ---------- Ctrl+L / Ctrl+Shift+L ----------

  const handleCtrlL = useCallback(() => {
    const selection = window.__BISCUIT_SELECTION_FOR_CHAT__;
    if (selection) {
      setInput(prev => {
        const quote = selection
          .split('\n')
          .map((l: string) => `> ${l}`)
          .join('\n');
        return prev ? `${prev}\n${quote}\n` : `${quote}\n`;
      });
      window.__BISCUIT_SELECTION_FOR_CHAT__ = undefined;
      textareaRef.current?.focus();
    }
  }, []);

  const handleCtrlShiftL = useCallback(() => {
    setMessages([]);
    setConversationId(null);
    setInput('');
    clearCards();
    setMentionOpen(false);
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    textareaRef.current?.focus();
  }, [clearCards]);

  // ---------- Textarea onChange — @-mention trigger (PM-05 fix) ----------

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);

    // Detect "@" trigger: the character just typed is "@" and we're at word start.
    // We check the updated value (not e.key) so the trigger works for pastes too.
    const lastAt = val.lastIndexOf('@');
    if (lastAt !== -1) {
      const after = val.slice(lastAt + 1);
      // Only open the picker if there's no space after the @.
      if (!after.includes(' ') && !after.includes('\n')) {
        setMentionOpen(true);
        setMentionQuery(after);
        return;
      }
    }
    setMentionOpen(false);
    setMentionQuery('');
  }, []);

  // ---------- Keyboard handler ----------

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // Mention picker navigation takes priority.
      if (mentionOpen) {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          setMentionIndex((i) => {
            // Find next non-disabled candidate.
            for (let j = i + 1; j < mentionCandidates.length; j++) {
              if (!mentionCandidates[j].disabled) return j;
            }
            return i;
          });
          return;
        }
        if (e.key === 'ArrowUp') {
          e.preventDefault();
          setMentionIndex((i) => {
            for (let j = i - 1; j >= 0; j--) {
              if (!mentionCandidates[j].disabled) return j;
            }
            return i;
          });
          return;
        }
        if (e.key === 'Enter' || e.key === 'Tab') {
          e.preventDefault();
          const c = mentionCandidates[mentionIndex];
          if (c) commitMention(c);
          else setMentionOpen(false);
          return;
        }
        if (e.key === 'Escape') {
          e.preventDefault();
          setMentionOpen(false);
          return;
        }
      }

      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
      if (e.key === 'l' && e.ctrlKey && !e.shiftKey) {
        e.preventDefault();
        handleCtrlL();
      }
      if (e.key === 'L' && e.ctrlKey && e.shiftKey) {
        e.preventDefault();
        handleCtrlShiftL();
      }
    },
    [handleSend, handleCtrlL, handleCtrlShiftL, mentionOpen, mentionCandidates, mentionIndex, commitMention],
  );

  // ---------- Drag-and-drop ----------

  const handleDragOver = useCallback((e: React.DragEvent<HTMLTextAreaElement>) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
  }, []);

  const handleDrop = useCallback((e: React.DragEvent<HTMLTextAreaElement>) => {
    e.preventDefault();
    // Files can be dragged from the file tree; the tree sets a
    // "biscuitcode/file-path" data transfer item (or falls back to plain text).
    const path =
      e.dataTransfer.getData('biscuitcode/file-path') ||
      e.dataTransfer.getData('text/plain');
    if (path) {
      setInput((prev) => {
        const token = `@file:${path} `;
        return prev ? `${prev} ${token}` : token;
      });
      textareaRef.current?.focus();
    }
  }, []);

  // ---------- Cleanup listener on unmount ----------

  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, []);

  // ---------- Rewind ----------

  const handleRewind = useCallback(async (msg: ChatMessage) => {
    if (!msg.dbMessageId || !conversationId) return;
    try {
      // Use a sensible default cache root. Phase 8 will wire this from app dir.
      const cacheRoot = `${(window as Window & { __BISCUIT_CACHE_ROOT__?: string }).__BISCUIT_CACHE_ROOT__ ?? '/tmp/biscuitcode-cache'}`;
      await invoke('agent_rewind', {
        req: {
          conversation_id: conversationId,
          rewind_to_message_id: msg.dbMessageId,
          cache_root: cacheRoot,
        },
      });
      // Remove messages after the rewound message from local state.
      setMessages((prev) => {
        const idx = prev.findIndex((m) => m.id === msg.id);
        return idx === -1 ? prev : prev.slice(0, idx + 1);
      });
    } catch (e) {
      const errMsg = typeof e === 'string' ? e : t('chat.rewindError');
      // eslint-disable-next-line no-console
      console.error('Rewind failed:', errMsg);
    }
  }, [conversationId, t]);

  // ---------- Code block components (Apply / Run) ----------

  const CodeBlock = useCallback(({ children, className }: { children?: React.ReactNode; className?: string }) => {
    const code = String(children ?? '').replace(/\n$/, '');
    const language = className?.replace('language-', '') ?? '';
    return (
      <div className="relative group my-2">
        <pre className={`${className ?? ''} rounded bg-cocoa-800 p-3 text-xs font-mono overflow-x-auto`}>
          <code>{code}</code>
        </pre>
        <div className="absolute top-1.5 right-1.5 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <button
            aria-label={t('chat.applyCode')}
            title={t('chat.applyCode')}
            onClick={() => {
              // Dispatch to EditorArea to apply patch / write contents.
              window.dispatchEvent(new CustomEvent('biscuitcode:apply-code', { detail: { code, language } }));
            }}
            className="text-[10px] px-1.5 py-0.5 rounded bg-cocoa-600 border border-cocoa-400 text-cocoa-200 hover:bg-biscuit-500 hover:text-cocoa-900 transition-colors"
          >
            {t('chat.apply')}
          </button>
          <button
            aria-label={t('chat.runCode')}
            title={t('chat.runCode')}
            onClick={() => {
              // Push code to a new terminal tab (no auto-exec — user hits Enter).
              window.dispatchEvent(new CustomEvent('biscuitcode:run-code', { detail: { code } }));
            }}
            className="text-[10px] px-1.5 py-0.5 rounded bg-cocoa-600 border border-cocoa-400 text-cocoa-200 hover:bg-accent-ok hover:text-cocoa-900 transition-colors"
          >
            {t('chat.run')}
          </button>
        </div>
      </div>
    );
  }, [t]);

  // ---------- Render helpers ----------

  const renderMessage = (msg: ChatMessage) => {
    const isUser = msg.role === 'user';
    return (
      <div
        key={msg.id}
        className={`px-3 py-2 text-sm ${isUser ? 'text-cocoa-100' : 'text-cocoa-50'}`}
      >
        <div
          className={`text-[11px] font-semibold uppercase tracking-wider mb-1 flex items-center gap-2 ${
            isUser ? 'text-biscuit-400' : 'text-cocoa-300'
          }`}
        >
          <span>{isUser ? t('chat.you') : t('chat.assistant')}</span>
          {/* Rewind button — only on non-streaming assistant messages with a snapshot */}
          {!isUser && !msg.streaming && msg.hasSnapshot && (
            <button
              aria-label={t('chat.rewindLabel')}
              title={t('chat.rewindLabel')}
              onClick={() => handleRewind(msg)}
              className="ml-auto text-[10px] px-1.5 py-0.5 rounded border border-cocoa-400 text-cocoa-400 hover:text-accent-warn hover:border-accent-warn/40 transition-colors"
            >
              ↩ {t('chat.rewind')}
            </button>
          )}
        </div>
        <div className="prose prose-invert prose-sm max-w-none">
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            components={{
              code: ({ children, className, ...props }) => {
                const isBlock = String(children).includes('\n') || className;
                if (isBlock) {
                  return <CodeBlock className={className}>{children}</CodeBlock>;
                }
                return <code className="bg-cocoa-800 px-1 rounded text-xs font-mono" {...props}>{children}</code>;
              },
            }}
          >
            {msg.text || (msg.streaming ? '▋' : '')}
          </ReactMarkdown>
        </div>
      </div>
    );
  };

  const noKeyBanner = keyPresent === false ? (
    <div className="mx-3 my-2 rounded border border-accent-warn/40 bg-cocoa-600 px-3 py-2 text-xs text-accent-warn">
      {t('chat.noKeyBanner')}
    </div>
  ) : null;

  return (
    <aside
      aria-label={t('panels.chatPanel')}
      className="h-full bg-cocoa-700 border-l border-cocoa-500 flex flex-col"
    >
      {/* Header + model picker */}
      <header className="px-3 py-2 border-b border-cocoa-500 flex items-center gap-2 min-h-[40px]">
        <span className="text-xs font-semibold uppercase tracking-wider text-cocoa-200 flex-1">
          {t('panels.chats')}
        </span>
        <select
          aria-label={t('chat.modelPickerLabel')}
          value={selectedModel}
          onChange={e => setSelectedModel(e.target.value)}
          className="text-xs bg-cocoa-600 border border-cocoa-400 rounded px-1 py-0.5 text-cocoa-100 cursor-pointer"
        >
          {models.map(m => (
            <option key={m.id} value={m.id}>
              {m.display_name}
              {m.legacy ? ' ⚠' : ''}
            </option>
          ))}
          {models.length === 0 && (
            <option value="claude-opus-4-7">Claude Opus 4.7</option>
          )}
        </select>
        {/* Agent mode toggle */}
        <label
          className="flex items-center gap-1 cursor-pointer"
          title={t('chat.agentModeTitle')}
        >
          <input
            type="checkbox"
            aria-label={t('chat.agentModeLabel')}
            checked={agentMode}
            onChange={(e) => setAgentMode(e.target.checked)}
            className="accent-biscuit-500 w-3 h-3"
          />
          <span className="text-[10px] text-cocoa-300 select-none">
            {t('chat.agentMode')}
          </span>
        </label>
        <button
          aria-label={t('chat.newChat')}
          title={t('chat.newChat')}
          onClick={handleCtrlShiftL}
          className="text-xs text-cocoa-300 hover:text-cocoa-100 px-1"
        >
          +
        </button>
      </header>

      {noKeyBanner}

      {/* Message list */}
      <div className="flex-1 overflow-hidden" role="log" aria-live="polite" aria-label={t('panels.chatPanel')}>
        {messages.length === 0 ? (
          <div className="h-full flex items-center justify-center text-xs text-cocoa-400 px-4 text-center">
            {t('chat.emptyHint')}
          </div>
        ) : (
          <Virtuoso
            ref={virtuosoRef}
            data={messages}
            itemContent={(_index, msg) => renderMessage(msg)}
            followOutput="smooth"
            className="h-full"
          />
        )}
      </div>

      {/* Input area */}
      <footer className="border-t border-cocoa-500 p-2 flex flex-col gap-2 relative">
        {/* @-mention picker popup */}
        {mentionOpen && (
          <div
            role="listbox"
            aria-label={t('chat.mentionPickerLabel')}
            className="absolute bottom-full left-2 right-2 mb-1 bg-cocoa-600 border border-cocoa-400 rounded shadow-lg max-h-48 overflow-y-auto z-50"
          >
            {mentionCandidates.length === 0 ? (
              <div className="px-3 py-2 text-xs text-cocoa-400">
                {t('chat.mentionNoResults')}
              </div>
            ) : (
              mentionCandidates.map((c, i) => (
                <button
                  key={c.path}
                  role="option"
                  aria-selected={i === mentionIndex}
                  aria-disabled={c.disabled}
                  type="button"
                  onClick={() => !c.disabled && commitMention(c)}
                  className={`w-full text-left px-3 py-1.5 text-xs font-mono ${
                    c.disabled
                      ? 'text-cocoa-500 cursor-not-allowed'
                      : i === mentionIndex
                      ? 'bg-biscuit-500 text-cocoa-900'
                      : 'text-cocoa-100 hover:bg-cocoa-500'
                  }`}
                >
                  {c.label}
                  {c.disabled && (
                    <span className="ml-2 text-cocoa-500 text-[10px]">
                      {c.specialType === 'terminal-output'
                        ? t('mentions.noTerminals')
                        : c.specialType === 'problems'
                        ? t('mentions.noProblems')
                        : ''}
                    </span>
                  )}
                  {!c.specialType && !c.disabled && (
                    <span className="ml-2 text-cocoa-400 text-[10px]">{c.path}</span>
                  )}
                </button>
              ))
            )}
          </div>
        )}

        <textarea
          ref={textareaRef}
          aria-label={t('chat.inputLabel')}
          placeholder={t('chat.inputPlaceholder')}
          value={input}
          onChange={handleInputChange}
          onKeyDown={handleKeyDown}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
          rows={3}
          className="w-full resize-none rounded bg-cocoa-600 border border-cocoa-400 text-sm text-cocoa-100 placeholder:text-cocoa-400 px-2 py-1.5 focus:outline-none focus:border-biscuit-500"
          disabled={isStreaming}
        />
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-cocoa-400">
            {t('chat.shortcutHint')}
          </span>
          <button
            aria-label={t('chat.sendButton')}
            onClick={handleSend}
            disabled={isStreaming || !input.trim()}
            className="px-3 py-1 rounded bg-biscuit-500 text-cocoa-900 text-xs font-semibold disabled:opacity-40 hover:bg-biscuit-400 transition-colors"
          >
            {isStreaming ? t('chat.sending') : t('chat.sendButton')}
          </button>
        </div>
      </footer>
    </aside>
  );
}

// Augment Window type for the Ctrl+L selection hand-off.
declare global {
  interface Window {
    __BISCUIT_SELECTION_FOR_CHAT__?: string;
  }
}
