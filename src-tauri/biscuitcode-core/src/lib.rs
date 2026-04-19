//! `biscuitcode-core` — shared primitives for the BiscuitCode app.
//!
//! Phase 1 contents:
//!   - `palette` — brand color constants (mirror of `src/theme/tokens.ts`)
//!   - `errors`  — catalogued error enum (mirror of `src/errors/types.ts`)
//!
//! Other Phase 1 modules will be added as needed (likely a `paths` helper
//! for $APPCONFIG / $APPDATA / $APPCACHE resolution, but not until the
//! `tauri::AppHandle` is wired in `main.rs`).
//!
//! See `docs/plan.md` Phase 1 deliverables and `CLAUDE.md` workspace-crate
//! convention. Sibling crates (`biscuitcode-providers`, `biscuitcode-db`,
//! `biscuitcode-pty`, `biscuitcode-agent`, `biscuitcode-lsp`) are created
//! in the phase that first uses each.

#![warn(missing_docs)]

pub mod errors;
pub mod palette;
pub mod secrets;

pub use errors::CatalogueError;
pub use palette::Rgb;

/// Common Result alias for crate-level conveniences.
pub type Result<T> = std::result::Result<T, CatalogueError>;
