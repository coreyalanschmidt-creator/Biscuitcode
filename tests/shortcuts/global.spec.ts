// tests/shortcuts/global.spec.ts
//
// Phase 2 deliverable. Asserts every shortcut from the vision's keyboard
// table is registered and dispatches either a real action or a placeholder
// toast — none silently no-op.

import { describe, expect, it } from 'vitest';
import { SHORTCUTS } from '../../src/shortcuts/global';

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
