// src/components/GitPanel.tsx
//
// Phase 7 deliverable: Git panel in the Side Panel.
// Shows staged / unstaged / untracked file groups with stage/unstage buttons,
// a commit message input, commit/push/pull buttons, and a branch switcher.
//
// Write operations stream output to the terminal via biscuitcode:terminal-open-in.
// Error E012 GitPushFailed is emitted as a biscuitcode:error-toast event.

import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { GitCommit, GitBranch, RefreshCw, Upload, Download, Plus, Minus } from 'lucide-react';

// ---------- Types ----------

interface GitFileStatus {
  path: string;
  bucket: 'staged' | 'unstaged' | 'untracked';
  status_code: string;
}

interface GitStatus {
  branch: string;
  files: GitFileStatus[];
}

interface BranchSwitcherProps {
  current: string;
  onClose: () => void;
  onSwitch: (branch: string) => void;
}

// ---------- Branch Switcher Dropdown ----------

function BranchSwitcher({ current, onClose, onSwitch }: BranchSwitcherProps) {
  const { t } = useTranslation();
  const [branches, setBranches] = useState<string[]>([]);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<string[]>('git_branches')
      .then(setBranches)
      .catch(() => setBranches([]));
  }, []);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [onClose]);

  return (
    <div
      ref={ref}
      role="listbox"
      aria-label={t('git.switchBranch')}
      className="absolute top-full left-0 z-50 mt-1 bg-cocoa-600 border border-cocoa-400 rounded shadow-xl min-w-[180px] max-h-48 overflow-y-auto"
    >
      {branches.length === 0 && (
        <p className="px-3 py-2 text-xs text-cocoa-300">{t('git.refreshing')}</p>
      )}
      {branches.map((b) => (
        <button
          key={b}
          role="option"
          aria-selected={b === current}
          className={`w-full text-left px-3 py-1.5 text-xs hover:bg-cocoa-500 ${
            b === current ? 'text-biscuit-400 font-semibold' : 'text-cocoa-100'
          }`}
          style={{ fontFamily: "'JetBrains Mono', 'Ubuntu Mono', monospace" }}
          onClick={() => { onSwitch(b); onClose(); }}
        >
          {b === current ? `✓ ${b}` : b}
        </button>
      ))}
    </div>
  );
}

// ---------- File Item ----------

interface FileItemProps {
  file: GitFileStatus;
  onStage: (path: string) => void;
  onUnstage: (path: string) => void;
  onOpen: (path: string) => void;
}

function statusColor(code: string): string {
  switch (code) {
    case 'M': return 'text-yellow-400';
    case 'A': return 'text-green-400';
    case 'D': return 'text-red-400';
    case 'R': return 'text-blue-400';
    default:  return 'text-cocoa-300';
  }
}

function FileItem({ file, onStage, onUnstage, onOpen }: FileItemProps) {
  const { t } = useTranslation();
  const name = file.path.split('/').pop() ?? file.path;
  const isStaged = file.bucket === 'staged';

  return (
    <div
      className="flex items-center gap-1 py-0.5 pl-4 pr-2 hover:bg-cocoa-600 group cursor-pointer"
      onClick={() => onOpen(file.path)}
    >
      <span className={`text-xs font-mono w-4 shrink-0 ${statusColor(file.status_code)}`}>
        {file.status_code}
      </span>
      <span
        className="flex-1 text-xs text-cocoa-100 truncate"
        style={{ fontFamily: "'JetBrains Mono', 'Ubuntu Mono', monospace" }}
        title={file.path}
      >
        {name}
      </span>
      <button
        aria-label={isStaged ? t('git.unstageFile', { path: file.path }) : t('git.stageFile', { path: file.path })}
        className="opacity-0 group-hover:opacity-100 p-0.5 rounded text-cocoa-300 hover:text-cocoa-50"
        onClick={(e) => {
          e.stopPropagation();
          if (isStaged) onUnstage(file.path);
          else onStage(file.path);
        }}
      >
        {isStaged ? <Minus className="w-3 h-3" /> : <Plus className="w-3 h-3" />}
      </button>
    </div>
  );
}

// ---------- File Group ----------

interface FileGroupProps {
  label: string;
  files: GitFileStatus[];
  onStageAll?: () => void;
  onUnstageAll?: () => void;
  onStage: (path: string) => void;
  onUnstage: (path: string) => void;
  onOpen: (path: string) => void;
}

function FileGroup({ label, files, onStageAll, onUnstageAll, onStage, onUnstage, onOpen }: FileGroupProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(true);
  if (files.length === 0) return null;

  return (
    <div>
      <button
        className="w-full flex items-center gap-1 px-2 py-1 text-xs font-semibold uppercase tracking-wider text-cocoa-300 hover:bg-cocoa-600"
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
      >
        <span className="flex-1 text-left">{label} ({files.length})</span>
        {onStageAll && (
          <button
            aria-label={t('git.stageAll')}
            className="text-cocoa-400 hover:text-cocoa-50 p-0.5"
            onClick={(e) => { e.stopPropagation(); onStageAll(); }}
            title={t('git.stageAll')}
          >
            <Plus className="w-3 h-3" />
          </button>
        )}
        {onUnstageAll && (
          <button
            aria-label={t('git.unstageAll')}
            className="text-cocoa-400 hover:text-cocoa-50 p-0.5"
            onClick={(e) => { e.stopPropagation(); onUnstageAll(); }}
            title={t('git.unstageAll')}
          >
            <Minus className="w-3 h-3" />
          </button>
        )}
      </button>
      {expanded && files.map((f) => (
        <FileItem
          key={f.path}
          file={f}
          onStage={onStage}
          onUnstage={onUnstage}
          onOpen={onOpen}
        />
      ))}
    </div>
  );
}

// ---------- GitPanel ----------

export function GitPanel() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<GitStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [commitMsg, setCommitMsg] = useState('');
  const [pushing, setPushing] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [showBranches, setShowBranches] = useState(false);
  const branchBtnRef = useRef<HTMLButtonElement>(null);

  const refresh = useCallback(() => {
    invoke<GitStatus>('git_status')
      .then((s) => { setStatus(s); setError(null); })
      .catch((e) => {
        const msg = String(e);
        if (msg.includes('no workspace')) {
          setError(t('git.noWorkspace'));
        } else {
          setError(t('git.noRepo'));
        }
        setStatus(null);
      });
  }, [t]);

  useEffect(() => {
    refresh();
    // Re-refresh when a file is saved (biscuitcode:file-saved event from EditorArea).
    const handler = () => refresh();
    window.addEventListener('biscuitcode:file-saved', handler);
    return () => window.removeEventListener('biscuitcode:file-saved', handler);
  }, [refresh]);

  const handleStage = useCallback(async (path: string) => {
    await invoke('git_stage', { path }).catch(() => {});
    refresh();
  }, [refresh]);

  const handleUnstage = useCallback(async (path: string) => {
    await invoke('git_unstage', { path }).catch(() => {});
    refresh();
  }, [refresh]);

  const handleStageAll = useCallback(async () => {
    await invoke('git_stage', { path: '.' }).catch(() => {});
    refresh();
  }, [refresh]);

  const handleUnstageAll = useCallback(async () => {
    await invoke('git_unstage', { path: '.' }).catch(() => {});
    refresh();
  }, [refresh]);

  const handleCommit = useCallback(async () => {
    if (!commitMsg.trim()) return;
    await invoke('git_commit', { message: commitMsg }).catch(() => {});
    setCommitMsg('');
    refresh();
  }, [commitMsg, refresh]);

  const handlePush = useCallback(async () => {
    setPushing(true);
    try {
      await invoke('git_push');
    } catch (e) {
      const msg = String(e);
      if (msg.startsWith('E012:')) {
        const stderr = msg.slice(5);
        // Emit E012 toast event.
        window.dispatchEvent(new CustomEvent('biscuitcode:error-toast', {
          detail: {
            code: 'E012',
            messageKey: 'errors.E012.msg',
            interpolations: { git_stderr: stderr.slice(0, 200) },
            recovery: { kind: 'dismiss_only' },
          },
        }));
      }
    }
    setPushing(false);
    refresh();
  }, [refresh]);

  const handlePull = useCallback(async () => {
    setPulling(true);
    await invoke('git_pull').catch(() => {});
    setPulling(false);
    refresh();
  }, [refresh]);

  const handleSwitchBranch = useCallback(async (branch: string) => {
    await invoke('git_checkout', { branch }).catch(() => {});
    refresh();
  }, [refresh]);

  const handleOpenFile = useCallback((path: string) => {
    // Reuse the biscuitcode:open-file-at event from Phase 6b (EditorArea extension).
    window.dispatchEvent(new CustomEvent('biscuitcode:open-file-at', { detail: { path } }));
  }, []);

  if (error) {
    return (
      <div className="px-3 py-4 text-xs text-cocoa-300">
        {error}
      </div>
    );
  }

  if (!status) {
    return (
      <div className="px-3 py-4 text-xs text-cocoa-300">
        {t('git.refreshing')}
      </div>
    );
  }

  const staged   = status.files.filter((f) => f.bucket === 'staged');
  const unstaged = status.files.filter((f) => f.bucket === 'unstaged');
  const untracked = status.files.filter((f) => f.bucket === 'untracked');

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Branch bar */}
      <div className="flex items-center gap-1 px-2 py-1.5 border-b border-cocoa-600 shrink-0 relative">
        <GitBranch className="w-3.5 h-3.5 text-biscuit-400 shrink-0" aria-hidden />
        <button
          ref={branchBtnRef}
          className="flex-1 text-left text-xs text-cocoa-100 hover:text-cocoa-50 font-mono truncate"
          onClick={() => setShowBranches((v) => !v)}
          aria-label={t('git.currentBranch', { branch: status.branch })}
          aria-expanded={showBranches}
          aria-haspopup="listbox"
          title={t('git.switchBranch')}
        >
          {status.branch}
        </button>
        <button
          aria-label="Refresh git status"
          className="p-0.5 text-cocoa-400 hover:text-cocoa-50 rounded"
          onClick={refresh}
        >
          <RefreshCw className="w-3 h-3" />
        </button>
        {showBranches && (
          <BranchSwitcher
            current={status.branch}
            onClose={() => setShowBranches(false)}
            onSwitch={handleSwitchBranch}
          />
        )}
      </div>

      {/* File groups */}
      <div className="flex-1 overflow-y-auto">
        <FileGroup
          label={t('git.staged')}
          files={staged}
          onUnstageAll={handleUnstageAll}
          onStage={handleStage}
          onUnstage={handleUnstage}
          onOpen={handleOpenFile}
        />
        <FileGroup
          label={t('git.unstaged')}
          files={unstaged}
          onStageAll={handleStageAll}
          onStage={handleStage}
          onUnstage={handleUnstage}
          onOpen={handleOpenFile}
        />
        <FileGroup
          label={t('git.untracked')}
          files={untracked}
          onStageAll={() => Promise.all(untracked.map((f) => handleStage(f.path)))}
          onStage={handleStage}
          onUnstage={handleUnstage}
          onOpen={handleOpenFile}
        />
        {staged.length === 0 && unstaged.length === 0 && untracked.length === 0 && (
          <p className="px-3 py-3 text-xs text-cocoa-400">No changes.</p>
        )}
      </div>

      {/* Commit controls */}
      <div className="shrink-0 border-t border-cocoa-600 p-2 space-y-1.5">
        <textarea
          value={commitMsg}
          onChange={(e) => setCommitMsg(e.target.value)}
          placeholder={t('git.commitPlaceholder')}
          rows={2}
          className="w-full bg-cocoa-600 border border-cocoa-400 rounded px-2 py-1 text-xs text-cocoa-50 placeholder-cocoa-400 resize-none outline-none focus:border-biscuit-500"
          style={{ fontFamily: "'Inter', 'Ubuntu', sans-serif" }}
          aria-label={t('git.commitPlaceholder')}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && e.ctrlKey) { e.preventDefault(); handleCommit(); }
          }}
        />
        <div className="flex gap-1">
          <button
            className="flex-1 flex items-center justify-center gap-1 py-1 text-xs bg-biscuit-500 text-cocoa-900 font-semibold rounded hover:bg-biscuit-400 disabled:opacity-40"
            onClick={handleCommit}
            disabled={!commitMsg.trim() || staged.length === 0}
            aria-label={t('git.commitButton')}
          >
            <GitCommit className="w-3 h-3" />
            {t('git.commitButton')}
          </button>
          <button
            className="px-2 py-1 text-xs bg-cocoa-600 text-cocoa-100 rounded hover:bg-cocoa-500 disabled:opacity-40"
            onClick={handlePush}
            disabled={pushing}
            aria-label={t('git.pushButton')}
            title={t('git.pushButton')}
          >
            {pushing ? '…' : <Upload className="w-3 h-3" />}
          </button>
          <button
            className="px-2 py-1 text-xs bg-cocoa-600 text-cocoa-100 rounded hover:bg-cocoa-500 disabled:opacity-40"
            onClick={handlePull}
            disabled={pulling}
            aria-label={t('git.pullButton')}
            title={t('git.pullButton')}
          >
            {pulling ? '…' : <Download className="w-3 h-3" />}
          </button>
        </div>
      </div>
    </div>
  );
}
