//! Agent commands — Phase 6a-ii / Phase 6b Tauri IPC layer.
//!
//! Phase 6a-ii adds:
//!  - `agent_run` — drives `ReActExecutor` and forwards `ChatEvent`s to the
//!    frontend via the `"agent:event"` Tauri event.
//!  - `agent_pause` — sets the global pause flag so the running executor
//!    stops at its next iteration boundary.
//!
//! Phase 6b adds:
//!  - `agent_confirm_decision` — frontend sends back the user's Approve/Deny
//!    decision for a pending write/shell confirmation modal.
//!  - `agent_rewind` — frontend triggers a rewind from a given message.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, State};

use biscuitcode_agent::{
    executor::{
        confirmation::{Decision, PendingConfirmations},
        ExecutorContext,
    },
    tools::ToolRegistry,
    ReActExecutor,
};
use biscuitcode_core::secrets;
use biscuitcode_db::{ConversationId, MessageId};
use biscuitcode_providers::{AnthropicProvider, ChatEvent, ChatOptions, Message, ModelProvider};

use super::chat::{ChatDb, ChatEventPayload};

// ---------- Agent run managed state ----------

/// Managed state: the global pause flag for `agent_run`.
///
/// `agent_run` clones this flag into the executor so `agent_pause` can set it.
/// Reset to `false` at the start of each `agent_run` call.
pub struct AgentPauseFlag(pub Arc<AtomicBool>);

/// Managed state: the read-only tool registry.
///
/// Constructed once and shared across all `agent_run` calls.
pub struct AgentToolRegistry(pub Arc<ToolRegistry>);

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

// ---------- Phase 6a-ii: agent_run + agent_pause ----------

/// Drives `ReActExecutor` over an existing conversation, forwarding every
/// `ChatEvent` to the frontend via the `"agent:event"` Tauri event.
///
/// Uses the managed `AgentPauseFlag` so a concurrent `agent_pause` call can
/// stop the loop at its next iteration boundary (< 5 s when no tool runs).
///
/// Emits `{ type: "done" }` when the run finishes (normal or paused).
#[tauri::command]
pub async fn agent_run(
    app: AppHandle,
    db_state: State<'_, ChatDb>,
    pause_state: State<'_, AgentPauseFlag>,
    registry_state: State<'_, AgentToolRegistry>,
    conversation_id: String,
    model_id: String,
    #[allow(unused_variables)] agent_mode: bool,
) -> Result<(), String> {
    // Reset the pause flag for this run.
    pause_state.0.store(false, Ordering::SeqCst);

    let conv_id = ConversationId(conversation_id.clone());

    // Load conversation history. DB lock released before async streaming work.
    let messages: Vec<Message> = {
        let guard = db_state.0.lock().map_err(|_| "db lock poisoned")?;
        let db = guard.as_ref().ok_or("db not initialised")?;
        db.list_messages(&conv_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|m| Message {
                role: m.role,
                content: m.content,
                tool_calls: m.tool_calls,
                tool_results: m.tool_results,
            })
            .collect()
    };

    // Load API key.
    let api_key = secrets::get(secrets::SERVICE, "anthropic_api_key")
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "E001".to_string())?;

    let provider: Arc<dyn ModelProvider> = Arc::new(AnthropicProvider::new(api_key));

    let opts = ChatOptions {
        model: model_id.clone(),
        system: String::new(),
        prompt_caching_enabled: true,
        ..Default::default()
    };

    // Clone the managed pause flag so agent_pause affects this run.
    let pause_flag = pause_state.0.clone();

    // Workspace root (best-effort; falls back to cwd).
    let workspace_root = {
        use super::fs::WorkspaceState;
        let ws = app.state::<WorkspaceState>();
        let guard = ws.0.lock().map_err(|_| "workspace lock poisoned")?;
        match guard.as_deref() {
            Some(p) => std::path::PathBuf::from(p),
            None => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")),
        }
    };

    let registry = registry_state.0.clone();

    // Emit callback: each ChatEvent goes to "agent:event" on the Tauri bus.
    let app_clone = app.clone();
    let emit_event: biscuitcode_agent::executor::EventEmitter = Arc::new(move |ev: &ChatEvent| {
        let _ = app_clone.emit("agent:event", ChatEventPayload::from_event(ev));
    });

    // Minimal ExecutorContext — no confirmation gate for read-only 6a-ii tools.
    let pending = Arc::new(PendingConfirmations::new());
    let emit_confirm: Arc<
        dyn Fn(biscuitcode_agent::executor::confirmation::ConfirmationRequest) -> Result<(), String>
            + Send
            + Sync,
    > = {
        let app2 = app.clone();
        Arc::new(move |req| {
            app2.emit("biscuitcode:confirm-request", &req)
                .map_err(|e| e.to_string())
        })
    };
    let cache_root = app.path().app_cache_dir().map_err(|e| e.to_string())?;

    let exec_ctx = Arc::new(ExecutorContext {
        cache_root,
        pending,
        workspace_trusted: false,
        emit_confirm,
        emit_event: Some(emit_event),
    });

    let mut executor = ReActExecutor::new(registry, workspace_root, conv_id).with_context(exec_ctx);
    // Replace the executor's own pause flag with the managed one (PM-01 fix).
    executor.pause = pause_flag;

    let _outcome = executor
        .run(provider, messages, opts, true)
        .await
        .map_err(|e| e.to_string())?;

    // Emit a synthetic Done so the frontend knows the run ended.
    let done = ChatEventPayload {
        event_type: "done".into(),
        stop_reason: Some("end_turn".into()),
        usage: None,
        ..ChatEventPayload::empty()
    };
    let _ = app.emit("agent:event", done);

    Ok(())
}

/// Set the global pause flag so the running `agent_run` executor stops at its
/// next iteration boundary (< 5 s when no tool is executing).
#[tauri::command]
pub fn agent_pause(pause_state: State<'_, AgentPauseFlag>) {
    pause_state.0.store(true, Ordering::SeqCst);
}
