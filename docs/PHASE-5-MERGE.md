# Phase 5 — Merging the foundational Rust crates

> Read this when starting Phase 5 (Keyring + Anthropic Provider + Chat Panel). The trait + types + DB schema + Anthropic skeleton have been pre-staged; this guide explains how to land them and what's still TODO.

## Pre-staged files (Phase 5 foundation)

| Path | What |
|---|---|
| `src-tauri/biscuitcode-providers/Cargo.toml` | Workspace member crate; reqwest+rustls, async-trait, eventsource-stream, wiremock for tests |
| `src-tauri/biscuitcode-providers/src/lib.rs` | Module structure + re-exports |
| `src-tauri/biscuitcode-providers/src/types.rs` | `Message` / `ContentBlock` / `MentionKind` / `ToolCall` / `ToolResult` / `ToolSpec` / `ChatEvent` / `ChatOptions` / `ReasoningEffort` / `ModelInfo` / `Usage` / `ProviderError` |
| `src-tauri/biscuitcode-providers/src/trait.rs` | `ModelProvider` async trait surface (matches `docs/design/PROVIDER-TRAIT.md` exactly) |
| `src-tauri/biscuitcode-providers/src/anthropic/mod.rs` | `AnthropicProvider` skeleton: model list, sampling-strip helper + tests, `chat_stream` returns "not yet implemented" until you fill it in |
| `src-tauri/biscuitcode-db/Cargo.toml` | rusqlite-bundled, ulid, chrono |
| `src-tauri/biscuitcode-db/src/lib.rs` | `Database::open` with WAL+busy_timeout+FKs, `open_in_memory` for tests |
| `src-tauri/biscuitcode-db/src/types.rs` | `WorkspaceId`/`ConversationId`/`MessageId`/`SnapshotId` ULID newtypes; row structs `Workspace`/`Conversation`/`StoredMessage`/`Snapshot`/`SnapshotFile`; `DbError` |
| `src-tauri/biscuitcode-db/src/migrations.rs` | Migration runner + `MAX_SCHEMA_VERSION` + tests for fresh-db / double-run / schema-too-new |
| `src-tauri/biscuitcode-db/src/migrations/0001_initial.sql` | Full schema: workspaces / conversations / messages / snapshots / snapshot_files. STRICT tables. Indexes for hot queries. |

## Add to the top-level workspace

In `src-tauri/Cargo.toml`'s `[workspace]` table:

```toml
[workspace]
members = [
    ".",
    "biscuitcode-core",
    "biscuitcode-providers",   # add
    "biscuitcode-db",          # add
]
```

In the top-level `src-tauri/Cargo.toml`'s `[dependencies]`:

```toml
biscuitcode-providers = { path = "biscuitcode-providers" }
biscuitcode-db        = { path = "biscuitcode-db" }
```

## What's still TODO for the Phase 5 coder

### 1. Implement `AnthropicProvider::chat_stream`

The skeleton at `src-tauri/biscuitcode-providers/src/anthropic/mod.rs` returns `ProviderError::Other("not yet implemented")`. Fill in:

1. Build the request body. Apply `cache_control: {"type": "ephemeral"}` to the system prompt + tool definitions when `opts.prompt_caching_enabled`. Call `model_strips_sampling(&opts.model)` and omit `temperature`/`top_p`/`top_k` when true.

2. POST to `https://api.anthropic.com/v1/messages` with headers:
   - `x-api-key: <self.api_key>`
   - `anthropic-version: 2023-06-01` (verify current pin against docs.anthropic.com)
   - `content-type: application/json`
   - `accept: text/event-stream`

3. Map HTTP errors to `ProviderError`:
   - 401 → `AuthInvalid`
   - 429 → `RateLimited` (parse `retry-after` header)
   - other 4xx → `BadRequest`
   - 5xx → `ServerError`
   - `reqwest::Error` IO → `Network`

4. Parse the SSE stream via `eventsource-stream` and translate per the table in `docs/design/PROVIDER-TRAIT.md` § AnthropicProvider.

5. Return `Pin<Box<dyn Stream<Item = Result<ChatEvent, ProviderError>> + Send>>`. Use `async-stream::stream!` for clean construction.

### 2. Add the wiremock-based integration test

In `biscuitcode-providers/tests/anthropic_integration.rs`:

```rust
#[tokio::test]
async fn requests_strip_sampling_for_opus_47() {
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        // Body matcher that asserts NO temperature/top_p/top_k keys present:
        .and(body_partial_json(serde_json::json!({"model": "claude-opus-4-7"})))
        .respond_with(ResponseTemplate::new(200).set_body_string(MINIMAL_SSE))
        .mount(&mock)
        .await;
    // …call chat_stream against `mock.uri()` and consume the stream.
    // Assert no body field ever sent contains those keys.
}
```

(Inject the base URL via env or constructor param; the production path stays `https://api.anthropic.com`.)

### 3. Wire the `secrets` module in `biscuitcode-core`

Phase 5 also delivers `biscuitcode-core::secrets` wrapping the `keyring` crate (see `docs/plan.md` Phase 5 deliverables). Pre-staging this is risky because the `keyring` crate's exact feature flags and async-vs-blocking API may have drifted between releases. Phase 5 coder's job to land:

```rust
// biscuitcode-core/src/secrets.rs
pub async fn set(service: &str, key: &str, value: &str) -> Result<()> { ... }
pub async fn get(service: &str, key: &str) -> Result<Option<String>> { ... }
pub async fn delete(service: &str, key: &str) -> Result<()> { ... }

/// Read-only Secret Service availability check via `busctl --user list`.
/// MUST run BEFORE any keyring API call (per docs/design/CAPABILITIES.md
/// and the synthesis log).
pub async fn secret_service_available() -> bool { ... }
```

### 4. Replace `ChatPanel.tsx` (Phase 2 shell) with the real virtualized chat

See `docs/plan.md` Phase 5 deliverables. Add `react-virtuoso` + `react-markdown` + `remark-gfm` to package.json. Wire the model picker to read from the providers list. Persist conversations via `biscuitcode-db` over Tauri IPC.

### 5. Run the Phase 5 ACs

Most importantly: the TTFT bench (`tests/ttft-bench.ts`) over 20 prompts after a 1-minute prewarm — p50 < 500ms, p95 < 1200ms.
