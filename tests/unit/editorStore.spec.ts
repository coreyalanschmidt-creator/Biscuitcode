// tests/unit/editorStore.spec.ts
//
// Phase 3 unit tests for the editorStore Zustand store.
//
// Covers: openTab, closeTab, reopenLastClosed (Ctrl+Shift+T), toggleSplit,
// markDirty, setCursorPosition, and languageFromPath.
//
// Monaco (ITextModel) is NOT involved here — the store holds serializable
// metadata only. These tests falsify PM-03 by demonstrating that the store
// functions correctly in a jsdom environment with no Monaco dependency.

import { beforeEach, describe, expect, it } from 'vitest';
import { useEditorStore, languageFromPath } from '../../src/state/editorStore';

// Reset store to a clean state before each test.
beforeEach(() => {
  useEditorStore.setState({
    tabs: [],
    activeTabId: null,
    closedStack: [],
    splitVisible: false,
    splitTabId: null,
    workspaceRoot: null,
  });
});

describe('languageFromPath', () => {
  it('maps .ts to typescript', () => {
    expect(languageFromPath('/project/main.ts')).toBe('typescript');
  });
  it('maps .rs to rust', () => {
    expect(languageFromPath('/project/main.rs')).toBe('rust');
  });
  it('maps .py to python', () => {
    expect(languageFromPath('script.py')).toBe('python');
  });
  it('falls back to plaintext for unknown extension', () => {
    expect(languageFromPath('file.xyz')).toBe('plaintext');
  });
  it('handles no extension', () => {
    expect(languageFromPath('Makefile')).toBe('plaintext');
  });
});

describe('openTab', () => {
  it('adds a new tab and makes it active', () => {
    useEditorStore.getState().openTab('/foo/bar.ts');
    const { tabs, activeTabId } = useEditorStore.getState();
    expect(tabs).toHaveLength(1);
    expect(tabs[0].name).toBe('bar.ts');
    expect(tabs[0].language).toBe('typescript');
    expect(activeTabId).toBe('/foo/bar.ts');
  });

  it('does not duplicate tabs for the same path', () => {
    useEditorStore.getState().openTab('/foo/bar.ts');
    useEditorStore.getState().openTab('/foo/bar.ts');
    expect(useEditorStore.getState().tabs).toHaveLength(1);
  });

  it('switching to existing tab just sets it active', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().openTab('/b.ts');
    useEditorStore.getState().openTab('/a.ts');
    const { tabs, activeTabId } = useEditorStore.getState();
    expect(tabs).toHaveLength(2);
    expect(activeTabId).toBe('/a.ts');
  });
});

describe('closeTab', () => {
  it('removes a tab and pushes path to closedStack', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().closeTab('/a.ts');
    const { tabs, closedStack } = useEditorStore.getState();
    expect(tabs).toHaveLength(0);
    expect(closedStack).toContain('/a.ts');
  });

  it('activates the next tab when active tab is closed', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().openTab('/b.ts');
    useEditorStore.getState().closeTab('/b.ts');
    expect(useEditorStore.getState().activeTabId).toBe('/a.ts');
  });

  it('collapses split when the split tab is closed', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().toggleSplit();
    expect(useEditorStore.getState().splitVisible).toBe(true);
    useEditorStore.getState().closeTab('/a.ts');
    const { splitVisible, splitTabId } = useEditorStore.getState();
    expect(splitVisible).toBe(false);
    expect(splitTabId).toBeNull();
  });

  it('closedStack is capped at 20 entries', () => {
    // Open and close 25 tabs.
    for (let i = 0; i < 25; i++) {
      useEditorStore.getState().openTab(`/file${i}.ts`);
      useEditorStore.getState().closeTab(`/file${i}.ts`);
    }
    expect(useEditorStore.getState().closedStack.length).toBeLessThanOrEqual(20);
  });
});

describe('reopenLastClosed (Ctrl+Shift+T)', () => {
  it('reopens the most recently closed tab', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().openTab('/b.ts');
    useEditorStore.getState().closeTab('/b.ts');
    useEditorStore.getState().reopenLastClosed();
    const { tabs, activeTabId } = useEditorStore.getState();
    expect(tabs.some((t) => t.id === '/b.ts')).toBe(true);
    expect(activeTabId).toBe('/b.ts');
  });

  it('is a no-op when closedStack is empty', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().reopenLastClosed(); // nothing to reopen
    expect(useEditorStore.getState().tabs).toHaveLength(1);
  });
});

describe('toggleSplit (Ctrl+\\)', () => {
  it('shows the split pane when a tab is active', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().toggleSplit();
    const { splitVisible, splitTabId } = useEditorStore.getState();
    expect(splitVisible).toBe(true);
    expect(splitTabId).toBe('/a.ts');
  });

  it('hides the split pane on second toggle', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().toggleSplit();
    useEditorStore.getState().toggleSplit();
    expect(useEditorStore.getState().splitVisible).toBe(false);
    expect(useEditorStore.getState().splitTabId).toBeNull();
  });

  it('does not open split when no tab is active', () => {
    useEditorStore.getState().toggleSplit();
    expect(useEditorStore.getState().splitVisible).toBe(false);
  });
});

describe('markDirty', () => {
  it('marks a tab as dirty', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().markDirty('/a.ts', true);
    expect(useEditorStore.getState().tabs[0].isDirty).toBe(true);
  });

  it('clears dirty flag', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().markDirty('/a.ts', true);
    useEditorStore.getState().markDirty('/a.ts', false);
    expect(useEditorStore.getState().tabs[0].isDirty).toBe(false);
  });
});

describe('setCursorPosition', () => {
  it('updates cursorLine and cursorColumn for the tab', () => {
    useEditorStore.getState().openTab('/a.ts');
    useEditorStore.getState().setCursorPosition('/a.ts', 42, 7);
    const tab = useEditorStore.getState().tabs.find((t) => t.id === '/a.ts');
    expect(tab?.cursorLine).toBe(42);
    expect(tab?.cursorColumn).toBe(7);
  });
});
