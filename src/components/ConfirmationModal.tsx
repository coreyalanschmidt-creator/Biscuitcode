// src/components/ConfirmationModal.tsx
//
// Phase 6b deliverable.
//
// Renders when the backend emits `biscuitcode:confirm-request` with a
// ConfirmationRequest payload. The user can:
//   Approve    → invokes `agent_confirm_decision` with decision="approve"
//   Deny       → invokes with decision="deny"
//   Deny with feedback → invokes with decision="deny_with_feedback" + text
//
// This component is rendered by App.tsx and listens to the Tauri event
// channel globally (not per-conversation) so it works regardless of which
// panel is focused.

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';

// ---------- Types ----------

interface ConfirmationRequest {
  request_id: string;
  tool_class: 'write' | 'shell';
  summary: string;
  paths: string[];
}

// ---------- Component ----------

export function ConfirmationModal() {
  const { t } = useTranslation();
  const [request, setRequest] = useState<ConfirmationRequest | null>(null);
  const [feedback, setFeedback] = useState('');
  const [showFeedback, setShowFeedback] = useState(false);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    listen<ConfirmationRequest>('biscuitcode:confirm-request', (evt) => {
      setRequest(evt.payload);
      setFeedback('');
      setShowFeedback(false);
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, []);

  const sendDecision = useCallback(
    async (decision: 'approve' | 'deny' | 'deny_with_feedback', fb?: string) => {
      if (!request) return;
      try {
        await invoke('agent_confirm_decision', {
          req: {
            request_id: request.request_id,
            decision,
            feedback: fb ?? null,
          },
        });
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error('agent_confirm_decision error:', e);
      }
      setRequest(null);
    },
    [request],
  );

  if (!request) return null;

  const isShell = request.tool_class === 'shell';
  const titleKey = isShell ? 'agent.confirmShellTitle' : 'agent.confirmWriteTitle';

  return (
    <div
      className="fixed inset-0 z-[200] flex items-center justify-center"
      style={{ backgroundColor: 'rgba(8,5,4,0.75)' }}
      aria-modal="true"
      role="dialog"
      aria-labelledby="confirm-modal-title"
    >
      <div className="w-full max-w-lg mx-4 bg-cocoa-600 border border-cocoa-400 rounded shadow-2xl flex flex-col">
        {/* Header */}
        <div className="px-4 py-3 border-b border-cocoa-400 flex items-center gap-2">
          <span
            className={`text-xs font-bold uppercase tracking-wider px-1.5 py-0.5 rounded ${
              isShell
                ? 'bg-accent-warn/20 text-accent-warn'
                : 'bg-biscuit-500/20 text-biscuit-400'
            }`}
          >
            {isShell ? t('agent.toolClassShell') : t('agent.toolClassWrite')}
          </span>
          <h2
            id="confirm-modal-title"
            className="text-sm font-semibold text-cocoa-50 flex-1"
          >
            {t(titleKey)}
          </h2>
        </div>

        {/* Summary */}
        <div className="px-4 py-3">
          <pre
            className="text-xs font-mono text-cocoa-100 bg-cocoa-800 rounded p-3 overflow-auto max-h-48 whitespace-pre-wrap break-all"
            aria-label={t('agent.confirmSummaryLabel')}
          >
            {request.summary}
          </pre>
        </div>

        {/* Feedback area (shown when user wants to deny-with-feedback) */}
        {showFeedback && (
          <div className="px-4 pb-3">
            <textarea
              aria-label={t('agent.confirmFeedbackLabel')}
              placeholder={t('agent.confirmFeedbackPlaceholder')}
              value={feedback}
              onChange={(e) => setFeedback(e.target.value)}
              rows={2}
              className="w-full text-xs bg-cocoa-700 border border-cocoa-400 rounded px-2 py-1.5 text-cocoa-100 placeholder-cocoa-400 focus:outline-none focus:border-biscuit-500 resize-none"
              autoFocus
            />
          </div>
        )}

        {/* Buttons */}
        <div className="px-4 py-3 border-t border-cocoa-500 flex items-center justify-end gap-2">
          {!showFeedback ? (
            <>
              <button
                onClick={() => setShowFeedback(true)}
                className="text-xs px-3 py-1.5 rounded border border-cocoa-400 text-cocoa-200 hover:bg-cocoa-500 transition-colors"
              >
                {t('agent.confirmDenyWithFeedback')}
              </button>
              <button
                onClick={() => sendDecision('deny')}
                className="text-xs px-3 py-1.5 rounded border border-accent-error/40 text-accent-error hover:bg-accent-error/10 transition-colors"
              >
                {t('agent.confirmDeny')}
              </button>
              <button
                onClick={() => sendDecision('approve')}
                className="text-xs px-3 py-1.5 rounded bg-biscuit-500 text-cocoa-900 font-semibold hover:bg-biscuit-400 transition-colors"
              >
                {t('agent.confirmApprove')}
              </button>
            </>
          ) : (
            <>
              <button
                onClick={() => setShowFeedback(false)}
                className="text-xs px-3 py-1.5 rounded border border-cocoa-400 text-cocoa-200 hover:bg-cocoa-500 transition-colors"
              >
                {t('common.cancel')}
              </button>
              <button
                onClick={() => sendDecision('deny_with_feedback', feedback)}
                className="text-xs px-3 py-1.5 rounded bg-accent-warn text-cocoa-900 font-semibold hover:opacity-90 transition-colors"
              >
                {t('agent.confirmSendFeedback')}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
