//! `biscuitcode-lsp` — language-server child management.
//!
//! Phase 7 deliverable. One [`LspSession`] per (language, workspace) pair.
//! Frontend wires `monaco-languageclient` with `MessageTransports` that
//! send via the `lsp_write` Tauri command and receive via the
//! `lsp_msg_in_<session_id>` Tauri event.
//!
//! Supported languages (matched by project markers in the workspace root):
//!   - Rust          : `Cargo.toml`            -> `rust-analyzer`
//!   - TypeScript/JS : `package.json` or `tsconfig.json` -> `typescript-language-server --stdio`
//!   - Python        : `pyproject.toml` or `requirements.txt` -> `pyright-langserver --stdio`
//!   - Go            : `go.mod`                -> `gopls`
//!   - C/C++         : `CMakeLists.txt` or `compile_commands.json` -> `clangd`
//!
//! Missing language server (binary not on $PATH): emit catalogue code
//! E013 with the install command per the language. NEVER auto-install.

#![warn(missing_docs)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

/// Stable identifier for one running LSP child.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(format!("lsp_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    Rust,
    Typescript,
    Python,
    Go,
    Cpp,
}

impl Language {
    /// LSP server binary name. Detected via `which` against PATH.
    pub fn server_binary(self) -> &'static str {
        match self {
            Self::Rust       => "rust-analyzer",
            Self::Typescript => "typescript-language-server",
            Self::Python     => "pyright-langserver",
            Self::Go         => "gopls",
            Self::Cpp        => "clangd",
        }
    }

    /// Default stdin/stdout LSP invocation args.
    pub fn server_args(self) -> &'static [&'static str] {
        match self {
            Self::Rust       => &[],
            Self::Typescript => &["--stdio"],
            Self::Python     => &["--stdio"],
            Self::Go         => &[],
            Self::Cpp        => &[],
        }
    }

    /// Suggested install command (for the E013 toast). Mint 22 / apt-based.
    pub fn install_command(self) -> &'static str {
        match self {
            Self::Rust       => "rustup component add rust-analyzer",
            Self::Typescript => "sudo npm install -g typescript-language-server typescript",
            Self::Python     => "sudo apt install pyright    # or: pip install pyright",
            Self::Go         => "go install golang.org/x/tools/gopls@latest",
            Self::Cpp        => "sudo apt install clangd",
        }
    }
}

/// Detect which languages should be active for a workspace by scanning
/// for marker files in the root. Multi-language workspaces return >1.
pub fn detect_languages_in(workspace_root: &Path) -> Vec<Language> {
    let mut found = Vec::new();
    let exists = |name: &str| workspace_root.join(name).exists();

    if exists("Cargo.toml") { found.push(Language::Rust); }
    if exists("package.json") || exists("tsconfig.json") { found.push(Language::Typescript); }
    if exists("pyproject.toml") || exists("requirements.txt") { found.push(Language::Python); }
    if exists("go.mod") { found.push(Language::Go); }
    if exists("CMakeLists.txt") || exists("compile_commands.json") { found.push(Language::Cpp); }

    found
}

/// Per-session state. Phase 7 coder fills in tokio::process::Child +
/// reader/writer JoinHandles + stdin Sender + dropping logic.
pub struct LspSession {
    pub id: SessionId,
    pub language: Language,
    pub workspace_root: PathBuf,
}

#[derive(Default)]
pub struct LspRegistry {
    sessions: Arc<RwLock<HashMap<SessionId, LspSession>>>,
}

impl LspRegistry {
    pub fn new() -> Self { Self::default() }

    /// Spawn an LSP child. Returns `LspError::ServerMissing` (catalogue E013)
    /// if the binary isn't on $PATH; the caller surfaces the toast.
    pub fn spawn(
        &self,
        _language: Language,
        _workspace_root: PathBuf,
    ) -> Result<SessionId, LspError> {
        // ---- Phase 7 coder fills in ----
        // 1. which::which(language.server_binary()) - if Err -> ServerMissing
        // 2. tokio::process::Command::new(...).args(language.server_args())
        //    .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        //    .current_dir(workspace_root).spawn()
        // 3. Spawn reader task: parse Content-Length headers + JSON body,
        //    emit each frame as Tauri event lsp_msg_in_<session_id>
        // 4. Spawn writer task: receive frames from a channel, write to stdin
        //    with proper Content-Length framing
        // 5. Spawn stderr-drain task that traces lines (debug)
        Err(LspError::NotImplemented)
    }

    /// Send a JSON-RPC frame to the session's stdin.
    pub async fn write(&self, _id: &SessionId, _frame: serde_json::Value) -> Result<(), LspError> {
        Err(LspError::NotImplemented)
    }

    /// Shut down a session: send LSP `shutdown` + `exit`, await child exit
    /// with timeout, then SIGKILL if necessary.
    pub async fn shutdown(&self, _id: &SessionId) -> Result<(), LspError> {
        Err(LspError::NotImplemented)
    }
}

#[derive(Debug, Error)]
pub enum LspError {
    #[error("not implemented (Phase 7 stub)")]
    NotImplemented,

    #[error("server binary {binary} not on PATH (catalogue E013)")]
    ServerMissing { binary: &'static str, install_command: &'static str },

    #[error("session {0:?} not found")]
    SessionNotFound(SessionId),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    // tempfile is a dev-dep; if not in Cargo.toml, the Phase 7 coder
    // adds it. For now, gate the test on the macro being a no-op when
    // tempfile isn't available — which we accept as a TODO until Phase 7.
    fn _build_check_only_demo() {
        // Just ensure the type-level surface compiles.
        let _ = Language::Rust.server_binary();
        let _ = Language::Cpp.install_command();
    }

    #[test]
    fn detect_finds_rust_when_cargo_toml_present() {
        let dir = tempdir().expect("tempdir requires tempfile dev-dep");
        File::create(dir.path().join("Cargo.toml")).unwrap();
        let langs = detect_languages_in(dir.path());
        assert!(langs.contains(&Language::Rust));
    }

    #[test]
    fn install_command_for_clangd_is_apt() {
        assert!(Language::Cpp.install_command().contains("apt install clangd"));
    }
}
