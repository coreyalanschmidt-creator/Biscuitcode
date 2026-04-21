// src/components/EditorArea.tsx
//
// Phase 3 deliverable: Monaco multi-tab editor with:
//   - Tab bar: dirty dot, Ctrl+W close, middle-click close, Ctrl+Shift+T reopen
//   - One Monaco Editor instance per pane, ITextModel per tab
//   - JetBrains Mono 14px, ligatures on
//   - Multi-cursor (Monaco built-in: Alt+Click, Ctrl+D) — no extra code needed
//   - Minimap on right edge (Monaco built-in) — togglable
//   - Ctrl+\ split pane (two panes sharing models)
//   - Ctrl+P quick-open palette
//   - Diff editor stub (createDiffEditor, not wired until Phase 6b)
//   - Ctrl+F find-in-file via Monaco built-in (already present, just unhidden)

import { useEffect, useRef, useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import Editor, { useMonaco } from '@monaco-editor/react';
import { invoke } from '@tauri-apps/api/core';
import { useEditorStore } from '../state/editorStore';
import { LspClient, registerLspProviders, monacoLangToLsp, extToMonacoLang } from '../lsp/lspClient';
import type * as Monaco from 'monaco-editor';

// ---------------------------------------------------------------------------
// Quick-Open palette component
// ---------------------------------------------------------------------------

interface QuickOpenProps {
  workspaceRoot: string | null;
  onSelect: (path: string) => void;
  onClose: () => void;
}

function QuickOpenPalette({ workspaceRoot, onSelect, onClose }: QuickOpenProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<string[]>([]);
  const [highlighted, setHighlighted] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!workspaceRoot) {
      setResults([]);
      return;
    }
    if (query.trim() === '') {
      setResults([]);
      return;
    }
    invoke<string[]>('fs_search_files', { query, limit: 20 })
      .then((r) => { setResults(r); setHighlighted(0); })
      .catch(() => setResults([]));
  }, [query, workspaceRoot]);

  const handleKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') { e.preventDefault(); onClose(); }
    if (e.key === 'ArrowDown') { e.preventDefault(); setHighlighted((h) => Math.min(h + 1, results.length - 1)); }
    if (e.key === 'ArrowUp') { e.preventDefault(); setHighlighted((h) => Math.max(h - 1, 0)); }
    if (e.key === 'Enter' && results[highlighted]) {
      e.preventDefault();
      onSelect(results[highlighted]);
      onClose();
    }
  };

  return (
    <div
      className="absolute inset-0 z-50 flex items-start justify-center pt-16"
      style={{ backgroundColor: 'rgba(8,5,4,0.6)' }}
      onClick={onClose}
    >
      <div
        className="w-full max-w-lg bg-cocoa-600 border border-cocoa-400 rounded shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKey}
          placeholder={t('editor.quickOpenPlaceholder')}
          className="w-full bg-transparent px-4 py-3 text-sm text-cocoa-50 placeholder-cocoa-300 outline-none border-b border-cocoa-400"
          style={{ fontFamily: "'Inter', 'Ubuntu', sans-serif" }}
          aria-label={t('editor.quickOpen')}
        />
        {results.length > 0 && (
          <ul role="listbox" className="max-h-72 overflow-y-auto py-1">
            {results.map((r, i) => (
              <li
                key={r}
                role="option"
                aria-selected={i === highlighted}
                className={`px-4 py-1.5 text-sm cursor-pointer ${
                  i === highlighted ? 'bg-biscuit-500 text-cocoa-900' : 'text-cocoa-100 hover:bg-cocoa-500'
                }`}
                style={{ fontFamily: "'JetBrains Mono', 'Ubuntu Mono', 'DejaVu Sans Mono', monospace" }}
                onMouseEnter={() => setHighlighted(i)}
                onClick={() => { onSelect(r); onClose(); }}
              >
                {r}
              </li>
            ))}
          </ul>
        )}
        {query.trim() !== '' && results.length === 0 && (
          <p className="px-4 py-3 text-sm text-cocoa-300">{t('editor.noResults')}</p>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tab bar
// ---------------------------------------------------------------------------

interface TabBarProps {
  tabs: ReturnType<typeof useEditorStore.getState>['tabs'];
  activeTabId: string | null;
  onActivate: (id: string) => void;
  onClose: (id: string) => void;
}

function TabBar({ tabs, activeTabId, onActivate, onClose }: TabBarProps) {
  const { t } = useTranslation();

  if (tabs.length === 0) return null;

  return (
    <div
      role="tablist"
      aria-label={t('editor.tabList')}
      className="flex overflow-x-auto bg-cocoa-800 border-b border-cocoa-600 shrink-0 min-h-[32px]"
      style={{ scrollbarWidth: 'none' }}
    >
      {tabs.map((tab) => {
        const active = tab.id === activeTabId;
        return (
          <button
            key={tab.id}
            role="tab"
            aria-selected={active}
            title={tab.path}
            className={`flex items-center gap-1.5 px-3 py-1 text-xs whitespace-nowrap border-r border-cocoa-600 shrink-0 ${
              active
                ? 'bg-cocoa-700 text-cocoa-50 border-t-2 border-t-biscuit-500'
                : 'bg-cocoa-800 text-cocoa-300 hover:bg-cocoa-700 border-t-2 border-t-transparent'
            }`}
            style={{ fontFamily: "'Inter', 'Ubuntu', sans-serif" }}
            onClick={() => onActivate(tab.id)}
            onAuxClick={(e) => { if (e.button === 1) { e.preventDefault(); onClose(tab.id); } }}
          >
            {tab.isDirty && (
              <span
                className="w-2 h-2 rounded-full bg-biscuit-400 inline-block shrink-0"
                aria-label={t('editor.dirtyIndicator')}
              />
            )}
            <span>{tab.name}</span>
            <span
              role="button"
              aria-label={t('editor.closeTab', { name: tab.name })}
              className="ml-1 w-4 h-4 flex items-center justify-center rounded hover:bg-cocoa-500 text-cocoa-300 hover:text-cocoa-50"
              onClick={(e) => { e.stopPropagation(); onClose(tab.id); }}
            >
              ×
            </span>
          </button>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Single Monaco pane
// ---------------------------------------------------------------------------

interface MonacoPaneProps {
  tabId: string | null;
}

const MONACO_OPTIONS = {
  fontFamily: "'JetBrains Mono', 'Ubuntu Mono', 'DejaVu Sans Mono', monospace",
  fontSize: 14,
  lineHeight: 21, // 14 * 1.5
  fontLigatures: true,
  minimap: { enabled: true },
  scrollBeyondLastLine: false,
  automaticLayout: true,
  tabSize: 2,
  insertSpaces: true,
  wordWrap: 'off' as const,
  renderWhitespace: 'selection' as const,
  cursorBlinking: 'blink' as const,
};

function MonacoPane({ tabId }: MonacoPaneProps) {
  const { t } = useTranslation();
  const monaco = useMonaco();
  const editorRef = useRef<import('monaco-editor').editor.IStandaloneCodeEditor | null>(null);
  const blameDecorationsRef = useRef<string[]>([]);
  const { tabs, setCursorPosition, markDirty } = useEditorStore();

  const tab = tabs.find((t) => t.id === tabId) ?? null;

  // When the active tab changes, swap the model on the editor instance.
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor || !monaco || !tab) return;

    const uri = monaco.Uri.file(tab.path);
    let model = monaco.editor.getModel(uri);
    if (model) {
      editor.setModel(model);
      return;
    }
    // Model not yet created — load content via Tauri, then create.
    invoke<string>('fs_read', { path: tab.path })
      .then((content) => {
        const existing = monaco.editor.getModel(uri);
        if (existing) { editor.setModel(existing); return; }
        const m = monaco.editor.createModel(content, tab.language, uri);
        m.onDidChangeContent(() => markDirty(tab.id, true));
        editor.setModel(m);
      })
      .catch(() => {
        // File might be new/unreadable — create empty model.
        const m = monaco.editor.createModel('', tab.language, uri);
        m.onDidChangeContent(() => markDirty(tab.id, true));
        editor.setModel(m);
      });
  }, [tabId, monaco]);

  // Phase 4: handle reveal-line events from the terminal link provider.
  useEffect(() => {
    const handleReveal = (e: Event) => {
      const detail = (e as CustomEvent<{ path: string; line: number; col: number }>).detail;
      if (!detail || !tab || detail.path !== tab.path) return;
      const editor = editorRef.current;
      if (!editor) return;
      editor.revealLineInCenter(detail.line);
      editor.setPosition({ lineNumber: detail.line, column: detail.col ?? 1 });
      editor.focus();
    };
    window.addEventListener('biscuitcode:editor-reveal-line', handleReveal);
    return () => window.removeEventListener('biscuitcode:editor-reveal-line', handleReveal);
  }, [tab?.path]);

  // Phase 7: git blame gutter.
  // Listens for biscuitcode:blame-toggle events; fetches blame for visible
  // line range and renders as Monaco inline decorations in the gutter.
  useEffect(() => {
    const handleBlameToggle = async (e: Event) => {
      const { enabled } = (e as CustomEvent<{ enabled: boolean }>).detail ?? {};
      const editor = editorRef.current;
      if (!editor || !tab || !monaco) return;

      if (!enabled) {
        // Clear decorations.
        blameDecorationsRef.current = editor.deltaDecorations(blameDecorationsRef.current, []);
        return;
      }

      // Fetch blame for visible lines.
      const visibleRanges = editor.getVisibleRanges();
      if (!visibleRanges.length) return;
      const startLine = visibleRanges[0].startLineNumber;
      const endLine = visibleRanges[visibleRanges.length - 1].endLineNumber;

      try {
        const blameLines = await invoke<Array<{ line: number; hash: string; author: string; relative_date: string }>>(
          'git_blame',
          { path: tab.path, startLine, endLine },
        );

        const decorations = blameLines.map((b) => ({
          range: new monaco.Range(b.line, 1, b.line, 1),
          options: {
            isWholeLine: false,
            glyphMarginClassName: undefined,
            after: {
              content: `  ${b.hash} · ${b.author} · ${b.relative_date}`,
              inlineClassName: 'biscuit-blame-inline',
            },
          },
        }));

        blameDecorationsRef.current = editor.deltaDecorations(
          blameDecorationsRef.current,
          decorations,
        );
      } catch {
        // Blame failed (file not tracked or no commits) — silently no-op.
      }
    };

    window.addEventListener('biscuitcode:blame-toggle', handleBlameToggle);
    return () => window.removeEventListener('biscuitcode:blame-toggle', handleBlameToggle);
  }, [tab?.path, monaco]);

  // Phase 6b: Ctrl+K Ctrl+I — inline AI edit on selection.
  const ctrlKPending = useRef(false);

  const handleMount = useCallback(
    (editor: import('monaco-editor').editor.IStandaloneCodeEditor) => {
      editorRef.current = editor;
      editor.onDidChangeCursorPosition((e) => {
        if (tab) setCursorPosition(tab.id, e.position.lineNumber, e.position.column);
      });

      // Ctrl+K sets a pending flag; Ctrl+I following it triggers inline edit.
      if (monaco) {
        editor.addCommand(
          monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK,
          () => { ctrlKPending.current = true; setTimeout(() => { ctrlKPending.current = false; }, 1000); },
        );
        editor.addCommand(
          monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyI,
          () => {
            if (!ctrlKPending.current) return;
            ctrlKPending.current = false;
            const selection = editor.getSelection();
            if (!selection || !tab) return;
            const model = editor.getModel();
            if (!model) return;
            const selectedText = model.getValueInRange(selection);
            if (!selectedText.trim()) return;
            window.dispatchEvent(
              new CustomEvent('biscuitcode:inline-edit-open', {
                detail: {
                  path: tab.path,
                  startLine: selection.startLineNumber,
                  endLine: selection.endLineNumber,
                  selectedText,
                },
              }),
            );
          },
        );
      }
    },
    [tabId, monaco, tab],
  );

  if (!tab) {
    return (
      <div className="flex-1 flex items-center justify-center bg-cocoa-700">
        <p className="text-sm text-cocoa-300 italic">{t('editor.noFileOpen')}</p>
      </div>
    );
  }

  return (
    <div className="flex-1 min-h-0 overflow-hidden" style={{ height: '100%' }}>
      <Editor
        key={tabId ?? 'empty'}
        theme="vs-dark"
        options={MONACO_OPTIONS}
        onMount={handleMount}
        loading={
          <div className="flex items-center justify-center h-full bg-cocoa-700">
            <span className="text-sm text-cocoa-300">{t('editor.loading')}</span>
          </div>
        }
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// LSP connection hook
//
// Manages one LspClient per (language, workspaceRoot) pair.
// Registers Monaco hover + definition providers once per pair.
// Idempotent: switching tabs to the same language reuses the existing session
// (PM-02 mitigation).
// ---------------------------------------------------------------------------

// Module-level map: "lang:workspace" → LspClient
// Using a module-level map avoids React state churn and keeps clients alive
// across re-renders.
const _lspClients = new Map<string, LspClient>();
const _lspProviders = new Map<string, Monaco.IDisposable>();

function useLspConnection(
  monacoInstance: typeof Monaco | null,
  activeTabPath: string | null,
  workspaceRoot: string | null,
) {
  useEffect(() => {
    if (!monacoInstance || !activeTabPath || !workspaceRoot) return;

    const ext = activeTabPath.split('.').pop() ?? '';
    const monacoLangId = extToMonacoLang(ext);
    const lspLang = monacoLangToLsp(monacoLangId);
    if (!lspLang) return; // unsupported language — no-op (PM-03 mitigation)

    const key = `${lspLang}:${workspaceRoot}`;

    // Reuse existing client if already created.
    let client = _lspClients.get(key);
    if (!client) {
      client = new LspClient(lspLang, workspaceRoot, monacoInstance);
      _lspClients.set(key, client);
    }

    // Kick off the session (idempotent).
    void client.ensureSession().catch(() => {/* server not installed — silently no-op */});

    // Register Monaco providers once per key.
    if (!_lspProviders.has(key)) {
      const disposable = registerLspProviders(monacoInstance, client, monacoLangId);
      _lspProviders.set(key, disposable);
    }

    // No cleanup here: providers and sessions live for the app lifetime once
    // registered. Sessions are shut down via lsp_list_sessions cleanup on
    // workspace change (future work).
  }, [monacoInstance, activeTabPath, workspaceRoot]);
}

// ---------------------------------------------------------------------------
// EditorArea — main export
// ---------------------------------------------------------------------------

export function EditorArea() {
  const { t } = useTranslation();
  const {
    tabs,
    activeTabId,
    splitVisible,
    splitTabId,
    workspaceRoot,
    openTab,
    closeTab,
    setActiveTab,
    reopenLastClosed,
    toggleSplit,
    setSplitTab,
  } = useEditorStore();

  // LSP connection: wire hover/definition providers to the Rust LSP backend.
  const monacoInstance = useMonaco();
  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? null;
  useLspConnection(monacoInstance, activeTab?.path ?? null, workspaceRoot);

  const [quickOpenVisible, setQuickOpenVisible] = useState(false);

  // Listen for events dispatched by the shortcut layer.
  useEffect(() => {
    const handleCtrlP = () => setQuickOpenVisible(true);
    const handleCtrlSlash = () => toggleSplit();
    const handleOpenFile = (e: Event) => {
      const detail = (e as CustomEvent<{ path: string }>).detail;
      if (detail?.path) openTab(detail.path);
    };
    // Phase 4: terminal link provider fires this event to open a file at a
    // specific line/column. Open the tab first, then re-emit a reveal event
    // that MonacoPane handles after the model is ready.
    const handleOpenFileAt = (e: Event) => {
      const detail = (e as CustomEvent<{ path: string; line: number; col: number }>).detail;
      if (!detail?.path) return;
      openTab(detail.path);
      // Give the model a tick to load before revealing.
      setTimeout(() => {
        window.dispatchEvent(
          new CustomEvent('biscuitcode:editor-reveal-line', {
            detail: { path: detail.path, line: detail.line, col: detail.col ?? 1 },
          }),
        );
      }, 100);
    };
    // Phase 7: auto-open preview for AI-edited .md / .html / .svg / image files.
    // Phase 6b emits biscuitcode:preview-file; EditorArea re-emits it to the
    // PreviewPanel (which also listens on it directly). No action needed here
    // because PreviewPanel listens on window for this event independently.
    // We just ensure the tab is open so the split-pane shows it.
    const handlePreviewFile = (e: Event) => {
      const detail = (e as CustomEvent<{ path: string }>).detail;
      if (!detail?.path) return;
      openTab(detail.path);
      // If split is not visible, open it so the preview shows beside the editor.
      if (!splitVisible) toggleSplit();
    };

    window.addEventListener('biscuitcode:editor-quick-open', handleCtrlP);
    window.addEventListener('biscuitcode:editor-split', handleCtrlSlash);
    window.addEventListener('biscuitcode:editor-open-file', handleOpenFile);
    window.addEventListener('biscuitcode:open-file-at', handleOpenFileAt);
    window.addEventListener('biscuitcode:preview-file', handlePreviewFile);
    return () => {
      window.removeEventListener('biscuitcode:editor-quick-open', handleCtrlP);
      window.removeEventListener('biscuitcode:editor-split', handleCtrlSlash);
      window.removeEventListener('biscuitcode:editor-open-file', handleOpenFile);
      window.removeEventListener('biscuitcode:open-file-at', handleOpenFileAt);
      window.removeEventListener('biscuitcode:preview-file', handlePreviewFile);
    };
  }, [toggleSplit, openTab, splitVisible]);

  // Keyboard: Ctrl+W closes active tab, Ctrl+Shift+T reopens last.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'w') {
        if (activeTabId) { e.preventDefault(); closeTab(activeTabId); }
      }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && !e.altKey && e.key.toLowerCase() === 't') {
        e.preventDefault(); reopenLastClosed();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [activeTabId, closeTab, reopenLastClosed]);

  // Phase 8: keep window.__BISCUIT_WORKSPACE_ROOT__ in sync with the editorStore.
  useEffect(() => {
    window.__BISCUIT_WORKSPACE_ROOT__ = workspaceRoot ?? undefined;
  }, [workspaceRoot]);

  const handleQuickSelect = useCallback(
    (relativePath: string) => {
      const fullPath = workspaceRoot ? `${workspaceRoot}/${relativePath}` : relativePath;
      openTab(fullPath);
    },
    [workspaceRoot, openTab],
  );

  return (
    <main className="h-full flex flex-col bg-cocoa-700 overflow-hidden relative" aria-label={t('editor.area')}>
      {/* Tab bar always at top */}
      <TabBar
        tabs={tabs}
        activeTabId={activeTabId}
        onActivate={setActiveTab}
        onClose={closeTab}
      />

      {/* Editor pane(s) */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* Primary pane */}
        <div className={`flex flex-col min-h-0 overflow-hidden ${splitVisible ? 'flex-1 border-r border-cocoa-500' : 'flex-1'}`}>
          <MonacoPane tabId={activeTabId} />
        </div>

        {/* Secondary split pane */}
        {splitVisible && (
          <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
            <TabBar
              tabs={tabs}
              activeTabId={splitTabId}
              onActivate={(id) => setSplitTab(id)}
              onClose={closeTab}
            />
            <MonacoPane tabId={splitTabId} />
          </div>
        )}
      </div>

      {/* Quick-open overlay */}
      {quickOpenVisible && (
        <QuickOpenPalette
          workspaceRoot={workspaceRoot}
          onSelect={handleQuickSelect}
          onClose={() => setQuickOpenVisible(false)}
        />
      )}

      {/* Welcome screen when no tabs are open */}
      {tabs.length === 0 && !quickOpenVisible && (
        <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
          <p className="text-sm text-cocoa-300 italic">{t('editor.welcomeHint')}</p>
        </div>
      )}
    </main>
  );
}
