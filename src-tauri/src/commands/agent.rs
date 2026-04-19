//! Agent commands — Phase 6b Tauri IPC layer.
//!
//! Covers:
//!  - `agent_confirm_decision` — frontend sends back the user's Approve/Deny
//!    decision for a pending write/shell confirmation modal.
//!  - `agent_rewind` — frontend triggers a rewind from a given message.

use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Manager, State};

use biscuitcode_agent::executor::confirmation::{Decision, PendingConfirmations};
use biscuitcode_db::{ConversationId, Database, MessageId};

use super::chat::ChatDb;

// ---------- Confirmation gate ----------

/// Managed state for the pending confirmations map.
pub struct ConfirmationState(pub Arc<PendingConfirmations>);

/// Payload from the frontend's confirmation modal.
#[derive(Debug, Deserialize)]
pub struct ConfirmDecisionRequest {
    /// The request_id from the `ConfirmationRequest` the frontend received.
    pub request_id: String,
    /// "approve", "deny", or "deny_with_feedback"
    pub decision: String,
    /// Only set when decision == "deny_with_feedback"
    pub feedback: Option<String>,
}

/// Receive a user decision for a pending confirmation request.
///
/// Returns `true` if the decision was delivered; `false` if the request
/// was already resolved (timed out, or duplicate invoke).
#[tauri::command]
pub fn agent_confirm_decision(
    state: State<'_, ConfirmationState>,
    req: ConfirmDecisionRequest,
) -> bool {
    let decision = match req.decision.as_str() {
        "approve" => Decision::Approve,
        "deny" => Decision::Deny,
        "deny_with_feedback" => Decision::DenyWithFeedback {
            feedback: req.feedback.unwrap_or_default(),
        },
        _ => Decision::Deny,
    };
    state.0.resolve(&req.request_id, decision)
}

// ---------- Rewind ----------

/// Request to rewind a conversation to the state before a given message.
#[derive(Debug, Deserialize)]
pub struct RewindRequest {
    pub conversation_id: String,
    /// The assistant message that performed the write/shell tools. All
    /// snapshots at or after this message are restored; all messages after
    /// this message are deleted.
    pub rewind_to_message_id: String,
    /// Absolute path to `~/.cache/biscuitcode/`. Used to locate .bak files.
    pub cache_root: String,
}

/// Rewind a conversation. Restores snapshots and truncates messages.
///
/// On partial failure, returns an error string indicating which file
/// failed to restore. Already-restored files are left in their restored state.
///
/// The frontend is expected to refresh open editor models after this command
/// succeeds.
#[tauri::command]
pub async fn agent_rewind(
    _app: AppHandle,
    state: State<'_, ChatDb>,
    req: RewindRequest,
) -> Result<(), String> {
    let conv_id = ConversationId(req.conversation_id.clone());
    let msg_id = MessageId(req.rewind_to_message_id.clone());
    let cache_root = std::path::PathBuf::from(&req.cache_root);

    // Load snapshots from DB that are at or after the rewind point.
    let snapshots = {
        let guard = state.0.lock().map_err(|_| "db lock poisoned".to_string())?;
        let db = guard.as_ref().ok_or("db not initialised")?;
        db.list_snapshots_from_message(&conv_id, &msg_id)
            .map_err(|e| e.to_string())?
    };

    // Restore each snapshot in reverse-chronological order (newest first).
    // (list_snapshots_from_message already returns DESC order.)
    for snap in &snapshots {
        let snap_dir = biscuitcode_agent::executor::snapshot::snapshot_dir(
            &cache_root,
            &snap.conversation_id.0,
            &snap.message_id.0,
        );

        if !snap_dir.exists() {
            tracing::warn!(
                snapshot_id = %snap.snapshot_id.0,
                "snapshot directory missing — skipping"
            );
            continue;
        }

        let manifest = biscuitcode_agent::executor::snapshot::load_manifest(&snap_dir)
            .await
            .map_err(|e| format!("E011 RewindFailed: load manifest: {e}"))?;

        biscuitcode_agent::executor::snapshot::restore(&snap_dir, &manifest)
            .await
            .map_err(|e| format!("E011 RewindFailed: {e}"))?;
    }

    // Truncate messages after the rewind point and update conversation leaf.
    {
        let mut guard = state.0.lock().map_err(|_| "db lock poisoned".to_string())?;
        let db = guard.as_mut().ok_or("db not initialised")?;
        db.truncate_messages_after(&conv_id, &msg_id)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
