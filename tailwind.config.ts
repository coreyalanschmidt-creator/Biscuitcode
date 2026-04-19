// tailwind.config.ts
//
// Tailwind 3.x config for BiscuitCode. Brand tokens VERBATIM from
// `src/theme/tokens.ts` — these two files must stay in lockstep
// (CI grep test catches drift).
//
// Phase 1 deliverable. The Phase 1 coder's only job here is to verify
// (a) Tailwind 3.x major-version pin in package.json, (b) PostCSS
// config picks this up correctly, (c) the `cocoa-700` background
// renders on the empty shell.

import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './index.html',
    './src/**/*.{ts,tsx,js,jsx}',
  ],
  darkMode: 'class', // we control theme via class on <html>, not media query
  theme: {
    extend: {
      colors: {
        biscuit: {
          50:  '#FDF7E6',
          100: '#FAE8B3',
          200: '#F5D380',
          300: '#F0C065',
          400: '#EBB553',
          500: '#E8B04C', // PRIMARY ACCENT
          600: '#C7913A',
          700: '#9E722A',
          800: '#74531E',
          900: '#4A3413',
        },
        cocoa: {
          50:  '#F6F0E8',
          100: '#E0D3BE',
          200: '#B9A582',
          300: '#8A7658',
          400: '#584938',
          500: '#3A2F24', // dividers
          600: '#28201A',
          700: '#1C1610', // PRIMARY DARK BG
          800: '#120D08',
          900: '#080504', // DEEPEST DARK
        },
        accent: {
          ok:    '#6FBF6E',
          warn:  '#E8833E',
          error: '#E06B5B',
        },
      },
      fontFamily: {
        // Inter for UI; named-system fallback (Ubuntu) — NOT system-ui.
        // CI grep test (Phase 10 Global AC) fails on `system-ui` in src/.
        sans: ['Inter', 'Ubuntu', 'sans-serif'],
        // JetBrains Mono for code/terminal; named-system fallbacks.
        mono: ['"JetBrains Mono"', '"Ubuntu Mono"', '"DejaVu Sans Mono"', 'monospace'],
      },
      fontSize: {
        // Vision: 12 (secondary) / 13 (default) / 14 (primary) / 16 (headings)
        // Line-height 1.5 across the board.
        xs:   ['12px', { lineHeight: '1.5' }],
        sm:   ['13px', { lineHeight: '1.5' }],
        base: ['14px', { lineHeight: '1.5' }],
        lg:   ['16px', { lineHeight: '1.5' }],
      },
      borderRadius: {
        // Subtle by default; sharp corners in dense chrome (status bar etc.)
        DEFAULT: '4px',
        sm:      '2px',
        md:      '6px',
        lg:      '8px',
      },
    },
  },
  plugins: [],
};

export default config;
