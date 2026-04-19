//! OpenAI provider — Chat Completions API.
//!
//! Phase 6a deliverable. Wire-format reference:
//!   docs/design/PROVIDER-TRAIT.md § OpenAIProvider.
//!
//! Critical implementation notes:
//!  - Per-index `tool_calls[i].index` accumulation is the OpenAI quirk
//!    that the impl must get right. A single delta chunk may contain
//!    partial args for multiple tool calls in the same response.
//!  - Reasoning models (`gpt-5.4-pro`) emit no `delta.content` until
//!    reasoning finishes (3-30s). UI shows `Thinking…`. The TTFT
//!    < 500ms gate does NOT apply to reasoning models — Global AC exempts.
//!  - `gpt-4o` was retired 2026-04-03; default is `gpt-5.4-mini`.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::r#trait::ModelProvider;
use crate::types::{ChatEvent, ChatOptions, Message, ModelInfo, ProviderError, ToolSpec};

pub struct OpenAIProvider {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
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
impl ModelProvider for OpenAIProvider {
    fn id(&self) -> &'static str { "openai" }
    fn display_name(&self) -> &'static str { "OpenAI" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { true }
    fn supports_thinking(&self) -> bool { true } // reasoning models surface this
    fn supports_prompt_caching(&self) -> bool { false } // automatic, not user-controllable

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        // Curated set per the synthesis. Reasoning models flagged so the
        // chat panel exempts them from the TTFT gate.
        Ok(vec![
            mi("gpt-5.4-mini",      "GPT-5.4 mini", false, false),
            mi("gpt-5.4",           "GPT-5.4",       false, false),
            mi("gpt-5.4-nano",      "GPT-5.4 nano",  false, false),
            mi("gpt-5.4-pro",       "GPT-5.4 pro (reasoning)",  true,  false),
            mi("gpt-5.3-instant",   "GPT-5.3 Instant",          false, false),
            // Legacy entries — visible in picker, marked legacy.
            ModelInfo {
                id: "gpt-5.2-thinking".to_string(),
                display_name: "GPT-5.2 Thinking (legacy until 2026-06-05)".to_string(),
                supports_tools: true,
                supports_vision: false,
                is_reasoning_model: true,
                legacy: true,
            },
        ])
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
        // 1. Build request body (chat completions schema). System message
        //    is the first entry of `messages` array.
        // 2. POST https://api.openai.com/v1/chat/completions with
        //    Authorization: Bearer <self.api_key>, stream: true.
        // 3. Map errors per docs/design/PROVIDER-TRAIT.md.
        // 4. Parse SSE deltas. Maintain a HashMap<usize /*index*/, ToolCallAccum>
        //    that tracks each in-flight tool call's id, name, and concatenated
        //    args delta strings. Emit ChatEvent::ToolCallStart on first
        //    appearance of an index, ToolCallDelta for each subsequent
        //    args chunk, and ToolCallEnd for every accumulated entry on
        //    finish_reason == "tool_calls".
        // 5. Reasoning models: no special handling at the provider layer —
        //    the model just stays silent, then emits content_delta. The
        //    UI's TTFT timer is told to skip the assertion for reasoning
        //    models via ModelInfo::is_reasoning_model.
        Err(ProviderError::Other("not yet implemented (Phase 6a)".into()))
    }
}

fn mi(id: &str, display: &str, reasoning: bool, vision: bool) -> ModelInfo {
    ModelInfo {
        id: id.to_string(),
        display_name: display.to_string(),
        supports_tools: true,
        supports_vision: vision,
        is_reasoning_model: reasoning,
        legacy: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_models_default_is_gpt_5_4_mini_and_excludes_4o() {
        let p = OpenAIProvider::new("dummy".into());
        let ms = p.list_models().await.unwrap();
        // The first entry is the conventional default in the picker.
        assert_eq!(ms[0].id, "gpt-5.4-mini");
        // gpt-4o was retired 2026-04-03 and must NOT appear.
        assert!(!ms.iter().any(|m| m.id == "gpt-4o"));
        // Reasoning model is flagged.
        let pro = ms.iter().find(|m| m.id == "gpt-5.4-pro").unwrap();
        assert!(pro.is_reasoning_model);
    }
}
