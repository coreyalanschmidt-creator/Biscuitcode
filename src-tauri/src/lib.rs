//! BiscuitCode Tauri application entry point.
//!
//! Phase 1 deliverable. Sets up the Tauri builder with the plugins and
//! capability files declared in `src-tauri/capabilities/`. Each later
//! phase wires its own commands and plugins here.
//!
//! Window chrome: custom titlebar (decorations off), cocoa-700 background
//! set via HTML/CSS — not via Tauri's window background colour, which
//! can flicker before WebKit paints.

mod commands;

use biscuitcode_core::errors::CatalogueError;
use biscuitcode_pty::PtyRegistry;
use commands::fs::WorkspaceState;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tracing_subscriber::EnvFilter;

/// Wire Tauri commands, plugins, and window setup, then run.
pub fn run() {
    // Initialise structured logging before anything else.
    // RUST_LOG=debug enables verbose output; default is warn.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        // --- Plugins (Phase 1 baseline set) ---
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // ----------------------------------------
        .manage(WorkspaceState(Mutex::new(None)))
        .manage(Arc::new(PtyRegistry::new()))
        .invoke_handler(tauri::generate_handler![
            check_secret_service,
            emit_mock_error,
            commands::fs::fs_open_folder,
            commands::fs::fs_list,
            commands::fs::fs_read,
            commands::fs::fs_write,
            commands::fs::fs_rename,
            commands::fs::fs_delete,
            commands::fs::fs_create_dir,
            commands::fs::fs_search_files,
            commands::fs::fs_search_content,
            commands::terminal::terminal_open,
            commands::terminal::terminal_input,
            commands::terminal::terminal_resize,
            commands::terminal::terminal_close,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Decorations off: custom titlebar renders in HTML.
            // NOTE: on Linux/WebKitGTK the shadow/roundcorners come from
            // the compositor; no extra Tauri config required.
            let _ = window.set_decorations(false);
            let _ = window.set_title("BiscuitCode");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running biscuitcode");
}

// ---------- Tauri commands ----------

/// Phase 1 / Phase 5 pre-flight: check whether org.freedesktop.secrets is
/// reachable on the user's DBus session.
///
/// Uses a **read-only** busctl probe — never activates the daemon.
/// Returns `true` if the Secret Service is available; `false` otherwise.
/// The frontend shows E001 KeyringMissing if this returns false before
/// allowing API-key entry.
#[tauri::command]
fn check_secret_service() -> Result<bool, String> {
    biscuitcode_core::secrets::secret_service_available()
        .map_err(|e| e.to_string())
}

/// Development helper: emit a mock E001 error to the frontend so the
/// ErrorToast acceptance criterion can be verified without a full Phase 5
/// keyring integration.
///
/// Called by: devtools console → `window.__TAURI__.core.invoke('emit_mock_error')`
/// Accepted criteria: ErrorToast renders with the user-friendly E001 message
/// and the copy-install-command button.
#[tauri::command]
fn emit_mock_error(app: tauri::AppHandle) -> Result<(), String> {
    let payload = MockErrorPayload {
        code: "E001".to_string(),
        message_key: "errors.E001.msg".to_string(),
        recovery: Some(MockRecovery {
            kind: "copy_command".to_string(),
            command: Some(
                "sudo apt install gnome-keyring libsecret-1-0 libsecret-tools".to_string(),
            ),
            label: Some("Copy install command".to_string()),
        }),
    };

    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?
        .emit("biscuitcode:error-toast", payload)
        .map_err(|e| e.to_string())
}

// Minimal wire-format for the mock — matches what the TS ErrorToast expects.
#[derive(Serialize, Clone)]
struct MockErrorPayload {
    code: String,
    #[serde(rename = "messageKey")]
    message_key: String,
    recovery: Option<MockRecovery>,
}

#[derive(Serialize, Clone)]
struct MockRecovery {
    kind: String,
    command: Option<String>,
    label: Option<String>,
}

// Ensure CatalogueError is accessible even without explicit use in this
// file — keeps the `use` import above from triggering "unused import".
const _: fn() -> CatalogueError = || CatalogueError::KeyringMissing;
