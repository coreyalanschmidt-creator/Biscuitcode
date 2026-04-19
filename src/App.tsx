// src/App.tsx
//
// Top-level app shell. Mounts the four-region WorkspaceGrid + the global
// shortcut handler + the toast/error layer + the command palette modal.
//
// Phase 1+2 deliverable. Each later phase rewrites individual children
// (Phase 3 -> EditorArea Monaco wrapper, Phase 4 -> TerminalPanel xterm,
// Phase 5 -> ChatPanel virtualized chat, etc.) WITHOUT changing this
// shell.

import { useGlobalShortcuts } from './shortcuts/global';
import { WorkspaceGrid } from './layout/WorkspaceGrid';
import { ToastLayer } from './components/ToastLayer';
import { CommandPalette } from './components/CommandPalette';
import { ConfirmationModal } from './components/ConfirmationModal';
import { InlineEditPane } from './components/InlineEditPane';

export default function App() {
  // Install global keyboard shortcut handlers once at mount.
  useGlobalShortcuts();

  return (
    <>
      <WorkspaceGrid />
      <ToastLayer />
      <CommandPalette />
      {/* Phase 6b — write-tool confirmation gate */}
      <ConfirmationModal />
      {/* Phase 6b — inline AI edit split-diff */}
      <InlineEditPane />
    </>
  );
}
