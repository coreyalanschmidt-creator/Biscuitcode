//! Anthropic provider — `claude-opus-4-7` etc.
//!
//! Wire-format reference: `docs/design/PROVIDER-TRAIT.md` § AnthropicProvider.
//!
//! Phase 5 deliverable. This module is a SKELETON pre-staged from Windows;
//! the WSL2 coder fills in the SSE consumer + reqwest call body and runs
//! the unit tests. The trait surface and the model list are correct as-is.
//!
//! Critical gotchas (must be in the unit tests before the impl is shipped):
//!  - `claude-opus-4-7` rejects `temperature` / `top_p` / `top_k` (HTTP 400).
//!    The impl must omit them from the request body; test
//!    `requests_strip_sampling_for_opus_47` asserts.
//!  - Prompt caching default-on: `cache_control: {"type": "ephemeral"}`
//!    on system + tool definitions when `opts.prompt_caching_enabled`.
//!  - `input_json_delta` arrives as partial strings; full input is only
//!    safe to parse at `content_block_stop`.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::r#trait::ModelProvider;
use crate::types::{ChatEvent, ChatOptions, Message, ModelInfo, ProviderError, ToolSpec};

/// Anthropic Messages API client.
///
/// Construct with [`AnthropicProvider::new`] passing the API key (loaded
/// from libsecret in the calling layer — never plaintext).
pub struct AnthropicProvider {
    api_key: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// Construct with an API key. The key is held in memory for the life
    /// of the process — never serialized, never logged, never written to
    /// disk. Caller is responsible for fetching it from libsecret.
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("reqwest client construction is infallible with default config");
        Self { api_key, client }
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    fn id(&self) -> &'static str {
        "anthropic"
    }

    fn display_name(&self) -> &'static str {
        "Anthropic"
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }

    fn supports_thinking(&self) -> bool {
        true
    }

    fn supports_prompt_caching(&self) -> bool {
        true
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        // Anthropic's `/v1/models` endpoint returns the live list. For v1
        // we hard-code the curated set documented in the vision; the impl
        // can swap to a live fetch in v1.1 once we want auto-discovery of
        // new model IDs.
        Ok(vec![
            ModelInfo {
                id: "claude-opus-4-7".to_string(),
                display_name: "Claude Opus 4.7".to_string(),
                supports_tools: true,
                supports_vision: true,
                is_reasoning_model: false,
                legacy: false,
            },
            ModelInfo {
                id: "claude-sonnet-4-6".to_string(),
                display_name: "Claude Sonnet 4.6".to_string(),
                supports_tools: true,
                supports_vision: true,
                is_reasoning_model: false,
                legacy: false,
            },
            ModelInfo {
                id: "claude-haiku-4-5-20251001".to_string(),
                display_name: "Claude Haiku 4.5".to_string(),
                supports_tools: true,
                supports_vision: true,
                is_reasoning_model: false,
                legacy: false,
            },
            ModelInfo {
                id: "claude-opus-4-6".to_string(),
                display_name: "Claude Opus 4.6 (legacy)".to_string(),
                supports_tools: true,
                supports_vision: true,
                is_reasoning_model: false,
                legacy: true,
            },
        ])
    }

    async fn chat_stream(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSpec>,
        opts: ChatOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatEvent, ProviderError>> + Send>>,
        ProviderError,
    > {
        // Sanity guard so the test suite catches the gotcha even before
        // the request is built.
        if model_strips_sampling(&opts.model)
            && (opts.temperature.is_some() || opts.top_p.is_some() || opts.top_k.is_some())
        {
            // Phase 5 coder: this branch should NEVER be hit at runtime
            // because the request builder strips these fields BEFORE
            // sending. Keep the guard as a defense-in-depth assertion.
            tracing::warn!(
                model = %opts.model,
                "sampling fields ignored for {} (Opus 4.7+ rejects them)",
                opts.model,
            );
        }

        // ---- Phase 5 coder fills in below ----
        //
        // 1. Build request body. Use serde_json::json! macro. Apply
        //    `cache_control: {"type": "ephemeral"}` to system prompt
        //    when opts.prompt_caching_enabled. Strip sampling fields
        //    when model_strips_sampling(opts.model).
        //
        // 2. POST to https://api.anthropic.com/v1/messages with headers:
        //      x-api-key: <self.api_key>
        //      anthropic-version: 2023-06-01    (or current pin)
        //      content-type: application/json
        //      accept: text/event-stream
        //
        // 3. Map HTTP errors to ProviderError variants:
        //      401      -> AuthInvalid
        //      429      -> RateLimited { retry_after_seconds: parse retry-after }
        //      4xx other-> BadRequest
        //      5xx      -> ServerError
        //      io error -> Network
        //
        // 4. Parse SSE stream via eventsource-stream and translate per
        //    the table in docs/design/PROVIDER-TRAIT.md § AnthropicProvider.
        //
        // 5. Return the Pin<Box<dyn Stream<...>>>. async-stream::stream! is
        //    a clean way to build it.

        Err(ProviderError::Other(
            "AnthropicProvider::chat_stream not yet implemented (Phase 5)".to_string(),
        ))
    }
}

/// True if this model rejects `temperature`/`top_p`/`top_k` and the impl
/// must omit them from the request body. Currently: every Opus 4.7+ model.
pub(crate) fn model_strips_sampling(model: &str) -> bool {
    model.starts_with("claude-opus-4-7") || model.starts_with("claude-opus-5")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_4_7_strips_sampling() {
        assert!(model_strips_sampling("claude-opus-4-7"));
        assert!(model_strips_sampling("claude-opus-4-7-20260101"));
    }

    #[test]
    fn opus_4_6_does_not_strip_sampling() {
        assert!(!model_strips_sampling("claude-opus-4-6"));
        assert!(!model_strips_sampling("claude-sonnet-4-6"));
        assert!(!model_strips_sampling("claude-haiku-4-5"));
    }

    #[tokio::test]
    async fn list_models_includes_default_opus() {
        let p = AnthropicProvider::new("dummy".into());
        let models = p.list_models().await.expect("hard-coded list cannot fail");
        assert!(models.iter().any(|m| m.id == "claude-opus-4-7" && !m.legacy));
        assert!(models.iter().any(|m| m.id == "claude-opus-4-6" && m.legacy));
    }

    // Phase 5 coder: add `requests_strip_sampling_for_opus_47` here as a
    // wiremock-based integration test once chat_stream is wired. Asserts
    // the actual JSON body sent to wiremock contains no `temperature`,
    // `top_p`, or `top_k` keys when model is claude-opus-4-7.
}
