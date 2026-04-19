//! Tauri command handlers for the terminal subsystem.
//!
//! Phase 4 deliverable. These four commands are the only interface between
//! the xterm.js frontend and the `biscuitcode-pty` backend crate. Each
//! command takes a Tauri `State` reference to the shared `PtyRegistry`.
//!
//! Output from the PTY is pushed to the frontend via `terminal_data_<id>`
//! Tauri events (emitted by the reader task inside `PtyRegistry::open`).

use std::path::PathBuf;
use std::sync::Arc;

use biscuitcode_pty::{detect_shell, PtyRegistry, SessionId};
use tauri::{AppHandle, Emitter, State};

/// Open a new terminal session.
///
/// Returns the `SessionId` string the frontend uses to identify this tab.
/// If `shell` is `None` or empty, the detected login shell is used.
/// If `cwd` is `None` or empty, the process working directory is used.
#[tauri::command]
pub async fn terminal_open(
    state: State<'_, Arc<PtyRegistry>>,
    app: AppHandle,
    shell: Option<String>,
    cwd: Option<String>,
    rows: u16,
    cols: u16,
) -> Result<SessionId, String> {
    let shell = shell
        .filter(|s| !s.is_empty())
        .unwrap_or_else(detect_shell);

    let cwd: PathBuf = cwd
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"))
        });

    // Pre-generate the session ID so the event name can be embedded in the
    // reader callback before the session is created.
    let id = SessionId::new();
    let event_name = format!("terminal_data_{}", id.0);
    let app_clone = app.clone();

    state
        .open(
            shell,
            cwd,
            rows,
            cols,
            Some(id),
            move |chunk| {
                let _ = app_clone.emit(&event_name, TerminalDataPayload { data: chunk });
            },
        )
        .map_err(|e| e.to_string())
}

/// Send bytes from the frontend keyboard into a session's PTY stdin.
#[tauri::command]
pub fn terminal_input(
    state: State<'_, Arc<PtyRegistry>>,
    session_id: SessionId,
    data: Vec<u8>,
) -> Result<(), String> {
    state.write_input(&session_id, data).map_err(|e| e.to_string())
}

/// Resize a session's PTY to the given dimensions.
///
/// Must be called whenever the terminal panel is resized so that `tput lines`
/// and `tput cols` report the correct values.
#[tauri::command]
pub fn terminal_resize(
    state: State<'_, Arc<PtyRegistry>>,
    session_id: SessionId,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    state
        .resize(&session_id, rows, cols)
        .map_err(|e| e.to_string())
}

/// Close a terminal tab and wait for the child process to exit.
///
/// After this returns, no orphan processes from this session should remain.
#[tauri::command]
pub async fn terminal_close(
    state: State<'_, Arc<PtyRegistry>>,
    session_id: SessionId,
) -> Result<(), String> {
    state.close(&session_id).await.map_err(|e| e.to_string())
}

// ---------- Payload types ----------

/// Payload sent on the `terminal_data_<session_id>` event.
#[derive(serde::Serialize, Clone)]
pub struct TerminalDataPayload {
    /// Raw bytes from the PTY master.
    pub data: Vec<u8>,
}
