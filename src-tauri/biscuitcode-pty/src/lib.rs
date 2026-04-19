//! `biscuitcode-pty` — multi-tab PTY backend for the integrated terminal.
//!
//! Phase 4 deliverable. Each xterm.js tab in the frontend corresponds to
//! one [`PtySession`] here. The frontend calls four Tauri commands:
//!   - `terminal_open(shell, cwd, rows, cols) -> SessionId`
//!   - `terminal_input(session_id, bytes)`
//!   - `terminal_resize(session_id, rows, cols)`
//!   - `terminal_close(session_id)`
//!
//! And subscribes to `terminal_data_<session_id>` Tauri events for
//! output streaming.
//!
//! Two Tokio tasks per session: reader (pty master -> Tauri event),
//! writer (consumes queued input). Hash-map of sessions under
//! `Arc<RwLock<HashMap<SessionId, PtySession>>>`. Cleanup on close
//! drops master+slave and waits for the child to exit.
//!
//! Phase 4 coder fills in the actual portable-pty calls; this skeleton
//! locks in the public API surface so the Tauri command layer can be
//! written against it before the impl lands.

#![warn(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

/// Stable session identifier for a single terminal tab.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(format!("term_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-session state. The actual `portable_pty::PtyPair` + child handle
/// + Tokio task handles live behind a `Box<dyn …>` so this struct stays
/// `Send + Sync` at the registry layer.
pub struct PtySession {
    pub id: SessionId,
    pub shell: String,
    pub cwd: PathBuf,
    pub rows: u16,
    pub cols: u16,
    // Phase 4 coder: hold the master, child, reader/writer task JoinHandles
    // here. Closing the session aborts both tasks then drops the master,
    // which sends SIGHUP to the child.
}

/// Process-global registry of all open terminal sessions.
#[derive(Default)]
pub struct PtyRegistry {
    sessions: Arc<RwLock<HashMap<SessionId, PtySession>>>,
}

impl PtyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a new PTY-backed shell. `shell` may be e.g. `/bin/bash`.
    /// Spawn order:
    ///   1. portable_pty::native_pty_system().openpty()
    ///   2. master.spawn_command(...) with cwd, rows, cols, env
    ///   3. spawn reader task: master.try_clone_reader().read_to_buf
    ///      -> Tauri emit "terminal_data_<id>"
    ///   4. spawn writer task: receives from input channel, writes to
    ///      master.try_clone_writer()
    pub fn open(
        &self,
        _shell: String,
        _cwd: PathBuf,
        _rows: u16,
        _cols: u16,
    ) -> Result<SessionId, PtyError> {
        // Phase 4 coder fills in.
        Err(PtyError::NotImplemented)
    }

    /// Send input bytes to a session's child process.
    pub fn write_input(&self, _session_id: &SessionId, _bytes: &[u8]) -> Result<(), PtyError> {
        Err(PtyError::NotImplemented)
    }

    /// Resize the PTY's window dimensions. Important: must call this
    /// when the frontend resizes the panel, otherwise tput lines / cols
    /// will report stale values.
    pub fn resize(&self, _session_id: &SessionId, _rows: u16, _cols: u16) -> Result<(), PtyError> {
        Err(PtyError::NotImplemented)
    }

    /// Close a session: abort tasks, drop master (-> child SIGHUP), wait
    /// for child exit. Returns when the child has actually exited (Phase 4
    /// AC: pgrep returns no orphans 2s after close).
    pub async fn close(&self, _session_id: &SessionId) -> Result<(), PtyError> {
        Err(PtyError::NotImplemented)
    }
}

/// Detect the user's preferred login shell. Order: `$SHELL`, then
/// `getent passwd $UID`, then `/bin/bash` as the universal fallback.
pub fn detect_shell() -> String {
    if let Ok(s) = std::env::var("SHELL") {
        if !s.is_empty() {
            return s;
        }
    }
    // /etc/passwd lookup — fallback if SHELL is unset (e.g. detached daemons).
    // Phase 4 coder: parse `getent passwd $UID` 7th field; fall back to bash.
    "/bin/bash".to_string()
}

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("not implemented (Phase 4 stub)")]
    NotImplemented,

    #[error("session {0:?} not found")]
    SessionNotFound(SessionId),

    #[error("portable_pty: {0}")]
    Pty(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_shell_prefers_env() {
        // Save and restore $SHELL so we don't pollute other tests.
        let prev = std::env::var("SHELL").ok();
        std::env::set_var("SHELL", "/bin/zsh");
        assert_eq!(detect_shell(), "/bin/zsh");
        match prev {
            Some(p) => std::env::set_var("SHELL", p),
            None => std::env::remove_var("SHELL"),
        }
    }

    #[test]
    fn session_ids_are_unique_and_prefixed() {
        let a = SessionId::new();
        let b = SessionId::new();
        assert_ne!(a, b);
        assert!(a.0.starts_with("term_"));
        assert!(b.0.starts_with("term_"));
    }

    #[test]
    fn registry_open_is_stub() {
        let r = PtyRegistry::new();
        let result = r.open("/bin/bash".into(), "/tmp".into(), 24, 80);
        assert!(matches!(result, Err(PtyError::NotImplemented)));
    }
}
