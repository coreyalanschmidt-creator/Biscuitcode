// tests/shortcuts/global.spec.ts
//
// Phase 2 deliverable. Asserts every shortcut from the vision's keyboard
// table is registered and dispatches either a real action or a placeholder
// toast — none silently no-op.

import { describe, expect, it, beforeEach } from 'vitest';
import { SHORTCUTS } from '../../src/shortcuts/global';
import { usePanelsStore } from '../../src/state/panelsStore';

/**
 * Verbatim from `docs/vision.md` "Keyboard shortcuts" table. Synced with
 * `src/shortcuts/global.ts` SHORTCUTS map. CI fails if these diverge.
 */
const VISION_TABLE: ReadonlyArray<string> = [
  'Ctrl+B',
  'Ctrl+J',
  'Ctrl+Alt+C',
  'Ctrl+P',
  'Ctrl+Shift+P',
  'Ctrl+`',
  'Ctrl+\\',
  'Ctrl+K Ctrl+I',
  'Ctrl+L',
  'Ctrl+Shift+L',
  'F1',
];

describe('global shortcuts registry', () => {
  it('registers every shortcut in the vision table', () => {
    for (const combo of VISION_TABLE) {
      expect(SHORTCUTS[combo], `missing shortcut: ${combo}`).toBeDefined();
    }
  });

  it('every entry has a callable handler', () => {
    for (const [combo, spec] of Object.entries(SHORTCUTS)) {
      expect(typeof spec.handler, `${combo}: handler not a function`).toBe('function');
    }
  });

  it('every entry has a phase number documenting when it goes live', () => {
    for (const [combo, spec] of Object.entries(SHORTCUTS)) {
      expect(spec.phase, `${combo}: phase missing`).toBeGreaterThan(0);
      expect(spec.phase, `${combo}: phase impossible`).toBeLessThanOrEqual(10);
    }
  });

  it('contains no extra entries not in the vision table', () => {
    const expectedSet = new Set(VISION_TABLE);
    for (const combo of Object.keys(SHORTCUTS)) {
      expect(expectedSet.has(combo), `unexpected shortcut: ${combo}`).toBe(true);
    }
  });

  it('vision table size matches registry size (no drift)', () => {
    expect(Object.keys(SHORTCUTS).length).toBe(VISION_TABLE.length);
  });
});

// ---------------------------------------------------------------------------
// Handler dispatch tests — AC: "asserts either an action ran or the
// placeholder toast fired. None silently no-op."
//
// Phase-2 real-action shortcuts mutate Zustand store state.
// Placeholder shortcuts dispatch a `biscuitcode:toast` CustomEvent.
// ---------------------------------------------------------------------------
describe('shortcut handler dispatch', () => {
  // Track toast events dispatched by placeholder handlers.
  let toastEvents: CustomEvent[] = [];

  beforeEach(() => {
    toastEvents = [];
    // Reset panels store to known defaults so toggle assertions are deterministic.
    usePanelsStore.setState({
      sideVisible: true,
      bottomVisible: true,
      chatVisible: true,
    });
    const listener = (e: Event) => { toastEvents.push(e as CustomEvent); };
    window.addEventListener('biscuitcode:toast', listener);
    // Clean up after this test by re-registering fresh each time.
    return () => window.removeEventListener('biscuitcode:toast', listener);
  });

  it('Ctrl+B toggles side panel visibility', () => {
    const before = usePanelsStore.getState().sideVisible;
    SHORTCUTS['Ctrl+B'].handler(new KeyboardEvent('keydown'));
    expect(usePanelsStore.getState().sideVisible).toBe(!before);
  });

  it('Ctrl+J toggles bottom panel visibility', () => {
    const before = usePanelsStore.getState().bottomVisible;
    SHORTCUTS['Ctrl+J'].handler(new KeyboardEvent('keydown'));
    expect(usePanelsStore.getState().bottomVisible).toBe(!before);
  });

  it('Ctrl+Alt+C toggles chat panel visibility', () => {
    const before = usePanelsStore.getState().chatVisible;
    SHORTCUTS['Ctrl+Alt+C'].handler(new KeyboardEvent('keydown'));
    expect(usePanelsStore.getState().chatVisible).toBe(!before);
  });

  it('Ctrl+Shift+P fires the open-command-palette event', () => {
    const paletteEvents: Event[] = [];
    const listener = (e: Event) => paletteEvents.push(e);
    window.addEventListener('biscuitcode:open-command-palette', listener);
    SHORTCUTS['Ctrl+Shift+P'].handler(new KeyboardEvent('keydown'));
    window.removeEventListener('biscuitcode:open-command-palette', listener);
    expect(paletteEvents.length).toBe(1);
  });

  // Phase 3 real-action shortcuts that dispatch custom events (not toasts).
  it('Ctrl+P fires the editor-quick-open event', () => {
    const events: Event[] = [];
    const listener = (e: Event) => events.push(e);
    window.addEventListener('biscuitcode:editor-quick-open', listener);
    SHORTCUTS['Ctrl+P'].handler(new KeyboardEvent('keydown'));
    window.removeEventListener('biscuitcode:editor-quick-open', listener);
    expect(events.length).toBe(1);
  });

  it('Ctrl+\\ fires the editor-split event', () => {
    const events: Event[] = [];
    const listener = (e: Event) => events.push(e);
    window.addEventListener('biscuitcode:editor-split', listener);
    SHORTCUTS['Ctrl+\\'].handler(new KeyboardEvent('keydown'));
    window.removeEventListener('biscuitcode:editor-split', listener);
    expect(events.length).toBe(1);
  });

  // Phase 4 real-action shortcut: Ctrl+` shows the bottom panel and fires
  // biscuitcode:terminal-focus (no longer a placeholder toast).
  it('Ctrl+` shows bottom panel and fires terminal-focus event', () => {
    usePanelsStore.setState({ bottomVisible: false });
    const focusEvents: Event[] = [];
    const focusListener = (e: Event) => focusEvents.push(e);
    window.addEventListener('biscuitcode:terminal-focus', focusListener);
    SHORTCUTS['Ctrl+`'].handler(new KeyboardEvent('keydown'));
    window.removeEventListener('biscuitcode:terminal-focus', focusListener);
    expect(focusEvents.length).toBe(1);
    expect(usePanelsStore.getState().bottomVisible).toBe(true);
  });

  // Placeholder shortcuts — must fire biscuitcode:toast (not silently no-op).
  const placeholderCombos = [
    'Ctrl+K Ctrl+I',
    'Ctrl+L',
    'Ctrl+Shift+L',
    'F1',
  ] as const;

  for (const combo of placeholderCombos) {
    it(`${combo} fires a placeholder toast (not a silent no-op)`, () => {
      const fired: CustomEvent[] = [];
      const listener = (e: Event) => fired.push(e as CustomEvent);
      window.addEventListener('biscuitcode:toast', listener);
      SHORTCUTS[combo].handler(new KeyboardEvent('keydown'));
      window.removeEventListener('biscuitcode:toast', listener);
      expect(fired.length, `${combo} did not fire biscuitcode:toast`).toBe(1);
      expect(fired[0].detail.kind).toBe('info');
      expect(fired[0].detail.text).toContain('registered');
    });
  }
});
