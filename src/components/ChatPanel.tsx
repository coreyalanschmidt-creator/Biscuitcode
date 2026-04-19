// src/components/ChatPanel.tsx
//
// Phase 2 ships an empty shell. Phase 5 wires:
//   - react-virtuoso-virtualized message list
//   - markdown rendering, model picker, send button
//   - Anthropic streaming, prompt caching
// Phase 6a adds: agent-mode toggle, @ mention picker, drag-file-into-chat.

import { useTranslation } from 'react-i18next';

export function ChatPanel() {
  const { t } = useTranslation();
  return (
    <aside
      aria-label={t('panels.chatPanel')}
      className="h-full bg-cocoa-700 border-l border-cocoa-500 flex flex-col"
    >
      <header className="px-3 py-2 text-xs font-semibold uppercase tracking-wider text-cocoa-200 border-b border-cocoa-500">
        {t('panels.chats')}
      </header>
      <div className="flex-1 px-3 py-4 text-sm text-cocoa-300 overflow-auto">
        <em>{t('panels.chats')} wires in Phase 5 (Anthropic E2E).</em>
      </div>
      <footer className="border-t border-cocoa-500 px-3 py-2 text-xs text-cocoa-400">
        [model picker] [send] — Phase 5
      </footer>
    </aside>
  );
}
