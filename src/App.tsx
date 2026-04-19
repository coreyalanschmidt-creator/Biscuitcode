// src/App.tsx
//
// Top-level app shell. Mounts the four-region WorkspaceGrid + the global
// shortcut handler + the toast/error layer + the command palette modal.
//
// Phase 1+2 deliverable. Each later phase rewrites individual children
// (Phase 3 -> EditorArea Monaco wrapper, Phase 4 -> TerminalPanel xterm,
// Phase 5 -> ChatPanel virtualized chat, etc.) WITHOUT changing this
// shell.
//
// Phase 8 additions:
//   - OnboardingModal shown on first launch; blocks main UI
//   - Theme applied from localStorage on startup
//   - window.__BISCUIT_CACHE_ROOT__ set from Tauri app_cache_dir
//   - Font-load canary (E016 FontLoadFailed)

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useGlobalShortcuts } from './shortcuts/global';
import { WorkspaceGrid } from './layout/WorkspaceGrid';
import { ToastLayer } from './components/ToastLayer';
import { CommandPalette } from './components/CommandPalette';
import { ConfirmationModal } from './components/ConfirmationModal';
import { InlineEditPane } from './components/InlineEditPane';
import { OnboardingModal, useOnboardingDone } from './components/OnboardingModal';
import { applyTheme, getStoredThemeId } from './theme/themes';

const GTK_THEME_OFFERED_KEY = 'biscuitcode-gtk-theme-offered';

// ---------------------------------------------------------------------------
// Font-load canary (Phase 8 / E016 FontLoadFailed)
// ---------------------------------------------------------------------------

function checkFontLoaded(fontFamily: string, testChar = 'M'): boolean {
  // Compare measured width of the target font vs. a definitely-system font.
  // If they match, the target font failed to load.
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d');
  if (!ctx) return true; // can't test → assume OK

  const testFont = `14px '${fontFamily}', monospace`;
  const fallbackFont = `14px monospace`;

  ctx.font = testFont;
  const w1 = ctx.measureText(testChar).width;
  ctx.font = fallbackFont;
  const w2 = ctx.measureText(testChar).width;

  // If widths are identical the target font is not loaded.
  return w1 !== w2;
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

// Extend global Window for BiscuitCode globals set by Tauri.
declare global {
  interface Window {
    __BISCUIT_CACHE_ROOT__?: string;
    __BISCUIT_WORKSPACE_ROOT__?: string;
  }
}

export default function App() {
  useGlobalShortcuts();

  const alreadyOnboarded = useOnboardingDone();
  const [onboardingDone, setOnboardingDone] = useState(alreadyOnboarded);

  // Apply persisted theme on mount.
  useEffect(() => {
    applyTheme(getStoredThemeId());
  }, []);

  // Wire window.__BISCUIT_CACHE_ROOT__ from Tauri's app_cache_dir.
  useEffect(() => {
    invoke<string>('get_app_cache_dir')
      .then((dir) => {
        window.__BISCUIT_CACHE_ROOT__ = dir;
      })
      .catch(() => {
        // Fallback to /tmp if command not available (dev environment).
        if (!window.__BISCUIT_CACHE_ROOT__) {
          window.__BISCUIT_CACHE_ROOT__ = '/tmp';
        }
      });
  }, []);

  // GTK theme detection: on first run with a light GTK theme, offer Cream.
  useEffect(() => {
    if (localStorage.getItem(GTK_THEME_OFFERED_KEY)) return; // already offered
    const storedTheme = getStoredThemeId();
    if (storedTheme !== 'warm') return; // user already picked a theme

    invoke<string>('detect_gtk_theme')
      .then((gtkVariant) => {
        if (gtkVariant === 'light') {
          // Dispatch event that ToastLayer can pick up to offer Cream.
          window.dispatchEvent(
            new CustomEvent('biscuitcode:gtk-light-detected', { detail: { offer: 'cream' } })
          );
        }
        localStorage.setItem(GTK_THEME_OFFERED_KEY, '1');
      })
      .catch(() => {
        // GTK detection unavailable — mark as offered so we don't retry.
        localStorage.setItem(GTK_THEME_OFFERED_KEY, '1');
      });
  }, []);

  // Font-load canary — check Inter loaded; emit E016 toast if not.
  useEffect(() => {
    // Run after DOM paints.
    const id = requestAnimationFrame(() => {
      const interLoaded = checkFontLoaded('Inter');
      if (!interLoaded) {
        // Emit E016 via the error event channel.
        window.dispatchEvent(
          new CustomEvent('biscuitcode:font-canary-failed', { detail: { code: 'E016' } })
        );
      }
    });
    return () => cancelAnimationFrame(id);
  }, []);

  return (
    <>
      <WorkspaceGrid />
      <ToastLayer />
      <CommandPalette />
      {/* Phase 6b — write-tool confirmation gate */}
      <ConfirmationModal />
      {/* Phase 6b — inline AI edit split-diff */}
      <InlineEditPane />
      {/* Phase 8 — onboarding modal (blocks main UI on first launch) */}
      {!onboardingDone && (
        <OnboardingModal onComplete={() => setOnboardingDone(true)} />
      )}
    </>
  );
}
