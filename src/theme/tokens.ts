// src/theme/tokens.ts
//
// BiscuitCode brand tokens — verbatim from `docs/vision.md` "Brand Tokens"
// section. Source of truth in TypeScript. Mirrored in:
//   - tailwind.config.ts        (Tailwind theme extension)
//   - src/index.css             (CSS custom properties on :root)
//   - src-tauri/biscuitcode-core/src/palette.rs   (Rust constants)
//
// **Do not edit individual values without updating all four mirrors at once.**
// A simple grep test in CI (Phase 10) catches drift.
//
// Values are LOCKED per the vision; any change requires a vision-doc edit.

// ---------- Biscuit (primary brand — warm gold) ----------
export const BISCUIT = {
  50:  '#FDF7E6',
  100: '#FAE8B3',
  200: '#F5D380',
  300: '#F0C065',
  400: '#EBB553',
  500: '#E8B04C',  // PRIMARY ACCENT
  600: '#C7913A',
  700: '#9E722A',
  800: '#74531E',
  900: '#4A3413',
} as const;

// ---------- Cocoa (warm dark neutrals — backgrounds + chrome) ----------
export const COCOA = {
  50:  '#F6F0E8',
  100: '#E0D3BE',
  200: '#B9A582',
  300: '#8A7658',
  400: '#584938',
  500: '#3A2F24',
  600: '#28201A',
  700: '#1C1610',  // PRIMARY DARK BG
  800: '#120D08',
  900: '#080504',  // DEEPEST DARK
} as const;

// ---------- Semantic accents ----------
export const SEMANTIC = {
  ok:    '#6FBF6E',  // sage green — complements warm palette
  warn:  '#E8833E',  // terracotta — distinct from biscuit gold
  error: '#E06B5B',  // salmon — readable on warm dark
} as const;

// ---------- Type helpers ----------
export type BiscuitShade = keyof typeof BISCUIT;
export type CocoaShade   = keyof typeof COCOA;
export type SemanticKey  = keyof typeof SEMANTIC;

// ---------- Convenience getters ----------
export const biscuit = (shade: BiscuitShade): string => BISCUIT[shade];
export const cocoa   = (shade: CocoaShade): string   => COCOA[shade];
export const accent  = (key: SemanticKey): string    => SEMANTIC[key];

// ---------- Aliases for clarity in component code ----------
export const PRIMARY_ACCENT = BISCUIT[500];   // #E8B04C
export const PRIMARY_BG     = COCOA[700];     // #1C1610
export const DEEPEST_BG     = COCOA[900];     // #080504
export const DIVIDER        = COCOA[500];     // #3A2F24

// ---------- All-tokens-in-one for theme bridges ----------
export const TOKENS = {
  biscuit: BISCUIT,
  cocoa:   COCOA,
  ...SEMANTIC,
} as const;
