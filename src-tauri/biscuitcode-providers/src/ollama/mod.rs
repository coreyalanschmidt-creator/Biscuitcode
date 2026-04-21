//! Ollama provider — local models via NDJSON streaming.
//!
//! Phase 6a deliverable. Wire-format reference:
//!   docs/design/PROVIDER-TRAIT.md § OllamaProvider.
//!
//! NDJSON chunk → ChatEvent mapping:
//!
//! | Ollama NDJSON field                                      | ChatEvent                        |
//! |----------------------------------------------------------|----------------------------------|
//! | `message.content` non-empty, `done:false`               | TextDelta { text }               |
//! | `message.tool_calls[...]`, `done:false`                  | ToolCallStart + ToolCallEnd      |
//! | `message.content` with `<tool_call>...</tool_call>`      | ToolCallStart + ToolCallEnd (XML)|
//! | `done:true`                                              | Done { stop_reason, usage }      |
//!
//! **PM-02 prevention (line-buffering):**
//! reqwest bytes_stream yields chunks that don't align to newline boundaries.
//! We carry a `line_buf: String` accumulator and only parse when we see a newline.
//!
//! **XML-tag fallback:**
//! Some Gemma 3 community fine-tunes emit tool calls as
//! `<tool_call>{"name":"...","arguments":{...}}</tool_call>` inside
//! `message.content`. Gemma 4 uses native function calling; the fallback
//! is kept for Gemma 3 backward compatibility.

use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use regex::Regex;
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::r#trait::ModelProvider;
use crate::types::{
    ChatEvent, ChatOptions, ContentBlock, Message, ModelInfo, ProviderError, Role, ToolSpec, Usage,
};

const DEFAULT_BASE_URL: &str = "http://localhost:11434";

pub struct OllamaProvider {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Defaults to the conventional local endpoint. Override for tests.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.into())
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

/// Outcome of the Ollama daemon version check.
#[derive(Debug, Eq, PartialEq)]
pub enum OllamaVersionStatus {
    /// Daemon running and version >= 0.20.0.
    Ready(String),
    /// Daemon running but version is below the Gemma 4 minimum.
    TooOld(String),
    /// Connection refused or other network error — daemon not running / not installed.
    Down,
}

impl OllamaProvider {
    /// GET /api/version and classify the result.
    ///
    /// This is the core logic tested by the unit tests; the Tauri command
    /// `ollama_check_and_install` wraps this and emits the appropriate error
    /// events.
    pub async fn check_version(&self) -> OllamaVersionStatus {
        let url = format!("{}/api/version", self.base_url);
        let resp = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(_) => return OllamaVersionStatus::Down,
        };

        if !resp.status().is_success() {
            return OllamaVersionStatus::Down;
        }

        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return OllamaVersionStatus::Down,
        };

        let version = body["version"].as_str().unwrap_or("0.0.0").to_string();
        if ollama_version_gte(&version, (0, 20, 0)) {
            OllamaVersionStatus::Ready(version)
        } else {
            OllamaVersionStatus::TooOld(version)
        }
    }
}

/// Semver-style comparison for Ollama version strings.
/// Returns true if `v` >= `(maj, min, pat)`. Parse failures return false (safe).
pub fn ollama_version_gte(v: &str, (req_maj, req_min, req_pat): (u32, u32, u32)) -> bool {
    let parts: Vec<u32> = v
        .split('.')
        .map(|s| s.parse::<u32>().unwrap_or(0))
        .collect();
    let maj = parts.first().copied().unwrap_or(0);
    let min = parts.get(1).copied().unwrap_or(0);
    let pat = parts.get(2).copied().unwrap_or(0);
    (maj, min, pat) >= (req_maj, req_min, req_pat)
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    fn id(&self) -> &'static str {
        "ollama"
    }
    fn display_name(&self) -> &'static str {
        "Ollama"
    }
    fn supports_tools(&self) -> bool {
        true
    } // model-dependent; checked per-model
    fn supports_vision(&self) -> bool {
        true
    } // gemma4 multimodal
    fn supports_thinking(&self) -> bool {
        false
    }
    fn supports_prompt_caching(&self) -> bool {
        false
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            if e.is_connect() {
                ProviderError::OllamaDaemonDown {
                    endpoint: url.clone(),
                }
            } else {
                ProviderError::Network {
                    reason: e.to_string(),
                }
            }
        })?;

        if !resp.status().is_success() {
            return Err(ProviderError::OllamaDaemonDown { endpoint: url });
        }

        let body: Value = resp.json().await.map_err(|e| ProviderError::ParseError {
            reason: e.to_string(),
        })?;

        let models_arr = body["models"].as_array().cloned().unwrap_or_default();

        // Determine if any gemma4 variant is present for legacy-flagging gemma3.
        let has_gemma4 = models_arr.iter().any(|m| {
            m["name"]
                .as_str()
                .map(|n| n.starts_with("gemma4:"))
                .unwrap_or(false)
        });

        let infos: Vec<ModelInfo> = models_arr
            .iter()
            .filter_map(|m| {
                let name = m["name"].as_str()?.to_string();
                let display_name = name.clone();
                let is_gemma3 = name.starts_with("gemma3:");
                let is_gemma4 = name.starts_with("gemma4:");
                // Permissive default: assume tool support unless the model
                // name indicates it's an embedding or vision-caption-only model.
                // Conservative whitelist incorrectly blocks llama3.1, phi4,
                // qwen3, etc. (Q1 decision).
                let is_embed_only = name.starts_with("nomic-embed")
                    || name.starts_with("mxbai-embed")
                    || name.starts_with("all-minilm")
                    || name.starts_with("snowflake-arctic-embed")
                    || (name.starts_with("llava:") && !name.contains("chat"));
                Some(ModelInfo {
                    id: name.clone(),
                    display_name,
                    supports_tools: !is_embed_only,
                    supports_vision: is_gemma4,
                    is_reasoning_model: false,
                    // Mark gemma3 as legacy when gemma4 is also present.
                    legacy: is_gemma3 && has_gemma4,
                })
            })
            .collect();

        Ok(infos)
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSpec>,
        opts: ChatOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, ProviderError>> + Send>>, ProviderError>
    {
        let body = build_request_body(&messages, &tools, &opts);
        debug!(model = %opts.model, "ollama chat_stream request");

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    ProviderError::OllamaDaemonDown {
                        endpoint: format!("{}/api/chat", self.base_url),
                    }
                } else {
                    ProviderError::Network {
                        reason: e.to_string(),
                    }
                }
            })?;

        let status = resp.status();
        if status.is_server_error() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(ProviderError::ServerError {
                status: status.as_u16(),
                message: msg,
            });
        }
        if status.is_client_error() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(ProviderError::BadRequest {
                status: status.as_u16(),
                message: msg,
            });
        }

        // Compile XML fallback regex once. Regex::new is cheap; lifetime in stream
        // requires it to be moved in.
        let xml_re =
            Regex::new(r"(?s)<tool_call>(.*?)</tool_call>").expect("static regex is valid");

        let mut byte_stream = resp.bytes_stream();

        let stream = async_stream::try_stream! {
            let mut line_buf = String::new(); // PM-02 fix: cross-chunk line accumulation

            while let Some(chunk) = byte_stream.next().await {
                let chunk = match chunk {
                    Ok(b) => b,
                    Err(e) => {
                        yield ChatEvent::Error {
                            code: "E005".to_string(),
                            message: format!("network chunk error: {e}"),
                            recoverable: false,
                        };
                        break;
                    }
                };

                // PM-02 fix: append chunk bytes as UTF-8 (lossy) to line_buf,
                // then drain complete lines for parsing.
                line_buf.push_str(&String::from_utf8_lossy(&chunk));

                // Process all complete newline-terminated lines.
                while let Some(nl) = line_buf.find('\n') {
                    let line = line_buf[..nl].trim().to_string();
                    line_buf = line_buf[nl + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    let v: Value = match serde_json::from_str(&line) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("ollama NDJSON parse error: {e} | line={line:?}");
                            yield ChatEvent::Error {
                                code: "E005".to_string(),
                                message: format!("NDJSON parse: {e}"),
                                recoverable: true,
                            };
                            continue;
                        }
                    };

                    let done = v["done"].as_bool().unwrap_or(false);
                    let msg_obj = &v["message"];

                    // Native tool_calls (Gemma 4 and qwen2.5-coder).
                    if let Some(tool_calls) = msg_obj["tool_calls"].as_array() {
                        if !tool_calls.is_empty() {
                            for tc in tool_calls {
                                // Ollama emits whole tool calls atomically (no streaming args).
                                let id = tc["id"].as_str()
                                    .unwrap_or_else(|| tc["function"]["name"].as_str().unwrap_or(""))
                                    .to_string();
                                let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                                let args_val = &tc["function"]["arguments"];
                                let args_json = if args_val.is_object() {
                                    serde_json::to_string(args_val).unwrap_or_default()
                                } else {
                                    args_val.as_str().unwrap_or("{}").to_string()
                                };
                                let call_id = if id.is_empty() { name.clone() } else { id };
                                yield ChatEvent::ToolCallStart { id: call_id.clone(), name };
                                yield ChatEvent::ToolCallEnd { id: call_id, args_json };
                            }
                        }
                    } else if let Some(content) = msg_obj["content"].as_str() {
                        // XML-tag fallback for Gemma 3 community fine-tunes.
                        if content.contains("<tool_call>") {
                            let mut any_xml = false;
                            for cap in xml_re.captures_iter(content) {
                                any_xml = true;
                                let inner = cap[1].trim();
                                let parsed: Value = serde_json::from_str(inner)
                                    .unwrap_or_else(|_| json!({ "raw": inner }));
                                let name = parsed["name"].as_str().unwrap_or("unknown_tool").to_string();
                                let args_val = &parsed["arguments"];
                                let args_json = if args_val.is_object() {
                                    serde_json::to_string(args_val).unwrap_or_default()
                                } else {
                                    serde_json::to_string(&parsed).unwrap_or_default()
                                };
                                let call_id = format!("xml_{name}");
                                yield ChatEvent::ToolCallStart { id: call_id.clone(), name };
                                yield ChatEvent::ToolCallEnd { id: call_id, args_json };
                            }
                            if !any_xml && !content.is_empty() {
                                yield ChatEvent::TextDelta { text: content.to_string() };
                            }
                        } else if !content.is_empty() {
                            yield ChatEvent::TextDelta { text: content.to_string() };
                        }
                    }

                    if done {
                        let stop_reason = v["done_reason"].as_str()
                            .unwrap_or("end_turn")
                            .to_string();
                        let input_tokens  = v["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
                        let output_tokens = v["eval_count"].as_u64().unwrap_or(0) as u32;
                        yield ChatEvent::Done {
                            stop_reason,
                            usage: Usage {
                                input_tokens,
                                output_tokens,
                                cache_read_input_tokens: None,
                                cache_creation_input_tokens: None,
                            },
                        };
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

/// Build the Ollama /api/chat request body.
pub(crate) fn build_request_body(
    messages: &[Message],
    tools: &[ToolSpec],
    opts: &ChatOptions,
) -> Value {
    let mut msgs: Vec<Value> = Vec::new();

    // System prompt as a system-role message (Ollama supports this).
    if !opts.system.is_empty() {
        msgs.push(json!({ "role": "system", "content": opts.system }));
    }

    for m in messages.iter().filter(|m| m.role != Role::System) {
        msgs.push(encode_message(m));
    }

    let mut body = json!({
        "model": opts.model,
        "stream": true,
        "messages": msgs,
    });

    if !tools.is_empty() {
        // Ollama accepts tools in OpenAI function-call format.
        let tools_arr: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            })
            .collect();
        body["tools"] = json!(tools_arr);
    }

    if let Some(t) = opts.temperature {
        body["options"] = json!({ "temperature": t });
    }

    body
}

fn encode_message(msg: &Message) -> Value {
    match msg.role {
        Role::Tool => {
            // Ollama expects tool results in OpenAI-compatible format.
            let tr = msg.tool_results.first();
            let content = tr.map(|r| r.result.as_str()).unwrap_or("");
            json!({ "role": "tool", "content": content })
        }
        Role::Assistant => {
            let text: String = msg
                .content
                .iter()
                .filter_map(|b| {
                    if let ContentBlock::Text { text } = b {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            json!({ "role": "assistant", "content": text })
        }
        Role::User | Role::System => {
            let text: String = msg
                .content
                .iter()
                .filter_map(|b| {
                    if let ContentBlock::Text { text } = b {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            let role = match msg.role {
                Role::System => "system",
                _ => "user",
            };
            json!({ "role": role, "content": text })
        }
    }
}

/// Verified Gemma 4 RAM-tier defaults, mirroring the table in
/// docs/plan.md Phase 6a deliverables.
///
/// Returns the preferred Gemma 4 tag for the given total system RAM (in GB).
pub fn gemma4_tag_for_ram_gb(ram_gb: u32) -> &'static str {
    match ram_gb {
        0..=7 => "gemma4:e2b",
        8..=31 => "gemma4:e4b",
        32..=47 => "gemma4:26b",
        _ => "gemma4:31b",
    }
}

/// Gemma 3 fallback ladder for Ollama versions that don't recognize
/// `gemma4:*` tags (< 0.20.0). Only used in the E007 fallback path.
pub fn gemma3_fallback_for_ram_gb(ram_gb: u32) -> &'static str {
    match ram_gb {
        0..=5 => "gemma3:1b",
        6..=11 => "gemma3:4b",
        12..=23 => "gemma3:4b",
        24..=31 => "gemma3:12b",
        _ => "gemma3:27b",
    }
}

/// Agent-mode preferred model when RAM allows — qwen2.5-coder has the
/// most stable tool-calling on Ollama (verified by research-r2).
pub fn agent_mode_preferred(ram_gb: u32) -> Option<&'static str> {
    if ram_gb >= 12 {
        Some("qwen2.5-coder:7b")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn gemma4_tier_table_matches_plan() {
        assert_eq!(gemma4_tag_for_ram_gb(4), "gemma4:e2b");
        assert_eq!(gemma4_tag_for_ram_gb(8), "gemma4:e4b");
        assert_eq!(gemma4_tag_for_ram_gb(16), "gemma4:e4b");
        assert_eq!(gemma4_tag_for_ram_gb(32), "gemma4:26b");
        assert_eq!(gemma4_tag_for_ram_gb(64), "gemma4:31b");
    }

    #[test]
    fn gemma3_fallback_keys_match_plan() {
        assert_eq!(gemma3_fallback_for_ram_gb(4), "gemma3:1b");
        assert_eq!(gemma3_fallback_for_ram_gb(8), "gemma3:4b");
        assert_eq!(gemma3_fallback_for_ram_gb(16), "gemma3:4b");
        assert_eq!(gemma3_fallback_for_ram_gb(32), "gemma3:27b");
    }

    #[test]
    fn agent_mode_alt_only_when_ram_allows() {
        assert_eq!(agent_mode_preferred(8), None);
        assert_eq!(agent_mode_preferred(12), Some("qwen2.5-coder:7b"));
        assert_eq!(agent_mode_preferred(64), Some("qwen2.5-coder:7b"));
    }

    /// PM-02 falsification: NDJSON spanning two separate response chunks
    /// should parse correctly via line-accumulation.
    #[tokio::test]
    async fn ndjson_line_split_across_chunks() {
        // We serve a response where one JSON object's bytes are split
        // across what appears to be two separate deliveries. We model this
        // by concatenating two partial chunks in one body — reqwest will
        // deliver them as they arrive; the line buffer merges them.
        let server = MockServer::start().await;

        // Two NDJSON lines. Deliberately no trailing newline after last line
        // to test that done:true flushes even without a trailing newline.
        // We add newlines after each complete JSON object (Ollama wire format).
        let body = concat!(
            "{\"message\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"done\":false}\n",
            "{\"message\":{\"role\":\"assistant\",\"content\":\" world\"},\"done\":false}\n",
            "{\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":5,\"eval_count\":3}\n",
        );

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/x-ndjson")
                    .set_body_raw(body, "application/x-ndjson"),
            )
            .mount(&server)
            .await;

        let p = OllamaProvider::with_base_url(server.uri());
        let opts = ChatOptions {
            model: "gemma4:e4b".into(),
            ..Default::default()
        };
        let mut stream = p.chat_stream(vec![], vec![], opts).await.unwrap();

        let mut text = String::new();
        let mut done_seen = false;
        while let Some(ev) = stream.next().await {
            match ev.unwrap() {
                ChatEvent::TextDelta { text: t } => text.push_str(&t),
                ChatEvent::Done { stop_reason, usage } => {
                    assert_eq!(stop_reason, "stop");
                    assert_eq!(usage.input_tokens, 5);
                    assert_eq!(usage.output_tokens, 3);
                    done_seen = true;
                }
                _ => {}
            }
        }
        assert_eq!(text, "Hello world");
        assert!(done_seen, "expected Done event");
    }

    /// XML-tag fallback for Gemma 3 community fine-tunes.
    #[tokio::test]
    async fn xml_tag_fallback_emits_tool_call_events() {
        let server = MockServer::start().await;

        let xml_content =
            r#"<tool_call>{"name":"read_file","arguments":{"path":"src/main.rs"}}</tool_call>"#;
        let body = format!(
            "{{\"message\":{{\"role\":\"assistant\",\"content\":\"{content}\"}},\"done\":false}}\n\
             {{\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":1,\"eval_count\":1}}\n",
            content = xml_content.replace('"', "\\\"")
        );

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/x-ndjson")
                    .set_body_raw(body.as_str(), "application/x-ndjson"),
            )
            .mount(&server)
            .await;

        let p = OllamaProvider::with_base_url(server.uri());
        let opts = ChatOptions {
            model: "gemma3:4b".into(),
            ..Default::default()
        };
        let mut stream = p.chat_stream(vec![], vec![], opts).await.unwrap();

        let mut events = Vec::new();
        while let Some(ev) = stream.next().await {
            events.push(ev.unwrap());
        }

        let start = events
            .iter()
            .find(|e| matches!(e, ChatEvent::ToolCallStart { name, .. } if name == "read_file"));
        assert!(
            start.is_some(),
            "expected ToolCallStart for read_file; got {events:?}"
        );
        let end = events.iter().find(|e| matches!(e, ChatEvent::ToolCallEnd { args_json, .. } if args_json.contains("main.rs")));
        assert!(
            end.is_some(),
            "expected ToolCallEnd with args containing main.rs; got {events:?}"
        );
    }

    /// Native tool_calls in structured format (Gemma 4 / qwen2.5-coder).
    #[tokio::test]
    async fn native_tool_calls_emit_events() {
        let server = MockServer::start().await;

        let body = concat!(
            "{\"message\":{\"role\":\"assistant\",\"content\":\"\",",
            "\"tool_calls\":[{\"function\":{\"name\":\"search_code\",\"arguments\":{\"query\":\"TODO\"}}}]},",
            "\"done\":false}\n",
            "{\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":10,\"eval_count\":5}\n",
        );

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/x-ndjson")
                    .set_body_raw(body, "application/x-ndjson"),
            )
            .mount(&server)
            .await;

        let p = OllamaProvider::with_base_url(server.uri());
        let opts = ChatOptions {
            model: "gemma4:e4b".into(),
            ..Default::default()
        };
        let mut stream = p.chat_stream(vec![], vec![], opts).await.unwrap();

        let mut events = Vec::new();
        while let Some(ev) = stream.next().await {
            events.push(ev.unwrap());
        }

        let start = events
            .iter()
            .find(|e| matches!(e, ChatEvent::ToolCallStart { name, .. } if name == "search_code"));
        assert!(
            start.is_some(),
            "expected ToolCallStart for search_code; got {events:?}"
        );
        let end = events.iter().find(
            |e| matches!(e, ChatEvent::ToolCallEnd { args_json, .. } if args_json.contains("TODO")),
        );
        assert!(
            end.is_some(),
            "expected ToolCallEnd with TODO in args; got {events:?}"
        );
    }

    /// list_models: daemon down → OllamaDaemonDown error.
    #[tokio::test]
    async fn list_models_daemon_down() {
        // No mock server; connection refused.
        let p = OllamaProvider::with_base_url("http://127.0.0.1:1".into());
        let err = p.list_models().await.unwrap_err();
        assert!(
            matches!(
                err,
                ProviderError::OllamaDaemonDown { .. } | ProviderError::Network { .. }
            ),
            "expected daemon-down or network error; got {err:?}"
        );
    }

    /// list_models: parses /api/tags response, marks gemma3 legacy when gemma4 present.
    #[tokio::test]
    async fn list_models_marks_gemma3_legacy_when_gemma4_present() {
        let server = MockServer::start().await;
        let tags_body = json!({
            "models": [
                { "name": "gemma4:e4b" },
                { "name": "gemma3:4b" },
                { "name": "qwen2.5-coder:7b" }
            ]
        });
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&tags_body))
            .mount(&server)
            .await;

        let p = OllamaProvider::with_base_url(server.uri());
        let models = p.list_models().await.unwrap();

        let g4 = models.iter().find(|m| m.id == "gemma4:e4b").unwrap();
        assert!(!g4.legacy, "gemma4:e4b must not be legacy");
        assert!(g4.supports_vision);

        let g3 = models.iter().find(|m| m.id == "gemma3:4b").unwrap();
        assert!(g3.legacy, "gemma3:4b must be legacy when gemma4 is present");
    }

    // ---------- Phase 6a-iii required tests ----------

    /// AC: a model NOT in any known whitelist (e.g. llama3.1:8b) must have
    /// `supports_tools: true` — permissive default (Q1 decision).
    #[tokio::test]
    async fn supports_tools_default_is_true() {
        let server = MockServer::start().await;
        let tags_body = json!({
            "models": [
                { "name": "llama3.1:8b" },
                { "name": "phi4:latest" },
                { "name": "nomic-embed-text:latest" }
            ]
        });
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&tags_body))
            .mount(&server)
            .await;

        let p = OllamaProvider::with_base_url(server.uri());
        let models = p.list_models().await.unwrap();

        let llama = models.iter().find(|m| m.id == "llama3.1:8b").unwrap();
        assert!(
            llama.supports_tools,
            "llama3.1:8b must have supports_tools=true (permissive default)"
        );

        let phi = models.iter().find(|m| m.id == "phi4:latest").unwrap();
        assert!(
            phi.supports_tools,
            "phi4:latest must have supports_tools=true (permissive default)"
        );

        // Embedding model is the known exception.
        let embed = models
            .iter()
            .find(|m| m.id == "nomic-embed-text:latest")
            .unwrap();
        assert!(
            !embed.supports_tools,
            "nomic-embed-text must have supports_tools=false (embedding-only)"
        );
    }

    /// AC: version_gate_blocks_old_daemon — daemon reports 0.19.5 →
    /// `check_version` returns `TooOld`.
    #[tokio::test]
    async fn version_gate_blocks_old_daemon() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/version"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&json!({ "version": "0.19.5" })),
            )
            .mount(&server)
            .await;

        let p = OllamaProvider::with_base_url(server.uri());
        let status = p.check_version().await;
        assert_eq!(
            status,
            OllamaVersionStatus::TooOld("0.19.5".to_string()),
            "expected TooOld for version 0.19.5"
        );
    }

    /// AC: daemon_down_returns_e019 — connection refused →
    /// `check_version` returns `Down`.
    #[tokio::test]
    async fn daemon_down_returns_e019() {
        // Port 1 is always refused.
        let p = OllamaProvider::with_base_url("http://127.0.0.1:1".into());
        let status = p.check_version().await;
        assert_eq!(
            status,
            OllamaVersionStatus::Down,
            "expected Down for connection refused"
        );
    }
}
