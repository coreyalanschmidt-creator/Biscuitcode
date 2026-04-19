// src/components/StatusBar.tsx
//
// 22px tall status bar pinned to the bottom edge of the window. Shows:
//   - git branch (Phase 7 wires real value; Phase 2 = "git:main" placeholder)
//   - problem count (Phase 7 wires from LSP diagnostics; placeholder = 0)
//   - active LSP (Phase 7; placeholder = "—")
//   - current model (Phase 5 wires; placeholder = "claude-opus-4-7")
//   - cursor position (Phase 3 wires; placeholder = "Ln 1 C1" — valid 1-indexed)

import { useTranslation } from 'react-i18next';

export function StatusBar() {
  const { t } = useTranslation();

  return (
    <footer
      role="contentinfo"
      aria-label={t('panels.statusBar')}
      className="h-[22px] flex items-center gap-3 px-3 text-xs bg-cocoa-800 border-t border-cocoa-500 text-cocoa-200 font-mono"
    >
      <Segment>git:main</Segment>
      <Sep />
      <Segment title="Problems">0 ⚠</Segment>
      <Sep />
      <Segment title="LSP">LSP: —</Segment>
      <Sep />
      <Segment title="Active model">claude-opus-4-7</Segment>
      <Sep />
      <Segment title="Cursor position">Ln 1 C1</Segment>
    </footer>
  );
}

function Segment({ children, title }: { children: React.ReactNode; title?: string }) {
  return (
    <span title={title} className="hover:text-cocoa-50 transition-colors">
      {children}
    </span>
  );
}

function Sep() {
  return <span aria-hidden="true" className="text-cocoa-500">•</span>;
}
