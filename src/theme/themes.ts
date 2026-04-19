// src/theme/themes.ts
//
// Phase 8 deliverable: three themes for BiscuitCode.
//
// Each theme is a map of CSS custom-property names → values.
// Applying a theme calls `applyTheme(id)` which sets those properties on
// `document.documentElement` and stores the selection in localStorage.
//
// Theme ids:
//   'warm'    — BiscuitCode Warm (dark, default)
//   'cream'   — BiscuitCode Cream (light)
//   'hc'      — High Contrast
//
// The :root block in src/index.css defines the baseline (Warm).
// Cream and High Contrast override only the properties that differ.

export type ThemeId = 'warm' | 'cream' | 'hc';

export interface ThemeMeta {
  id: ThemeId;
  label: string;
  description: string;
}

export const THEMES: ThemeMeta[] = [
  { id: 'warm',  label: 'BiscuitCode Warm',   description: 'Dark theme — warm cocoa tones.' },
  { id: 'cream', label: 'BiscuitCode Cream',  description: 'Light theme — cocoa-50 background, biscuit-900 text.' },
  { id: 'hc',    label: 'High Contrast',      description: 'Maximum contrast for accessibility.' },
];

// CSS variable overrides per theme. 'warm' = empty (baseline from :root).
const THEME_VARS: Record<ThemeId, Record<string, string>> = {
  warm: {},

  cream: {
    '--bg-base':         '#F6F0E8',  // cocoa-50
    '--bg-secondary':    '#E0D3BE',  // cocoa-100
    '--bg-elevated':     '#F0E8D8',
    '--text-primary':    '#4A3413',  // biscuit-900
    '--text-secondary':  '#74531E',  // biscuit-800
    '--text-muted':      '#9E722A',  // biscuit-700
    '--border-subtle':   '#B9A582',  // cocoa-200
    '--border-strong':   '#8A7658',  // cocoa-300
    // Override body bg via data attribute selector in index.css
    '--cocoa-50':        '#F6F0E8',
    '--cocoa-700':       '#F6F0E8',  // body bg becomes light
    '--cocoa-800':       '#E0D3BE',  // tab bar bg
    '--cocoa-900':       '#080504',  // deepest dark stays for contrast items
    '--cocoa-600':       '#F0E8D8',
    '--cocoa-500':       '#B9A582',
    '--cocoa-400':       '#8A7658',
    '--cocoa-300':       '#584938',
    '--cocoa-200':       '#3A2F24',
    '--cocoa-100':       '#28201A',
    'color-scheme':      'light',
  },

  hc: {
    '--bg-base':         '#000000',
    '--bg-secondary':    '#0a0a0a',
    '--bg-elevated':     '#111111',
    '--text-primary':    '#ffffff',
    '--text-secondary':  '#ffff00',
    '--text-muted':      '#aaaaaa',
    '--border-subtle':   '#ffffff',
    '--border-strong':   '#ffffff',
    '--biscuit-500':     '#ffff00',
    '--biscuit-400':     '#ffd700',
    '--cocoa-50':        '#ffffff',
    '--cocoa-100':       '#eeeeee',
    '--cocoa-700':       '#000000',
    '--cocoa-800':       '#0a0a0a',
    '--cocoa-900':       '#000000',
    '--cocoa-600':       '#111111',
    '--cocoa-500':       '#333333',
    '--cocoa-400':       '#555555',
    '--cocoa-300':       '#888888',
    '--cocoa-200':       '#aaaaaa',
    '--accent-ok':       '#00ff00',
    '--accent-warn':     '#ffaa00',
    '--accent-error':    '#ff4444',
    'color-scheme':      'dark',
  },
};

const STORAGE_KEY = 'biscuitcode-theme';

export function getStoredThemeId(): ThemeId {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'warm' || stored === 'cream' || stored === 'hc') return stored;
  return 'warm';
}

export function applyTheme(id: ThemeId): void {
  const root = document.documentElement;

  // Reset overrides set by previous non-warm theme.
  const allKeys = new Set([
    ...Object.keys(THEME_VARS.cream),
    ...Object.keys(THEME_VARS.hc),
  ]);
  for (const key of allKeys) {
    if (key !== 'color-scheme') {
      root.style.removeProperty(key);
    }
  }

  // Apply new overrides.
  const overrides = THEME_VARS[id];
  for (const [key, value] of Object.entries(overrides)) {
    if (key === 'color-scheme') {
      root.style.setProperty('color-scheme', value);
    } else {
      root.style.setProperty(key, value);
    }
  }

  // Mark the html element with a data attribute so tests and CSS selectors
  // can target the active theme.
  root.dataset.theme = id;

  localStorage.setItem(STORAGE_KEY, id);
}

/** Preview theme without persisting; call `applyTheme(getStoredThemeId())` to revert. */
export function previewTheme(id: ThemeId): void {
  const root = document.documentElement;

  const allKeys = new Set([
    ...Object.keys(THEME_VARS.cream),
    ...Object.keys(THEME_VARS.hc),
  ]);
  for (const key of allKeys) {
    if (key !== 'color-scheme') {
      root.style.removeProperty(key);
    }
  }

  const overrides = THEME_VARS[id];
  for (const [key, value] of Object.entries(overrides)) {
    if (key === 'color-scheme') {
      root.style.setProperty('color-scheme', value);
    } else {
      root.style.setProperty(key, value);
    }
  }

  root.dataset.theme = id;
}
