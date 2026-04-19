//! Ollama provider — local models (Gemma 4 primary, Gemma 3 fallback).
//!
//! Phase 6a deliverable. Wire-format reference:
//!   docs/design/PROVIDER-TRAIT.md § OllamaProvider.
//!
//! Critical implementation notes:
//!  - Endpoint is `http://localhost:11434/api/chat` (NDJSON, not SSE).
//!  - **Gemma 4 is the primary default ladder.** Gemma 3 is a documented
//!    fallback only when the user's Ollama version (< 0.20.0) doesn't
//!    recognize `gemma4:*` tags. Verified tags (against
//!    https://ollama.com/library/gemma4 on 2026-04-18):
//!      gemma4:e2b   2.3B effective, 7.2GB, 128K
//!      gemma4:e4b   4.5B effective, 9.6GB, 128K (also gemma4:latest)
//!      gemma4:26b   MoE 25.2B/3.8B active, 18GB, 256K
//!      gemma4:31b   30.7B dense, 20GB, 256K
//!  - All Gemma 4 variants natively support function calling. The
//!    XML-tag fallback below is for Gemma 3 community fine-tunes ONLY.
//!  - Empty `tool_calls` + `message.content` containing `<tool_call>...</tool_call>`
//!    -> regex-extract and synthesize ToolCallStart/End. Common with
//!    Gemma 3 community fine-tunes.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::r#trait::ModelProvider;
use crate::types::{ChatEvent, ChatOptions, Message, ModelInfo, ProviderError, ToolSpec};

pub struct OllamaProvider {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Defaults to the conventional local endpoint. Override for tests.
    pub fn new() -> Self {
        Self::with_base_url("http://localhost:11434".into())
    }

    pub fn with_base_url(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client construction is infallible with default config");
        Self { base_url, client }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    fn id(&self) -> &'static str { "ollama" }
    fn display_name(&self) -> &'static str { "Ollama" }
    fn supports_tools(&self) -> bool { true } // model-dependent; checked per-model
    fn supports_vision(&self) -> bool { true } // gemma4 multimodal
    fn supports_thinking(&self) -> bool { false }
    fn supports_prompt_caching(&self) -> bool { false }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        // ---- Phase 6a coder fills in ----
        // GET <base_url>/api/tags
        // Translate each entry to ModelInfo. Mark gemma3:* as legacy
        // when gemma4:* is also present (UI hint, picker preference).
        // Mark gemma4:* with supports_vision=true and is_reasoning_model=false.
        // Mark qwen2.5-coder:* with supports_tools=true (proven tool-calling).
        // On daemon-down: return ProviderError::OllamaDaemonDown.
        Err(ProviderError::OllamaDaemonDown {
            endpoint: format!("{}/api/tags", self.base_url),
        })
    }

    async fn chat_stream(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSpec>,
        _opts: ChatOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatEvent, ProviderError>> + Send>>,
        ProviderError,
    > {
        // ---- Phase 6a coder fills in ----
        //
        // 1. POST <base_url>/api/chat with stream: true (NDJSON, not SSE!).
        //    Headers: content-type: application/json. No auth header.
        // 2. Use reqwest::Response::bytes_stream() + a line-buffer adapter
        //    (the chunks may not align to NDJSON line boundaries).
        // 3. For each parsed JSON object:
        //      - "message.content" non-empty -> ChatEvent::TextDelta { text: content }
        //      - "message.tool_calls" non-empty -> emit ToolCallStart + ToolCallEnd
        //        per call (Ollama emits whole calls atomically; no streaming args)
        //      - "done": true -> emit ChatEvent::Done with stop_reason from
        //        "done_reason" and usage from "prompt_eval_count" + "eval_count"
        // 4. **XML-tag fallback** (Gemma 3 community fine-tunes only): if
        //    a chunk has empty tool_calls but message.content matches
        //    `/<tool_call>(.+?)<\/tool_call>/s`, extract the JSON inside
        //    and synthesize a ToolCallStart + ToolCallEnd pair instead of
        //    emitting it as TextDelta. Gemma 4 variants don't need this
        //    (native function calling).

        Err(ProviderError::Other("not yet implemented (Phase 6a)".into()))
    }
}

/// Verified Gemma 4 RAM-tier defaults, mirroring the table in
/// docs/plan.md Phase 6a deliverables.
///
/// Returns the preferred Gemma 4 tag for the given total system RAM (in GB).
/// The Phase 6a coder pairs this with `select_default_model()` which checks
/// availability via `GET /api/tags` and falls back to Gemma 3 + E007 toast.
pub fn gemma4_tag_for_ram_gb(ram_gb: u32) -> &'static str {
    match ram_gb {
        0..=7   => "gemma4:e2b",
        8..=31  => "gemma4:e4b",
        32..=47 => "gemma4:26b",
        _       => "gemma4:31b",
    }
}

/// Gemma 3 fallback ladder for Ollama versions that don't recognize
/// `gemma4:*` tags (< 0.20.0). Only used in the E007 fallback path.
pub fn gemma3_fallback_for_ram_gb(ram_gb: u32) -> &'static str {
    match ram_gb {
        0..=5   => "gemma3:1b",
        6..=11  => "gemma3:4b",
        12..=23 => "gemma3:4b",
        24..=31 => "gemma3:12b",
        _       => "gemma3:27b",
    }
}

/// Agent-mode preferred model when RAM allows — qwen2.5-coder has the
/// most stable tool-calling on Ollama (verified by research-r2).
pub fn agent_mode_preferred(ram_gb: u32) -> Option<&'static str> {
    if ram_gb >= 12 { Some("qwen2.5-coder:7b") } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemma4_tier_table_matches_plan() {
        assert_eq!(gemma4_tag_for_ram_gb(4),  "gemma4:e2b");
        assert_eq!(gemma4_tag_for_ram_gb(8),  "gemma4:e4b");
        assert_eq!(gemma4_tag_for_ram_gb(16), "gemma4:e4b");
        assert_eq!(gemma4_tag_for_ram_gb(32), "gemma4:26b");
        assert_eq!(gemma4_tag_for_ram_gb(64), "gemma4:31b");
    }

    #[test]
    fn gemma3_fallback_keys_match_plan() {
        assert_eq!(gemma3_fallback_for_ram_gb(4),  "gemma3:1b");
        assert_eq!(gemma3_fallback_for_ram_gb(8),  "gemma3:4b");
        assert_eq!(gemma3_fallback_for_ram_gb(16), "gemma3:4b");
        assert_eq!(gemma3_fallback_for_ram_gb(32), "gemma3:27b");
    }

    #[test]
    fn agent_mode_alt_only_when_ram_allows() {
        assert_eq!(agent_mode_preferred(8),  None);
        assert_eq!(agent_mode_preferred(12), Some("qwen2.5-coder:7b"));
        assert_eq!(agent_mode_preferred(64), Some("qwen2.5-coder:7b"));
    }
}
