//! Ollama Tauri commands — Phase 6a-iii deliverable.
//!
//! Provides:
//! - `ollama_check_and_install` — version gate + E019/E007 emission
//! - `ollama_pull`              — stream `ollama pull <model>` via shell plugin
//! - `ollama_select_model`      — pure RAM-tier → Gemma 4 tag mapping
//! - `ollama_detect_ram`        — reads total system RAM via sysinfo

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use biscuitcode_providers::{
    gemma3_fallback_for_ram_gb, gemma4_tag_for_ram_gb, OllamaProvider, OllamaVersionStatus,
};

// ---------- Types ----------

/// Return value for `ollama_check_and_install`.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OllamaStatus {
    /// Daemon is running and version >= 0.20.0.
    Ready { version: String },
    /// Daemon is running but version is below the Gemma 4 minimum.
    /// The `fallback_model` field carries the Gemma 3 tag to use.
    VersionTooOld {
        version: String,
        fallback_model: String,
    },
    /// Daemon not running (connection refused) or Ollama not installed.
    NotInstalled,
}

// ---------- Commands ----------

/// Check whether Ollama is running and its version is new enough for Gemma 4.
///
/// - Connection refused → emit E019 toast, return `NotInstalled`.
/// - Version < 0.20.0  → emit E007 toast, return `VersionTooOld`.
/// - Version >= 0.20.0 → return `Ready { version }`.
#[tauri::command]
pub async fn ollama_check_and_install(app_handle: AppHandle) -> OllamaStatus {
    let provider = OllamaProvider::new();
    match provider.check_version().await {
        OllamaVersionStatus::Down => {
            let payload = serde_json::json!({
                "code": "E019",
                "messageKey": "errors.E019.msg",
                "recovery": {
                    "kind": "copy_command",
                    "command": "ollama serve",
                    "label": "Copy start command"
                }
            });
            let _ = app_handle.emit("biscuitcode:error", payload);
            OllamaStatus::NotInstalled
        }
        OllamaVersionStatus::TooOld(version) => {
            let ram_gb = detect_ram_gb();
            let fallback_model = gemma3_fallback_for_ram_gb(ram_gb).to_string();
            let payload = serde_json::json!({
                "code": "E007",
                "messageKey": "errors.E007.msg",
                "recovery": {
                    "kind": "copy_command",
                    "command": "curl -fsSL https://ollama.com/install.sh | sh",
                    "label": "Copy upgrade command"
                }
            });
            let _ = app_handle.emit("biscuitcode:error", payload);
            OllamaStatus::VersionTooOld {
                version,
                fallback_model,
            }
        }
        OllamaVersionStatus::Ready(version) => OllamaStatus::Ready { version },
    }
}

/// Pull a model with `ollama pull <model>`, streaming progress lines to the
/// frontend as `"ollama:pull-progress"` events.
///
/// Returns `Ok(())` on exit code 0; returns an `Err(String)` on failure.
#[tauri::command]
pub async fn ollama_pull(model: String, app_handle: AppHandle) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;

    let shell = app_handle.shell();
    let (mut rx, _child) = shell
        .command("ollama")
        .args(["pull", &model])
        .spawn()
        .map_err(|e| format!("failed to spawn ollama pull: {e}"))?;

    while let Some(event) = rx.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                let text = String::from_utf8_lossy(&line).to_string();
                let _ = app_handle.emit("ollama:pull-progress", &text);
            }
            tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let text = String::from_utf8_lossy(&line).to_string();
                let _ = app_handle.emit("ollama:pull-progress", &text);
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                let code = payload.code.unwrap_or(-1);
                if code != 0 {
                    return Err(format!("ollama pull exited with code {code}"));
                }
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Pure function: returns the recommended Gemma 4 tag for the given RAM in GB.
///
/// | RAM (GB) | Tag            |
/// |----------|----------------|
/// | < 8      | `gemma4:e2b`   |
/// | 8–31     | `gemma4:e4b`   |
/// | 32–47    | `gemma4:26b`   |
/// | >= 48    | `gemma4:31b`   |
#[tauri::command]
pub fn ollama_select_model(ram_gb: u32) -> String {
    gemma4_tag_for_ram_gb(ram_gb).to_string()
}

/// Reads total system RAM via `sysinfo` and returns it in GB (floor).
#[tauri::command]
pub fn ollama_detect_ram() -> u32 {
    detect_ram_gb()
}

// ---------- Helpers ----------

fn detect_ram_gb() -> u32 {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    let bytes = sys.total_memory(); // bytes
    (bytes / 1_073_741_824) as u32 // bytes → GiB
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_model_matches_plan_table() {
        assert_eq!(ollama_select_model(4), "gemma4:e2b");
        assert_eq!(ollama_select_model(7), "gemma4:e2b");
        assert_eq!(ollama_select_model(8), "gemma4:e4b");
        assert_eq!(ollama_select_model(16), "gemma4:e4b");
        assert_eq!(ollama_select_model(31), "gemma4:e4b");
        assert_eq!(ollama_select_model(32), "gemma4:26b");
        assert_eq!(ollama_select_model(47), "gemma4:26b");
        assert_eq!(ollama_select_model(48), "gemma4:31b");
        assert_eq!(ollama_select_model(128), "gemma4:31b");
    }
}
