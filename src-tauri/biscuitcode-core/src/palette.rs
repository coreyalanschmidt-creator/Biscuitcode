//! BiscuitCode brand palette — Rust constants mirroring `src/theme/tokens.ts`.
//!
//! Source of truth for the values is `docs/vision.md` "Brand Tokens". This
//! module mirrors them so Rust code (e.g. window background, native menu
//! tints, system-tray icon overlay) uses identical values.
//!
//! Keep in sync with:
//!   - `src/theme/tokens.ts`  (TypeScript; same values)
//!   - `tailwind.config.ts`   (Tailwind theme extension; same values)
//!   - `src/index.css`        (CSS custom properties; same values)
//!
//! Phase 10 CI runs a grep test that fails if any of the four diverges.

/// `#RRGGBB` color, opaque.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Construct from a packed 24-bit `0xRRGGBB` integer.
    pub const fn from_hex(rgb: u32) -> Self {
        Rgb(
            ((rgb >> 16) & 0xff) as u8,
            ((rgb >> 8) & 0xff) as u8,
            (rgb & 0xff) as u8,
        )
    }
    /// Format as `#RRGGBB` uppercase hex string.
    pub fn to_hex_string(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

// ---------- Biscuit (primary brand — warm gold) ----------
/// Biscuit-50: lightest warm gold tint.
pub const BISCUIT_50: Rgb = Rgb::from_hex(0xFDF7E6);
/// Biscuit-100.
pub const BISCUIT_100: Rgb = Rgb::from_hex(0xFAE8B3);
/// Biscuit-200.
pub const BISCUIT_200: Rgb = Rgb::from_hex(0xF5D380);
/// Biscuit-300.
pub const BISCUIT_300: Rgb = Rgb::from_hex(0xF0C065);
/// Biscuit-400.
pub const BISCUIT_400: Rgb = Rgb::from_hex(0xEBB553);
/// PRIMARY ACCENT — `#E8B04C`.
pub const BISCUIT_500: Rgb = Rgb::from_hex(0xE8B04C);
/// Biscuit-600.
pub const BISCUIT_600: Rgb = Rgb::from_hex(0xC7913A);
/// Biscuit-700.
pub const BISCUIT_700: Rgb = Rgb::from_hex(0x9E722A);
/// Biscuit-800.
pub const BISCUIT_800: Rgb = Rgb::from_hex(0x74531E);
/// Biscuit-900: darkest warm gold.
pub const BISCUIT_900: Rgb = Rgb::from_hex(0x4A3413);

// ---------- Cocoa (warm dark neutrals) ----------
/// Cocoa-50: lightest warm neutral.
pub const COCOA_50: Rgb = Rgb::from_hex(0xF6F0E8);
/// Cocoa-100.
pub const COCOA_100: Rgb = Rgb::from_hex(0xE0D3BE);
/// Cocoa-200.
pub const COCOA_200: Rgb = Rgb::from_hex(0xB9A582);
/// Cocoa-300.
pub const COCOA_300: Rgb = Rgb::from_hex(0x8A7658);
/// Cocoa-400.
pub const COCOA_400: Rgb = Rgb::from_hex(0x584938);
/// Cocoa-500: divider colour.
pub const COCOA_500: Rgb = Rgb::from_hex(0x3A2F24);
/// Cocoa-600.
pub const COCOA_600: Rgb = Rgb::from_hex(0x28201A);
/// PRIMARY DARK BG — `#1C1610`.
pub const COCOA_700: Rgb = Rgb::from_hex(0x1C1610);
/// Cocoa-800.
pub const COCOA_800: Rgb = Rgb::from_hex(0x120D08);
/// DEEPEST DARK — `#080504`.
pub const COCOA_900: Rgb = Rgb::from_hex(0x080504);

// ---------- Semantic accents ----------
/// Sage green — success / ok state.
pub const ACCENT_OK: Rgb = Rgb::from_hex(0x6FBF6E);
/// Terracotta — warning state.
pub const ACCENT_WARN: Rgb = Rgb::from_hex(0xE8833E);
/// Salmon — error state.
pub const ACCENT_ERROR: Rgb = Rgb::from_hex(0xE06B5B);

// ---------- Aliases ----------
/// Alias for `BISCUIT_500` — the primary accent colour.
pub const PRIMARY_ACCENT: Rgb = BISCUIT_500;
/// Alias for `COCOA_700` — the primary dark background.
pub const PRIMARY_BG: Rgb = COCOA_700;
/// Alias for `COCOA_900` — the deepest dark background.
pub const DEEPEST_BG: Rgb = COCOA_900;
/// Alias for `COCOA_500` — used for dividers and separators.
pub const DIVIDER: Rgb = COCOA_500;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_accent_is_locked_value() {
        // Vision: "biscuit-gold #E8B04C". A drift here is a vision-doc-level
        // edit — fail loud.
        assert_eq!(PRIMARY_ACCENT.to_hex_string(), "#E8B04C");
    }

    #[test]
    fn primary_bg_is_locked_value() {
        // Vision: "cocoa #1C1610".
        assert_eq!(PRIMARY_BG.to_hex_string(), "#1C1610");
    }

    #[test]
    fn rgb_roundtrip() {
        let c = Rgb::from_hex(0xE8B04C);
        assert_eq!(c.to_hex_string(), "#E8B04C");
        assert_eq!(c, Rgb(0xE8, 0xB0, 0x4C));
    }
}
