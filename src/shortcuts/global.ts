// src/shortcuts/global.ts
//
// Global shortcut registration. Wires every shortcut from the vision's
// keyboard table to either a real action (if its target system is shipped)
// or a placeholder toast ("registered; wiring lands in Phase N").
//
// Phase 2 deliverable. No silent no-ops — every key combo in the table
// either DOES something or fires the placeholder toast. CI test
// `tests/shortcuts/global.spec.ts` iterates over this module's table
// to assert.
//
// Chord support: Ctrl+K Ctrl+I needs a two-stage handler — pressing
// Ctrl+K enters "chord" state, the next keypress (within 800ms) is
// matched against chord followers; if no match, chord state clears
// silently.

import { useEffect } from 'react';
import { usePanelsStore } from '../state/panelsStore';

export type ShortcutHandler = (e: KeyboardEvent) => void;

export interface ShortcutSpec {
  /** Display label, e.g. `"Ctrl+B"` or `"Ctrl+K Ctrl+I"`. */
  label: string;
  /** Owning phase number — used in placeholder toast text. */
  phase: number;
  /** What this shortcut does. Pure side-effect. */
  handler: ShortcutHandler;
}

/**
 * The full vision-mandated shortcut table. Every entry MUST resolve to
 * either a real action or a placeholder toast — never a silent no-op.
 *
 * When a later phase ships, REPLACE the relevant `placeholder('Foo', N)`
 * with the real handler — do not add new keys here.
 */
export const SHORTCUTS: Readonly<Record<string, ShortcutSpec>> = Object.freeze({
  'Ctrl+B': {
    label: 'Ctrl+B',
    phase: 2,
    handler: () => usePanelsStore.getState().toggleSide(),
  },
  'Ctrl+J': {
    label: 'Ctrl+J',
    phase: 2,
    handler: () => usePanelsStore.getState().toggleBottom(),
  },
  'Ctrl+Alt+C': {
    label: 'Ctrl+Alt+C',
    phase: 2,
    handler: () => usePanelsStore.getState().toggleChat(),
  },
  'Ctrl+P': {
    label: 'Ctrl+P',
    phase: 3,
    handler: () => window.dispatchEvent(new CustomEvent('biscuitcode:editor-quick-open')),
  },
  'Ctrl+Shift+P': {
    label: 'Ctrl+Shift+P',
    phase: 2,
    handler: () => openCommandPalette(),
  },
  'Ctrl+`': {
    label: 'Ctrl+`',
    phase: 4,
    handler: () => {
      // Show the bottom panel (where the terminal lives) and focus the terminal.
      usePanelsStore.getState().setBottomVisible(true);
      window.dispatchEvent(new CustomEvent('biscuitcode:terminal-focus'));
    },
  },
  'Ctrl+\\': {
    label: 'Ctrl+\\',
    phase: 3,
    handler: () => window.dispatchEvent(new CustomEvent('biscuitcode:editor-split')),
  },
  'Ctrl+K Ctrl+I': {
    label: 'Ctrl+K Ctrl+I',
    phase: 6,
    handler: placeholder('Ctrl+K Ctrl+I inline AI edit', 6),
  },
  'Ctrl+L': {
    label: 'Ctrl+L',
    phase: 5,
    handler: placeholder('Ctrl+L send selection to chat', 5),
  },
  'Ctrl+Shift+L': {
    label: 'Ctrl+Shift+L',
    phase: 5,
    handler: placeholder('Ctrl+Shift+L new chat', 5),
  },
  F1: {
    label: 'F1',
    phase: 8,
    handler: placeholder('F1 help', 8),
  },
});

/**
 * React hook that installs all global shortcuts on mount and removes
 * them on unmount. Mount once, in `App.tsx`.
 */
export function useGlobalShortcuts(): void {
  useEffect(() => {
    const dispatch = makeDispatcher();
    window.addEventListener('keydown', dispatch);
    return () => window.removeEventListener('keydown', dispatch);
  }, []);
}

// ---------- Internals ----------

const CHORD_LEADER = 'Ctrl+K';
const CHORD_TIMEOUT_MS = 800;

function makeDispatcher(): (e: KeyboardEvent) => void {
  let chordPending: number | null = null;

  return (e: KeyboardEvent) => {
    // Skip if event target is an editable element — Monaco/text inputs
    // own most key combos within their bounds.
    if (isEditableTarget(e.target)) return;

    const combo = comboFromEvent(e);
    if (!combo) return;

    // Chord follow-up?
    if (chordPending !== null) {
      const chord = `${CHORD_LEADER} ${combo}`;
      const spec = SHORTCUTS[chord];
      if (spec) {
        e.preventDefault();
        spec.handler(e);
      }
      window.clearTimeout(chordPending);
      chordPending = null;
      return;
    }

    // Chord leader?
    if (combo === CHORD_LEADER) {
      e.preventDefault();
      chordPending = window.setTimeout(() => { chordPending = null; }, CHORD_TIMEOUT_MS);
      return;
    }

    // Single-combo shortcut?
    const spec = SHORTCUTS[combo];
    if (spec) {
      e.preventDefault();
      spec.handler(e);
    }
  };
}

function comboFromEvent(e: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');

  const k = e.key;
  // Ignore pure modifier presses.
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(k)) return null;

  // Normalize a few keys to match the vision's notation.
  let keyName: string;
  if (k === ' ') keyName = 'Space';
  else if (k === 'Escape') keyName = 'Escape';
  else if (k === 'Enter') keyName = 'Enter';
  else if (k === 'F1') keyName = 'F1';
  else if (k.length === 1) keyName = k.toUpperCase();
  else keyName = k;

  parts.push(keyName);
  return parts.join('+');
}

function isEditableTarget(t: EventTarget | null): boolean {
  if (!(t instanceof HTMLElement)) return false;
  if (t.isContentEditable) return true;
  const tag = t.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
}

function placeholder(name: string, phase: number): ShortcutHandler {
  return () => {
    // Fire a placeholder toast. The toast layer (Phase 1's ErrorToast pattern,
    // generalized to non-error toasts in Phase 2) consumes this event.
    window.dispatchEvent(new CustomEvent('biscuitcode:toast', {
      detail: {
        kind: 'info',
        text: `${name} registered; wiring lands in Phase ${phase}`,
      },
    }));
  };
}

function openCommandPalette(): void {
  window.dispatchEvent(new CustomEvent('biscuitcode:open-command-palette'));
}
