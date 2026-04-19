// src/layout/WorkspaceGrid.tsx
//
// The four-region resizable layout: ActivityBar (fixed 48px) | SidePanel
// (collapsible) | (EditorArea / BottomPanel split) | ChatPanel (collapsible).
// Bottom of everything: StatusBar (22px).
//
// Phase 2 deliverable. Uses `react-resizable-panels` for the resize logic;
// panel sizes persist via the Zustand store + localStorage bridge.
//
// Vision UI Layout reference (from docs/vision.md):
//   Background --cocoa-700, 1px dividers --cocoa-500, accent --biscuit-500.

import { ReactNode } from 'react';
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels';
import { usePanelsStore } from '../state/panelsStore';
import { ActivityBar } from '../components/ActivityBar';
import { SidePanel } from '../components/SidePanel';
import { EditorArea } from '../components/EditorArea';
import { TerminalPanel } from '../components/TerminalPanel';
import { ChatPanel } from '../components/ChatPanel';
import { StatusBar } from '../components/StatusBar';

export function WorkspaceGrid(): ReactNode {
  const { sideVisible, bottomVisible, chatVisible } = usePanelsStore();

  return (
    <div className="flex flex-col h-screen w-screen bg-cocoa-700 text-cocoa-50">
      {/* Main content row: activity bar + 3 resizable regions */}
      <div className="flex flex-1 min-h-0">
        <ActivityBar />

        <PanelGroup direction="horizontal" autoSaveId="biscuitcode-h">
          {sideVisible && (
            <>
              <Panel defaultSize={20} minSize={12} maxSize={40} order={1}>
                <SidePanel />
              </Panel>
              <PanelResizeHandle className="w-px bg-cocoa-500 hover:bg-biscuit-500/50 transition-colors" />
            </>
          )}

          <Panel defaultSize={60} minSize={30} order={2}>
            {/* Vertical split: editor on top, bottom panel below */}
            <PanelGroup direction="vertical" autoSaveId="biscuitcode-v">
              <Panel defaultSize={70} minSize={20} order={1}>
                <EditorArea />
              </Panel>
              {bottomVisible && (
                <>
                  <PanelResizeHandle className="h-px bg-cocoa-500 hover:bg-biscuit-500/50 transition-colors" />
                  <Panel defaultSize={30} minSize={10} maxSize={70} order={2}>
                    <TerminalPanel />
                  </Panel>
                </>
              )}
            </PanelGroup>
          </Panel>

          {chatVisible && (
            <>
              <PanelResizeHandle className="w-px bg-cocoa-500 hover:bg-biscuit-500/50 transition-colors" />
              <Panel defaultSize={20} minSize={15} maxSize={45} order={3}>
                <ChatPanel />
              </Panel>
            </>
          )}
        </PanelGroup>
      </div>

      {/* Status bar pinned to the bottom (full-width across activity bar + content) */}
      <StatusBar />
    </div>
  );
}
