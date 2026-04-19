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

use std::sync::{Arc, Mutex};

use biscuitcode_agent::executor::confirmation::PendingConfirmations;
use biscuitcode_core::errors::CatalogueError;
use biscuitcode_db::Database;
use biscuitcode_lsp::LspRegistry;
use biscuitcode_pty::PtyRegistry;
use commands::agent::ConfirmationState;
use commands::chat::ChatDb;
use commands::fs::WorkspaceState;
use commands::lsp::LspState;
use serde::Serialize;
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
        // Phase 9 — auto-update (AppImage path).
        .plugin(tauri_plugin_updater::Builder::new().build())
        // ----------------------------------------
        .manage(WorkspaceState(Mutex::new(None)))
        .manage(Arc::new(PtyRegistry::new()))
        // Phase 5 — DB state (None until setup initialises it).
        .manage(ChatDb(Mutex::new(None)))
        // Phase 6b — confirmation gate shared state.
        .manage(ConfirmationState(Arc::new(PendingConfirmations::new())))
        // Phase 7 — LSP session registry.
        .manage(LspState(Arc::new(Mutex::new(LspRegistry::new()))))
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
            // Phase 5 — chat + keyring + DB
            commands::chat::anthropic_key_present,
            commands::chat::anthropic_set_key,
            commands::chat::anthropic_delete_key,
            commands::chat::anthropic_list_models,
            commands::chat::chat_create_conversation,
            commands::chat::chat_list_conversations,
            commands::chat::chat_list_messages,
            commands::chat::chat_send,
            // Phase 6b — agent confirmation + rewind
            commands::agent::agent_confirm_decision,
            commands::agent::agent_rewind,
            // Phase 6b — inline edit
            commands::chat::chat_inline_edit,
            commands::chat::chat_apply_inline_edit,
            // Phase 7 — git panel
            commands::git::git_status,
            commands::git::git_stage,
            commands::git::git_unstage,
            commands::git::git_commit,
            commands::git::git_push,
            commands::git::git_pull,
            commands::git::git_log,
            commands::git::git_branches,
            commands::git::git_checkout,
            commands::git::git_diff_file,
            commands::git::git_blame,
            commands::git::git_diff_all,
            // Phase 7 — LSP
            commands::lsp::lsp_spawn,
            commands::lsp::lsp_write,
            commands::lsp::lsp_shutdown,
            commands::lsp::lsp_list_sessions,
            commands::lsp::lsp_detect_languages,
            // Phase 8 — conversations + settings
            commands::conversations::get_app_cache_dir,
            commands::conversations::detect_gtk_theme,
            commands::conversations::conversations_export,
            commands::conversations::conversations_import,
            commands::conversations::snapshots_cleanup_now,
            commands::conversations::fork_message,
            commands::conversations::list_message_branches,
            // Phase 9 — auto-update
            commands::update::check_for_deb_update,
            commands::update::check_for_appimage_update,
            commands::update::install_appimage_update,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Decorations off: custom titlebar renders in HTML.
            let _ = window.set_decorations(false);
            let _ = window.set_title("BiscuitCode");

            // Phase 5 — open/create the SQLite DB in the app data dir.
            // tauri::api::path is the v2 way; we use Manager::path().
            if let Ok(data_dir) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(&data_dir);
                let db_path = data_dir.join("biscuitcode.db");
                match Database::open(&db_path) {
                    Ok(db) => {
                        let state = app.state::<ChatDb>();
                        *state.0.lock().unwrap() = Some(db);
                        tracing::info!("database opened at {}", db_path.display());
                    }
                    Err(e) => {
                        tracing::error!("failed to open database: {e}");
                    }
                }
            }

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
