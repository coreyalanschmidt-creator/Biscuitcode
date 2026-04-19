// src/components/StatusBar.tsx
//
// 22px tall status bar pinned to the bottom edge of the window. Shows:
//   - git branch (Phase 7 wires real value; clicks open git panel)
//   - problem count (Phase 7 wires from LSP diagnostics)
//   - active LSP (Phase 7)
//   - current model (Phase 5 wires; placeholder = "claude-opus-4-7")
//   - cursor position (Phase 3 wires; placeholder = "Ln 1 C1" — valid 1-indexed)

import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { usePanelsStore } from '../state/panelsStore';
import { useLspStore } from '../state/lspStore';

interface GitStatus {
  branch: string;
  files: unknown[];
}

export function StatusBar() {
  const { t } = useTranslation();
  const setActiveActivity = usePanelsStore((s) => s.setActiveActivity);
  const diagnostics = useLspStore((s) => s.diagnostics);
  const activeSessions = useLspStore((s) => s.activeSessions);

  const [branch, setBranch] = useState<string>('git');

  // Poll git branch every 10s and on workspace change events.
  const refreshBranch = useCallback(() => {
    invoke<GitStatus>('git_status')
      .then((s) => setBranch(s.branch))
      .catch(() => setBranch('git'));
  }, []);

  useEffect(() => {
    refreshBranch();
    const interval = setInterval(refreshBranch, 10_000);
    const handler = () => refreshBranch();
    window.addEventListener('biscuitcode:file-saved', handler);
    window.addEventListener('biscuitcode:git-changed', handler);
    return () => {
      clearInterval(interval);
      window.removeEventListener('biscuitcode:file-saved', handler);
      window.removeEventListener('biscuitcode:git-changed', handler);
    };
  }, [refreshBranch]);

  const problemCount = diagnostics.filter((d) => d.severity <= 2).length;
  const sessionNames = Object.values(activeSessions);
  const lspLabel = sessionNames.length > 0 ? sessionNames[0] : '—';

  const handleBranchClick = useCallback(() => {
    setActiveActivity('git');
  }, [setActiveActivity]);

  const handleProblemsClick = useCallback(() => {
    // Switch to the Problems tab in the bottom panel (Phase 9 wires full panel).
    // For now: just show the count. Real problems tab lands in Phase 9.
  }, []);

  return (
    <footer
      role="contentinfo"
      aria-label={t('panels.statusBar')}
      className="h-[22px] flex items-center gap-3 px-3 text-xs bg-cocoa-800 border-t border-cocoa-500 text-cocoa-200 font-mono"
    >
      <button
        className="hover:text-cocoa-50 transition-colors"
        onClick={handleBranchClick}
        title={t('git.switchBranch')}
        aria-label={t('git.currentBranch', { branch })}
      >
        git:{branch}
      </button>
      <Sep />
      <button
        className="hover:text-cocoa-50 transition-colors"
        onClick={handleProblemsClick}
        aria-label={`${problemCount} problems`}
      >
        {problemCount} ⚠
      </button>
      <Sep />
      <Segment title="Active language server">LSP: {lspLabel}</Segment>
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
