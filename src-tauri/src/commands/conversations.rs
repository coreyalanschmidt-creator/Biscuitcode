//! Conversation export/import + snapshot cleanup — Phase 8.
//!
//! Commands:
//!   - `conversations_export` — dump all conversations to a JSON file (CONVERSATION-EXPORT-SCHEMA.md)
//!   - `conversations_import` — import a previously-exported file (skip duplicates)
//!   - `snapshots_cleanup_now` — run the 30-day snapshot cleanup immediately
//!   - `detect_gtk_theme` — detect light/dark GTK theme via xfconf-query / gsettings
//!   - `get_app_cache_dir` — return the Tauri app cache dir for `window.__BISCUIT_CACHE_ROOT__`

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use biscuitcode_db::{Database, WorkspaceId};

use super::chat::ChatDb;

// ---------------------------------------------------------------------------
// Export schema types (matches docs/CONVERSATION-EXPORT-SCHEMA.md)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct ExportWorkspace {
    pub workspace_id: WorkspaceId,
    pub root_path: Option<String>,
    pub first_seen_at: String,
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportSnapshotFile {
    pub path: String,
    pub pre_sha256: Option<String>,
    pub pre_size_bytes: Option<u64>,
    pub snapshot_path_relative_to_cache: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportSnapshot {
    pub snapshot_id: String,
    pub tool_call_id: String,
    pub files: Vec<ExportSnapshotFile>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportMessage {
    pub message_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub created_at: String,
    pub model: String,
    pub content: serde_json::Value,
    pub tool_calls: serde_json::Value,
    pub tool_results: serde_json::Value,
    pub snapshots: Vec<ExportSnapshot>,
    pub usage: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct ExportConversation {
    pub conversation_id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub active_model: String,
    pub active_branch_message_id: Option<String>,
    pub messages: Vec<ExportMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct ConversationExport {
    pub schema_version: u32,
    pub exported_at: String,
    pub exported_by: String,
    pub workspaces: Vec<ExportWorkspace>,
    pub conversations: Vec<ExportConversation>,
}

// ---------------------------------------------------------------------------
// get_app_cache_dir
// ---------------------------------------------------------------------------

/// Return the Tauri app cache directory so the frontend can set
/// `window.__BISCUIT_CACHE_ROOT__`.
#[tauri::command]
pub fn get_app_cache_dir(app: AppHandle) -> Result<String, String> {
    app.path()
        .app_cache_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// detect_gtk_theme
// ---------------------------------------------------------------------------

/// Detect the running GTK theme via xfconf-query, then gsettings as fallback.
/// Returns `"dark"` or `"light"`.
#[tauri::command]
pub fn detect_gtk_theme() -> String {
    // Try xfconf-query first (XFCE).
    let xfconf = std::process::Command::new("xfconf-query")
        .args(["-c", "xsettings", "-p", "/Net/ThemeName"])
        .output();

    let theme_name = if let Ok(out) = xfconf {
        if out.status.success() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // If xfconf gave nothing, try gsettings (GNOME / Cinnamon).
    let theme_name = if theme_name.is_empty() {
        let gsettings = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
            .output();
        if let Ok(out) = gsettings {
            if out.status.success() {
                // gsettings wraps value in quotes e.g. `'Mint-Xia-Dark'`
                String::from_utf8_lossy(&out.stdout)
                    .trim()
                    .trim_matches('\'')
                    .to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        theme_name
    };

    // Regex: -dark$ (case-insensitive) → dark.
    if theme_name.to_lowercase().ends_with("-dark") || theme_name.to_lowercase().ends_with("dark") {
        "dark".to_string()
    } else {
        "light".to_string()
    }
}

// ---------------------------------------------------------------------------
// conversations_export
// ---------------------------------------------------------------------------

/// Export all conversations in the database to a JSON file matching
/// `docs/CONVERSATION-EXPORT-SCHEMA.md`. Returns the path of the written file.
#[tauri::command]
pub fn conversations_export(
    app: AppHandle,
    db_state: tauri::State<'_, ChatDb>,
) -> Result<String, String> {
    let guard = db_state.0.lock().map_err(|e| e.to_string())?;
    let db = guard.as_ref().ok_or("database not initialised")?;

    let export = build_export(db, &app).map_err(|e| e.to_string())?;

    let json = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;

    // Write to app data dir.
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let filename = format!(
        "biscuitcode-conversations-{}.json",
        Utc::now().format("%Y-%m-%d")
    );
    let out_path = data_dir.join(&filename);
    std::fs::write(&out_path, &json).map_err(|e| e.to_string())?;

    Ok(out_path.to_string_lossy().into_owned())
}

fn build_export(db: &Database, app: &AppHandle) -> Result<ConversationExport, biscuitcode_db::DbError> {
    use rusqlite::params;

    let conn = db.conn();

    // Load all workspaces.
    let mut ws_stmt = conn.prepare(
        "SELECT workspace_id, root_path, first_seen_at, label FROM workspaces ORDER BY first_seen_at"
    )?;
    let workspaces: Vec<ExportWorkspace> = ws_stmt
        .query_map([], |row| {
            Ok(ExportWorkspace {
                workspace_id: WorkspaceId(row.get(0)?),
                root_path: row.get(1)?,
                first_seen_at: row.get(2)?,
                label: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Load all conversations.
    let mut conv_stmt = conn.prepare(
        "SELECT conversation_id, workspace_id, title, created_at, updated_at, active_model, active_branch_message_id \
         FROM conversations ORDER BY created_at"
    )?;
    let conv_rows: Vec<(String, String, String, String, String, String, Option<String>)> = conv_stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?, row.get(1)?, row.get(2)?,
                row.get(3)?, row.get(4)?, row.get(5)?,
                row.get(6)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let cache_root = app
        .path()
        .app_cache_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut conversations = Vec::new();
    for (conv_id, ws_id, title, created_at, updated_at, active_model, active_branch) in conv_rows {
        // Load messages for this conversation.
        let mut msg_stmt = conn.prepare(
            "SELECT message_id, parent_id, role, created_at, model, content_json, tool_calls_json, tool_results_json, snapshot_id, usage_json \
             FROM messages WHERE conversation_id = ?1 ORDER BY created_at"
        )?;
        let msgs: Vec<ExportMessage> = msg_stmt
            .query_map(params![conv_id], |row| {
                let snap_id: Option<String> = row.get(8)?;
                let content_json: String = row.get(5)?;
                let tc_json: String = row.get(6)?;
                let tr_json: String = row.get(7)?;
                let usage_json: Option<String> = row.get(9)?;

                let snapshots: Vec<ExportSnapshot> = Vec::new(); // populated below
                Ok((
                    row.get::<_, String>(0)?,  // message_id
                    row.get::<_, Option<String>>(1)?,  // parent_id
                    row.get::<_, String>(2)?,  // role
                    row.get::<_, String>(3)?,  // created_at
                    row.get::<_, String>(4)?,  // model
                    content_json,
                    tc_json,
                    tr_json,
                    snap_id,
                    usage_json,
                    snapshots,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(msg_id, parent_id, role, created_at, model, content_json, tc_json, tr_json, snap_id, usage_json, _)| {
                // Load snapshot files if this message has a snapshot.
                let snapshots = snap_id.map(|sid| {
                    let mut sf_stmt = conn.prepare(
                        "SELECT abs_path, pre_sha256, pre_size_bytes, snapshot_filename \
                         FROM snapshot_files WHERE snapshot_id = ?1"
                    ).ok()?;
                    let files: Vec<ExportSnapshotFile> = sf_stmt.query_map(params![sid], |row| {
                        let abs_path: String = row.get(0)?;
                        let pre_sha256: Option<String> = row.get(1)?;
                        let pre_size: Option<i64> = row.get(2)?;
                        let snap_file: Option<String> = row.get(3)?;
                        Ok(ExportSnapshotFile {
                            path: abs_path.clone(),
                            pre_sha256,
                            pre_size_bytes: pre_size.map(|s| s as u64),
                            snapshot_path_relative_to_cache: snap_file.map(|f| {
                                // Make relative to cache root.
                                let full = format!("{}/snapshots/{}", cache_root, f);
                                full.strip_prefix(&cache_root)
                                    .map(|s| s.trim_start_matches('/').to_string())
                                    .unwrap_or(f)
                            }),
                        })
                    }).ok()?.filter_map(|r| r.ok()).collect();

                    // Get tool_call_id from snapshots table.
                    let tc_id: String = conn.query_row(
                        "SELECT tool_call_id FROM snapshots WHERE snapshot_id = ?1",
                        params![sid],
                        |row| row.get(0),
                    ).unwrap_or_default();

                    Some(ExportSnapshot {
                        snapshot_id: sid,
                        tool_call_id: tc_id,
                        files,
                    })
                }).flatten().into_iter().collect::<Vec<_>>();

                ExportMessage {
                    message_id: msg_id,
                    parent_id,
                    role,
                    created_at,
                    model,
                    content: serde_json::from_str(&content_json).unwrap_or(serde_json::Value::Null),
                    tool_calls: serde_json::from_str(&tc_json).unwrap_or(serde_json::Value::Array(vec![])),
                    tool_results: serde_json::from_str(&tr_json).unwrap_or(serde_json::Value::Array(vec![])),
                    snapshots,
                    usage: usage_json
                        .as_deref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or(serde_json::Value::Null),
                }
            })
            .collect();

        conversations.push(ExportConversation {
            conversation_id: conv_id,
            workspace_id: ws_id,
            title,
            created_at,
            updated_at,
            active_model,
            active_branch_message_id: active_branch,
            messages: msgs,
        });
    }

    Ok(ConversationExport {
        schema_version: 1,
        exported_at: Utc::now().to_rfc3339(),
        exported_by: format!("biscuitcode {}", env!("CARGO_PKG_VERSION")),
        workspaces,
        conversations,
    })
}

// ---------------------------------------------------------------------------
// conversations_import
// ---------------------------------------------------------------------------

/// Import a conversation export file. Skips duplicates by (conversation_id, message_id).
/// Returns the count of imported conversations and messages.
#[tauri::command]
pub fn conversations_import(
    _app: AppHandle,
    path: String,
    db_state: tauri::State<'_, ChatDb>,
) -> Result<ImportResult, String> {
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let export: ConversationExport = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    if export.schema_version != 1 {
        return Err(format!(
            "E018b SchemaVersionUnsupported: schema_version {} is not supported by this version of BiscuitCode.",
            export.schema_version
        ));
    }

    let mut guard = db_state.0.lock().map_err(|e| e.to_string())?;
    let db = guard.as_mut().ok_or("database not initialised")?;

    let conn = db.conn_mut();
    let mut imported_conversations = 0u32;
    let mut imported_messages = 0u32;

    // Use a single transaction for the import.
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Import workspaces.
    for ws in &export.workspaces {
        let root = ws.root_path.as_deref().unwrap_or("");
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM workspaces WHERE workspace_id = ?1",
                rusqlite::params![ws.workspace_id.0],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !exists {
            let _ = tx.execute(
                "INSERT OR IGNORE INTO workspaces (workspace_id, root_path, first_seen_at, label) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![ws.workspace_id.0, root, ws.first_seen_at, ws.label],
            );
        }
    }

    // Import conversations + messages.
    for conv in &export.conversations {
        let already: bool = tx
            .query_row(
                "SELECT 1 FROM conversations WHERE conversation_id = ?1",
                rusqlite::params![conv.conversation_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !already {
            let _ = tx.execute(
                "INSERT OR IGNORE INTO conversations \
                 (conversation_id, workspace_id, title, created_at, updated_at, active_model, active_branch_message_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    conv.conversation_id,
                    conv.workspace_id,
                    conv.title,
                    conv.created_at,
                    conv.updated_at,
                    conv.active_model,
                    conv.active_branch_message_id,
                ],
            );
            imported_conversations += 1;
        }

        for msg in &conv.messages {
            let msg_exists: bool = tx
                .query_row(
                    "SELECT 1 FROM messages WHERE message_id = ?1",
                    rusqlite::params![msg.message_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            if !msg_exists {
                let content_json = serde_json::to_string(&msg.content).unwrap_or("[]".to_string());
                let tc_json = serde_json::to_string(&msg.tool_calls).unwrap_or("[]".to_string());
                let tr_json = serde_json::to_string(&msg.tool_results).unwrap_or("[]".to_string());
                let usage_json = if msg.usage.is_null() {
                    None
                } else {
                    Some(serde_json::to_string(&msg.usage).unwrap_or_default())
                };
                let _ = tx.execute(
                    "INSERT OR IGNORE INTO messages \
                     (message_id, conversation_id, parent_id, role, created_at, model, content_json, tool_calls_json, tool_results_json, usage_json) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        msg.message_id,
                        conv.conversation_id,
                        msg.parent_id,
                        msg.role,
                        msg.created_at,
                        msg.model,
                        content_json,
                        tc_json,
                        tr_json,
                        usage_json,
                    ],
                );
                imported_messages += 1;
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(ImportResult {
        imported_conversations,
        imported_messages,
    })
}

#[derive(Serialize)]
pub struct ImportResult {
    pub imported_conversations: u32,
    pub imported_messages: u32,
}

// ---------------------------------------------------------------------------
// snapshots_cleanup_now
// ---------------------------------------------------------------------------

/// Delete snapshot directories + DB rows for:
///   - conversations that have been deleted, OR
///   - snapshots older than `max_age_days` where the conversation is closed.
///
/// "Closed" = conversation has no messages added in the past `max_age_days`.
#[tauri::command]
pub fn snapshots_cleanup_now(
    app: AppHandle,
    max_age_days: Option<u32>,
    db_state: tauri::State<'_, ChatDb>,
) -> Result<CleanupResult, String> {
    let max_days = max_age_days.unwrap_or(30) as i64;

    let guard = db_state.0.lock().map_err(|e| e.to_string())?;
    let db = guard.as_ref().ok_or("database not initialised")?;

    let cache_root = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?;
    let snapshots_dir = cache_root.join("snapshots");

    let conn = db.conn();
    let cutoff = (Utc::now() - chrono::Duration::days(max_days)).to_rfc3339();

    // Find snapshot IDs to delete: older than cutoff AND conversation is not active
    // (no messages updated recently).
    let mut stmt = conn
        .prepare(
            "SELECT s.snapshot_id, s.conversation_id FROM snapshots s \
             WHERE s.snapshotted_at < ?1 \
             AND NOT EXISTS ( \
               SELECT 1 FROM messages m \
               WHERE m.conversation_id = s.conversation_id \
               AND m.created_at >= ?1 \
             )",
        )
        .map_err(|e| e.to_string())?;

    let candidates: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![cutoff], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut deleted_snapshots = 0u32;
    let mut deleted_files = 0u32;

    for (snap_id, conv_id) in &candidates {
        // Delete snapshot files from disk.
        let snap_dir = snapshots_dir.join(conv_id).join(snap_id);
        if snap_dir.exists() {
            if std::fs::remove_dir_all(&snap_dir).is_ok() {
                deleted_files += 1;
            }
        }

        // Delete from DB (CASCADE removes snapshot_files rows).
        let _ = conn.execute(
            "DELETE FROM snapshots WHERE snapshot_id = ?1",
            rusqlite::params![snap_id],
        );
        deleted_snapshots += 1;
    }

    Ok(CleanupResult {
        deleted_snapshots,
        deleted_files,
    })
}

#[derive(Serialize)]
pub struct CleanupResult {
    pub deleted_snapshots: u32,
    pub deleted_files: u32,
}

// ---------------------------------------------------------------------------
// fork_message (conversation branching)
// ---------------------------------------------------------------------------

/// Fork a conversation from a given parent message. Creates a new user
/// message with the provided content, branching the conversation DAG.
/// Returns the new message's ID.
#[tauri::command]
pub fn fork_message(
    conversation_id: String,
    parent_id: String,
    content: String,
    db_state: tauri::State<'_, ChatDb>,
) -> Result<String, String> {
    use biscuitcode_db::{ConversationId, MessageId};
    use biscuitcode_providers::{ContentBlock, MessageRole};

    let mut guard = db_state.0.lock().map_err(|e| e.to_string())?;
    let db = guard.as_mut().ok_or("database not initialised")?;

    let conv_id = ConversationId(conversation_id);
    let par_id = MessageId(parent_id);

    let content_block = vec![ContentBlock::Text { text: content }];
    let msg = db
        .append_message(
            &conv_id,
            Some(&par_id),
            MessageRole::User,
            "",
            &content_block,
            &[],
            &[],
            None,
        )
        .map_err(|e| e.to_string())?;

    Ok(msg.message_id.0)
}

// ---------------------------------------------------------------------------
// list_message_branches (return the DAG for a conversation)
// ---------------------------------------------------------------------------

/// Return the parent_id map for all messages in a conversation — enough
/// for the frontend to render the branch tree.
#[tauri::command]
pub fn list_message_branches(
    conversation_id: String,
    db_state: tauri::State<'_, ChatDb>,
) -> Result<Vec<MessageBranchNode>, String> {
    let guard = db_state.0.lock().map_err(|e| e.to_string())?;
    let db = guard.as_ref().ok_or("database not initialised")?;

    let conv_id = biscuitcode_db::ConversationId(conversation_id);
    let messages = db.list_messages(&conv_id).map_err(|e| e.to_string())?;

    let nodes = messages
        .into_iter()
        .map(|m| MessageBranchNode {
            message_id: m.message_id.0,
            parent_id: m.parent_id.map(|p| p.0),
            role: format!("{:?}", m.role).to_lowercase(),
            created_at: m.created_at.to_rfc3339(),
        })
        .collect();

    Ok(nodes)
}

#[derive(Serialize)]
pub struct MessageBranchNode {
    pub message_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_schema_version_is_1() {
        // Build minimal export to test that the types serialize correctly.
        let export = ConversationExport {
            schema_version: 1,
            exported_at: "2026-04-19T00:00:00Z".to_string(),
            exported_by: "biscuitcode 0.1.0".to_string(),
            workspaces: vec![],
            conversations: vec![],
        };
        let json = serde_json::to_string(&export).unwrap();
        let back: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(back["schema_version"], 1);
    }

    #[test]
    fn cleanup_result_serializes() {
        let r = CleanupResult { deleted_snapshots: 3, deleted_files: 2 };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("deleted_snapshots"));
    }
}
