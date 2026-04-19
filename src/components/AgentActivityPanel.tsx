// src/components/AgentActivityPanel.tsx
//
// "Watch the AI work" panel. Phase 2 ships an empty shell. Phase 6a wires:
//   - react-virtuoso-virtualized list of tool-call cards
//   - performance.mark instrumentation for the 250ms render-gate
//   - Pretty-JSON args, streamed result, status icon, timing

import { useTranslation } from 'react-i18next';

export function AgentActivityPanel() {
  const { t } = useTranslation();
  return (
    <section
      aria-label={t('panels.agentActivity')}
      className="h-full bg-cocoa-800 border-t border-cocoa-500 overflow-auto"
    >
      <header className="px-3 py-2 text-xs font-semibold uppercase tracking-wider text-cocoa-200">
        {t('panels.agentActivity')}
      </header>
      <div className="px-3 py-4 text-sm text-cocoa-300">
        <em>{t('panels.agentActivity')} wires in Phase 6a (tool-call cards).</em>
      </div>
    </section>
  );
}
