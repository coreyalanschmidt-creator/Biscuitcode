//! Tauri command handlers, split by subsystem.
//!
//! Phase 3 adds the `fs` module for workspace-scoped filesystem operations.
//! Phase 4 adds the `terminal` module for PTY-backed terminal sessions.

pub mod fs;
pub mod terminal;
