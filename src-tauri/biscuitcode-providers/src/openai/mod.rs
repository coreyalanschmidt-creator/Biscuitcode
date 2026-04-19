//! OpenAI provider — Chat Completions API (SSE streaming).
//!
//! Phase 6a deliverable. Wire-format reference:
//!   docs/design/PROVIDER-TRAIT.md § OpenAIProvider.
//!
//! SSE delta → ChatEvent mapping:
//!
//! | OpenAI SSE delta field                           | ChatEvent                        |
//! |--------------------------------------------------|----------------------------------|
//! | `choices[0].delta.content` non-null              | TextDelta { text }               |
//! | `choices[0].delta.tool_calls[i]` first seen      | ToolCallStart { id, name }       |
//! | `choices[0].delta.tool_calls[i].function.arguments` | ToolCallDelta { id, delta }  |
//! | `choices[0].finish_reason == "tool_calls"`       | ToolCallEnd per accumulated entry|
//! | `choices[0].finish_reason` non-null              | Done { stop_reason, usage }      |
//!
//! **Indexed accumulation (PM-01 prevention):**
//! OpenAI sends `tool_calls[i].id` and `tool_calls[i].function.name` only in the
//! FIRST delta for each index. Subsequent deltas for that index carry only
//! `function.arguments` fragments. The accumulator is a `Vec<ToolCallAccum>`
//! indexed by `tool_calls[i].index`, not keyed by id.
//!
//! **Reasoning models:**
//! `gpt-5.4-pro` and similar emit silence until reasoning is complete.
//! No special handling at the provider layer — `ModelInfo::is_reasoning_model` tells
//! the UI to skip the TTFT gate.

use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::r#trait::ModelProvider;
use crate::types::{
    ChatEvent, ChatOptions, ContentBlock, Message, ModelInfo, ProviderError, Role, ToolSpec, Usage,
};

const OPENAI_API_BASE: &str = "https://api.openai.com";
const DEFAULT_MAX_TOKENS: u32 = 8192;

/// In-flight tool-call accumulator (one per `tool_calls[i].index`).
#[derive(Debug, Default)]
struct ToolCallAccum {
    id: String,
    name: String,
    args: String,
}

pub struct OpenAIProvider {
    api_key: String,
    client: reqwest::Client,
    /// Override for tests.
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, OPENAI_API_BASE.into())
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("reqwest client construction is infallible with default config");
        Self { api_key, client, base_url }
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
            mi("gpt-5.4-mini",      "GPT-5.4 mini",                      false, false),
            mi("gpt-5.4",           "GPT-5.4",                            false, false),
            mi("gpt-5.4-nano",      "GPT-5.4 nano",                       false, false),
            mi("gpt-5.4-pro",       "GPT-5.4 pro (reasoning)",            true,  false),
            mi("gpt-5.3-instant",   "GPT-5.3 Instant",                    false, false),
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
        messages: Vec<Message>,
        tools: Vec<ToolSpec>,
        opts: ChatOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatEvent, ProviderError>> + Send>>,
        ProviderError,
    > {
        let body = build_request_body(&messages, &tools, &opts);
        debug!(model = %opts.model, "openai chat_stream request");

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network { reason: e.to_string() })?;

        let status = resp.status();
        if status == 401 {
            return Err(ProviderError::AuthInvalid);
        }
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
            return Err(ProviderError::RateLimited { retry_after_seconds: retry_after });
        }
        if status.is_server_error() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(ProviderError::ServerError { status: status.as_u16(), message: msg });
        }
        if status.is_client_error() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(ProviderError::BadRequest { status: status.as_u16(), message: msg });
        }

        let byte_stream = resp.bytes_stream();
        let sse_stream = byte_stream.eventsource();

        let stream = async_stream::try_stream! {
            let mut sse = std::pin::pin!(sse_stream);

            // Per-index tool-call accumulation (PM-01 fix: index-keyed, not id-keyed).
            let mut accums: HashMap<usize, ToolCallAccum> = HashMap::new();
            let mut usage = Usage::default();

            while let Some(item) = sse.next().await {
                let event = match item {
                    Ok(e) => e,
                    Err(e) => {
                        yield ChatEvent::Error {
                            code: "E005".to_string(),
                            message: format!("SSE read error: {e}"),
                            recoverable: false,
                        };
                        break;
                    }
                };

                // Skip empty data and the OpenAI [DONE] sentinel.
                if event.data.is_empty() || event.data == "[DONE]" {
                    continue;
                }

                let v: Value = match serde_json::from_str(&event.data) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("openai SSE parse error: {e} | data={:?}", event.data);
                        yield ChatEvent::Error {
                            code: "E005".to_string(),
                            message: format!("SSE parse: {e}"),
                            recoverable: true,
                        };
                        continue;
                    }
                };

                // Usage may arrive on the final data chunk (stream_options.include_usage).
                if let Some(u) = v.get("usage").and_then(|u| u.as_object()) {
                    usage.input_tokens  = u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
                    usage.output_tokens = u.get("completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
                }

                let choices = match v["choices"].as_array() {
                    Some(c) if !c.is_empty() => c,
                    _ => continue,
                };
                let choice = &choices[0];
                let delta = &choice["delta"];

                // Text content delta.
                if let Some(text) = delta["content"].as_str() {
                    if !text.is_empty() {
                        yield ChatEvent::TextDelta { text: text.to_string() };
                    }
                }

                // Tool-call deltas (PM-01 fix: accumulate by index).
                if let Some(tcs) = delta["tool_calls"].as_array() {
                    for tc_delta in tcs {
                        let idx = tc_delta["index"].as_u64().unwrap_or(0) as usize;
                        let entry = accums.entry(idx).or_insert_with(ToolCallAccum::default);

                        // id and name only arrive on the first delta for each index.
                        if let Some(id) = tc_delta["id"].as_str() {
                            if entry.id.is_empty() {
                                entry.id = id.to_string();
                            }
                        }
                        if let Some(name) = tc_delta["function"]["name"].as_str() {
                            if entry.name.is_empty() {
                                entry.name = name.to_string();
                                // Emit ToolCallStart once, when we first know the name.
                                yield ChatEvent::ToolCallStart {
                                    id: entry.id.clone(),
                                    name: entry.name.clone(),
                                };
                            }
                        }

                        // Arguments fragment.
                        if let Some(args_frag) = tc_delta["function"]["arguments"].as_str() {
                            if !args_frag.is_empty() {
                                entry.args.push_str(args_frag);
                                yield ChatEvent::ToolCallDelta {
                                    id: entry.id.clone(),
                                    args_delta: args_frag.to_string(),
                                };
                            }
                        }
                    }
                }

                // Finish reason — emit ToolCallEnd for each accumulated tool, then Done.
                if let Some(reason) = choice["finish_reason"].as_str() {
                    let stop_reason = normalize_stop_reason(reason);
                    if reason == "tool_calls" {
                        // Emit in index order.
                        let mut entries: Vec<(usize, ToolCallAccum)> = accums.drain().collect();
                        entries.sort_by_key(|(i, _)| *i);
                        for (_, acc) in entries {
                            yield ChatEvent::ToolCallEnd {
                                id: acc.id,
                                args_json: acc.args,
                            };
                        }
                    }
                    yield ChatEvent::Done { stop_reason, usage };
                    break;
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

/// Build the Chat Completions request body.
pub(crate) fn build_request_body(
    messages: &[Message],
    tools: &[ToolSpec],
    opts: &ChatOptions,
) -> Value {
    let max_tokens = opts.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

    // System prompt is the first element of the messages array in OpenAI's format.
    let mut msgs: Vec<Value> = Vec::new();
    if !opts.system.is_empty() {
        msgs.push(json!({ "role": "system", "content": opts.system }));
    }

    for m in messages.iter().filter(|m| m.role != Role::System) {
        msgs.push(encode_message(m));
    }

    let mut body = json!({
        "model": opts.model,
        "max_tokens": max_tokens,
        "stream": true,
        // Ask OpenAI to include token usage on the last data chunk.
        "stream_options": { "include_usage": true },
        "messages": msgs,
    });

    if !tools.is_empty() {
        let tools_arr: Vec<Value> = tools.iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })
        }).collect();
        body["tools"] = json!(tools_arr);
        body["tool_choice"] = json!("auto");
    }

    // Sampling params.
    if let Some(t) = opts.temperature {
        body["temperature"] = json!(t);
    }
    if let Some(p) = opts.top_p {
        body["top_p"] = json!(p);
    }

    // reasoning_effort → OpenAI's `reasoning_effort` field (for o-class / reasoning models).
    if let Some(re) = &opts.reasoning_effort {
        let effort_str = match re {
            crate::types::ReasoningEffort::Low    => "low",
            crate::types::ReasoningEffort::Medium => "medium",
            crate::types::ReasoningEffort::High   => "high",
        };
        body["reasoning_effort"] = json!(effort_str);
    }

    body
}

/// Encode an internal Message to OpenAI's wire format.
fn encode_message(msg: &Message) -> Value {
    match msg.role {
        Role::Tool => {
            // OpenAI tool results are individual messages with role=tool.
            // The executor appends one result message per call.
            let tr = msg.tool_results.first();
            let content = tr.map(|r| r.result.as_str()).unwrap_or("");
            let tool_call_id = tr.map(|r| r.tool_call_id.as_str()).unwrap_or("");
            json!({
                "role": "tool",
                "tool_call_id": tool_call_id,
                "content": content,
            })
        }
        Role::Assistant => {
            let mut obj = json!({ "role": "assistant" });

            let text: String = msg.content.iter().filter_map(|b| {
                if let ContentBlock::Text { text } = b { Some(text.as_str()) } else { None }
            }).collect::<Vec<_>>().join("");

            if !text.is_empty() {
                obj["content"] = json!(text);
            }

            if !msg.tool_calls.is_empty() {
                let tcs: Vec<Value> = msg.tool_calls.iter().map(|tc| {
                    let args: Value = serde_json::from_str(&tc.args_json).unwrap_or(Value::Null);
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": serde_json::to_string(&args).unwrap_or_default(),
                        }
                    })
                }).collect();
                obj["tool_calls"] = json!(tcs);
            }
            obj
        }
        Role::User | Role::System => {
            let text: String = msg.content.iter().filter_map(|b| {
                if let ContentBlock::Text { text } = b { Some(text.as_str()) } else { None }
            }).collect::<Vec<_>>().join("");
            let role = match msg.role { Role::System => "system", _ => "user" };
            json!({ "role": role, "content": text })
        }
    }
}

fn normalize_stop_reason(reason: &str) -> String {
    match reason {
        "stop"       => "end_turn".to_string(),
        "tool_calls" => "tool_use".to_string(),
        "length"     => "max_tokens".to_string(),
        other        => other.to_string(),
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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

    /// Verify request body shape: tools array, stream:true, messages order.
    #[test]
    fn build_request_body_with_tools() {
        let tools = vec![ToolSpec {
            name: "read_file".into(),
            description: "Reads a file".into(),
            input_schema: json!({ "type": "object", "properties": { "path": { "type": "string" } } }),
        }];
        let opts = ChatOptions {
            model: "gpt-5.4-mini".into(),
            system: "You are helpful.".into(),
            ..Default::default()
        };
        let body = build_request_body(&[], &tools, &opts);
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["function"]["name"], "read_file");
        assert_eq!(body["tool_choice"], "auto");
    }

    /// No tools field when tools is empty.
    #[test]
    fn build_request_body_no_tools_omits_tools_key() {
        let opts = ChatOptions { model: "gpt-5.4-mini".into(), ..Default::default() };
        let body = build_request_body(&[], &[], &opts);
        assert!(body.get("tools").is_none());
        assert!(body.get("tool_choice").is_none());
    }

    /// PM-01 falsification: accumulator keyed by index, name from first delta only.
    #[tokio::test]
    async fn sse_two_tool_calls_index_accumulation() {
        let server = MockServer::start().await;

        // Two tool calls; each name arrives ONLY in the first delta for that index.
        let sse_body = concat!(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{",
            "\"tool_calls\":[",
            "{\"index\":0,\"id\":\"tc_A\",\"type\":\"function\",\"function\":{\"name\":\"read_file\",\"arguments\":\"\"}}",
            ",{\"index\":1,\"id\":\"tc_B\",\"type\":\"function\",\"function\":{\"name\":\"search_code\",\"arguments\":\"\"}}",
            "]},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{",
            "\"tool_calls\":[",
            "{\"index\":0,\"function\":{\"arguments\":\"{\\\"path\\\":\\\"foo.ts\\\"}\"}},",
            "{\"index\":1,\"function\":{\"arguments\":\"{\\\"query\\\":\\\"TODO\\\"}\"}}",
            "]},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}],",
            "\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20}}\n\n",
            "data: [DONE]\n\n",
        );

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_raw(sse_body, "text/event-stream"),
            )
            .mount(&server)
            .await;

        let p = OpenAIProvider::with_base_url("test-key".into(), server.uri());
        let opts = ChatOptions { model: "gpt-5.4-mini".into(), ..Default::default() };
        let mut stream = p.chat_stream(vec![], vec![], opts).await.unwrap();

        let mut events = Vec::new();
        while let Some(ev) = stream.next().await {
            events.push(ev.unwrap());
        }

        let start_a = events.iter().find(|e| matches!(e, ChatEvent::ToolCallStart { id, .. } if id == "tc_A"));
        assert!(start_a.is_some(), "expected ToolCallStart for tc_A; got {events:?}");
        let start_b = events.iter().find(|e| matches!(e, ChatEvent::ToolCallStart { id, .. } if id == "tc_B"));
        assert!(start_b.is_some(), "expected ToolCallStart for tc_B; got {events:?}");

        // Names must be populated (PM-01 check).
        if let Some(ChatEvent::ToolCallStart { name, .. }) = start_a {
            assert_eq!(name, "read_file");
        }
        if let Some(ChatEvent::ToolCallStart { name, .. }) = start_b {
            assert_eq!(name, "search_code");
        }

        let end_a = events.iter().find(|e| matches!(e, ChatEvent::ToolCallEnd { id, args_json } if id == "tc_A" && args_json.contains("foo.ts")));
        assert!(end_a.is_some(), "expected ToolCallEnd for tc_A; got {events:?}");
        let end_b = events.iter().find(|e| matches!(e, ChatEvent::ToolCallEnd { id, args_json } if id == "tc_B" && args_json.contains("TODO")));
        assert!(end_b.is_some(), "expected ToolCallEnd for tc_B; got {events:?}");

        let done = events.iter().find(|e| matches!(e, ChatEvent::Done { stop_reason, .. } if stop_reason == "tool_use"));
        assert!(done.is_some(), "expected Done with stop_reason=tool_use; got {events:?}");
    }

    /// Text-only SSE (no tool calls).
    #[tokio::test]
    async fn sse_text_only_stream() {
        let server = MockServer::start().await;

        let sse_body = concat!(
            "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
            "\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2}}\n\n",
            "data: [DONE]\n\n",
        );

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_raw(sse_body, "text/event-stream"),
            )
            .mount(&server)
            .await;

        let p = OpenAIProvider::with_base_url("test-key".into(), server.uri());
        let opts = ChatOptions { model: "gpt-5.4-mini".into(), ..Default::default() };
        let mut stream = p.chat_stream(vec![], vec![], opts).await.unwrap();

        let mut text = String::new();
        let mut done_seen = false;
        while let Some(ev) = stream.next().await {
            match ev.unwrap() {
                ChatEvent::TextDelta { text: t } => text.push_str(&t),
                ChatEvent::Done { stop_reason, .. } => {
                    assert_eq!(stop_reason, "end_turn");
                    done_seen = true;
                }
                _ => {}
            }
        }
        assert_eq!(text, "Hello world");
        assert!(done_seen);
    }

    /// 401 → AuthInvalid.
    #[tokio::test]
    async fn status_401_returns_auth_invalid() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let p = OpenAIProvider::with_base_url("bad-key".into(), server.uri());
        let opts = ChatOptions { model: "gpt-5.4-mini".into(), ..Default::default() };
        let result = p.chat_stream(vec![], vec![], opts).await;
        assert!(result.is_err(), "expected Err from 401 response");
        let err = result.err().unwrap();
        assert!(matches!(err, ProviderError::AuthInvalid));
    }

    /// stop_reason normalization.
    #[test]
    fn normalize_stop_reason_mapping() {
        assert_eq!(normalize_stop_reason("stop"),       "end_turn");
        assert_eq!(normalize_stop_reason("tool_calls"), "tool_use");
        assert_eq!(normalize_stop_reason("length"),     "max_tokens");
        assert_eq!(normalize_stop_reason("content_filter"), "content_filter");
    }
}
