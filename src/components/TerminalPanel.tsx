// src/components/TerminalPanel.tsx
//
// Phase 2 ships an empty shell (also serves as the bottom-panel shell).
// Phase 4 wires xterm.js + portable-pty.

import { useTranslation } from 'react-i18next';

export function TerminalPanel() {
  const { t } = useTranslation();
  return (
    <section
      aria-label={t('panels.terminal')}
      className="h-full bg-cocoa-800 border-t border-cocoa-500 overflow-auto"
    >
      <header className="px-3 py-2 text-xs font-semibold uppercase tracking-wider text-cocoa-200">
        {t('panels.terminal')}
      </header>
      <div className="px-3 py-4 text-sm text-cocoa-300">
        <em>{t('panels.terminal')} wires in Phase 4 (xterm.js + portable-pty).</em>
      </div>
    </section>
  );
}
