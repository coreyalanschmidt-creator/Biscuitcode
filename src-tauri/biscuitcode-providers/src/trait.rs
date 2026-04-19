//! The `ModelProvider` trait. See `docs/design/PROVIDER-TRAIT.md`.
//!
//! Every provider impl ships behind this single dyn-compatible trait so
//! the agent loop and chat panel consume a uniform `ChatEvent` stream
//! regardless of which provider was selected.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::types::{ChatEvent, ChatOptions, Message, ModelInfo, ProviderError, ToolSpec};

/// One streaming chat provider.
///
/// Implementations live in `crate::anthropic`, `crate::openai`,
/// `crate::ollama`. Adding a fourth in v1.1 should require ONLY a new
/// module — never a change to this trait.
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Stable identifier. e.g. "anthropic", "openai", "ollama".
    /// Used in capability scopes, settings keys, log targets.
    fn id(&self) -> &'static str;

    /// User-facing display name. e.g. "Anthropic", "OpenAI", "Ollama".
    fn display_name(&self) -> &'static str;

    /// List of models the user can pick. May make a network call (e.g.
    /// Ollama's `GET /api/tags`); failures return Err so the settings
    /// status badge can show red.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError>;

    /// The single streaming endpoint. Returns a `Stream` of `ChatEvent`s
    /// that the agent loop consumes uniformly across providers.
    ///
    /// `tools` is empty for non-agent text-only chat. `opts` carries the
    /// model selection, sampling params, system prompt, etc.
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSpec>,
        opts: ChatOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatEvent, ProviderError>> + Send>>,
        ProviderError,
    >;

    /// Whether the active model can call tools. Frontend grays out the
    /// agent toggle when false.
    fn supports_tools(&self) -> bool {
        true
    }

    /// Whether the active model accepts image inputs.
    fn supports_vision(&self) -> bool {
        false
    }

    /// Whether the provider returns Anthropic-style "thinking" blocks
    /// (or OpenAI reasoning content). Frontend renders a Thinking… UI.
    fn supports_thinking(&self) -> bool {
        false
    }

    /// Whether the provider supports Anthropic-style prompt caching
    /// (`cache_control: ephemeral`). Frontend exposes the toggle only
    /// when this is true.
    fn supports_prompt_caching(&self) -> bool {
        false
    }
}
