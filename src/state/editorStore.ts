// src/state/editorStore.ts
//
// Zustand store for the Monaco editor tab state.
//
// Phase 3 deliverable. Tracks open tabs, the active tab, closed-tab history
// (for Ctrl+Shift+T reopen), and split-pane state. The Monaco ITextModel
// instances are managed in EditorArea.tsx via refs — not stored here, since
// they are non-serializable. The store only holds serializable metadata.

import { create } from 'zustand';

/** Language detected from file extension. */
export function languageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  const map: Record<string, string> = {
    ts: 'typescript',
    tsx: 'typescript',
    js: 'javascript',
    jsx: 'javascript',
    json: 'json',
    md: 'markdown',
    rs: 'rust',
    py: 'python',
    go: 'go',
    html: 'html',
    css: 'css',
    scss: 'scss',
    sh: 'shell',
    toml: 'toml',
    yaml: 'yaml',
    yml: 'yaml',
    txt: 'plaintext',
    xml: 'xml',
    sql: 'sql',
    c: 'c',
    cpp: 'cpp',
    h: 'c',
    hpp: 'cpp',
    java: 'java',
    kt: 'kotlin',
    rb: 'ruby',
    php: 'php',
    swift: 'swift',
  };
  return map[ext] ?? 'plaintext';
}

export interface EditorTab {
  /** Unique ID — use the absolute file path as the key. */
  id: string;
  /** Display name (last path component). */
  name: string;
  /** Absolute path to file. */
  path: string;
  /** Whether the buffer has unsaved changes. */
  isDirty: boolean;
  /** Monaco language identifier. */
  language: string;
  /** Saved cursor position to restore on re-open. */
  cursorLine: number;
  cursorColumn: number;
}

interface EditorState {
  /** Ordered list of open tabs. */
  tabs: EditorTab[];
  /** Id of the currently focused tab (null = welcome screen). */
  activeTabId: string | null;
  /** Stack of recently closed tab paths for Ctrl+Shift+T. */
  closedStack: string[];
  /** Whether the split pane is visible. */
  splitVisible: boolean;
  /** Id of the focused tab in the right split pane (null = not in use). */
  splitTabId: string | null;

  // Workspace root (set when a folder is opened)
  workspaceRoot: string | null;

  // Actions
  openTab: (path: string, content?: string) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  markDirty: (id: string, dirty: boolean) => void;
  reopenLastClosed: () => void;
  toggleSplit: () => void;
  setSplitTab: (id: string | null) => void;
  setCursorPosition: (id: string, line: number, col: number) => void;
  setWorkspaceRoot: (root: string | null) => void;
}

export const useEditorStore = create<EditorState>()((set, get) => ({
  tabs: [],
  activeTabId: null,
  closedStack: [],
  splitVisible: false,
  splitTabId: null,
  workspaceRoot: null,

  openTab: (path: string, _content?: string) => {
    const { tabs } = get();
    const existing = tabs.find((t) => t.id === path);
    if (existing) {
      set({ activeTabId: path });
      return;
    }
    const name = path.split('/').pop() ?? path;
    const newTab: EditorTab = {
      id: path,
      name,
      path,
      isDirty: false,
      language: languageFromPath(path),
      cursorLine: 1,
      cursorColumn: 1,
    };
    set((s) => ({ tabs: [...s.tabs, newTab], activeTabId: path }));
  },

  closeTab: (id: string) => {
    const { tabs, activeTabId, splitTabId } = get();
    const idx = tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;

    const closedPath = tabs[idx].path;
    const remaining = tabs.filter((t) => t.id !== id);

    // Determine next active tab.
    let nextActive = activeTabId === id
      ? (remaining[idx] ?? remaining[idx - 1] ?? remaining[0] ?? null)?.id ?? null
      : activeTabId;

    // If split was showing this tab, collapse split.
    const nextSplitTab = splitTabId === id ? null : splitTabId;
    const nextSplitVisible = nextSplitTab !== null ? get().splitVisible : false;

    set((s) => ({
      tabs: remaining,
      activeTabId: nextActive,
      closedStack: [closedPath, ...s.closedStack].slice(0, 20),
      splitTabId: nextSplitTab,
      splitVisible: nextSplitVisible,
    }));
  },

  setActiveTab: (id: string) => set({ activeTabId: id }),

  markDirty: (id: string, dirty: boolean) =>
    set((s) => ({
      tabs: s.tabs.map((t) => (t.id === id ? { ...t, isDirty: dirty } : t)),
    })),

  reopenLastClosed: () => {
    const { closedStack } = get();
    if (closedStack.length === 0) return;
    const [path, ...rest] = closedStack;
    set({ closedStack: rest });
    get().openTab(path);
  },

  toggleSplit: () => {
    const { splitVisible, activeTabId } = get();
    if (!splitVisible && activeTabId) {
      set({ splitVisible: true, splitTabId: activeTabId });
    } else {
      set({ splitVisible: false, splitTabId: null });
    }
  },

  setSplitTab: (id) => set({ splitTabId: id }),

  setCursorPosition: (id, line, col) =>
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === id ? { ...t, cursorLine: line, cursorColumn: col } : t,
      ),
    })),

  setWorkspaceRoot: (root) => set({ workspaceRoot: root }),
}));
