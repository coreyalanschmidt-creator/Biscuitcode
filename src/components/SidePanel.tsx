// src/components/SidePanel.tsx
//
// Contextual panel driven by ActivityBar selection.
// Phase 3 wires the Files activity with a real file tree and
// the Search activity with cross-file find (Ctrl+Shift+F).
// Other activities remain as labelled placeholders until their phases land:
//   - Git            → Phase 7
//   - Chats          → Phase 5
//   - Settings       → Phase 8

import { useCallback, useEffect, useState, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { ChevronRight, ChevronDown, File, Folder, FolderOpen } from 'lucide-react';
import { usePanelsStore } from '../state/panelsStore';
import { useEditorStore } from '../state/editorStore';
import { GitPanel } from './GitPanel';
import { SettingsPage } from './SettingsPage';

// ---------------------------------------------------------------------------
// File tree types
// ---------------------------------------------------------------------------

interface DirEntry {
  path: string;
  name: string;
  isDir: boolean;
}

interface TreeNode extends DirEntry {
  children?: TreeNode[];
  expanded: boolean;
}

// ---------------------------------------------------------------------------
// Context menu (right-click on file/dir)
// ---------------------------------------------------------------------------

interface ContextMenuState {
  x: number;
  y: number;
  node: TreeNode;
}

interface ContextMenuProps {
  state: ContextMenuState;
  onClose: () => void;
  onNewFile: (dir: string) => void;
  onNewFolder: (dir: string) => void;
  onRename: (node: TreeNode) => void;
  onDelete: (node: TreeNode) => void;
  onCopyPath: (path: string) => void;
  onOpenInTerminal: (dir: string) => void;
}

function ContextMenu({
  state, onClose,
  onNewFile, onNewFolder, onRename, onDelete, onCopyPath, onOpenInTerminal,
}: ContextMenuProps) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDivElement>(null);
  const dir = state.node.isDir
    ? state.node.path
    : state.node.path.split('/').slice(0, -1).join('/');

  useEffect(() => {
    const onClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener('mousedown', onClickOutside);
    return () => document.removeEventListener('mousedown', onClickOutside);
  }, [onClose]);

  const item = (label: string, action: () => void) => (
    <button
      className="w-full text-left px-3 py-1.5 text-xs text-cocoa-100 hover:bg-cocoa-500 whitespace-nowrap"
      style={{ fontFamily: "'Inter', 'Ubuntu', sans-serif" }}
      onClick={() => { action(); onClose(); }}
    >
      {label}
    </button>
  );

  return (
    <div
      ref={ref}
      role="menu"
      aria-label={t('fileTree.contextMenu')}
      className="fixed z-50 bg-cocoa-600 border border-cocoa-400 rounded shadow-xl py-1 min-w-[160px]"
      style={{ top: state.y, left: state.x }}
    >
      {state.node.isDir && item(t('fileTree.newFile'), () => onNewFile(dir))}
      {state.node.isDir && item(t('fileTree.newFolder'), () => onNewFolder(dir))}
      {item(t('fileTree.rename'), () => onRename(state.node))}
      {item(t('fileTree.delete'), () => onDelete(state.node))}
      <hr className="border-cocoa-500 my-1" />
      {item(t('fileTree.copyPath'), () => onCopyPath(state.node.path))}
      {item(t('fileTree.openInTerminal'), () => onOpenInTerminal(dir))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Inline name editor (for new file / rename)
// ---------------------------------------------------------------------------

interface InlineInputProps {
  defaultValue: string;
  onCommit: (name: string) => void;
  onCancel: () => void;
}

function InlineInput({ defaultValue, onCommit, onCancel }: InlineInputProps) {
  const [val, setVal] = useState(defaultValue);
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => { ref.current?.select(); }, []);
  return (
    <input
      ref={ref}
      value={val}
      className="w-full bg-cocoa-500 text-cocoa-50 text-xs px-1 outline outline-1 outline-biscuit-500 rounded"
      style={{ fontFamily: "'JetBrains Mono', 'Ubuntu Mono', monospace" }}
      onChange={(e) => setVal(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') { e.preventDefault(); onCommit(val.trim()); }
        if (e.key === 'Escape') { e.preventDefault(); onCancel(); }
      }}
      onBlur={() => onCommit(val.trim())}
    />
  );
}

// ---------------------------------------------------------------------------
// File tree node
// ---------------------------------------------------------------------------

interface FileTreeNodeProps {
  node: TreeNode;
  depth: number;
  onToggle: (path: string) => void;
  onOpen: (path: string) => void;
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
  inlineInput: { parentPath: string; isFolder: boolean } | null;
  renamingPath: string | null;
  onInlineCommit: (name: string) => void;
  onInlineCancel: () => void;
  onRenameCommit: (oldPath: string, newName: string) => void;
}

function FileTreeNodeItem({
  node, depth, onToggle, onOpen, onContextMenu,
  inlineInput, renamingPath,
  onInlineCommit, onInlineCancel, onRenameCommit,
}: FileTreeNodeProps) {
  const indentPx = depth * 12 + 8;
  const isRenaming = renamingPath === node.path;

  return (
    <>
      <div
        className="flex items-center gap-1 py-0.5 cursor-pointer hover:bg-cocoa-600 select-none"
        style={{ paddingLeft: `${indentPx}px` }}
        onClick={() => node.isDir ? onToggle(node.path) : onOpen(node.path)}
        onContextMenu={(e) => onContextMenu(e, node)}
      >
        {node.isDir ? (
          <>
            {node.expanded ? (
              <ChevronDown className="w-3 h-3 text-cocoa-300 shrink-0" />
            ) : (
              <ChevronRight className="w-3 h-3 text-cocoa-300 shrink-0" />
            )}
            {node.expanded ? (
              <FolderOpen className="w-4 h-4 text-biscuit-400 shrink-0" />
            ) : (
              <Folder className="w-4 h-4 text-biscuit-400 shrink-0" />
            )}
          </>
        ) : (
          <>
            <span className="w-3 shrink-0" />
            <File className="w-4 h-4 text-cocoa-300 shrink-0" />
          </>
        )}
        {isRenaming ? (
          <InlineInput
            defaultValue={node.name}
            onCommit={(name) => onRenameCommit(node.path, name)}
            onCancel={onInlineCancel}
          />
        ) : (
          <span
            className="text-xs text-cocoa-100 truncate"
            style={{ fontFamily: "'JetBrains Mono', 'Ubuntu Mono', 'DejaVu Sans Mono', monospace" }}
          >
            {node.name}
          </span>
        )}
      </div>

      {/* Inline new-file/folder input inside expanded dir */}
      {node.isDir && node.expanded && inlineInput?.parentPath === node.path && (
        <div
          className="flex items-center gap-1 py-0.5"
          style={{ paddingLeft: `${indentPx + 12 + 8 + 16}px` }}
        >
          <InlineInput
            defaultValue={inlineInput.isFolder ? 'new-folder' : 'new-file.ts'}
            onCommit={onInlineCommit}
            onCancel={onInlineCancel}
          />
        </div>
      )}

      {node.isDir && node.expanded && node.children?.map((child) => (
        <FileTreeNodeItem
          key={child.path}
          node={child}
          depth={depth + 1}
          onToggle={onToggle}
          onOpen={onOpen}
          onContextMenu={onContextMenu}
          inlineInput={inlineInput}
          renamingPath={renamingPath}
          onInlineCommit={onInlineCommit}
          onInlineCancel={onInlineCancel}
          onRenameCommit={onRenameCommit}
        />
      ))}
    </>
  );
}

// ---------------------------------------------------------------------------
// Find-in-files panel (Ctrl+Shift+F)
// ---------------------------------------------------------------------------

interface SearchMatch {
  path: string;
  line: number;
  text: string;
}

function FindInFilesPanel() {
  const { t } = useTranslation();
  const workspaceRoot = useEditorStore((s) => s.workspaceRoot);
  const openTab = useEditorStore((s) => s.openTab);
  const [query, setQuery] = useState('');
  const [useRegex, setUseRegex] = useState(false);
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [results, setResults] = useState<SearchMatch[]>([]);
  const [searching, setSearching] = useState(false);

  const doSearch = useCallback(() => {
    if (!query.trim() || !workspaceRoot) return;
    setSearching(true);
    invoke<SearchMatch[]>('fs_search_content', { query, useRegex, caseSensitive })
      .then((r) => setResults(r))
      .catch(() => setResults([]))
      .finally(() => setSearching(false));
  }, [query, useRegex, caseSensitive, workspaceRoot]);

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="px-3 py-2 space-y-1 border-b border-cocoa-600">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') doSearch(); }}
          placeholder={t('search.placeholder')}
          className="w-full bg-cocoa-600 border border-cocoa-400 rounded px-2 py-1 text-xs text-cocoa-50 placeholder-cocoa-300 outline-none focus:border-biscuit-500"
          style={{ fontFamily: "'JetBrains Mono', 'Ubuntu Mono', monospace" }}
          aria-label={t('search.placeholder')}
        />
        <div className="flex gap-3 text-xs text-cocoa-300">
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="checkbox"
              checked={useRegex}
              onChange={(e) => setUseRegex(e.target.checked)}
              className="accent-biscuit-500"
            />
            {t('search.regex')}
          </label>
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="checkbox"
              checked={caseSensitive}
              onChange={(e) => setCaseSensitive(e.target.checked)}
              className="accent-biscuit-500"
            />
            {t('search.caseSensitive')}
          </label>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto">
        {searching && (
          <p className="px-3 py-2 text-xs text-cocoa-300">{t('search.searching')}</p>
        )}
        {!searching && results.length === 0 && query && (
          <p className="px-3 py-2 text-xs text-cocoa-300">{t('search.noResults')}</p>
        )}
        {results.map((m, i) => (
          <button
            key={`${m.path}:${m.line}:${i}`}
            className="w-full text-left px-3 py-1 hover:bg-cocoa-600 border-b border-cocoa-700"
            onClick={() => {
              const full = workspaceRoot ? `${workspaceRoot}/${m.path}` : m.path;
              openTab(full);
            }}
          >
            <div
              className="text-xs text-biscuit-400 truncate"
              style={{ fontFamily: "'JetBrains Mono', monospace" }}
            >
              {m.path}:{m.line}
            </div>
            <div
              className="text-xs text-cocoa-200 truncate"
              style={{ fontFamily: "'JetBrains Mono', monospace" }}
            >
              {m.text.trim()}
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Files panel (main file tree)
// ---------------------------------------------------------------------------

function FilesPanel() {
  const { t } = useTranslation();
  const { setWorkspaceRoot, workspaceRoot, openTab } = useEditorStore();
  const [rootNodes, setRootNodes] = useState<TreeNode[]>([]);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [inlineInput, setInlineInput] = useState<{ parentPath: string; isFolder: boolean } | null>(null);
  const [renamingPath, setRenamingPath] = useState<string | null>(null);

  const loadDir = useCallback(async (path: string): Promise<TreeNode[]> => {
    const entries = await invoke<DirEntry[]>('fs_list', { path });
    return entries.map((e) => ({ ...e, expanded: false, children: e.isDir ? [] : undefined }));
  }, []);

  const openFolder = useCallback(async () => {
    try {
      const root = await invoke<string>('fs_open_folder');
      setWorkspaceRoot(root);
      const nodes = await loadDir(root);
      setRootNodes(nodes);
    } catch {
      // User cancelled dialog — no-op
    }
  }, [loadDir, setWorkspaceRoot]);

  const refreshDir = useCallback(async (path: string): Promise<TreeNode[]> => {
    return loadDir(path);
  }, [loadDir]);

  const toggleNode = useCallback(async (path: string) => {
    const findNode = (nodes: TreeNode[], p: string): TreeNode | undefined => {
      for (const n of nodes) {
        if (n.path === p) return n;
        if (n.children) { const found = findNode(n.children, p); if (found) return found; }
      }
    };

    const node = findNode(rootNodes, path);
    if (!node) return;

    if (!node.expanded && (!node.children || node.children.length === 0)) {
      try {
        const children = await loadDir(path);
        setRootNodes((prev) => {
          const update = (nodes: TreeNode[]): TreeNode[] =>
            nodes.map((n) => {
              if (n.path === path) return { ...n, expanded: true, children };
              if (n.children) return { ...n, children: update(n.children) };
              return n;
            });
          return update(prev);
        });
      } catch { /* non-directory */ }
    } else {
      setRootNodes((prev) => {
        const toggle = (nodes: TreeNode[]): TreeNode[] =>
          nodes.map((n) => {
            if (n.path === path) return { ...n, expanded: !n.expanded };
            if (n.children) return { ...n, children: toggle(n.children) };
            return n;
          });
        return toggle(prev);
      });
    }
  }, [rootNodes, loadDir]);

  const handleContextMenu = useCallback((e: React.MouseEvent, node: TreeNode) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, node });
  }, []);

  const handleNewFile = useCallback((parentDir: string) => {
    setInlineInput({ parentPath: parentDir, isFolder: false });
  }, []);

  const handleNewFolder = useCallback((parentDir: string) => {
    setInlineInput({ parentPath: parentDir, isFolder: true });
  }, []);

  const handleInlineCommit = useCallback(async (name: string) => {
    if (!name || !inlineInput) { setInlineInput(null); return; }
    const fullPath = `${inlineInput.parentPath}/${name}`;
    try {
      if (inlineInput.isFolder) {
        await invoke('fs_create_dir', { path: fullPath });
      } else {
        await invoke('fs_write', { path: fullPath, content: '' });
      }
      const pPath = inlineInput.parentPath;
      const children = await refreshDir(pPath);
      setRootNodes((prev) => {
        const update = (nodes: TreeNode[]): TreeNode[] =>
          nodes.map((n) => {
            if (n.path === pPath) return { ...n, expanded: true, children };
            if (n.children) return { ...n, children: update(n.children) };
            return n;
          });
        if (workspaceRoot === pPath) return children;
        return update(prev);
      });
      if (!inlineInput.isFolder) openTab(fullPath);
    } catch { /* permission error or conflict */ }
    setInlineInput(null);
  }, [inlineInput, refreshDir, openTab, workspaceRoot]);

  const handleRename = useCallback((node: TreeNode) => {
    setRenamingPath(node.path);
  }, []);

  const handleRenameCommit = useCallback(async (oldPath: string, newName: string) => {
    if (!newName || newName === oldPath.split('/').pop()) { setRenamingPath(null); return; }
    const parent = oldPath.split('/').slice(0, -1).join('/');
    const newPath = `${parent}/${newName}`;
    try {
      await invoke('fs_rename', { from: oldPath, to: newPath });
      const children = await refreshDir(parent);
      setRootNodes((prev) => {
        const update = (nodes: TreeNode[]): TreeNode[] =>
          nodes.map((n) => {
            if (n.path === parent) return { ...n, children };
            if (n.children) return { ...n, children: update(n.children) };
            return n;
          });
        if (workspaceRoot === parent) return children;
        return update(prev);
      });
    } catch { /* permission error */ }
    setRenamingPath(null);
  }, [refreshDir, workspaceRoot]);

  const handleDelete = useCallback(async (node: TreeNode) => {
    if (!window.confirm(`Delete "${node.name}"?`)) return;
    try {
      await invoke('fs_delete', { path: node.path });
      const parent = node.path.split('/').slice(0, -1).join('/');
      const children = await refreshDir(parent);
      setRootNodes((prev) => {
        const update = (nodes: TreeNode[]): TreeNode[] =>
          nodes.map((n) => {
            if (n.path === parent) return { ...n, children };
            if (n.children) return { ...n, children: update(n.children) };
            return n;
          });
        if (workspaceRoot === parent) return children;
        return update(prev);
      });
    } catch { /* permission error */ }
  }, [refreshDir, workspaceRoot]);

  const handleCopyPath = useCallback((path: string) => {
    navigator.clipboard.writeText(path).catch(() => {/* clipboard unavailable */});
  }, []);

  const handleOpenInTerminal = useCallback((dir: string) => {
    window.dispatchEvent(new CustomEvent('biscuitcode:terminal-open-in', { detail: { cwd: dir } }));
  }, []);

  const handleInlineCancel = useCallback(() => {
    setInlineInput(null);
    setRenamingPath(null);
  }, []);

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {!workspaceRoot ? (
        <div className="flex flex-col items-center justify-center h-full px-4 gap-3">
          <p className="text-xs text-cocoa-300 text-center">{t('fileTree.noFolder')}</p>
          <button
            className="px-3 py-1.5 text-xs bg-biscuit-500 text-cocoa-900 font-semibold rounded hover:bg-biscuit-400"
            onClick={openFolder}
          >
            {t('fileTree.openFolder')}
          </button>
        </div>
      ) : (
        <>
          <div className="px-3 py-1.5 flex items-center justify-between border-b border-cocoa-600 shrink-0">
            <span
              className="text-xs font-semibold text-cocoa-200 truncate"
              title={workspaceRoot}
            >
              {workspaceRoot.split('/').pop()}
            </span>
            <button
              aria-label={t('fileTree.openFolder')}
              className="text-xs text-cocoa-300 hover:text-cocoa-50 p-0.5 rounded"
              onClick={openFolder}
              title={t('fileTree.changeFolder')}
            >
              ⊕
            </button>
          </div>
          <div
            className="flex-1 overflow-y-auto py-1"
            role="tree"
            aria-label={t('fileTree.aria')}
          >
            {rootNodes.map((node) => (
              <FileTreeNodeItem
                key={node.path}
                node={node}
                depth={0}
                onToggle={toggleNode}
                onOpen={openTab}
                onContextMenu={handleContextMenu}
                inlineInput={inlineInput}
                renamingPath={renamingPath}
                onInlineCommit={handleInlineCommit}
                onInlineCancel={handleInlineCancel}
                onRenameCommit={handleRenameCommit}
              />
            ))}
          </div>
        </>
      )}
      {contextMenu && (
        <ContextMenu
          state={contextMenu}
          onClose={() => setContextMenu(null)}
          onNewFile={handleNewFile}
          onNewFolder={handleNewFolder}
          onRename={handleRename}
          onDelete={handleDelete}
          onCopyPath={handleCopyPath}
          onOpenInTerminal={handleOpenInTerminal}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// SidePanel — routes to the right sub-panel by activity
// ---------------------------------------------------------------------------

export function SidePanel() {
  const { t } = useTranslation();
  const { activeActivity } = usePanelsStore();

  const renderContent = () => {
    switch (activeActivity) {
      case 'files':    return <FilesPanel />;
      case 'search':   return <FindInFilesPanel />;
      case 'git':      return <GitPanel />;
      case 'settings': return <SettingsPage />;
      default:
        return (
          <div className="px-3 py-4 text-sm text-cocoa-300">
            <em>{t(`panels.${activeActivity}`)} {t('panels.comingSoon')}</em>
          </div>
        );
    }
  };

  const headerLabel: Record<string, string> = {
    files:    t('panels.files'),
    search:   t('panels.search'),
    git:      t('panels.git'),
    chats:    t('panels.chats'),
    settings: t('panels.settings'),
  };

  return (
    <aside
      aria-label={t('panels.sidePanel')}
      className="h-full flex flex-col bg-cocoa-700 border-r border-cocoa-500 overflow-hidden"
    >
      <header className="px-3 py-2 text-xs font-semibold uppercase tracking-wider text-cocoa-200 border-b border-cocoa-600 shrink-0">
        {headerLabel[activeActivity] ?? activeActivity}
      </header>
      <div className="flex-1 overflow-hidden">
        {renderContent()}
      </div>
    </aside>
  );
}
