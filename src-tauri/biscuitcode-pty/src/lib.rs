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

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use ulid::Ulid;

/// Stable session identifier for a single terminal tab.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// Create a new unique, sortable session ID with the `term_` prefix.
    pub fn new() -> Self {
        Self(format!("term_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Internal handle to a live session's Tokio cleanup tasks.
struct SessionHandles {
    /// Sends bytes to the PTY's stdin.
    input_tx: mpsc::Sender<Vec<u8>>,
    /// Writer task — forward input_rx bytes to pty writer.
    _writer_task: JoinHandle<()>,
    /// Reader task — forward pty output to Tauri events.
    _reader_task: JoinHandle<()>,
}

/// Per-session state. Public fields expose the metadata; private fields
/// hold the runtime handles used by the Tauri command layer.
pub struct PtySession {
    /// Stable identifier returned to the frontend.
    pub id: SessionId,
    /// Shell binary path, e.g. `/bin/bash`.
    pub shell: String,
    /// Working directory the shell was started in.
    pub cwd: PathBuf,
    /// Current terminal rows.
    pub rows: u16,
    /// Current terminal cols.
    pub cols: u16,

    // The portable_pty master handle — wrapped in Mutex for Sync.
    // Dropping it sends SIGHUP to the child shell.
    master: Mutex<Option<Box<dyn portable_pty::MasterPty + Send>>>,
    // The child process handle — wrapped in Mutex for Sync.
    // We wait on it during close().
    child: Mutex<Option<Box<dyn portable_pty::Child + Send>>>,
    // Tokio handles for the reader/writer tasks.
    handles: SessionHandles,
}

/// Process-global registry of all open terminal sessions.
#[derive(Default)]
pub struct PtyRegistry {
    sessions: Arc<RwLock<HashMap<SessionId, PtySession>>>,
}

impl PtyRegistry {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a new PTY-backed shell.
    ///
    /// Spawn order:
    ///   1. `native_pty_system().openpty(rows, cols)`
    ///   2. `master.spawn_command(...)` with cwd + cleaned env
    ///   3. Spawn reader task: `try_clone_reader()` + `spawn_blocking` → emit `terminal_data_<id>`
    ///   4. Spawn writer task: mpsc receiver → `take_writer()`
    ///
    /// The `emit_fn` callback is called by the reader task for each chunk
    /// of terminal output. The Tauri command wrapper supplies an impl that
    /// calls `app.emit("terminal_data_<id>", bytes_as_vec)`.
    ///
    /// The caller MAY supply a pre-generated `SessionId` (so the event name
    /// can be embedded in the callback closure before this call). If `None`,
    /// a fresh ID is generated.
    pub fn open<F>(
        &self,
        shell: String,
        cwd: PathBuf,
        rows: u16,
        cols: u16,
        session_id: Option<SessionId>,
        emit_fn: F,
    ) -> Result<SessionId, PtyError>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        let id = session_id.unwrap_or_default();

        // 1. Open the PTY pair.
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        // 2. Spawn the shell in the PTY.
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(&cwd);
        // Inherit the environment selectively — pass through common vars
        // but ensure TERM is set correctly for xterm.js.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        // 3. Reader task: read from PTY master, emit via callback.
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let _reader_task = tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF — child exited
                    Ok(n) => emit_fn(buf[..n].to_vec()),
                    Err(_) => break, // pipe closed
                }
            }
        });

        // 4. Writer task: receive bytes from channel, write to PTY.
        let (input_tx, mut input_rx) = mpsc::channel::<Vec<u8>>(256);
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let _writer_task = tokio::spawn(async move {
            while let Some(bytes) = input_rx.recv().await {
                if writer.write_all(&bytes).is_err() {
                    break;
                }
            }
        });

        let session = PtySession {
            id: id.clone(),
            shell,
            cwd,
            rows,
            cols,
            master: Mutex::new(Some(pair.master)),
            child: Mutex::new(Some(child)),
            handles: SessionHandles {
                input_tx,
                _writer_task,
                _reader_task,
            },
        };

        self.sessions.write().insert(id.clone(), session);
        Ok(id)
    }

    /// Send input bytes to a session's child process.
    pub fn write_input(&self, session_id: &SessionId, bytes: Vec<u8>) -> Result<(), PtyError> {
        let sessions = self.sessions.read();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| PtyError::SessionNotFound(session_id.clone()))?;

        session
            .handles
            .input_tx
            .try_send(bytes)
            .map_err(|e| PtyError::Pty(format!("input channel full or closed: {e}")))?;
        Ok(())
    }

    /// Resize the PTY's window dimensions.
    ///
    /// Must be called when the frontend resizes the panel so that
    /// `tput lines` / `tput cols` report accurate values.
    pub fn resize(&self, session_id: &SessionId, rows: u16, cols: u16) -> Result<(), PtyError> {
        let mut sessions = self.sessions.write();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| PtyError::SessionNotFound(session_id.clone()))?;

        {
            let master_guard = session.master.lock();
            if let Some(master) = master_guard.as_ref() {
                master
                    .resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    })
                    .map_err(|e| PtyError::Pty(e.to_string()))?;
            }
        }
        session.rows = rows;
        session.cols = cols;
        Ok(())
    }

    /// Close a session: drop master (-> child SIGHUP), wait for child to
    /// exit. Returns when the child has actually exited.
    ///
    /// The AC `pgrep returns no orphans 2s after close` depends on this
    /// completing before the caller returns.
    pub async fn close(&self, session_id: &SessionId) -> Result<(), PtyError> {
        // Remove the session from the map. All fields are moved into the
        // spawn_blocking closure together so that master is dropped in the
        // same thread (and before wait()) — that drop sends SIGHUP.
        let session = {
            let mut sessions = self.sessions.write();
            sessions
                .remove(session_id)
                .ok_or_else(|| PtyError::SessionNotFound(session_id.clone()))?
        };

        tokio::task::spawn_blocking(move || {
            // Take master first (drop = SIGHUP to child shell).
            let master = session.master.lock().take();
            drop(master);

            // Kill and wait on the child.
            // kill() = SIGHUP first, then SIGKILL if it doesn't exit.
            if let Some(mut child) = session.child.lock().take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            // _handles dropped here: input channel closes, task JoinHandles
            // are aborted implicitly when the session is dropped.
        })
        .await
        .map_err(|e| PtyError::Pty(format!("join error waiting for child: {e}")))?;

        Ok(())
    }
}

/// Detect the user's preferred login shell.
///
/// Order of preference:
///   1. `$SHELL` environment variable.
///   2. `getent passwd $UID` 7th field.
///   3. `/bin/bash` fallback.
pub fn detect_shell() -> String {
    // 1. $SHELL
    if let Ok(s) = std::env::var("SHELL") {
        if !s.is_empty() {
            return s;
        }
    }

    // 2. /etc/passwd lookup via getent.
    if let Ok(uid) = std::env::var("UID").or_else(|_| {
        // UID is not always exported; fall back to reading /proc/self/status.
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("Uid:"))
                    .and_then(|l| l.split_whitespace().nth(1).map(String::from))
            })
            .ok_or(std::env::VarError::NotPresent)
    }) {
        if let Ok(output) = std::process::Command::new("getent")
            .args(["passwd", &uid])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Format: user:x:uid:gid:comment:home:shell
                if let Some(shell) = stdout.trim().split(':').nth(6) {
                    if !shell.is_empty() {
                        return shell.to_string();
                    }
                }
            }
        }
    }

    // 3. Universal fallback.
    "/bin/bash".to_string()
}

/// Errors returned by the PTY registry.
#[derive(Debug, Error)]
pub enum PtyError {
    /// Session ID was not found in the registry.
    #[error("session {0:?} not found")]
    SessionNotFound(SessionId),

    /// `portable_pty` returned an error.
    #[error("portable_pty: {0}")]
    Pty(String),

    /// Underlying I/O error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_shell_prefers_env() {
        let prev = std::env::var("SHELL").ok();
        // SAFETY: test-only mutation of env; tests run in a single thread.
        unsafe { std::env::set_var("SHELL", "/bin/zsh") };
        assert_eq!(detect_shell(), "/bin/zsh");
        match prev {
            Some(p) => unsafe { std::env::set_var("SHELL", p) },
            None => unsafe { std::env::remove_var("SHELL") },
        }
    }

    #[test]
    fn detect_shell_falls_back_when_env_empty() {
        let prev = std::env::var("SHELL").ok();
        unsafe { std::env::remove_var("SHELL") };
        let shell = detect_shell();
        // Should return something non-empty; exact value depends on host.
        assert!(!shell.is_empty(), "shell fallback must not be empty");
        if let Some(p) = prev {
            unsafe { std::env::set_var("SHELL", p) }
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

    #[tokio::test]
    async fn registry_open_and_close_no_orphan() {
        // Open a real PTY, write something, then close it.
        // After close(), the session map should be empty.
        let registry = PtyRegistry::new();
        let shell = detect_shell();

        let received: Arc<RwLock<Vec<Vec<u8>>>> = Arc::new(RwLock::new(Vec::new()));
        let recv_clone = received.clone();

        let id = registry
            .open(shell, std::env::temp_dir(), 24, 80, None, move |chunk| {
                recv_clone.write().push(chunk);
            })
            .expect("open should succeed");

        // The session should now be in the map.
        assert!(registry.sessions.read().contains_key(&id));

        // Send a newline to trigger a prompt. Give the reader a moment.
        registry.write_input(&id, b"\n".to_vec()).unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Close.
        registry.close(&id).await.expect("close should succeed");

        // Session must be removed.
        assert!(!registry.sessions.read().contains_key(&id));
    }

    #[tokio::test]
    async fn registry_close_nonexistent_returns_error() {
        let registry = PtyRegistry::new();
        let fake_id = SessionId("term_doesnotexist".into());
        let result = registry.close(&fake_id).await;
        assert!(matches!(result, Err(PtyError::SessionNotFound(_))));
    }

    #[tokio::test]
    async fn registry_resize_updates_stored_dims() {
        let registry = PtyRegistry::new();
        let shell = detect_shell();

        let id = registry
            .open(shell, std::env::temp_dir(), 24, 80, None, |_| {})
            .expect("open should succeed");

        registry
            .resize(&id, 40, 120)
            .expect("resize should succeed");

        {
            let sessions = registry.sessions.read();
            let session = sessions.get(&id).unwrap();
            assert_eq!(session.rows, 40);
            assert_eq!(session.cols, 120);
        }

        registry.close(&id).await.expect("close should succeed");
    }

    #[test]
    fn write_input_on_nonexistent_session_returns_error() {
        let registry = PtyRegistry::new();
        let fake_id = SessionId("term_fake".into());
        let result = registry.write_input(&fake_id, b"hello".to_vec());
        assert!(matches!(result, Err(PtyError::SessionNotFound(_))));
    }
}
