# Phase 2 — Merging the layout source into the scaffold

> Read this AFTER `docs/PHASE-1-MERGE.md` is complete (i.e., after Phase 0 + Phase 1 ACs are green and `pnpm tauri dev` opens a brand-themed shell window).

## Pre-staged files (Phase 2)

| File | Drop-in or merge? |
|---|---|
| `src/state/panelsStore.ts` | Drop in (new file) |
| `src/shortcuts/global.ts` | Drop in |
| `src/layout/WorkspaceGrid.tsx` | Drop in |
| `src/components/ActivityBar.tsx` | Drop in |
| `src/components/SidePanel.tsx` | Drop in |
| `src/components/EditorArea.tsx` | Drop in (replaced in Phase 3 with Monaco wrapper) |
| `src/components/TerminalPanel.tsx` | Drop in (replaced in Phase 4) |
| `src/components/ChatPanel.tsx` | Drop in (rewritten in Phase 5) |
| `src/components/AgentActivityPanel.tsx` | Drop in (rewritten in Phase 6a) |
| `src/components/PreviewPanel.tsx` | Drop in (rewritten in Phase 7) |
| `src/components/StatusBar.tsx` | Drop in |
| `src/components/ToastLayer.tsx` | Drop in |
| `src/components/CommandPalette.tsx` | Drop in |

## Dependencies to add

Inside the Phase 1 merged scaffold, run from the repo root:

```bash
pnpm add react-resizable-panels zustand lucide-react i18next react-i18next
pnpm add -D @types/react @types/react-dom
```

Verify versions land at:
- `react-resizable-panels` ≥ 2.x
- `zustand` ≥ 4.5.x (the persist middleware API used by `panelsStore.ts` is stable since 4.x)
- `lucide-react` (any 0.x — icons are very stable)
- `i18next` ≥ 24.x, `react-i18next` ≥ 15.x

If `pnpm add` reports any peer-dep warnings (especially against React 18), confirm React major matches the scaffold's React.

## Rewire `src/App.tsx`

Replace the Phase 1 cocoa-700/biscuit centered text with the actual layout shell:

```tsx
import { useGlobalShortcuts } from './shortcuts/global';
import { WorkspaceGrid } from './layout/WorkspaceGrid';
import { ToastLayer } from './components/ToastLayer';
import { CommandPalette } from './components/CommandPalette';

export default function App() {
  useGlobalShortcuts();
  return (
    <>
      <WorkspaceGrid />
      <ToastLayer />
      <CommandPalette />
    </>
  );
}
```

## Verify Phase 2 ACs

From `docs/plan.md` Phase 2:

- [ ] All four regions visible at default sizes (Activity 48px, Side 260px, Bottom 240px, Chat 380px).
- [ ] `Ctrl+B` toggles side panel; resize drag works; size persists across restart.
- [ ] `Ctrl+J` toggles bottom; `Ctrl+Alt+C` toggles chat.
- [ ] `Ctrl+Shift+P` opens the command palette; typing "toggle bottom" + Enter toggles the bottom panel.
- [ ] Every other shortcut from the table fires either a real action OR the placeholder toast `"<shortcut> registered; wiring lands in Phase N"`. Verified by `tests/shortcuts/global.spec.ts`.
- [ ] `npx i18next-parser --dry-run --fail-on-untranslated-strings` exits 0 (every UI string is in `src/locales/en.json`).
- [ ] `pnpm tauri build` produces `biscuitcode_0.1.0_amd64.deb`.
- [ ] Install on a Mint 22 XFCE VM: `dpkg -s biscuitcode | grep 'Version: 0.1.0'` returns one line; Whisker menu → Development → BiscuitCode launches the app.
- [ ] `sudo apt remove biscuitcode` removes everything.

## Tests to add

The plan asserts a unit test iterating over the 11 vision shortcuts. Add at `tests/shortcuts/global.spec.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { SHORTCUTS } from '../../src/shortcuts/global';

const VISION_TABLE = [
  'Ctrl+B', 'Ctrl+J', 'Ctrl+Alt+C', 'Ctrl+P', 'Ctrl+Shift+P',
  'Ctrl+`', 'Ctrl+\\', 'Ctrl+K Ctrl+I', 'Ctrl+L', 'Ctrl+Shift+L', 'F1',
];

describe('global shortcuts', () => {
  it('registers every shortcut in the vision table', () => {
    for (const combo of VISION_TABLE) {
      expect(SHORTCUTS[combo]).toBeDefined();
      expect(typeof SHORTCUTS[combo].handler).toBe('function');
    }
  });
});
```

(Add `vitest` to dev deps if the scaffold didn't already include it.)
