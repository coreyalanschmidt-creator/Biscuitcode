//! Chat commands — Phase 5 Tauri IPC layer.
//!
//! These commands wire the frontend chat panel to the Anthropic provider,
//! the DB persistence layer, and the libsecret keyring.
//!
//! **Key invariant:** API keys are never echoed back to the frontend.
//! The `get_anthropic_key` command returns only a boolean (present/absent).

use std::sync::{Arc, Mutex};

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use biscuitcode_agent::{
    executor::{confirmation::PendingConfirmations, ExecutorContext},
    ReActExecutor,
};
use biscuitcode_core::secrets;
use biscuitcode_db::{ConversationId, Database, MessageId, WorkspaceId};
use biscuitcode_providers::{
    AnthropicProvider, ChatEvent, ChatOptions, ContentBlock, Message, MessageRole, ModelInfo,
    ModelProvider, Role, Usage,
};

// ---------- State types ----------

/// Tauri managed state: the SQLite database.
///
/// Wrapped in `Mutex` so Tauri's multi-thread invoker can access it.
/// `Option` because it's `None` until the app finishes startup setup.
pub struct ChatDb(pub Mutex<Option<Database>>);

// ---------- Key management ----------

/// Check whether an Anthropic API key is stored in libsecret.
/// Returns `true` if present; `false` if absent or keyring unavailable.
/// Never returns the key itself.
#[tauri::command]
pub async fn anthropic_key_present() -> bool {
    matches!(
        secrets::get(secrets::SERVICE, "anthropic_api_key").await,
        Ok(Some(_))
    )
}

/// Store an Anthropic API key in libsecret.
///
/// Returns `E001` if the Secret Service is not available.
/// Returns `E004` format error string on other failures.
#[tauri::command]
pub async fn anthropic_set_key(key: String) -> Result<(), String> {
    // Pre-flight: never activate the daemon unless it's already up.
    let available =
        biscuitcode_core::secrets::secret_service_available().map_err(|e| e.to_string())?;
    if !available {
        return Err("E001".to_string());
    }
    secrets::set(secrets::SERVICE, "anthropic_api_key", &key)
        .await
        .map_err(|e| e.to_string())
}

/// Delete the stored Anthropic API key.
#[tauri::command]
pub async fn anthropic_delete_key() -> Result<(), String> {
    secrets::delete(secrets::SERVICE, "anthropic_api_key")
        .await
        .map_err(|e| e.to_string())
}

// ---------- Model listing ----------

/// List available Anthropic models. Does not require a live API call in
/// Phase 5 (hard-coded list). Phase 6a variants do a live fetch.
#[tauri::command]
pub async fn anthropic_list_models() -> Result<Vec<ModelInfoDto>, String> {
    // Load the key — if absent, still return models (UI shows yellow badge).
    let key = match secrets::get(secrets::SERVICE, "anthropic_api_key")
        .await
        .unwrap_or(None)
    {
        Some(k) => k,
        None => "placeholder".to_string(),
    };
    let provider = AnthropicProvider::new(key);
    provider
        .list_models()
        .await
        .map(|models| models.into_iter().map(ModelInfoDto::from).collect())
        .map_err(|e| e.to_string())
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelInfoDto {
    pub id: String,
    pub display_name: String,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub is_reasoning_model: bool,
    pub legacy: bool,
}

impl From<ModelInfo> for ModelInfoDto {
    fn from(m: ModelInfo) -> Self {
        Self {
            id: m.id,
            display_name: m.display_name,
            supports_tools: m.supports_tools,
            supports_vision: m.supports_vision,
            is_reasoning_model: m.is_reasoning_model,
            legacy: m.legacy,
        }
    }
}

// ---------- Conversation management ----------

/// Create a new conversation in the DB. Returns the conversation_id.
#[tauri::command]
pub fn chat_create_conversation(
    state: State<ChatDb>,
    workspace_id: String,
    title: String,
    model: String,
) -> Result<String, String> {
    let mut guard = state.0.lock().map_err(|_| "db lock poisoned".to_string())?;
    let db = guard.as_mut().ok_or("db not initialised")?;
    let ws_id = WorkspaceId(workspace_id);
    let conv = db
        .create_conversation(&ws_id, &title, &model)
        .map_err(|e| e.to_string())?;
    Ok(conv.conversation_id.0)
}

/// List conversations for a workspace.
#[tauri::command]
pub fn chat_list_conversations(
    state: State<ChatDb>,
    workspace_id: String,
) -> Result<Vec<ConversationDto>, String> {
    let guard = state.0.lock().map_err(|_| "db lock poisoned".to_string())?;
    let db = guard.as_ref().ok_or("db not initialised")?;
    let ws_id = WorkspaceId(workspace_id);
    db.list_conversations(&ws_id)
        .map(|list| list.into_iter().map(ConversationDto::from).collect())
        .map_err(|e| e.to_string())
}

/// List messages for a conversation.
#[tauri::command]
pub fn chat_list_messages(
    state: State<ChatDb>,
    conversation_id: String,
) -> Result<Vec<MessageDto>, String> {
    let guard = state.0.lock().map_err(|_| "db lock poisoned".to_string())?;
    let db = guard.as_ref().ok_or("db not initialised")?;
    let conv_id = ConversationId(conversation_id);
    db.list_messages(&conv_id)
        .map(|msgs| msgs.into_iter().map(MessageDto::from).collect())
        .map_err(|e| e.to_string())
}

#[derive(Clone, Debug, Serialize)]
pub struct ConversationDto {
    pub conversation_id: String,
    pub workspace_id: String,
    pub title: String,
    pub active_model: String,
    pub active_branch_message_id: Option<String>,
}

impl From<biscuitcode_db::Conversation> for ConversationDto {
    fn from(c: biscuitcode_db::Conversation) -> Self {
        Self {
            conversation_id: c.conversation_id.0,
            workspace_id: c.workspace_id.0,
            title: c.title,
            active_model: c.active_model,
            active_branch_message_id: c.active_branch_message_id.map(|m| m.0),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MessageDto {
    pub message_id: String,
    pub conversation_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub model: String,
    /// Serialized Vec<ContentBlock> as JSON string for simplicity.
    pub content_json: String,
}

impl From<biscuitcode_db::StoredMessage> for MessageDto {
    fn from(m: biscuitcode_db::StoredMessage) -> Self {
        Self {
            message_id: m.message_id.0,
            conversation_id: m.conversation_id.0,
            parent_id: m.parent_id.map(|p| p.0),
            role: format!("{:?}", m.role).to_lowercase(),
            model: m.model,
            content_json: serde_json::to_string(&m.content).unwrap_or_default(),
        }
    }
}

// ---------- Streaming chat ----------

/// Request shape from the frontend chat panel.
#[derive(Debug, Deserialize)]
pub struct ChatSendRequest {
    pub conversation_id: String,
    pub workspace_id: String,
    pub model: String,
    /// Plain-text user message. Mentions + attachments encoded inline.
    pub text: String,
    pub system: Option<String>,
    /// Optional parent message id for branching.
    pub parent_message_id: Option<String>,
    /// Phase 6b: when true, the executor runs in agent mode (auto-continues
    /// on tool calls). Defaults to false for backwards compatibility.
    #[serde(default)]
    pub agent_mode: bool,
}

/// Start a streaming Anthropic chat request.
///
/// Persists the user message immediately, then streams `ChatEvent` payloads
/// over the Tauri event channel `biscuitcode:chat-event:<conversation_id>`.
/// When the stream ends (`Done`), persists the assistant message.
///
/// When `agent_mode` is true, the request is routed through `ReActExecutor`
/// so write tools, the confirmation gate, and snapshot/rewind all function.
///
/// Frontend listens with:
/// ```ts
/// await listen(`biscuitcode:chat-event:${convId}`, (e) => handleEvent(e.payload));
/// ```
#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
    state: State<'_, ChatDb>,
    req: ChatSendRequest,
) -> Result<(), String> {
    let conv_id = ConversationId(req.conversation_id.clone());
    let _ws_id = WorkspaceId(req.workspace_id.clone());
    let parent_id = req
        .parent_message_id
        .as_deref()
        .map(|s| MessageId(s.to_string()));

    // Ensure workspace exists.
    {
        let mut guard = state.0.lock().map_err(|_| "db lock poisoned")?;
        let db = guard.as_mut().ok_or("db not initialised")?;
        db.upsert_workspace(&req.workspace_id)
            .map_err(|e| e.to_string())?;
    }

    // Persist the user message.
    let user_msg_id = {
        let mut guard = state.0.lock().map_err(|_| "db lock poisoned")?;
        let db = guard.as_mut().ok_or("db not initialised")?;
        let msg = db
            .append_message(
                &conv_id,
                parent_id.as_ref(),
                MessageRole::User,
                "",
                &[ContentBlock::Text {
                    text: req.text.clone(),
                }],
                &[],
                &[],
                None,
            )
            .map_err(|e| e.to_string())?;
        msg.message_id
    };

    // Load the API key.
    let api_key = secrets::get(secrets::SERVICE, "anthropic_api_key")
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "E001".to_string())?;

    let provider = AnthropicProvider::new(api_key);

    // Build messages for the provider (load from DB so history is included).
    // DB lock released before entering any async streaming work (PM-02: deadlock avoidance).
    let messages = {
        let guard = state.0.lock().map_err(|_| "db lock poisoned")?;
        let db = guard.as_ref().ok_or("db not initialised")?;
        let stored = db.list_messages(&conv_id).map_err(|e| e.to_string())?;
        stored
            .into_iter()
            .map(|m| Message {
                role: m.role,
                content: m.content,
                tool_calls: m.tool_calls,
                tool_results: m.tool_results,
            })
            .collect::<Vec<_>>()
    };
    // DB lock is now released — no lock held below.

    let opts = ChatOptions {
        model: req.model.clone(),
        system: req.system.unwrap_or_default(),
        prompt_caching_enabled: true,
        ..Default::default()
    };

    let event_channel = format!("biscuitcode:chat-event:{}", req.conversation_id);

    if req.agent_mode {
        // --- Agent mode: route through ReActExecutor ---
        //
        // Retrieves confirmation state from managed Tauri state, constructs
        // ExecutorContext, and drives the ReAct loop. Events are forwarded
        // to the frontend via the emit_event callback.

        let pending: Arc<PendingConfirmations> = {
            use super::agent::ConfirmationState;
            app.state::<ConfirmationState>().0.clone()
        };

        // Workspace root from WorkspaceState (None → use process cwd as fallback).
        let workspace_root = {
            use super::fs::WorkspaceState;
            let ws = app.state::<WorkspaceState>();
            let guard = ws.0.lock().map_err(|_| "workspace lock poisoned")?;
            match guard.as_deref() {
                Some(p) => std::path::PathBuf::from(p),
                None => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")),
            }
        };

        // Cache root for snapshots: ~/.cache/biscuitcode/
        let cache_root = app.path().app_cache_dir().map_err(|e| e.to_string())?;

        // Workspace trust from settings (best-effort read from localStorage-like value;
        // for now, default to false — the confirmation gate handles all write/shell tools).
        let workspace_trusted = false;

        let app_clone = app.clone();
        let event_channel_clone = event_channel.clone();

        let emit_confirm: Arc<
            dyn Fn(
                    biscuitcode_agent::executor::confirmation::ConfirmationRequest,
                ) -> Result<(), String>
                + Send
                + Sync,
        > = {
            let app2 = app.clone();
            Arc::new(move |req| {
                app2.emit("biscuitcode:confirm-request", &req)
                    .map_err(|e| e.to_string())
            })
        };

        let emit_event: Arc<dyn Fn(&ChatEvent) + Send + Sync> = Arc::new(move |ev| {
            let _ = app_clone.emit(&event_channel_clone, ChatEventPayload::from_event(ev));
        });

        let exec_ctx = Arc::new(ExecutorContext {
            cache_root,
            pending,
            workspace_trusted,
            emit_confirm,
            emit_event: Some(emit_event),
        });

        let registry = Arc::new(biscuitcode_agent::tools::ToolRegistry::full_default());
        let executor =
            ReActExecutor::new(registry, workspace_root, conv_id.clone()).with_context(exec_ctx);

        let original_msg_count = messages.len();
        let provider_arc: Arc<dyn ModelProvider> = Arc::new(provider);
        let outcome = executor
            .run(provider_arc, messages, opts, true)
            .await
            .map_err(|e| e.to_string())?;

        // Emit a synthetic Done event so the frontend knows the run ended.
        let done_payload = ChatEventPayload {
            event_type: "done".into(),
            stop_reason: Some("end_turn".into()),
            usage: Some(UsageDto {
                input_tokens: 0,
                output_tokens: 0,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            }),
            ..ChatEventPayload::empty()
        };
        let _ = app.emit(&event_channel, done_payload);

        // Persist the final assistant messages from the outcome.
        let final_messages = match outcome {
            biscuitcode_agent::RunOutcome::Done { messages } => messages,
            biscuitcode_agent::RunOutcome::Paused { messages } => messages,
            biscuitcode_agent::RunOutcome::ToolsAvailable { messages } => messages,
        };

        // Persist assistant turn(s) that the executor produced.
        let mut guard = state.0.lock().map_err(|_| "db lock poisoned")?;
        if let Some(db) = guard.as_mut() {
            let mut parent = Some(user_msg_id);
            for msg in final_messages.iter().skip(original_msg_count) {
                if msg.role == Role::Assistant {
                    let text = extract_text_from_content(&msg.content);
                    let content_blocks: Vec<ContentBlock> = if text.is_empty() {
                        vec![]
                    } else {
                        vec![ContentBlock::Text { text }]
                    };
                    if let Ok(stored) = db.append_message(
                        &conv_id,
                        parent.as_ref(),
                        MessageRole::Assistant,
                        &req.model,
                        &content_blocks,
                        &msg.tool_calls,
                        &msg.tool_results,
                        None,
                    ) {
                        parent = Some(stored.message_id);
                    }
                }
            }
        }
    } else {
        // --- Plain streaming mode (no agent tools) ---
        let mut stream = provider
            .chat_stream(messages, vec![], opts)
            .await
            .map_err(|e| e.to_string())?;

        let mut assistant_text = String::new();
        let mut final_usage: Option<Usage> = None;

        while let Some(item) = stream.next().await {
            let event = match item {
                Ok(e) => e,
                Err(e) => {
                    let _ = app.emit(&event_channel, ChatEventPayload::from_err(e.to_string()));
                    break;
                }
            };

            if let ChatEvent::TextDelta { ref text } = event {
                assistant_text.push_str(text);
            }
            if let ChatEvent::Done { ref usage, .. } = event {
                final_usage = Some(*usage);
            }

            let _ = app.emit(&event_channel, ChatEventPayload::from_event(&event));

            if matches!(event, ChatEvent::Done { .. }) {
                let content = if assistant_text.is_empty() {
                    vec![]
                } else {
                    vec![ContentBlock::Text {
                        text: assistant_text.clone(),
                    }]
                };
                let mut guard = state.0.lock().map_err(|_| "db lock poisoned")?;
                if let Some(db) = guard.as_mut() {
                    let _ = db.append_message(
                        &conv_id,
                        Some(&user_msg_id),
                        MessageRole::Assistant,
                        &req.model,
                        &content,
                        &[],
                        &[],
                        final_usage.as_ref(),
                    );
                }
                break;
            }
        }
    }

    Ok(())
}

/// Helper: extract plain text from a ContentBlock slice.
fn extract_text_from_content(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Serializable payload for the `biscuitcode:chat-event:<id>` Tauri event.
#[derive(Clone, Debug, Serialize)]
pub struct ChatEventPayload {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args_delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recoverable: Option<bool>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct UsageDto {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
}

impl From<Usage> for UsageDto {
    fn from(u: Usage) -> Self {
        Self {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
            cache_read_input_tokens: u.cache_read_input_tokens,
            cache_creation_input_tokens: u.cache_creation_input_tokens,
        }
    }
}

impl ChatEventPayload {
    fn from_event(event: &ChatEvent) -> Self {
        match event {
            ChatEvent::TextDelta { text } => Self {
                event_type: "text_delta".into(),
                text: Some(text.clone()),
                ..Self::empty()
            },
            ChatEvent::ThinkingDelta { text } => Self {
                event_type: "thinking_delta".into(),
                text: Some(text.clone()),
                ..Self::empty()
            },
            ChatEvent::ToolCallStart { id, name } => Self {
                event_type: "tool_call_start".into(),
                id: Some(id.clone()),
                name: Some(name.clone()),
                ..Self::empty()
            },
            ChatEvent::ToolCallDelta { id, args_delta } => Self {
                event_type: "tool_call_delta".into(),
                id: Some(id.clone()),
                args_delta: Some(args_delta.clone()),
                ..Self::empty()
            },
            ChatEvent::ToolCallEnd { id, args_json } => Self {
                event_type: "tool_call_end".into(),
                id: Some(id.clone()),
                args_json: Some(args_json.clone()),
                ..Self::empty()
            },
            ChatEvent::Done { stop_reason, usage } => Self {
                event_type: "done".into(),
                stop_reason: Some(stop_reason.clone()),
                usage: Some(UsageDto::from(*usage)),
                ..Self::empty()
            },
            ChatEvent::Error {
                code,
                message,
                recoverable,
            } => Self {
                event_type: "error".into(),
                code: Some(code.clone()),
                message: Some(message.clone()),
                recoverable: Some(*recoverable),
                ..Self::empty()
            },
        }
    }

    fn from_err(msg: String) -> Self {
        Self {
            event_type: "error".into(),
            code: Some("E005".into()),
            message: Some(msg),
            recoverable: Some(false),
            ..Self::empty()
        }
    }

    fn empty() -> Self {
        Self {
            event_type: String::new(),
            text: None,
            id: None,
            name: None,
            args_delta: None,
            args_json: None,
            stop_reason: None,
            usage: None,
            code: None,
            message: None,
            recoverable: None,
        }
    }
}

// ---------- Inline edit (Phase 6b) ----------

/// Request shape for the inline edit command.
#[derive(Debug, Deserialize)]
pub struct InlineEditRequest {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub selected_text: String,
    pub description: String,
}

/// Request shape to apply an accepted inline edit.
#[derive(Debug, Deserialize)]
pub struct ApplyInlineEditRequest {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub new_content: String,
}

/// Start a streaming inline edit request.
///
/// Emits `biscuitcode:inline-edit-delta:<file_path>` events with `{ delta, done, error }`.
/// The model is prompted with the file context + selected text + description.
///
/// This is a simplified implementation that sends the edit request to the Anthropic
/// provider and streams the response as raw text deltas.
#[tauri::command]
pub async fn chat_inline_edit(
    app: AppHandle,
    _state: State<'_, ChatDb>,
    req: InlineEditRequest,
) -> Result<(), String> {
    use futures::StreamExt;

    let api_key = crate::commands::chat::get_anthropic_key_inner()
        .await
        .ok_or("E001")?;

    let provider = AnthropicProvider::new(api_key);

    let system = "You are an expert code editor. The user will give you code to modify along with instructions. \
                  Reply with ONLY the modified code — no explanation, no markdown fences, no extra text. \
                  Preserve indentation and style.";

    let prompt = format!(
        "File: {}\nLines {}-{}:\n```\n{}\n```\n\nInstructions: {}\n\nReturn only the replacement code.",
        req.file_path,
        req.start_line,
        req.end_line,
        req.selected_text,
        req.description,
    );

    let messages = vec![Message {
        role: Role::User,
        content: vec![ContentBlock::Text { text: prompt }],
        tool_calls: vec![],
        tool_results: vec![],
    }];

    let opts = ChatOptions {
        model: "claude-opus-4-7".to_string(),
        system: system.to_string(),
        prompt_caching_enabled: false,
        ..Default::default()
    };

    let event_channel = format!("biscuitcode:inline-edit-delta:{}", req.file_path);

    #[derive(Clone, serde::Serialize)]
    struct DeltaPayload {
        delta: Option<String>,
        done: bool,
        error: Option<String>,
    }

    let mut stream = provider
        .chat_stream(messages, vec![], opts)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(item) = stream.next().await {
        match item {
            Ok(ChatEvent::TextDelta { text }) => {
                let _ = app.emit(
                    &event_channel,
                    DeltaPayload {
                        delta: Some(text),
                        done: false,
                        error: None,
                    },
                );
            }
            Ok(ChatEvent::Done { .. }) => {
                let _ = app.emit(
                    &event_channel,
                    DeltaPayload {
                        delta: None,
                        done: true,
                        error: None,
                    },
                );
                break;
            }
            Ok(ChatEvent::Error {
                message,
                recoverable,
                ..
            }) if !recoverable => {
                let _ = app.emit(
                    &event_channel,
                    DeltaPayload {
                        delta: None,
                        done: true,
                        error: Some(message.clone()),
                    },
                );
                return Err(message);
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = app.emit(
                    &event_channel,
                    DeltaPayload {
                        delta: None,
                        done: true,
                        error: Some(msg.clone()),
                    },
                );
                return Err(msg);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Helper to get the Anthropic API key (extracted so inline_edit can call it).
pub async fn get_anthropic_key_inner() -> Option<String> {
    use biscuitcode_core::secrets;
    secrets::get(secrets::SERVICE, "anthropic_api_key")
        .await
        .unwrap_or(None)
}

/// Apply an accepted inline edit to a file (replaces lines start_line..end_line with new_content).
#[tauri::command]
pub async fn chat_apply_inline_edit(req: ApplyInlineEditRequest) -> Result<(), String> {
    let content = tokio::fs::read_to_string(&req.file_path)
        .await
        .map_err(|e| e.to_string())?;

    let lines: Vec<&str> = content.lines().collect();
    let start = (req.start_line as usize).saturating_sub(1);
    let end = (req.end_line as usize).min(lines.len());

    let mut result = Vec::new();
    result.extend_from_slice(&lines[..start]);
    result.extend(req.new_content.lines());
    result.extend_from_slice(&lines[end..]);

    let new_content = result.join("\n");
    tokio::fs::write(&req.file_path, new_content)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
