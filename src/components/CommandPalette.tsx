// src/components/CommandPalette.tsx
//
// Ctrl+Shift+P command palette. Phase 2 ships with three view-toggle
// commands so the registry mechanism is proven; later phases push their
// own commands via `registerCommand()` (added when the first such phase
// lands — Phase 3 quick-open, Phase 7 git, etc.).
//
// Listens for the `biscuitcode:open-command-palette` event from the
// global shortcut layer.

import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { usePanelsStore } from '../state/panelsStore';

interface Command {
  id: string;
  label: string;
  run: () => void;
}

export function CommandPalette() {
  const { t } = useTranslation();
  const { toggleSide, toggleBottom, toggleChat } = usePanelsStore();

  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');

  // Built-in command registry. Later phases extend via window-event push.
  const commands = useMemo<Command[]>(() => [
    { id: 'view.toggleSide',   label: 'View: Toggle Side Panel',   run: toggleSide },
    { id: 'view.toggleBottom', label: 'View: Toggle Bottom Panel', run: toggleBottom },
    { id: 'view.toggleChat',   label: 'View: Toggle Chat Panel',   run: toggleChat },
  ], [toggleSide, toggleBottom, toggleChat]);

  useEffect(() => {
    const onOpen = () => { setOpen(true); setQuery(''); };
    window.addEventListener('biscuitcode:open-command-palette', onOpen);
    return () => window.removeEventListener('biscuitcode:open-command-palette', onOpen);
  }, []);

  if (!open) return null;

  const q = query.trim().toLowerCase();
  const filtered = q
    ? commands.filter((c) => c.label.toLowerCase().includes(q))
    : commands;

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      setOpen(false);
      e.preventDefault();
    } else if (e.key === 'Enter' && filtered.length > 0) {
      filtered[0].run();
      setOpen(false);
      e.preventDefault();
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t('common.openSettings')}
      className="fixed inset-0 z-50 flex items-start justify-center pt-24 bg-black/40"
      onClick={() => setOpen(false)}
    >
      <div
        className="w-full max-w-md bg-cocoa-700 border border-cocoa-500 rounded-lg shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          autoFocus
          type="text"
          value={query}
          placeholder="Type a command…"
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKeyDown}
          className="w-full px-4 py-3 bg-cocoa-800 text-cocoa-50 text-sm placeholder-cocoa-300 focus:outline-none border-b border-cocoa-500"
        />
        <ul className="max-h-72 overflow-auto py-1">
          {filtered.length === 0 && (
            <li className="px-4 py-2 text-sm text-cocoa-300">
              <em>No commands match "{query}"</em>
            </li>
          )}
          {filtered.map((c, i) => (
            <li key={c.id}>
              <button
                type="button"
                onClick={() => { c.run(); setOpen(false); }}
                className={`
                  w-full text-left px-4 py-2 text-sm transition-colors
                  ${i === 0 ? 'bg-cocoa-600 text-cocoa-50' : 'text-cocoa-100 hover:bg-cocoa-600 hover:text-cocoa-50'}
                `}
              >
                {c.label}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
