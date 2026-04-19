// src/components/InlineEditPane.tsx
//
// Phase 6b deliverable: inline AI edit (Ctrl+K Ctrl+I).
//
// Flow:
//   1. User selects code in Monaco and presses Ctrl+K Ctrl+I.
//   2. EditorArea fires `biscuitcode:inline-edit-open` with { path, selection, selectedText }.
//   3. This component renders a popover with a description input.
//   4. On submit, calls the backend `chat_inline_edit` command (streaming).
//   5. Streams the edit; opens a Monaco split-diff pane showing original vs. proposed.
//   6. Accept → writes the proposed content to the file; Reject → discards.
//   7. Regenerate → re-runs from step 4 with the same prompt.
//
// Zed-style split-diff: uses `monaco.editor.createDiffEditor` on a hidden
// container that becomes visible when the diff lands.

import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { useMonaco } from '@monaco-editor/react';

// ---------- Types ----------

interface InlineEditEvent {
  path: string;
  // 1-based start and end lines.
  startLine: number;
  endLine: number;
  selectedText: string;
}

// ---------- Component ----------

export function InlineEditPane() {
  const { t } = useTranslation();
  const monaco = useMonaco();

  const [event, setEvent] = useState<InlineEditEvent | null>(null);
  const [description, setDescription] = useState('');
  const [proposedContent, setProposedContent] = useState<string | null>(null);
  const [originalContent, setOriginalContent] = useState<string | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const diffEditorRef = useRef<import('monaco-editor').editor.IStandaloneDiffEditor | null>(null);
  const diffContainerRef = useRef<HTMLDivElement | null>(null);

  // Listen for inline-edit open events from EditorArea.
  useEffect(() => {
    // Listen via custom window event (EditorArea fires this synchronously).
    const handler = (e: Event) => {
      const detail = (e as CustomEvent<InlineEditEvent>).detail;
      setEvent(detail);
      setDescription('');
      setProposedContent(null);
      setOriginalContent(detail.selectedText);
      setError(null);
    };
    window.addEventListener('biscuitcode:inline-edit-open', handler);
    return () => {
      window.removeEventListener('biscuitcode:inline-edit-open', handler);
    };
  }, []);

  // Wire the Monaco diff editor when we have both original and proposed.
  useEffect(() => {
    if (!monaco || !diffContainerRef.current) return;
    if (originalContent === null || proposedContent === null) return;

    if (!diffEditorRef.current) {
      diffEditorRef.current = monaco.editor.createDiffEditor(diffContainerRef.current, {
        enableSplitViewResizing: false,
        renderSideBySide: true,
        readOnly: true,
        minimap: { enabled: false },
        fontFamily: "'JetBrains Mono', 'Ubuntu Mono', monospace",
        fontSize: 13,
        automaticLayout: true,
        scrollBeyondLastLine: false,
      });
    }

    const originalModel = monaco.editor.createModel(originalContent, 'plaintext');
    const modifiedModel = monaco.editor.createModel(proposedContent, 'plaintext');
    diffEditorRef.current.setModel({ original: originalModel, modified: modifiedModel });
  }, [monaco, originalContent, proposedContent]);

  // Cleanup diff editor on unmount / close.
  const closeDiff = useCallback(() => {
    if (diffEditorRef.current) {
      diffEditorRef.current.dispose();
      diffEditorRef.current = null;
    }
  }, []);

  const handleClose = useCallback(() => {
    closeDiff();
    setEvent(null);
    setProposedContent(null);
    setOriginalContent(null);
    setError(null);
  }, [closeDiff]);

  const handleSubmit = useCallback(async () => {
    if (!event || !description.trim() || streaming) return;

    setStreaming(true);
    setProposedContent('');
    setError(null);

    let unlisten: UnlistenFn | null = null;
    const eventChannel = `biscuitcode:inline-edit-delta:${event.path}`;

    try {
      unlisten = await listen<{ delta?: string; done?: boolean; error?: string }>(
        eventChannel,
        (evt) => {
          if (evt.payload.error) {
            setError(evt.payload.error);
            setStreaming(false);
            unlisten?.();
            return;
          }
          if (evt.payload.delta) {
            setProposedContent((prev) => (prev ?? '') + evt.payload.delta);
          }
          if (evt.payload.done) {
            setStreaming(false);
            unlisten?.();
          }
        },
      );

      await invoke('chat_inline_edit', {
        req: {
          file_path: event.path,
          start_line: event.startLine,
          end_line: event.endLine,
          selected_text: event.selectedText,
          description: description.trim(),
        },
      });
    } catch (e) {
      setError(typeof e === 'string' ? e : t('agent.inlineEditError'));
      setStreaming(false);
      unlisten?.();
    }
  }, [event, description, streaming, t]);

  const handleAccept = useCallback(async () => {
    if (!event || proposedContent === null) return;
    try {
      // Write the proposed content back to the file at the selected range.
      await invoke('chat_apply_inline_edit', {
        req: {
          file_path: event.path,
          start_line: event.startLine,
          end_line: event.endLine,
          new_content: proposedContent,
        },
      });
      // Notify EditorArea to reload the file model.
      window.dispatchEvent(
        new CustomEvent('biscuitcode:editor-file-changed', {
          detail: { path: event.path },
        }),
      );
    } catch (e) {
      setError(typeof e === 'string' ? e : t('agent.inlineEditApplyError'));
      return;
    }
    handleClose();
  }, [event, proposedContent, handleClose, t]);

  const handleReject = useCallback(() => {
    closeDiff();
    setProposedContent(null);
    setError(null);
  }, [closeDiff]);

  const handleRegenerate = useCallback(async () => {
    closeDiff();
    setProposedContent(null);
    setError(null);
    await handleSubmit();
  }, [closeDiff, handleSubmit]);

  if (!event) return null;

  const hasDiff = proposedContent !== null && !streaming;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center pt-24 px-8"
      style={{ backgroundColor: 'rgba(8,5,4,0.6)' }}
      onClick={(e) => { if (e.target === e.currentTarget) handleClose(); }}
    >
      <div
        className="w-full max-w-3xl bg-cocoa-600 border border-cocoa-400 rounded shadow-2xl flex flex-col max-h-[70vh]"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-4 py-2.5 border-b border-cocoa-400 flex items-center gap-2">
          <span className="text-xs font-semibold uppercase tracking-wider text-biscuit-400 flex-1">
            {t('agent.inlineEditTitle')}
          </span>
          <span className="text-[10px] text-cocoa-400 font-mono truncate max-w-xs">
            {event.path.split('/').pop()} L{event.startLine}–{event.endLine}
          </span>
          <button
            aria-label={t('common.close')}
            onClick={handleClose}
            className="text-cocoa-300 hover:text-cocoa-50 text-sm px-1"
          >
            ×
          </button>
        </div>

        {/* Input row */}
        {!hasDiff && (
          <div className="px-4 py-3 flex gap-2">
            <input
              type="text"
              aria-label={t('agent.inlineEditInputLabel')}
              placeholder={t('agent.inlineEditPlaceholder')}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSubmit(); }
                if (e.key === 'Escape') handleClose();
              }}
              disabled={streaming}
              className="flex-1 text-sm bg-cocoa-700 border border-cocoa-400 rounded px-3 py-1.5 text-cocoa-100 placeholder-cocoa-400 focus:outline-none focus:border-biscuit-500"
              autoFocus
              style={{ fontFamily: "'Inter', 'Ubuntu', sans-serif" }}
            />
            <button
              onClick={handleSubmit}
              disabled={!description.trim() || streaming}
              className="text-xs px-3 py-1.5 rounded bg-biscuit-500 text-cocoa-900 font-semibold disabled:opacity-40 hover:bg-biscuit-400 transition-colors"
            >
              {streaming ? t('agent.inlineEditGenerating') : t('agent.inlineEditGenerate')}
            </button>
          </div>
        )}

        {/* Streaming indicator */}
        {streaming && (
          <div className="px-4 pb-2 text-xs text-cocoa-400 italic">
            {t('agent.inlineEditGenerating')}…
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="mx-4 mb-3 text-xs text-accent-error bg-accent-error/10 border border-accent-error/30 rounded px-3 py-2">
            {error}
          </div>
        )}

        {/* Diff editor */}
        {(proposedContent !== null) && (
          <div className="flex-1 min-h-0 px-4 pb-3 flex flex-col gap-2">
            <div
              ref={diffContainerRef}
              className="flex-1 min-h-[200px] rounded overflow-hidden border border-cocoa-500"
              aria-label={t('agent.inlineEditDiffLabel')}
            />
          </div>
        )}

        {/* Accept / Reject / Regenerate buttons */}
        {hasDiff && (
          <div className="px-4 py-3 border-t border-cocoa-500 flex items-center gap-2 justify-end">
            <button
              onClick={handleRegenerate}
              className="text-xs px-3 py-1.5 rounded border border-cocoa-400 text-cocoa-200 hover:bg-cocoa-500 transition-colors"
            >
              {t('agent.inlineEditRegenerate')}
            </button>
            <button
              onClick={handleReject}
              className="text-xs px-3 py-1.5 rounded border border-accent-error/40 text-accent-error hover:bg-accent-error/10 transition-colors"
            >
              {t('agent.inlineEditReject')}
            </button>
            <button
              onClick={handleAccept}
              className="text-xs px-3 py-1.5 rounded bg-accent-ok text-cocoa-900 font-semibold hover:opacity-90 transition-colors"
            >
              {t('agent.inlineEditAccept')}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
