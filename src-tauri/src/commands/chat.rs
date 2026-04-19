//! Chat commands — Phase 5 Tauri IPC layer.
//!
//! These commands wire the frontend chat panel to the Anthropic provider,
//! the DB persistence layer, and the libsecret keyring.
//!
//! **Key invariant:** API keys are never echoed back to the frontend.
//! The `get_anthropic_key` command returns only a boolean (present/absent).

use std::sync::Mutex;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use biscuitcode_core::secrets;
use biscuitcode_db::{ConversationId, Database, MessageId, WorkspaceId};
use biscuitcode_providers::{
    AnthropicProvider, ChatEvent, ChatOptions, ContentBlock, Message, MessageRole, ModelInfo,
    ModelProvider, Role, ToolSpec, Usage,
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
    match secrets::get(secrets::SERVICE, "anthropic_api_key").await {
        Ok(Some(_)) => true,
        _ => false,
    }
}

/// Store an Anthropic API key in libsecret.
///
/// Returns `E001` if the Secret Service is not available.
/// Returns `E004` format error string on other failures.
#[tauri::command]
pub async fn anthropic_set_key(key: String) -> Result<(), String> {
    // Pre-flight: never activate the daemon unless it's already up.
    let available = biscuitcode_core::secrets::secret_service_available()
        .map_err(|e| e.to_string())?;
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
}

/// Start a streaming Anthropic chat request.
///
/// Persists the user message immediately, then streams `ChatEvent` payloads
/// over the Tauri event channel `biscuitcode:chat-event:<conversation_id>`.
/// When the stream ends (`Done`), persists the assistant message.
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
    let parent_id = req.parent_message_id.as_deref().map(|s| MessageId(s.to_string()));

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
                &[ContentBlock::Text { text: req.text.clone() }],
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

    let opts = ChatOptions {
        model: req.model.clone(),
        system: req.system.unwrap_or_default(),
        prompt_caching_enabled: true,
        ..Default::default()
    };

    let event_channel = format!("biscuitcode:chat-event:{}", req.conversation_id);

    // Start the stream.
    let mut stream = provider
        .chat_stream(messages, vec![], opts)
        .await
        .map_err(|e| e.to_string())?;

    // Collect assistant content so we can persist at stream end.
    let mut assistant_text = String::new();
    let mut final_usage: Option<Usage> = None;

    while let Some(item) = stream.next().await {
        let event = match item {
            Ok(e) => e,
            Err(e) => {
                let _ = app.emit(
                    &event_channel,
                    ChatEventPayload::from_err(e.to_string()),
                );
                break;
            }
        };

        // Accumulate text for persistence.
        if let ChatEvent::TextDelta { ref text } = event {
            assistant_text.push_str(text);
        }
        if let ChatEvent::Done { ref usage, .. } = event {
            final_usage = Some(*usage);
        }

        let _ = app.emit(&event_channel, ChatEventPayload::from_event(&event));

        // On Done, persist the assistant message and break.
        if matches!(event, ChatEvent::Done { .. }) {
            let content = if assistant_text.is_empty() {
                vec![]
            } else {
                vec![ContentBlock::Text { text: assistant_text.clone() }]
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

    Ok(())
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
            ChatEvent::Error { code, message, recoverable } => Self {
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
