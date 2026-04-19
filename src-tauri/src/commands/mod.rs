//! Tauri command handlers, split by subsystem.
//!
//! Phase 3 adds the `fs` module for workspace-scoped filesystem operations.
//! Phase 4 adds the `terminal` module for PTY-backed terminal sessions.
//! Phase 5 adds the `chat` module for Anthropic streaming + keyring + DB.
//! Phase 6b adds the `agent` module for confirmation + rewind commands.
//! Phase 7 adds the `git` module (git panel) and `lsp` module (language servers).
//! Phase 8 adds the `conversations` module (export/import/cleanup/branching).
//! Phase 9 adds the `update` module (auto-update wiring).

pub mod agent;
pub mod chat;
pub mod conversations;
pub mod fs;
pub mod git;
pub mod lsp;
pub mod terminal;
pub mod update;
