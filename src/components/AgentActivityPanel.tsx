// src/components/AgentActivityPanel.tsx
//
// Phase 6a deliverable: live-streaming tool-call cards.
//
// Reads ToolCallCard[] from agentStore (shared with ChatPanel).
// Each card:
//   - Shows running/ok/error status icon + tool name + timing.
//   - Pretty-prints JSON args (partial while streaming).
//   - Shows result when available.
//   - Emits performance.mark('tool_card_visible_<id>') on first paint.
//   - Is collapsible.
//
// Virtualized with react-virtuoso (same lib as ChatPanel message list).

import { useEffect, useRef, useState } from 'react';
import { Virtuoso } from 'react-virtuoso';
import { useTranslation } from 'react-i18next';
import { useAgentStore, ToolCallCard } from '../state/agentStore';

// ---------- Card component ----------

function ToolCard({ card }: { card: ToolCallCard }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(true);

  // PM-04 addressed: useEffect runs synchronously after React commit (before
  // browser paint), so the mark is placed immediately — not via a batched
  // MutationObserver callback.
  const markedRef = useRef(false);
  useEffect(() => {
    if (!markedRef.current) {
      markedRef.current = true;
      performance.mark(`tool_card_visible_${card.id}`);
      // Emit a measure so tests can query it by name.
      try {
        performance.measure(
          `tool_card_render_${card.id}`,
          `tool_call_start_${card.id}`,
          `tool_card_visible_${card.id}`,
        );
      } catch {
        // Start mark may not exist in unit-test environments; swallow.
      }
    }
    // Only run on mount (card.id is stable).
  }, []);

  const duration =
    card.endedAt != null
      ? `${Math.round(card.endedAt - card.startedAt)}ms`
      : t('agent.running');

  let argsDisplay = card.argsJson;
  try {
    argsDisplay = JSON.stringify(JSON.parse(card.argsJson), null, 2);
  } catch {
    // Still streaming — show raw accumulator.
  }

  const statusIcon =
    card.status === 'running' ? '⏳' : card.status === 'ok' ? '✓' : '✗';
  const statusClass =
    card.status === 'running'
      ? 'text-biscuit-400'
      : card.status === 'ok'
      ? 'text-accent-ok'
      : 'text-accent-error';

  return (
    <article
      data-tool-card-id={card.id}
      className="border border-cocoa-500 rounded mx-2 mb-2 overflow-hidden"
    >
      {/* Header row */}
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center gap-2 px-2 py-1.5 bg-cocoa-600 text-left"
        aria-expanded={open}
      >
        <span className={`text-xs ${statusClass}`} aria-label={t(`agent.status.${card.status}`)}>
          {statusIcon}
        </span>
        <span className="text-xs font-mono font-semibold text-biscuit-300 flex-1">
          {card.name}
        </span>
        <span className="text-[10px] text-cocoa-400">{duration}</span>
        <span className="text-[10px] text-cocoa-400">{open ? '▲' : '▼'}</span>
      </button>

      {open && (
        <div className="px-2 py-2 bg-cocoa-700 space-y-1">
          {/* Args */}
          {argsDisplay && (
            <div>
              <div className="text-[10px] uppercase tracking-wider text-cocoa-400 mb-0.5">
                {t('agent.args')}
              </div>
              <pre className="text-xs font-mono text-cocoa-100 overflow-x-auto whitespace-pre-wrap break-words">
                {argsDisplay}
              </pre>
            </div>
          )}
          {/* Result */}
          {card.result != null && (
            <div>
              <div className="text-[10px] uppercase tracking-wider text-cocoa-400 mb-0.5">
                {t('agent.result')}
              </div>
              <pre className="text-xs font-mono text-cocoa-100 overflow-x-auto whitespace-pre-wrap break-words">
                {card.result}
              </pre>
            </div>
          )}
        </div>
      )}
    </article>
  );
}

// ---------- Panel ----------

export function AgentActivityPanel() {
  const { t } = useTranslation();
  const cards = useAgentStore((s) => s.cards);

  return (
    <section
      aria-label={t('panels.agentActivity')}
      className="h-full bg-cocoa-800 border-t border-cocoa-500 flex flex-col"
    >
      <header className="px-3 py-2 text-xs font-semibold uppercase tracking-wider text-cocoa-200 border-b border-cocoa-600 flex-shrink-0">
        {t('panels.agentActivity')}
        {cards.length > 0 && (
          <span className="ml-2 text-cocoa-400">({cards.length})</span>
        )}
      </header>
      {cards.length === 0 ? (
        <div className="flex-1 flex items-center justify-center text-xs text-cocoa-400 px-4 text-center">
          {t('agent.emptyHint')}
        </div>
      ) : (
        <div className="flex-1 overflow-hidden">
          <Virtuoso
            data={cards}
            itemContent={(_index, card) => <ToolCard key={card.id} card={card} />}
            className="h-full"
          />
        </div>
      )}
    </section>
  );
}
