//! Tauri command handlers, split by subsystem.
//!
//! Phase 3 adds the `fs` module for workspace-scoped filesystem operations.
//! Phase 4 adds the `terminal` module for PTY-backed terminal sessions.
//! Phase 5 adds the `chat` module for Anthropic streaming + keyring + DB.

pub mod chat;
pub mod fs;
pub mod terminal;
