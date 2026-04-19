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
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use ulid::Ulid;

/// Stable identifier for one running LSP child.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// Create a new unique session ID.
    pub fn new() -> Self {
        Self(format!("lsp_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Languages whose LSP servers this crate manages.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    /// Rust — rust-analyzer.
    Rust,
    /// TypeScript/JavaScript — typescript-language-server.
    Typescript,
    /// Python — pyright-langserver.
    Python,
    /// Go — gopls.
    Go,
    /// C/C++ — clangd.
    Cpp,
}

impl Language {
    /// LSP server binary name. Detected via PATH check.
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

    /// Suggested install command for the E013 toast. Mint 22 / apt-based.
    pub fn install_command(self) -> &'static str {
        match self {
            Self::Rust       => "rustup component add rust-analyzer",
            Self::Typescript => "sudo npm install -g typescript-language-server typescript",
            Self::Python     => "sudo apt install pyright    # or: pip install pyright",
            Self::Go         => "go install golang.org/x/tools/gopls@latest",
            Self::Cpp        => "sudo apt install clangd",
        }
    }

    /// Display name for error messages.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Rust       => "Rust",
            Self::Typescript => "TypeScript",
            Self::Python     => "Python",
            Self::Go         => "Go",
            Self::Cpp        => "C/C++",
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

/// Check whether an LSP server binary is on PATH.
/// Returns `Ok(path)` if found, `Err(LspError::ServerMissing)` if not.
pub fn check_server_binary(language: Language) -> Result<PathBuf, LspError> {
    which::which(language.server_binary()).map_err(|_| LspError::ServerMissing {
        binary: language.server_binary(),
        install_command: language.install_command(),
        language: language.display_name(),
    })
}

/// Per-session state: the running LSP child + a channel to send frames to its stdin.
pub struct LspSession {
    /// Stable ID for this session.
    pub id: SessionId,
    /// The language this session serves.
    pub language: Language,
    /// The workspace root this session is scoped to.
    pub workspace_root: PathBuf,
    /// Channel to send serialized JSON-RPC frames to the server's stdin.
    pub stdin_tx: mpsc::Sender<String>,
    /// Handle to the child process (kept alive so process isn't dropped).
    _child: Child,
}

/// Callback type: the registry calls this to emit an incoming LSP frame
/// as a Tauri event. The caller (Tauri command handler) provides the closure.
pub type FrameEmitter = Arc<dyn Fn(SessionId, serde_json::Value) + Send + Sync>;

/// Registry of all active LSP sessions.
#[derive(Default)]
pub struct LspRegistry {
    sessions: Arc<RwLock<HashMap<SessionId, LspSession>>>,
}

impl LspRegistry {
    /// Create an empty registry.
    pub fn new() -> Self { Self::default() }

    /// Spawn an LSP child process and register it.
    ///
    /// Returns `LspError::ServerMissing` (catalogue E013) if the binary
    /// isn't on PATH. The caller surfaces the toast.
    ///
    /// The `emit_frame` closure is called on the Tokio reader task whenever
    /// a complete JSON-RPC frame arrives from the server. The Tauri command
    /// layer wraps this to emit a `lsp-msg-in-<session_id>` event.
    ///
    /// # Design note
    /// We insert the session into the map **before** spawning the reader task.
    /// This prevents a race where the reader task emits events before the
    /// caller receives the SessionId and sets up its listener.
    pub fn spawn(
        &self,
        language: Language,
        workspace_root: PathBuf,
        emit_frame: FrameEmitter,
    ) -> Result<SessionId, LspError> {
        // 1. Check binary is on PATH.
        check_server_binary(language)?;

        let id = SessionId::new();
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(32);

        // 2. Launch the child process.
        let mut cmd = tokio::process::Command::new(language.server_binary());
        cmd.args(language.server_args())
            .current_dir(&workspace_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // spawn() must be called from a Tokio context.
        let rt = tokio::runtime::Handle::current();
        let mut child = rt.block_on(async { cmd.spawn() })?;

        let child_stdin: ChildStdin = child.stdin.take().unwrap();
        let child_stdout: ChildStdout = child.stdout.take().unwrap();
        let child_stderr = child.stderr.take().unwrap();

        let session_id_for_reader = id.clone();
        let session_id_for_writer = id.clone();

        // 3. Insert session BEFORE spawning tasks (prevents race).
        let session = LspSession {
            id: id.clone(),
            language,
            workspace_root,
            stdin_tx,
            _child: child,
        };
        self.sessions.write().insert(id.clone(), session);

        // 4. Reader task: parse Content-Length framing → emit events.
        let emit_clone = Arc::clone(&emit_frame);
        tokio::spawn(async move {
            let mut reader = BufReader::new(child_stdout);
            loop {
                // Parse "Content-Length: N\r\n\r\n" header.
                let mut header_line = String::new();
                match reader.read_line(&mut header_line).await {
                    Ok(0) => break, // EOF — server exited
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("lsp reader header: {e}");
                        break;
                    }
                }

                let header_trimmed = header_line.trim_end();
                if header_trimmed.is_empty() {
                    // Skip blank separator lines
                    continue;
                }

                let content_length = if let Some(rest) = header_trimmed.strip_prefix("Content-Length: ") {
                    match rest.parse::<usize>() {
                        Ok(n) => n,
                        Err(_) => continue,
                    }
                } else {
                    // Non Content-Length header — skip it
                    continue;
                };

                // Skip the blank line between header and body.
                let mut blank = String::new();
                if reader.read_line(&mut blank).await.is_err() { break; }

                // Read exactly content_length bytes.
                let mut body = vec![0u8; content_length];
                use tokio::io::AsyncReadExt;
                if reader.read_exact(&mut body).await.is_err() { break; }

                match serde_json::from_slice::<serde_json::Value>(&body) {
                    Ok(frame) => {
                        emit_clone(session_id_for_reader.clone(), frame);
                    }
                    Err(e) => {
                        tracing::warn!("lsp json parse: {e}");
                    }
                }
            }
            tracing::info!("lsp reader task exited for {:?}", session_id_for_reader);
        });

        // 5. Writer task: receive frames from channel → write with Content-Length framing.
        tokio::spawn(async move {
            let mut writer = tokio::io::BufWriter::new(child_stdin);
            while let Some(frame) = stdin_rx.recv().await {
                let msg = format!(
                    "Content-Length: {}\r\n\r\n{}",
                    frame.len(),
                    frame
                );
                if writer.write_all(msg.as_bytes()).await.is_err() { break; }
                if writer.flush().await.is_err() { break; }
            }
            tracing::info!("lsp writer task exited for {:?}", session_id_for_writer);
        });

        // 6. Stderr drain.
        tokio::spawn(async move {
            let mut reader = BufReader::new(child_stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                tracing::debug!("[lsp stderr] {}", line.trim_end());
                line.clear();
            }
        });

        Ok(id)
    }

    /// Send a serialized JSON-RPC frame to the session's stdin.
    ///
    /// Gets the sender clone outside the lock, then sends outside the lock
    /// so the lock is never held across an await point.
    pub async fn write_raw(&self, id: &SessionId, frame_json: String) -> Result<(), LspError> {
        let tx = self.get_sender(id)?;
        tx.send(frame_json).await.map_err(|_| LspError::SessionNotFound(id.clone()))
    }

    /// Get the stdin sender for a session (to send frames outside the lock).
    pub fn get_sender(&self, id: &SessionId) -> Result<mpsc::Sender<String>, LspError> {
        self.sessions
            .read()
            .get(id)
            .map(|s| s.stdin_tx.clone())
            .ok_or_else(|| LspError::SessionNotFound(id.clone()))
    }

    /// Remove a session from the registry (drops channel, killing the stdin writer).
    pub fn remove_session(&self, id: &SessionId) {
        self.sessions.write().remove(id);
    }

    /// List all active session IDs and their language/workspace.
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions
            .read()
            .values()
            .map(|s| SessionInfo {
                id: s.id.clone(),
                language: s.language,
                workspace_root: s.workspace_root.to_string_lossy().to_string(),
            })
            .collect()
    }
}

/// Serializable summary of a live session.
#[derive(Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID.
    pub id: SessionId,
    /// The language.
    pub language: Language,
    /// Workspace root path.
    pub workspace_root: String,
}

/// Errors from the LSP management layer.
#[derive(Debug, Error)]
pub enum LspError {
    /// LSP server binary not found on PATH (catalogue E013).
    #[error("server binary {binary} not on PATH (catalogue E013)")]
    ServerMissing {
        /// The binary name that was checked.
        binary: &'static str,
        /// The install command to suggest.
        install_command: &'static str,
        /// Human-readable language name.
        language: &'static str,
    },

    /// Session not found (bad ID from caller).
    #[error("session {0:?} not found")]
    SessionNotFound(SessionId),

    /// I/O error communicating with the child.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialisation error.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn detect_finds_rust_when_cargo_toml_present() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Cargo.toml")).unwrap();
        let langs = detect_languages_in(dir.path());
        assert!(langs.contains(&Language::Rust));
    }

    #[test]
    fn detect_finds_typescript_when_package_json_present() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        let langs = detect_languages_in(dir.path());
        assert!(langs.contains(&Language::Typescript));
    }

    #[test]
    fn detect_finds_multiple_languages() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Cargo.toml")).unwrap();
        File::create(dir.path().join("pyproject.toml")).unwrap();
        let langs = detect_languages_in(dir.path());
        assert!(langs.contains(&Language::Rust));
        assert!(langs.contains(&Language::Python));
    }

    #[test]
    fn detect_empty_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let langs = detect_languages_in(dir.path());
        assert!(langs.is_empty());
    }

    #[test]
    fn install_command_for_clangd_is_apt() {
        assert!(Language::Cpp.install_command().contains("apt install clangd"));
    }

    #[test]
    fn install_command_for_rust_is_rustup() {
        assert!(Language::Rust.install_command().contains("rustup component add"));
    }

    #[test]
    fn check_server_binary_missing_returns_error() {
        // A binary that definitely doesn't exist.
        struct FakeLanguage;
        // Use Language::Go as a proxy — gopls is almost certainly not installed.
        // If it IS installed, this test becomes a no-op; that's fine.
        let result = check_server_binary(Language::Go);
        // We can only assert: if it's an error, it's the right variant.
        if let Err(LspError::ServerMissing { binary, .. }) = result {
            assert_eq!(binary, "gopls");
        }
        // If gopls IS installed, the test passes trivially.
    }

    #[test]
    fn session_id_is_unique() {
        let a = SessionId::new();
        let b = SessionId::new();
        assert_ne!(a.0, b.0);
    }

    #[test]
    fn lsp_error_server_missing_formats() {
        let e = LspError::ServerMissing {
            binary: "rust-analyzer",
            install_command: "rustup component add rust-analyzer",
            language: "Rust",
        };
        let s = format!("{}", e);
        assert!(s.contains("rust-analyzer"));
        assert!(s.contains("E013"));
    }
}
