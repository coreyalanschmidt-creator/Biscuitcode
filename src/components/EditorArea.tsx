// src/components/EditorArea.tsx
//
// Phase 2 ships an empty shell. Phase 3 wires:
//   - @monaco-editor/react with vite-plugin-monaco-editor
//   - Tab bar with dirty dot, middle-click close, Ctrl+W, Ctrl+Shift+T
//   - Diff editor stub for Phase 6b inline edit
//   - Multi-cursor + minimap (vision-mandated, Monaco built-ins)

import { useTranslation } from 'react-i18next';

export function EditorArea() {
  const { t } = useTranslation();
  return (
    <main className="h-full bg-cocoa-700 flex items-center justify-center">
      <div className="text-center">
        <p className="text-sm text-cocoa-300">
          {/* Vision: "Open a folder to start coding" — wired in Phase 3. */}
          <em>{t('common.appName')} editor — opens folder in Phase 3.</em>
        </p>
      </div>
    </main>
  );
}
