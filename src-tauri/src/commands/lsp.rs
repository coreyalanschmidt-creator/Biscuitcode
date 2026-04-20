//! LSP command handlers for Phase 7.
//!
//! Bridges the `biscuitcode-lsp` crate to Tauri events + commands.
//!
//! Protocol:
//! - Frontend calls `lsp_spawn` with language + workspace_root.
//! - Rust spawns the server; reader task emits `lsp-msg-in-<session_id>` events.
//! - Frontend sends frames via `lsp_write` command.
//! - Frontend calls `lsp_shutdown` to terminate a session.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use biscuitcode_lsp::{Language, LspRegistry, SessionId, SessionInfo};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

/// Tauri managed state wrapping the LSP session registry.
pub struct LspState(pub Arc<Mutex<LspRegistry>>);

/// Payload emitted as `lsp-msg-in-<session_id>` Tauri event.
#[derive(Serialize, Clone)]
struct LspMsgPayload {
    session_id: String,
    frame: serde_json::Value,
}

/// Parameters for `lsp_spawn`.
#[derive(Deserialize)]
pub struct LspSpawnRequest {
    pub language: String,
    pub workspace_root: String,
}

/// Parse a language string into the `Language` enum.
fn parse_language(s: &str) -> Result<Language, String> {
    match s.to_lowercase().as_str() {
        "rust" => Ok(Language::Rust),
        "typescript" | "javascript" => Ok(Language::Typescript),
        "python" => Ok(Language::Python),
        "go" => Ok(Language::Go),
        "cpp" | "c" | "c++" => Ok(Language::Cpp),
        other => Err(format!("unknown language: {}", other)),
    }
}

/// Spawn an LSP server for a language + workspace.
/// Returns the session ID or an error string.
/// On missing server: returns `"E013:<language>:<install_command>"`.
#[tauri::command]
pub fn lsp_spawn(
    request: LspSpawnRequest,
    app: tauri::AppHandle,
    state: State<'_, LspState>,
) -> Result<String, String> {
    let language = parse_language(&request.language)?;
    let workspace_root = PathBuf::from(&request.workspace_root);

    // Build the emit closure: emits `lsp-msg-in-<session_id>` Tauri events.
    let app_clone = app.clone();
    let emit_frame: biscuitcode_lsp::FrameEmitter =
        Arc::new(move |id: SessionId, frame: serde_json::Value| {
            let event_name = format!("lsp-msg-in-{}", id.0);
            let _ = app_clone.emit(
                &event_name,
                LspMsgPayload {
                    session_id: id.0.clone(),
                    frame,
                },
            );
        });

    let registry = state.0.lock().unwrap();
    match registry.spawn(language, workspace_root, emit_frame) {
        Ok(session_id) => Ok(session_id.0),
        Err(biscuitcode_lsp::LspError::ServerMissing {
            language: lang,
            install_command,
            ..
        }) => {
            // Structured error string for frontend toast.
            Err(format!("E013:{}:{}", lang, install_command))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Send a JSON-RPC frame to an LSP session.
///
/// Gets the sender channel from the registry (under lock), then sends
/// to it outside the lock to avoid holding MutexGuard across await.
#[tauri::command]
pub async fn lsp_write(
    session_id: String,
    frame: serde_json::Value,
    state: State<'_, LspState>,
) -> Result<(), String> {
    let id = SessionId(session_id);
    // Serialize the frame before acquiring the lock.
    let serialized = serde_json::to_string(&frame).map_err(|e| e.to_string())?;
    let sender = {
        let registry = state.0.lock().unwrap();
        registry.get_sender(&id).map_err(|e| e.to_string())?
    };
    sender
        .send(serialized)
        .await
        .map_err(|_| "session channel closed".to_string())
}

/// Shut down an LSP session.
///
/// Sends shutdown + exit messages outside the registry lock, then removes
/// the session from the map.
#[tauri::command]
pub async fn lsp_shutdown(session_id: String, state: State<'_, LspState>) -> Result<(), String> {
    let id = SessionId(session_id);
    let sender = {
        let registry = state.0.lock().unwrap();
        registry.get_sender(&id).ok() // best-effort — session may already be dead
    };
    if let Some(tx) = sender {
        let shutdown = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "shutdown", "params": null
        });
        let exit = serde_json::json!({
            "jsonrpc": "2.0", "method": "exit", "params": null
        });
        let _ = tx.send(serde_json::to_string(&shutdown).unwrap()).await;
        let _ = tx.send(serde_json::to_string(&exit).unwrap()).await;
    }
    // Remove from registry.
    state.0.lock().unwrap().remove_session(&id);
    Ok(())
}

/// List active LSP sessions.
#[tauri::command]
pub fn lsp_list_sessions(state: State<'_, LspState>) -> Vec<SessionInfo> {
    let registry = state.0.lock().unwrap();
    registry.list_sessions()
}

/// Detect which languages have project files in the given workspace root.
#[tauri::command]
pub fn lsp_detect_languages(workspace_root: String) -> Vec<String> {
    let path = PathBuf::from(&workspace_root);
    biscuitcode_lsp::detect_languages_in(&path)
        .into_iter()
        .map(|l| format!("{:?}", l).to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_language_variants() {
        assert!(matches!(parse_language("rust"), Ok(Language::Rust)));
        assert!(matches!(
            parse_language("typescript"),
            Ok(Language::Typescript)
        ));
        assert!(matches!(
            parse_language("javascript"),
            Ok(Language::Typescript)
        ));
        assert!(matches!(parse_language("python"), Ok(Language::Python)));
        assert!(matches!(parse_language("go"), Ok(Language::Go)));
        assert!(matches!(parse_language("cpp"), Ok(Language::Cpp)));
        assert!(parse_language("cobol").is_err());
    }

    #[test]
    fn parse_language_case_insensitive() {
        assert!(matches!(parse_language("RUST"), Ok(Language::Rust)));
        assert!(matches!(
            parse_language("TypeScript"),
            Ok(Language::Typescript)
        ));
    }
}
