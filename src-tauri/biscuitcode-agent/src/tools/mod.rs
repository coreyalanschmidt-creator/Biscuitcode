//! Tool trait + registry + per-tool ctx.
//!
//! Phase 6a ships `read_file` and `search_code`. Phase 6b adds
//! `write_file`, `apply_patch`, `run_shell` — all in this same module
//! tree under their own files (`mod.rs` re-exports).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use biscuitcode_db::ConversationId;
use biscuitcode_providers::ToolSpec;

pub mod apply_patch;
pub mod read_file;
pub mod run_shell;
pub mod search_code;
pub mod write_file;

// ---------- Tool surface ----------

/// Side-effect class. Drives the confirmation gate (Phase 6b) + snapshot
/// policy (Phase 6b). `Read` tools never confirm, never snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolClass {
    /// Read-only — no user prompt, no snapshot.
    Read,
    /// Modifies workspace files — confirm + snapshot affected paths.
    Write,
    /// Runs shell commands — confirm + snapshot any paths the command may
    /// modify (best-effort; if unknowable, snapshot the command's cwd).
    Shell,
}

/// What every tool produces. JSON-encoded by the caller before being
/// stored in the messages table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    /// Stringified result. Structured tools serialize their output.
    pub result: String,
    /// Set true if the result was truncated to fit the message budget.
    #[serde(default)]
    pub truncated: bool,
}

impl ToolResult {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            result: s.into(),
            truncated: false,
        }
    }
}

/// Per-call context. Constructed by the executor before dispatch.
pub struct ToolCtx {
    pub workspace_root: PathBuf,
    pub conversation_id: ConversationId,
    /// Maximum bytes any single tool may return. Defaults to 256 KB
    /// per docs/design/AGENT-LOOP.md.
    pub max_result_bytes: usize,
}

impl ToolCtx {
    /// True if `path` (after canonicalization) is a descendant of
    /// `workspace_root`. Used by every read/write tool to enforce the
    /// trust boundary.
    pub fn is_inside_workspace(&self, path: &Path) -> bool {
        let canon = match std::fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => return false,
        };
        canon.starts_with(&self.workspace_root)
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    /// JSON-Schema spec the model sees. Hand-authored — no derive.
    fn spec(&self) -> ToolSpec;

    /// Execute. `args` was already validated against `spec().input_schema`.
    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolCtx,
    ) -> Result<ToolResult, ToolError>;

    /// Side-effect class. Drives confirmation + snapshot policy.
    fn class(&self) -> ToolClass;

    /// Convenience: tool name (also `spec().name`).
    fn name(&self) -> &'static str;
}

// ---------- Registry ----------

/// Map of tool name -> impl. Constructed once at app launch and shared.
pub struct ToolRegistry {
    tools: HashMap<&'static str, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool. Replaces any previously-registered tool with the
    /// same name (used by Phase 6b to swap in real impls over Phase 6a's
    /// "not yet available" stubs).
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Specs for every registered tool. Sent to the provider so the model
    /// knows what's callable.
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|t| t.spec()).collect()
    }

    /// Phase 6a default registry: read-only tools only.
    pub fn read_only_default() -> Self {
        let mut r = Self::new();
        r.register(Arc::new(read_file::ReadFileTool));
        r.register(Arc::new(search_code::SearchCodeTool));
        r
    }

    /// Phase 6b full registry: read + write + shell tools.
    pub fn full_default() -> Self {
        let mut r = Self::new();
        r.register(Arc::new(read_file::ReadFileTool));
        r.register(Arc::new(search_code::SearchCodeTool));
        r.register(Arc::new(write_file::WriteFileTool));
        r.register(Arc::new(apply_patch::ApplyPatchTool));
        r.register(Arc::new(run_shell::RunShellTool));
        r
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::read_only_default()
    }
}

// ---------- Errors ----------

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("path {path} is outside the workspace root")]
    OutsideWorkspace { path: String },

    /// File not found within the workspace. Distinct from OutsideWorkspace
    /// so the model gets a clear "file doesn't exist" signal rather than
    /// a confusing workspace-escape error for a typo'd path.
    /// (Phase 6a follow-up, implemented in Phase 6b.)
    #[error("file not found: {path}")]
    FileNotFound { path: String },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid args: {0}")]
    InvalidArgs(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Phase 6a stub for write tools that haven't shipped yet.
    #[error("tool {tool} not available in this build (lands in Phase 6b)")]
    NotYetAvailable { tool: &'static str },

    #[error("{0}")]
    Other(String),
}
