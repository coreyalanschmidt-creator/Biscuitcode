// src/components/SidePanel.tsx
//
// Contextual panel on the left, content driven by ActivityBar selection.
// Phase 2 ships an empty shell with a labelled placeholder per region;
// real content arrives in:
//   - Files / Search → Phase 3
//   - Git            → Phase 7
//   - Chats          → Phase 5
//   - Settings       → Phase 8

import { useTranslation } from 'react-i18next';
import { usePanelsStore } from '../state/panelsStore';

export function SidePanel() {
  const { t } = useTranslation();
  const { activeActivity } = usePanelsStore();

  const labelKey = `panels.${activeActivity}`;

  return (
    <aside
      aria-label={t('panels.sidePanel')}
      className="h-full bg-cocoa-700 border-r border-cocoa-500 overflow-auto"
    >
      <header className="px-3 py-2 text-xs font-semibold uppercase tracking-wider text-cocoa-200">
        {t(labelKey)}
      </header>
      <div className="px-3 py-4 text-sm text-cocoa-300">
        {/* Phase-N placeholder — replaced as each subsystem ships. */}
        <em>{t(labelKey)} content lands in a later phase.</em>
      </div>
    </aside>
  );
}
