//! `biscuitcode-agent` — ReAct executor + tool registry + snapshot/rewind.
//!
//! Phase 6a foundation:
//!   - `Tool` trait + `ToolClass` enum
//!   - `ToolRegistry` for registering implementations
//!   - `ToolCtx` carrying workspace root, conversation id, frontend handle
//!   - Read-only tool stubs (`read_file`, `search_code`)
//!   - `ReActExecutor` skeleton (read-only mode in 6a; write dispatch in 6b)
//!
//! Phase 6b extensions (added to this same crate):
//!   - Write tools (`write_file`, `apply_patch`, `run_shell`)
//!   - `confirmation` module (per-tool gate + workspace-trust shortcut)
//!   - `snapshot` module (pre-write snapshot, manifest fsync ordering, rewind)
//!
//! Design contract: docs/design/AGENT-LOOP.md (READ THIS BEFORE 6b — a
//! correctness bug in rewind could delete user files).

#![allow(missing_docs)] // TODO: document public items and flip back to warn

pub mod executor;
pub mod tools;

pub use executor::{
    confirmation::{ConfirmationRequest, Decision, PendingConfirmations},
    ReActExecutor, RunOutcome,
};
pub use tools::{Tool, ToolClass, ToolCtx, ToolError, ToolRegistry, ToolResult};
