//! Shared types for the provider trait.
//!
//! Source of truth for the wire shape between providers and the agent loop.
//! See `docs/design/PROVIDER-TRAIT.md` for the design rationale and per-
//! provider normalization tables.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------- Conversation messages ----------

/// One message in a conversation. Mirrors the DB row shape and the
/// conversation-export schema (see `docs/CONVERSATION-EXPORT-SCHEMA.md`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
    /// Set on assistant messages that emitted tool calls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Set on tool-role messages — one entry per call this responds to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_results: Vec<ToolResult>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    Tool,
    /// System prompt — Anthropic puts this in a separate field but our
    /// internal representation treats it as a special-role message.
    System,
}

/// Alias kept for clarity in some call sites.
pub type MessageRole = Role;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    /// Structured @ mention from the chat input picker.
    Mention { mention_kind: MentionKind, value: serde_json::Value },
    /// Vision input. Only used when the selected model supports vision.
    Image { media_type: String, data_b64: String },
    /// Provider thinking content (Anthropic; OpenAI reasoning models).
    Thinking { text: String },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MentionKind {
    File,
    Folder,
    Selection,
    TerminalOutput,
    Problems,
    GitDiff,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    /// Provider-assigned id; opaque to us.
    pub id: String,
    /// Tool name from the registered ToolSpec.
    pub name: String,
    /// Fully-assembled args JSON (after `ToolCallEnd`).
    pub args_json: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    /// Tool's return value as a string (JSON-encoded if structured).
    pub result: String,
    /// True if the tool result was truncated (e.g. > 256KB).
    #[serde(default)]
    pub truncated: bool,
}

// ---------- Tool registration ----------

/// What a provider sees in its `tools` parameter — enough JSON Schema for
/// the model to emit valid args.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema for the tool's input. Hand-authored — no derive.
    pub input_schema: serde_json::Value,
}

// ---------- Streaming events ----------

/// Normalized streaming event emitted by every provider impl.
///
/// See `docs/design/PROVIDER-TRAIT.md` for the per-provider mapping table.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    /// Text token(s) appended to the assistant message content.
    TextDelta { text: String },

    /// Anthropic "thinking" / OpenAI reasoning content (when surfaced).
    /// Frontend renders in a collapsible "Thinking…" block.
    ThinkingDelta { text: String },

    /// A new tool call has started. Frontend renders the Agent Activity
    /// card on this event (250ms render-gate measured from here).
    ToolCallStart { id: String, name: String },

    /// Argument JSON for an in-flight tool call, partial. Concatenate all
    /// deltas with the same id; do NOT parse mid-stream.
    ToolCallDelta { id: String, args_delta: String },

    /// Tool call complete. `args_json` is fully-assembled and parseable.
    ToolCallEnd { id: String, args_json: String },

    /// Stream finished cleanly.
    Done {
        /// Provider-normalized: "end_turn", "max_tokens", "tool_use",
        /// "stop_sequence", "error".
        stop_reason: String,
        usage: Usage,
    },

    /// Non-fatal error mid-stream. Maps to a catalogue code at the
    /// chat panel boundary.
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Anthropic-specific (prompt caching). None for other providers.
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
}

// ---------- Provider call options ----------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChatOptions {
    /// Model identifier — provider-specific.
    pub model: String,

    /// Max output tokens. None = provider default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Sampling — Anthropic Opus 4.7 ignores these (the impl strips them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// OpenAI reasoning effort — None for non-reasoning providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,

    /// Anthropic prompt-caching default-on. Set false to opt out.
    #[serde(default = "default_true")]
    pub prompt_caching_enabled: bool,

    /// System prompt string. Empty = no system prompt.
    #[serde(default)]
    pub system: String,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Stable identifier the provider accepts. e.g. "claude-opus-4-7".
    pub id: String,
    /// User-facing display name.
    pub display_name: String,
    /// True if the model can call tools.
    pub supports_tools: bool,
    /// True if the model accepts image inputs.
    pub supports_vision: bool,
    /// Hint for the chat-panel "Thinking…" indicator and the Global AC's
    /// TTFT exemption.
    pub is_reasoning_model: bool,
    /// True if marked legacy (still callable, not the default).
    #[serde(default)]
    pub legacy: bool,
}

// ---------- Errors ----------

/// Top-level provider error. Each variant maps to a catalogue code at the
/// chat panel boundary (E004 / E005 / E006 / E013-equivalents).
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderError {
    /// 401 from the provider — bad API key.
    #[error("auth invalid")]
    AuthInvalid,

    /// Network failure (DNS, TLS, timeout, connection refused).
    #[error("network: {reason}")]
    Network { reason: String },

    /// 429 — provider is rate-limiting us. Surface Retry-After in seconds.
    #[error("rate limited; retry after {retry_after_seconds}s")]
    RateLimited { retry_after_seconds: u64 },

    /// 4xx other than 401/429 — bad request, bad model, etc.
    #[error("bad request ({status}): {message}")]
    BadRequest { status: u16, message: String },

    /// 5xx — provider down.
    #[error("server error ({status}): {message}")]
    ServerError { status: u16, message: String },

    /// SSE / NDJSON parse failure mid-stream. Recoverable — caller may
    /// continue consuming the stream.
    #[error("stream parse error: {reason}")]
    ParseError { reason: String },

    /// Local Ollama daemon unreachable (Phase 6a-specific).
    #[error("ollama daemon unreachable at {endpoint}")]
    OllamaDaemonDown { endpoint: String },

    /// Generic catch-all — should be rare, log noisy.
    #[error("{0}")]
    Other(String),
}
