// src/components/ChatPanel.tsx
//
// Phase 5 deliverable: Anthropic streaming E2E chat panel.
//
// - react-virtuoso-virtualized message list
// - react-markdown + remark-gfm for markdown rendering
// - Model picker backed by anthropic_list_models command
// - Ctrl+L (insert selection as quote) and Ctrl+Shift+L (new chat) from Phase 2
// - Streaming tokens via `biscuitcode:chat-event:<convId>` Tauri event
// - SQLite persistence via chat_create_conversation / chat_list_messages

import { useCallback, useEffect, useRef, useState } from 'react';
import { Virtuoso, VirtuosoHandle } from 'react-virtuoso';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';

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

  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

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
  }, []);

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

  // Send a message.
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
  }, [input, isStreaming, ensureConversation, selectedModel, t]);

  // Ctrl+L — insert editor selection as a quoted block (Phase 2 shortcut).
  // The selection text is stored in localStorage by the editor shortcut handler.
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

  // Ctrl+Shift+L — new conversation.
  const handleCtrlShiftL = useCallback(() => {
    setMessages([]);
    setConversationId(null);
    setInput('');
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    textareaRef.current?.focus();
  }, []);

  // Keyboard handlers on the textarea.
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
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
    [handleSend, handleCtrlL, handleCtrlShiftL],
  );

  // Cleanup listener on unmount.
  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, []);

  // ---------- Render helpers ----------

  const renderMessage = (msg: ChatMessage) => {
    const isUser = msg.role === 'user';
    return (
      <div
        key={msg.id}
        className={`px-3 py-2 text-sm ${isUser ? 'text-cocoa-100' : 'text-cocoa-50'}`}
      >
        <div
          className={`text-[11px] font-semibold uppercase tracking-wider mb-1 ${
            isUser ? 'text-biscuit-400' : 'text-cocoa-300'
          }`}
        >
          {isUser ? t('chat.you') : t('chat.assistant')}
        </div>
        <div className="prose prose-invert prose-sm max-w-none">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>
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
      <div className="flex-1 overflow-hidden">
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
      <footer className="border-t border-cocoa-500 p-2 flex flex-col gap-2">
        <textarea
          ref={textareaRef}
          aria-label={t('chat.inputLabel')}
          placeholder={t('chat.inputPlaceholder')}
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
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
