# Design — `ModelProvider` trait + `ChatEvent` enum

> Architecture spec consumed by Phase 5 (Anthropic) and Phase 6a (OpenAI + Ollama). Phase 5's coder writes the trait + the Anthropic impl; Phase 6a's coders write the OpenAI and Ollama impls **without changing the trait**. If the trait needs to change in 6a, that's a Law 1 signal: stop and report.

## Goal

Hide every provider's wire-protocol quirks behind a single Rust trait whose surface is small enough that adding a fourth provider in v1.1 is a one-week task, not a refactor. Frontend code consumes a single `ChatEvent` stream regardless of which provider is selected.

## The trait

```rust
use std::pin::Pin;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Stable identifier. e.g. "anthropic", "openai", "ollama".
    /// Used in capability scopes, settings keys, logs.
    fn id(&self) -> &'static str;

    /// User-facing display name. e.g. "Anthropic", "OpenAI", "Ollama".
    fn display_name(&self) -> &'static str;

    /// List of models the user can pick. Live (cached) for Anthropic/OpenAI;
    /// queries `GET /api/tags` for Ollama. Failures return Err so the
    /// settings UI can show a status badge red instead of an empty picker.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError>;

    /// The single streaming endpoint. Returns a stream of `ChatEvent`s
    /// that the agent loop consumes uniformly across providers.
    /// `tools` is empty for non-agent text-only chat.
    /// `opts` carries model selection, max_tokens, optional reasoning effort, etc.
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSpec>,
        opts: ChatOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, ProviderError>> + Send>>, ProviderError>;

    /// Capabilities. Frontend grays out features the model doesn't support.
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn supports_thinking(&self) -> bool { false }
    fn supports_prompt_caching(&self) -> bool { false }
}
```

Notes on shape:

- **`async_trait`**: required because the trait is dyn-compatible (Box<dyn ModelProvider>) and we want async methods. Cost is one heap allocation per method call — negligible.
- **`&'static str` for ids and display names**: providers are compile-time-known. Avoids lifetime headaches in registration code.
- **`Pin<Box<dyn Stream + Send>>`** for the streaming return: the only shape that works for dyn-traits with async streams. Verbose, but mature and well-understood.
- **`Result<Vec<ModelInfo>>` from `list_models`**: networked call, can fail — must be a Result so the UI can surface `E004`/`E005`/etc. via the catalogue.
- **`Result<Stream<Item = Result<...>>>` from `chat_stream`**: the outer Result fails if the request can't even *start* (auth invalid, model 404, network unreachable). The inner Result-per-event fails for mid-stream errors (connection drop, JSON parse failure on a single chunk) without killing the whole stream.

## The `ChatEvent` enum

The single normalized event type that the agent loop and the frontend consume:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    /// Plain text token(s) appended to the assistant message content.
    TextDelta { text: String },

    /// Anthropic "thinking" content (also OpenAI reasoning when surfaced).
    /// Frontend renders this in a collapsible "Thinking…" block,
    /// distinct from TextDelta. Provider may emit zero of these.
    ThinkingDelta { text: String },

    /// A new tool call has started. ID is provider-assigned, opaque to us.
    /// The frontend renders an Agent Activity card immediately on this event.
    /// (The 250ms render gate in Phase 6a/Global AC asserts this.)
    ToolCallStart { id: String, name: String },

    /// Argument JSON for an in-flight tool call, partial.
    /// Concatenate all deltas with the same id to get the full argument JSON.
    /// Do NOT attempt to parse mid-stream — wait for ToolCallEnd.
    ToolCallDelta { id: String, args_delta: String },

    /// Tool call complete. `args_json` is the fully-assembled argument string,
    /// guaranteed parseable as JSON by the provider impl. Agent loop now
    /// dispatches the tool, gathers a result, appends a `ToolResult` to the
    /// conversation, and (if agent mode is on) calls chat_stream again.
    ToolCallEnd { id: String, args_json: String },

    /// Stream finished cleanly. `stop_reason` is provider-normalized:
    /// "end_turn", "max_tokens", "tool_use", "stop_sequence", "error".
    /// `usage` is provider-reported token counts; cache fields nullable.
    Done { stop_reason: String, usage: Usage },

    /// Stream hit a non-fatal error (e.g. one chunk failed JSON-parse but
    /// the rest of the stream is recoverable). The agent loop surfaces this
    /// via the catalogue (E005/E014/etc.) but does not necessarily abort.
    Error { code: String, message: String, recoverable: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_input_tokens: Option<u32>,    // Anthropic-specific
    pub cache_creation_input_tokens: Option<u32>, // Anthropic-specific
}
```

## Provider-specific normalization rules

Each impl's job is to translate its wire protocol into `ChatEvent`s. Document the mapping in the impl module's doc comment. Summary:

### `AnthropicProvider` (Phase 5)

| Anthropic SSE event | → `ChatEvent` |
|---|---|
| `message_start` | (nothing emitted; capture `usage.input_tokens` for later `Done`) |
| `content_block_start` (type=`text`) | (nothing) |
| `content_block_delta` (delta type=`text_delta`) | `TextDelta { text }` |
| `content_block_delta` (delta type=`thinking_delta`) | `ThinkingDelta { text }` |
| `content_block_start` (type=`tool_use`) | `ToolCallStart { id, name }` |
| `content_block_delta` (delta type=`input_json_delta`) | `ToolCallDelta { id, args_delta }` |
| `content_block_stop` (for a `tool_use` block) | `ToolCallEnd { id, args_json }` (assemble from accumulated deltas) |
| `message_delta` | (capture `usage.output_tokens` and `stop_reason`) |
| `message_stop` | `Done { stop_reason, usage }` |
| Mid-stream error event | `Error { code, message, recoverable: false }` |

**Critical: `claude-opus-4-7` rejects `temperature`/`top_p`/`top_k`.** When `opts.model.starts_with("claude-opus-4")` and version >= 4.7, the impl unconditionally omits those fields from the request body. Unit test `requests_strip_sampling_for_opus_47` asserts.

**Prompt caching:** apply `cache_control: {type: "ephemeral"}` to the system prompt and every tool definition by default. ~5x cost reduction on long conversations. Disable only via an explicit opt-out in `ChatOptions` (default: caching on).

### `OpenAIProvider` (Phase 6a)

| OpenAI SSE delta | → `ChatEvent` |
|---|---|
| `choices[0].delta.content` (string) | `TextDelta { text }` |
| `choices[0].delta.tool_calls[i].id` (first appearance) | `ToolCallStart { id, name: <derived from same chunk's function.name> }` |
| `choices[0].delta.tool_calls[i].function.arguments` (delta string) | `ToolCallDelta { id: tool_calls[i].id, args_delta }` |
| `choices[0].finish_reason == "tool_calls"` | one `ToolCallEnd` per tool_calls index, in index order |
| `choices[0].finish_reason` (any non-null) | `Done { stop_reason, usage: from chunk.usage if present }` |

**Indexed tool-call accumulation:** OpenAI's deltas use `tool_calls[i].index` to identify which call's arguments are growing. A single chunk may contain partial deltas for multiple tool calls. Implementation must keep a `HashMap<usize, ToolCallAccumulator>` and emit `ToolCallEnd` for every non-empty entry on `finish_reason == "tool_calls"`.

**Reasoning models:** `gpt-5.4-pro` and reasoning variants emit no `delta.content` until reasoning completes (3–30s of silence). Frontend shows a `Thinking…` indicator. The TTFT < 500ms gate **does not apply to reasoning models** — Global AC explicitly exempts.

### `OllamaProvider` (Phase 6a)

| Ollama NDJSON chunk | → `ChatEvent` |
|---|---|
| `{"message":{"role":"assistant","content":"..."},"done":false}` | `TextDelta { text: content }` |
| `{"message":{"role":"assistant","tool_calls":[...]},"done":false}` | one `ToolCallStart` + `ToolCallEnd` per element (Ollama emits whole tool calls atomically, no streaming-args) |
| `{"done":true,"done_reason":"...","prompt_eval_count":N,"eval_count":M}` | `Done { stop_reason: done_reason, usage: { input_tokens: prompt_eval_count, output_tokens: eval_count } }` |

**Robust XML-tag fallback for tool calls in `message.content`:** some Gemma 3 community fine-tunes emit tool calls as `<tool_call>{"name":"...","arguments":{...}}</tool_call>` blocks INSIDE `message.content` rather than as structured `tool_calls` arrays. Ollama provider must regex-scan content for these and synthesize `ToolCallStart` + `ToolCallEnd` events from them. Gemma 4 does NOT need this fallback (native function calling), but the parser path stays in place defensively.

**Gemma 4 tag selection** at first launch: see Phase 6a deliverables in `docs/plan.md` for the verified RAM-tier table. Logic = ping `GET /api/tags`; if any `gemma4:*` is present use it; else attempt pull; else fall back to `gemma3:*` ladder + `E007 GemmaVersionFallback` toast.

## Error mapping

Provider errors go through `ProviderError` (a `thiserror` enum in `biscuitcode-providers`) and are converted to catalogue codes by the chat panel layer:

| `ProviderError` variant | Catalogue code |
|---|---|
| `AuthInvalid` (401) | `E004` (Anthropic) / equivalents for OpenAI/Ollama |
| `Network(reqwest::Error)` | `E005` |
| `RateLimited { retry_after }` | `E006` |
| `OllamaDown` (connection refused at 127.0.0.1:11434) | catalogue entry TBD in Phase 6a |

## Testing strategy

- **Unit tests** for each provider's wire-format → `ChatEvent` translation, using checked-in fixture transcripts of real API responses (sanitized of any auth tokens). Located at `src-tauri/biscuitcode-providers/tests/fixtures/`.
- **Integration tests** that hit a mock HTTP server (wiremock-rs) returning canned SSE/NDJSON to validate the full streaming path including HTTP/2 keep-alive behavior.
- **Cross-provider snapshot test** (`tests/provider-event-shape.spec.ts`, Phase 6a Global AC): for the canonical "hello" prompt, the `ChatEvent` sequence shape (count of each event type, ordering) is identical across all three providers. Asserts the abstraction holds.

## Things explicitly NOT in scope for v1

- Multi-modal output (provider returns image/audio). v1.1.
- Provider-side prompt-caching control beyond Anthropic's `cache_control`. OpenAI's prompt caching is automatic and not user-controllable; Ollama doesn't have an equivalent.
- Streaming JSON-mode responses (where the model is forced to emit valid JSON for the whole response). Tool-use covers the agent loop's needs; raw JSON-mode is a v1.1 feature for non-agent workflows.
- Provider-side rate-limit-aware queueing. v1 surfaces `E006` and lets the user retry; v1.1 adds an opt-in queue.
