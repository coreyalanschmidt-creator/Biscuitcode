// src/components/TerminalPanel.tsx
//
// Phase 4 — Multi-tab integrated terminal.
// - xterm.js with @xterm/addon-fit, @xterm/addon-web-links,
//   @xterm/addon-search, @xterm/addon-webgl (canvas fallback).
// - Each tab maps to a biscuitcode-pty SessionId.
// - Output arrives on `terminal_data_<session_id>` Tauri events.
// - Custom link provider: `path/to/file:line[:col]` → `open_file_at` event.
// - Ctrl+` focuses the active tab (wires the Phase 2 placeholder).
// - `+` button opens a new tab; `×` button closes and drops the PTY.

import {
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { SearchAddon } from '@xterm/addon-search';
import { WebglAddon } from '@xterm/addon-webgl';
import { useTranslation } from 'react-i18next';
import '@xterm/xterm/css/xterm.css';

// ---------- Types ----------

interface Tab {
  id: string;       // SessionId from the Rust backend
  title: string;
}

interface TerminalInstance {
  terminal: Terminal;
  fitAddon: FitAddon;
  unlisten: UnlistenFn;
}

// ---------- Constants ----------

const FONT_FAMILY = "'JetBrains Mono', 'Ubuntu Mono', 'DejaVu Sans Mono', monospace";

const XTERM_THEME = {
  background: '#1C1610',   // cocoa-700
  foreground: '#E0D3BE',   // cocoa-100
  cursor:     '#E8B04C',   // biscuit-500
  cursorAccent: '#1C1610',
  selectionBackground: 'rgba(232,176,76,0.3)',
  black:   '#080504',
  red:     '#E06B5B',
  green:   '#6FBF6E',
  yellow:  '#E8B04C',
  blue:    '#5B8CE0',
  magenta: '#9B6BE0',
  cyan:    '#6BBFE0',
  white:   '#E0D3BE',
  brightBlack:   '#584938',
  brightRed:     '#E8835B',
  brightGreen:   '#8FD96E',
  brightYellow:  '#F0C065',
  brightBlue:    '#7BAAE8',
  brightMagenta: '#B38AE8',
  brightCyan:    '#8AD9E8',
  brightWhite:   '#F6F0E8',
};

// Regex matching path/to/file:line or path/to/file:line:col
const FILE_LINK_RE = /(?:^|[\s"'(,])([./\w-][/\w.-]*\w\.[a-z]+):(\d+)(?::(\d+))?/g;

let tabCounter = 1;

// ---------- Component ----------

export function TerminalPanel() {
  const { t } = useTranslation();

  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);

  // Map: sessionId -> TerminalInstance (xterm + addons + unlisten fn)
  const instances = useRef<Map<string, TerminalInstance>>(new Map());
  // Map: sessionId -> DOM container element ref
  const containers = useRef<Map<string, HTMLDivElement>>(new Map());

  // ---------- Open a new tab ----------

  const openTab = useCallback(async (cwd?: string) => {
    const sessionId: string = await invoke('terminal_open', {
      shell: null,
      cwd: cwd ?? null,
      rows: 24,
      cols: 80,
    });

    const title = `Terminal ${tabCounter++}`;
    setTabs((prev) => [...prev, { id: sessionId, title }]);
    setActiveId(sessionId);
  }, []);

  // ---------- Mount xterm.js when a container becomes available ----------

  const mountTerminal = useCallback((sessionId: string, el: HTMLDivElement) => {
    if (instances.current.has(sessionId)) return; // already mounted

    const terminal = new Terminal({
      fontFamily: FONT_FAMILY,
      fontSize: 13,
      lineHeight: 1.5,
      theme: XTERM_THEME,
      cursorBlink: true,
      scrollback: 5000,
      allowTransparency: false,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();
    const searchAddon = new SearchAddon();

    terminal.loadAddon(fitAddon);
    terminal.loadAddon(webLinksAddon);
    terminal.loadAddon(searchAddon);

    // WebGL renderer with canvas fallback
    try {
      const webglAddon = new WebglAddon();
      webglAddon.onContextLoss(() => {
        webglAddon.dispose();
      });
      terminal.loadAddon(webglAddon);
    } catch {
      // Canvas renderer is the automatic fallback — nothing to do.
    }

    terminal.open(el);
    fitAddon.fit();

    // Custom link provider: path/to/file:line[:col] → open_file_at event
    terminal.registerLinkProvider({
      provideLinks(y, callback) {
        const line = terminal.buffer.active.getLine(y);
        if (!line) { callback([]); return; }
        const text = line.translateToString(true);
        const links: Array<{ range: { start: { x: number; y: number }; end: { x: number; y: number } }; text: string; activate: () => void }> = [];
        FILE_LINK_RE.lastIndex = 0;
        let match: RegExpExecArray | null;
        while ((match = FILE_LINK_RE.exec(text)) !== null) {
          const fullMatch = match[0];
          const filePath = match[1];
          const lineNum = parseInt(match[2], 10);
          const colNum = match[3] ? parseInt(match[3], 10) : 1;
          // Offset for leading whitespace/punctuation in the match group.
          const startX = match.index + (fullMatch.length - fullMatch.trimStart().length);
          const endX = startX + filePath.length + 1 + match[2].length + (match[3] ? 1 + match[3].length : 0);
          links.push({
            range: {
              start: { x: startX + 1, y },
              end:   { x: endX + 1,   y },
            },
            text: `${filePath}:${lineNum}`,
            activate() {
              window.dispatchEvent(
                new CustomEvent('biscuitcode:open-file-at', {
                  detail: { path: filePath, line: lineNum, col: colNum },
                }),
              );
            },
          });
        }
        callback(links);
      },
    });

    // Forward keyboard input to the PTY
    terminal.onData((data) => {
      invoke('terminal_input', {
        sessionId,
        data: Array.from(new TextEncoder().encode(data)),
      }).catch(() => {/* session may be closing */});
    });

    // Listen for PTY output
    let unlistenFn: UnlistenFn = () => {};
    listen<{ data: number[] }>(`terminal_data_${sessionId}`, (event) => {
      terminal.write(new Uint8Array(event.payload.data));
    }).then((fn) => { unlistenFn = fn; });

    instances.current.set(sessionId, {
      terminal,
      fitAddon,
      unlisten: () => unlistenFn(),
    });

    // Fit once mounted, then resize the PTY to match.
    setTimeout(() => {
      fitAddon.fit();
      invoke('terminal_resize', {
        sessionId,
        rows: terminal.rows,
        cols: terminal.cols,
      }).catch(() => {});
    }, 0);
  }, []);

  // ---------- Resize observer ----------

  const resizeObservers = useRef<Map<string, ResizeObserver>>(new Map());

  const attachResizeObserver = useCallback((sessionId: string, el: HTMLDivElement) => {
    if (resizeObservers.current.has(sessionId)) return;
    const ro = new ResizeObserver(() => {
      const inst = instances.current.get(sessionId);
      if (!inst) return;
      inst.fitAddon.fit();
      invoke('terminal_resize', {
        sessionId,
        rows: inst.terminal.rows,
        cols: inst.terminal.cols,
      }).catch(() => {});
    });
    ro.observe(el);
    resizeObservers.current.set(sessionId, ro);
  }, []);

  // ---------- Close a tab ----------

  const closeTab = useCallback(async (sessionId: string) => {
    const inst = instances.current.get(sessionId);
    if (inst) {
      inst.unlisten();
      inst.terminal.dispose();
      instances.current.delete(sessionId);
    }
    const ro = resizeObservers.current.get(sessionId);
    if (ro) {
      ro.disconnect();
      resizeObservers.current.delete(sessionId);
    }
    containers.current.delete(sessionId);

    await invoke('terminal_close', { sessionId }).catch(() => {});

    setTabs((prev) => {
      const remaining = prev.filter((t) => t.id !== sessionId);
      return remaining;
    });
    setActiveId((prev) => {
      if (prev !== sessionId) return prev;
      const remaining = tabs.filter((t) => t.id !== sessionId);
      return remaining.length > 0 ? remaining[remaining.length - 1].id : null;
    });
  }, [tabs]);

  // ---------- Container ref callback ----------

  const containerRef = useCallback(
    (sessionId: string) => (el: HTMLDivElement | null) => {
      if (el) {
        containers.current.set(sessionId, el);
        mountTerminal(sessionId, el);
        attachResizeObserver(sessionId, el);
      }
    },
    [mountTerminal, attachResizeObserver],
  );

  // ---------- Focus on Ctrl+` ----------

  useEffect(() => {
    const handleFocus = () => {
      if (activeId) {
        const inst = instances.current.get(activeId);
        inst?.terminal.focus();
      }
    };
    window.addEventListener('biscuitcode:terminal-focus', handleFocus);
    return () => window.removeEventListener('biscuitcode:terminal-focus', handleFocus);
  }, [activeId]);

  // ---------- Open in Terminal from file tree ----------

  useEffect(() => {
    const handleOpenInTerminal = (e: Event) => {
      const { cwd } = (e as CustomEvent<{ cwd: string }>).detail;
      openTab(cwd);
    };
    window.addEventListener('biscuitcode:terminal-open-in', handleOpenInTerminal);
    return () => window.removeEventListener('biscuitcode:terminal-open-in', handleOpenInTerminal);
  }, [openTab]);

  // Open a default tab on first mount if none exist
  useEffect(() => {
    if (tabs.length === 0) {
      openTab();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Focus the active terminal when the active tab changes
  useEffect(() => {
    if (activeId) {
      const inst = instances.current.get(activeId);
      inst?.terminal.focus();
    }
  }, [activeId]);

  // Cleanup all sessions on unmount
  useEffect(() => {
    return () => {
      for (const [id, inst] of instances.current) {
        inst.unlisten();
        inst.terminal.dispose();
        invoke('terminal_close', { sessionId: id }).catch(() => {});
      }
    };
  }, []);

  // ---------- Render ----------

  return (
    <section
      aria-label={t('panels.terminal')}
      className="h-full flex flex-col bg-cocoa-800 border-t border-cocoa-600"
    >
      {/* Tab bar */}
      <div
        className="flex items-center gap-0 bg-cocoa-700 border-b border-cocoa-600 select-none shrink-0"
        role="tablist"
        aria-label={t('panels.terminal')}
      >
        {tabs.map((tab) => (
          <div
            key={tab.id}
            role="tab"
            aria-selected={activeId === tab.id}
            className={[
              'flex items-center gap-1 px-3 py-1.5 text-xs cursor-pointer border-r border-cocoa-600',
              activeId === tab.id
                ? 'bg-cocoa-800 text-cocoa-50'
                : 'text-cocoa-300 hover:text-cocoa-100 hover:bg-cocoa-600',
            ].join(' ')}
            onClick={() => setActiveId(tab.id)}
          >
            <span>{tab.title}</span>
            <button
              aria-label={t('common.close')}
              className="ml-1 opacity-50 hover:opacity-100 rounded"
              onClick={(e) => {
                e.stopPropagation();
                closeTab(tab.id);
              }}
            >
              ×
            </button>
          </div>
        ))}

        {/* New tab button */}
        <button
          aria-label="New terminal"
          className="px-2 py-1.5 text-cocoa-300 hover:text-cocoa-100 hover:bg-cocoa-600"
          onClick={() => openTab()}
        >
          +
        </button>
      </div>

      {/* Terminal panes — all mounted, only active one visible */}
      <div className="flex-1 relative overflow-hidden">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            ref={containerRef(tab.id)}
            role="tabpanel"
            className={[
              'absolute inset-0',
              activeId === tab.id ? 'block' : 'hidden',
            ].join(' ')}
            style={{ padding: '4px' }}
          />
        ))}
        {tabs.length === 0 && (
          <div className="flex items-center justify-center h-full text-sm text-cocoa-400">
            {t('panels.terminal')}
          </div>
        )}
      </div>
    </section>
  );
}
