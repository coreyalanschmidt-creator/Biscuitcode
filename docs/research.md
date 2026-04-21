# BiscuitCode — Phase 6a Research Dossier

> Authored 2026-04-20. Scoped to Phase 6a: OpenAI provider, Ollama provider, read-only tool
> surface, Agent Activity UI, Gemma 4 verification, and the capability-detection question.
> Prior research rounds (r1, r2) and the synthesized plan.md are in-scope pre-reads; this
> document does not restate what they settled correctly.

---

## Topic & Scope

Phase 6a finishes the provider layer and wires it to a live agentic loop with read-only tools.
The deliverables are:

**In scope:**
- `OpenAIProvider` — full `chat_stream` implementation at `src-tauri/biscuitcode-providers/src/openai/mod.rs` (skeleton already exists; implementation is a TODO block).
- `OllamaProvider` — full `list_models` + `chat_stream` implementations at `src-tauri/biscuitcode-providers/src/ollama/mod.rs` (skeleton exists; most logic is already implemented per code inspection — see Prior Art summary).
- `search_code` tool execute body — at `src-tauri/biscuitcode-agent/src/tools/search_code.rs` (already fully implemented per code inspection).
- `AgentActivityPanel.tsx` — the real live-streaming UI (already fully implemented per code inspection).
- Tauri capability file updates: `http.json` (add OpenAI + Ollama), `shell.json` (add `ollama` command).
- Ollama install flow + RAM-tier auto-pull with Gemma 4 tags.
- `E007 GemmaVersionFallback` error code registration.
- Cross-provider round-trip tests against the canonical 3-tool fixture.
- Ollama runtime capability detection: how to determine version floor and function-calling support.

**Out of scope for Phase 6a:**
- Write tools (`write_file`, `apply_patch`, `run_shell`) — Phase 6b.
- Snapshot/rewind — Phase 6b.
- Inline edit (`Ctrl+K Ctrl+I`) — Phase 6b.
- Confirmation modals — Phase 6b.
- Multi-turn rewind UI — Phase 6b.
- Git panel, LSP, preview panel — Phase 7.
- Settings UI, onboarding — Phase 8.
- Auto-update, error catalogue audit — Phase 9.

---

## Assumptions

1. **[HIGH]** Phases 0-5 are marked Complete per `docs/plan.md`. The Anthropic provider is fully implemented with tests; the `ModelProvider` trait and `ChatEvent` enum are frozen.
2. **[HIGH]** The OpenAI provider skeleton and Ollama provider skeleton already exist in the repo. Code inspection confirms the Ollama provider is essentially fully implemented (not a stub). The OpenAI provider is also fully implemented with wiremock tests. The `search_code` tool and `AgentActivityPanel.tsx` are also fully implemented. The coder's primary task is integration, wiring, and gap-filling, not authoring from scratch.
3. **[HIGH]** `async-openai` crate is intentionally NOT used. The project hand-rolls reqwest-based HTTP for OpenAI, following the same pattern as the Anthropic provider. This is the right call — the OpenAI provider impl is already present and tested. Do not switch to `async-openai`.
4. **[HIGH]** The OpenAI Chat Completions API is the correct target (not the Responses API). See OpenAI provider section below for rationale.
5. **[HIGH]** Gemma 4 tags verified as: `gemma4:e2b`, `gemma4:e4b` (=`:latest`), `gemma4:26b`, `gemma4:31b`. Minimum Ollama version: `0.20.0`. All support function calling natively. These are recorded in CLAUDE.md as resolved Q16 — confirmed stable.
6. **[MED]** Ollama's `/api/show` response includes a `capabilities` array (observed as `["completion","vision"]` in API docs). Tool-calling capability for a given model is NOT surfaced as a named capability in this array — Ollama determines it internally from the model's template (presence of `.Tools` variable) and the assigned parser. The practical detection strategy is: try sending tools; if the model ignores them (returns no `tool_calls`), fall back. There is no documented "tools" capability enum value in `/api/show` as of April 2026.
7. **[MED]** OpenAI model list in the existing skeleton (`gpt-5.4-mini`, `gpt-5.4`, `gpt-5.4-nano`, `gpt-5.4-pro`, `gpt-5.3-instant`) is assumed current. The planner already confirmed `gpt-4o` was retired April 3, 2026. The coder should not alter this list without the maintainer's explicit direction.
8. **[LOW]** The `agentStore` Zustand state referenced by `AgentActivityPanel.tsx` is assumed to exist from Phase 5 or prior work. If it does not exist, the coder must create it as a minimal slice. This is a likely gap since Phase 5 focused on Anthropic text-only chat.

---

## Background & Landscape

### What the code inspection found

Contrary to the Phase 6a description as primarily a "TODO fill-in" task, code inspection reveals that all four major implementation components are already fully authored:

| File | Actual state |
|---|---|
| `src-tauri/biscuitcode-providers/src/openai/mod.rs` | Fully implemented with 5 wiremock tests |
| `src-tauri/biscuitcode-providers/src/ollama/mod.rs` | Fully implemented with 5 wiremock tests + Gemma4 tier table |
| `src-tauri/biscuitcode-agent/src/tools/search_code.rs` | Fully implemented with 7 unit tests |
| `src/components/AgentActivityPanel.tsx` | Fully implemented with virtuoso + performance.mark |

This means Phase 6a's coder role is primarily:
1. Wiring Tauri commands (`agent_run`, `agent_pause`) in `main.rs` / `lib.rs`.
2. Updating capability files (`http.json`, `shell.json`).
3. Implementing the Ollama install/pull flow (the `ollama_install()` + `ollama_pull()` Tauri commands).
4. Implementing RAM detection + model selection logic.
5. Registering `E007`.
6. Creating the `agentStore` Zustand slice (if not already present from Phase 5).
7. Running the cross-provider acceptance tests.

The research below serves to confirm the implementations are correct and identify any gaps.

### Prior art summary

**`CLAUDE.md`** — locks Q16 (Gemma 4 tags verified), locks identity and security posture. No open questions remain in the CLAUDE.md scope for Phase 6a.

**`docs/design/PROVIDER-TRAIT.md`** — fully specifies the OpenAI and Ollama normalization tables. Code inspection confirms both providers implement exactly the specified mapping. The trait is frozen — Phase 6a must not change it.

**`docs/design/AGENT-LOOP.md`** — specifies Phase 6a delivers read-only dispatch only. The `ReActExecutor` is present at `src-tauri/biscuitcode-agent/src/executor/mod.rs` and the code comments confirm the Write/Shell arms return `NotYetAvailable` (Phase 6b fills in).

**`docs/design/CAPABILITIES.md`** — specifies the exact capability JSON to add in Phase 6a: OpenAI URL to `http.json`, `http://localhost:11434/**` to `http.json`, `ollama` command with specific arg validators to `shell.json`.

**`docs/ERROR-CATALOGUE.md`** — `E007 GemmaVersionFallback` is defined. The Rust enum variant + TS union + en bundle key + trigger test must be authored in Phase 6a.

**`docs/PHASE-6-MERGE.md`** — lists 10 explicit TODO items for the Phase 6a coder. Items 1, 2, 3 (provider impls, search_code execute) are already done per code inspection. Items 4-10 remain for the coder.

**`tests/fixtures/canonical-tool-prompt.md`** — the 3-tool fixture is fully specified. The test workspace setup and expected tool sequence are deterministic.

**`src/components/AgentActivityPanel.tsx`** — fully implemented. Uses `react-virtuoso`, brand tokens, `performance.mark` on card mount (inside `useEffect` to run post-commit), `aria-label` on status icons.

**`src-tauri/biscuitcode-providers/src/types.rs`** — `ChatEvent`, `ProviderError`, `ModelInfo`, `ChatOptions`, `Message` are all defined and stable.

---

## 1. OpenAI Provider

### Current state

The OpenAI provider is fully implemented at `src-tauri/biscuitcode-providers/src/openai/mod.rs`. It uses:
- Hand-rolled `reqwest` with SSE via `eventsource_stream` (same pattern as Anthropic).
- `with_base_url` constructor for testability.
- `stream_options: { include_usage: true }` to get token counts on the final chunk.
- Per-index `HashMap<usize, ToolCallAccum>` to handle OpenAI's indexed tool-call accumulation.
- `normalize_stop_reason` to map `"stop"` → `"end_turn"`, `"tool_calls"` → `"tool_use"`, `"length"` → `"max_tokens"`.
- `reasoning_effort` field for o-class models.
- Five wiremock tests: two-tool-call index accumulation, text-only stream, 401 → AuthInvalid, stop-reason normalization, model list.

### Chat Completions vs. Responses API

The existing implementation correctly targets `POST /v1/chat/completions`. This is the right choice for Phase 6a:

- Chat Completions is fully supported and stable; OpenAI says "use Responses for new projects" but does not deprecate Chat Completions.
- The Responses API uses different event naming (`response.function_call_arguments.delta` vs `choices[0].delta.tool_calls[i].function.arguments`) and a different request shape (`input`/`instructions` vs `messages`). Switching would require a new decoder.
- Reasoning models (gpt-5.4-pro) work in Chat Completions with `reasoning_effort`.
- Chat Completions with `stream_options.include_usage` reliably returns token counts, which we already use.

**Conclusion:** Stay on Chat Completions. Do not add a Responses API decoder in Phase 6a.

### Auth via keyring

The provider receives the API key as a `String` in `new(api_key: String)`. The key is loaded from libsecret by the Tauri command layer, not inside the provider. This correctly implements the security posture. The provider never stores the key to disk or logs it.

### Rate-limit handling

HTTP 429 returns `ProviderError::RateLimited { retry_after_seconds }` extracted from the `Retry-After` header (defaults to 60s). The frontend maps this to `E006` and shows a countdown. Exponential backoff is explicitly out of scope for v1.

### Cancellation

Cancellation at the HTTP level happens by dropping the stream. The `ReActExecutor`'s pause flag checks between iterations; mid-stream cancellation is explicitly out of scope (per AGENT-LOOP.md). No change needed.

### Token metering

`stream_options.include_usage: true` is already in the request body. Token counts arrive on the final chunk with `prompt_tokens` / `completion_tokens`. The impl maps these to `usage.input_tokens` / `usage.output_tokens`.

### Identified gap: missing test for reasoning-model TTFT exemption

The `ModelInfo.is_reasoning_model` flag is set for `gpt-5.4-pro`. The frontend uses this to skip the 250ms TTFT gate. There is no unit test asserting this flag is set. The coder should add one:

```rust
let pro = ms.iter().find(|m| m.id == "gpt-5.4-pro").unwrap();
assert!(pro.is_reasoning_model);
```

This test already exists in the wiremock suite — confirmed present.

---

## 2. Ollama Provider

### Current state

The Ollama provider is fully implemented at `src-tauri/biscuitcode-providers/src/ollama/mod.rs`. It uses:
- `reqwest` bytes_stream with a `line_buf: String` accumulator for NDJSON (PM-02 fix: cross-chunk line joining).
- `GET /api/tags` for `list_models`.
- `POST /api/chat` with `stream: true` for `chat_stream`.
- Native `tool_calls` parsing (Gemma 4, qwen2.5-coder).
- XML-tag fallback regex for Gemma 3 community fine-tunes.
- `gemma4_tag_for_ram_gb(ram_gb)` and `gemma3_fallback_for_ram_gb(ram_gb)` and `agent_mode_preferred(ram_gb)` helper functions.
- `OllamaDaemonDown` error variant for connection refused.
- Five wiremock tests: NDJSON line split, XML fallback, native tool calls, daemon down, legacy flagging.

### /api/chat vs /api/generate

`/api/chat` is the correct endpoint. It supports multi-turn conversation via the `messages` array and tool calling. `/api/generate` is the single-turn completion endpoint without tool support.

### Ollama tools format

Ollama accepts tools in OpenAI function-call format (the impl already does this):

```json
{
  "type": "function",
  "function": {
    "name": "...",
    "description": "...",
    "parameters": { ... }
  }
}
```

This is confirmed by the Ollama blog post on tool support and community documentation.

### Tool-calling emission semantics

Unlike OpenAI, Ollama emits complete tool calls atomically (not streaming-args character-by-character). The call appears as a `tool_calls` array on a single NDJSON line with `done: false`. The impl handles this correctly: it emits `ToolCallStart` + `ToolCallEnd` pairs for each element without any intermediate `ToolCallDelta`.

This means Ollama tool cards appear in the Agent Activity UI after a chunk delay (not character-by-character like OpenAI). The 250ms gate measures from `ToolCallStart` to card render — not from request start — so this remains passable even for Ollama's atomic emission.

### Daemon discovery and health check

The health check flow should be:

1. `GET http://localhost:11434/api/version` — returns `{"version": "0.x.y"}`. Connection refused → daemon not running.
2. Parse the version string; compare semver against `0.20.0` minimum.
3. If daemon down: offer install.
4. If daemon running but version < 0.20.0: use Gemma 3 fallback, fire `E007`.
5. If version >= 0.20.0: proceed with Gemma 4 pull.

The `/api/version` endpoint returns a simple `{"version": "0.x.y"}` string per official documentation. This is the correct mechanism for version gating.

### Runtime capability detection for function calling

Ollama does not expose a stable `capabilities` field in `/api/show` that lists `"tools"` as a named capability. Internally, Ollama determines tool support by inspecting the model's template for the `.Tools` variable, but this is not surfaced via API. The practical approach already in the code is correct: call `GET /api/tags`, inspect model name prefixes (`gemma4:`, `qwen2.5-coder:`, `gemma3:`), and set `supports_tools` accordingly. This is a whitelist approach.

For models not on the known-good whitelist, the correct behavior is:
- Set `supports_tools: true` conservatively (let the model try).
- If the model returns content instead of structured `tool_calls`, the XML-tag fallback path handles it for known Gemma-3-style patterns.
- For entirely unknown models, the agent loop will receive text with no tool calls and stop gracefully.

**assumption:** The `supports_tools` field in `ModelInfo` is used by the frontend to gray out the agent toggle. The coder must verify that non-whitelisted models default to `supports_tools: true` in the `list_models` implementation. The current implementation sets `supports_tools: is_gemma4 || is_qwen_coder || is_gemma3` — this incorrectly marks other models (llama3.1, etc.) as not supporting tools. This is a bug: `llama3.1:8b` does support function calling in Ollama. See Open Questions.

### Model pull UX

The `ollama pull <model>` command streams stdout progress. This is handled via the `shell.json` capability (`ollama pull <model-tag>`). The frontend shows a progress bar fed by the shell command output.

Pull 404 (unknown model tag) is the signal that Ollama version is too old for Gemma 4. This triggers `E007 GemmaVersionFallback`.

### Gemma 4 tag verification

The ollama.com/library/gemma4 page was fetched. Confirmed tags as of 2026-04-20:
- `gemma4:e2b` — 2.3B effective, 7.2GB, 128K context, audio/image
- `gemma4:e4b` — 4.5B effective, 9.6GB, 128K context (= `:latest`)
- `gemma4:26b` — 25.2B MoE/3.8B active, 18GB, 256K context
- `gemma4:31b` — 30.7B, 20GB, 256K context

One additional entry observed: `gemma4:31b-cloud` (cloud deployment, not a local model). This is not relevant to BiscuitCode and the impl correctly ignores it.

**CLAUDE.md Q16 confirmed still valid.** No tag changes detected.

---

## 3. Tool-Call JSON Normalization

The `ChatEvent` enum already normalizes across providers. The mapping table is fully specified in `docs/design/PROVIDER-TRAIT.md` and correctly implemented in all three providers. This section records the normalization as a reference for the cross-provider test.

| Provider | Wire protocol | ToolCallStart trigger | ToolCallEnd trigger | Args delivery |
|---|---|---|---|---|
| Anthropic | SSE | `content_block_start` (type=tool_use) | `content_block_stop` | Accumulated from `input_json_delta` deltas |
| OpenAI | SSE | First delta with `function.name` for index `i` | `finish_reason == "tool_calls"` | Accumulated from `function.arguments` fragments, keyed by index |
| Ollama | NDJSON | Same chunk as ToolCallEnd (atomic) | Same chunk as ToolCallStart (atomic) | Single JSON object in `function.arguments` |

All three normalize to:
```
ToolCallStart { id, name }
ToolCallDelta { id, args_delta }  ← zero or more; Ollama emits none
ToolCallEnd   { id, args_json }   ← fully assembled
```

The cross-provider snapshot test (`tests/provider-event-shape.spec.ts`) will assert the sequence count and ordering are identical across providers for the canonical "hello" prompt.

**One known normalization gap in Ollama:** The `id` field in Ollama's `tool_calls` array is not consistently present. The impl falls back to `function.name` as the id when `tc["id"]` is empty. This means for models that don't emit an id, two simultaneous calls to the same tool will have the same id. The executor currently processes tools sequentially so this is safe for Phase 6a. Phase 6b (parallel tools, v1.1 consideration) would need a synthetic id generator.

---

## 4. Read-Only Tool Surface

### `read_file`

Fully implemented. The implementation:
- Accepts `path` (workspace-relative or absolute).
- Calls `ctx.is_inside_workspace(&p)` which canonicalizes the path before checking — symlink traversal handled correctly.
- Truncates at `ctx.max_result_bytes` (default 256KB).
- Returns lossy UTF-8 for binary files.
- Returns `ToolError::OutsideWorkspace` for paths outside the workspace root, mapping to `E002`.

Four unit tests: reads content, path outside workspace, truncation, missing file.

**No gaps found.**

### `search_code`

Fully implemented. The implementation:
- Accepts `query` (required), `glob` (optional), `regex` (optional boolean).
- Uses `globset::GlobSetBuilder` for glob matching — correctly handles brace expansion (`{src,tests}/**/*.ts`).
- Uses `ignore::WalkBuilder` with `.gitignore` / `.git_global` / `.git_exclude` respecting.
- Binary heuristic: first 8KB null-byte check.
- Outputs `file:lineno: line_content` grouped by file.
- Truncates at `ctx.max_result_bytes`.
- Runs on a blocking thread via `tokio::task::spawn_blocking` (correct for sync I/O).

Seven unit tests: substring match, brace-expansion glob, regex mode, no-matches message, invalid regex error, simple glob, truncation flag.

**No gaps found.**

### Security scope

The `ToolCtx::is_inside_workspace` implementation uses `std::fs::canonicalize`, which resolves symlinks. A symlink pointing outside the workspace root will correctly be denied. The `ToolClass::Read` class means neither tool triggers confirmation or snapshot — correct.

The deny-list (`**/.git/**`, `**/node_modules/**`, etc.) from CAPABILITIES.md is for write tools (Phase 6b). Read tools are not subject to the deny-list. This is intentional: the agent can read `.env` files but cannot write them (Phase 6b enforces the deny-list before writing). If the maintainer wants to block reads of `.env*` and `*.pem` files, this should be addressed in a Phase 6a open question.

---

## 5. Agent Activity UI

### Current state

`src/components/AgentActivityPanel.tsx` is fully implemented. The component:
- Reads `cards: ToolCallCard[]` from `useAgentStore`.
- Renders each card via a `ToolCard` component inside `react-virtuoso`.
- Each card shows: status icon (running/ok/error), tool name, duration, collapsible args (pretty JSON or raw accumulator mid-stream), result when available.
- On first mount, emits `performance.mark('tool_card_visible_<id>')` from `useEffect` and computes `performance.measure('tool_card_render_<id>', ...)`.
- Uses brand tokens: `border-cocoa-500`, `bg-cocoa-600`, `text-biscuit-300`, `text-accent-ok`, `text-accent-error`.
- Uses `aria-label` on the status icon for accessibility.
- Uses `aria-expanded` on the collapsible toggle button.

### Performance mark timing

The implementation uses `useEffect` rather than a `MutationObserver`. The AGENT-LOOP.md design originally specified MutationObserver, but `useEffect` fires synchronously after React's DOM commit (before the browser's next paint frame in most implementations). This is actually correct for the performance gate: it captures when React has committed the card's DOM — the card is visible from the browser's next paint. The existing implementation notes this in a comment ("PM-04 addressed").

The key constraint: the card must be created on `ToolCallStart`, not on `ToolCallEnd`. The `agentStore` must add a card with `status: 'running'` when it receives a `ToolCallStart` event, and update it to `status: 'ok' | 'error'` when the tool completes.

### Missing: `agentStore` Zustand slice

The `AgentActivityPanel.tsx` imports from `'../state/agentStore'` (a `useAgentStore` hook and `ToolCallCard` type). This module is not found in the repo. This is the most likely gap for the Phase 6a coder. The store needs:

```typescript
interface ToolCallCard {
  id: string;
  name: string;
  status: 'running' | 'ok' | 'error';
  argsJson: string;      // accumulates during ToolCallDelta events
  result: string | null;
  startedAt: number;     // performance.now() at ToolCallStart
  endedAt: number | null;
}

interface AgentStore {
  cards: ToolCallCard[];
  addCard(id: string, name: string): void;
  updateCardArgs(id: string, delta: string): void;
  completeCard(id: string, result: string): void;
  errorCard(id: string, error: string): void;
  clearCards(): void;
}
```

The Tauri event layer must emit `ChatEvent` stream events to the frontend, which the agent store processes. The event-wiring pattern from Phase 5 (chat streaming to ChatPanel) applies here.

### Card lifecycle

```
ToolCallStart  → addCard(id, name)   → status: 'running', argsJson: ''
ToolCallDelta  → updateCardArgs(id, delta) → argsJson grows
ToolCallEnd    → (args complete; tool dispatch begins)
[tool executes]
ToolResult     → completeCard(id, result) → status: 'ok', endedAt: now
(or error)     → errorCard(id, error)     → status: 'error', endedAt: now
```

The `ToolCallEnd` event does not itself close the card — the card is still `running` while the tool executes. Only when the tool result returns does the card transition. This means `argsJson` at `ToolCallEnd` is the final value; subsequent events update `result` and `status`.

### Event debouncing

No debouncing is needed. `ToolCallDelta` events from OpenAI are frequent but small; Ollama emits none. React's batching (React 18 automatic batching) handles rapid state updates without jank. `react-virtuoso` handles large card lists efficiently.

### A11y

- `aria-label` on the status icon (already present).
- `aria-expanded` on the collapse toggle (already present).
- Each card is an `<article>` element.
- The panel has `aria-label={t('panels.agentActivity')}` on the `<section>`.
- Focus management: the keyboard shortcut for the bottom panel (`Ctrl+J`) brings focus to the active bottom tab. Within Agent Activity, Tab navigates through card headers. This is standard DOM tab order.

---

## 6. Q16 Gemma 4 Verification

**Tags as of 2026-04-20 (fetched from ollama.com/library/gemma4):**

| Tag | Effective params | VRAM | Context | Notes |
|---|---|---|---|---|
| `gemma4:e2b` | 2.3B | 7.2GB | 128K | Audio + image capable |
| `gemma4:e4b` = `:latest` | 4.5B | 9.6GB | 128K | Audio + image capable |
| `gemma4:26b` | 25.2B MoE / 3.8B active | 18GB | 256K | |
| `gemma4:31b` | 30.7B | 20GB | 256K | |
| `gemma4:31b-cloud` | 30.7B | N/A | 256K | Cloud only — not locally pullable |

**CLAUDE.md Q16 resolution is confirmed unchanged.**

The `gemma4_tag_for_ram_gb` function in the Ollama provider matches the plan:
- `0-7 GB` → `gemma4:e2b`
- `8-31 GB` → `gemma4:e4b`
- `32-47 GB` → `gemma4:26b`
- `48+ GB` → `gemma4:31b`

**Runtime version gating for Gemma 4:**

1. `GET http://localhost:11434/api/version` → `{"version": "0.x.y"}`.
2. Parse version string (simple string split on `.`; no semver crate needed).
3. If major=0, minor < 20 → version too old → fire `E007`, fall back to `gemma3_fallback_for_ram_gb(ram_gb)`.
4. If version >= 0.20.0 → attempt `ollama pull gemma4:<tag>`.
5. If pull returns 404 or error indicating unknown model → also fire `E007` (belt-and-suspenders).

**Function calling capability confirmation:**

All Gemma 4 variants natively support function calling per CLAUDE.md Q16 resolution. The Ollama provider already whitelists `gemma4:*` for `supports_tools: true` in `list_models`. The wiremock test `native_tool_calls_emit_events` exercises the `gemma4:e4b` path. No further action needed on capability detection for Gemma 4.

For other models pulled by the user, `supports_tools` falls back to the whitelist check in `list_models`. See Open Questions for the whitelist gap.

---

## 7. Best Practices

### Provider implementation pattern

The existing providers establish the pattern: `with_base_url` constructor for testability, `eventsource_stream` for SSE, hand-rolled NDJSON line accumulation for Ollama, wiremock for integration tests, fixture-based SSE bodies in tests. Follow this pattern exactly.

### Error surface for Ollama

Phase 6a must register the Ollama-specific error code for daemon down. The ERROR-CATALOGUE.md table does not list an Ollama daemon-down code other than `E007`. The `OllamaDaemonDown` variant exists in `ProviderError` but has no catalogue entry. The planner should decide whether to add `E019 OllamaDaemonDown` or subsume it under existing codes. Recommend a new code because the recovery action (start daemon / install Ollama) is distinct from `E005` (network error).

### Shell capability for Ollama

`shell.json` Phase 6a addition per CAPABILITIES.md:

```json
{
  "name": "ollama",
  "cmd": "ollama",
  "args": [
    { "validator": "^(list|show|--version)$" },
    { "validator": "^pull$" },
    { "validator": "^[a-z][a-z0-9._:-]*$" },
    { "validator": "^serve$" }
  ]
}
```

The `ollama-install` entry with `sh -c "curl -fsSL https://ollama.com/install.sh | sh"` is also Phase 6a. Both are specified in CAPABILITIES.md verbatim — copy as-is.

### HTTP capability for Phase 6a

`http.json` Phase 6a addition per CAPABILITIES.md:

```json
[
  { "url": "https://api.anthropic.com/**" },
  { "url": "https://api.openai.com/**" },
  { "url": "http://localhost:11434/**" }
]
```

The GitHub releases URL is Phase 9. Do not add it in Phase 6a.

### Tauri command wiring

Two new Tauri commands needed per PHASE-6-MERGE.md:

```rust
#[tauri::command]
async fn agent_run(
    registry: tauri::State<'_, Arc<ToolRegistry>>,
    /* ... */
) -> Result<RunOutcome, String>

#[tauri::command]
fn agent_pause(pause: tauri::State<'_, Arc<AtomicBool>>) { ... }
```

The `Arc<ToolRegistry>` and `Arc<AtomicBool>` must be managed as Tauri state:

```rust
app.manage(Arc::new(ToolRegistry::read_only_default()));
app.manage(Arc::new(AtomicBool::new(false)));
```

### `ToolRegistry::read_only_default()`

This constructor registers only `ReadFileTool` and `SearchCodeTool`. The Write/Shell tools have `NotYetAvailable` stubs (Phase 6b fills in). The registry is already defined in `src-tauri/biscuitcode-agent/src/tools/mod.rs`.

### RAM detection

Use the `sysinfo` crate (already in scope per PHASE-6-MERGE.md). `sysinfo::System::total_memory()` returns bytes; divide by `1024^3` for GB. The `gemma4_tag_for_ram_gb` function accepts GB as a `u32`.

---

## 8. Recommended Approach

**Phase 6a is an integration phase, not an authoring phase.** The four major implementation components (OpenAI provider, Ollama provider, search_code tool, AgentActivityPanel) are already authored and tested. The coder's primary work is:

1. **Verify the existing implementations compile and tests pass** (`cargo test -p biscuitcode-providers`, `cargo test -p biscuitcode-agent`).

2. **Create `src/state/agentStore.ts`** — the missing Zustand slice for card state. This is the single most critical gap.

3. **Wire Tauri commands** — `agent_run` and `agent_pause` in `src-tauri/src/lib.rs` (or `main.rs` per project convention). Register `ToolRegistry` and `AtomicBool` as Tauri state.

4. **Wire ChatEvent stream to agentStore** — the Tauri event emission from the agent_run command must forward `ChatEvent` variants to the frontend over `tauri::Emitter`. The frontend's event listener must call `agentStore.addCard()`, `updateCardArgs()`, etc.

5. **Implement Ollama install flow** — `ollama_install()` and `ollama_pull(model: String)` Tauri commands. Include version check against 0.20.0. Fire `E007` on version-too-old. Use `sysinfo` for RAM detection to select the right Gemma 4 tier.

6. **Update capability files** — `http.json` (OpenAI + Ollama URLs), `shell.json` (ollama commands).

7. **Register `E007`** — Rust `thiserror` variant + TS union member + `en.json` bundle key + trigger test.

8. **Run Phase 6a acceptance criteria** — the canonical 3-tool prompt against all three providers.

The simplest approach for the Tauri event channel is to emit individual `ChatEvent` structs serialized as JSON via `app_handle.emit("agent:event", event)` and have the frontend subscribe with `listen("agent:event", handler)`. This follows the Phase 5 Anthropic chat streaming pattern.

---

## 9. Trade-offs & Alternatives

| Option | Pros | Cons | When to use |
|---|---|---|---|
| Hand-rolled `reqwest` for OpenAI (current) | Matches Anthropic pattern; full control; no extra dep | More code than using `async-openai` | Always, for consistency |
| `async-openai` crate | Type-safe request builders; built-in retry | Opinionated shapes may not match our `ChatEvent` enum; adds dependency | Only if starting fresh without existing code |
| Ollama `GET /api/show` for capability detection | Structured response | No `tools` capability exposed via API | Not viable; use whitelist approach |
| Whitelist approach for Ollama tool support (current) | Simple; works for known-good models | Unknown models default to wrong value | Phase 6a; expand whitelist as needed |
| `api/version` semver parse for Ollama floor | Reliable; documented endpoint | Requires version string parsing | Correct for the E007 gating |
| MutationObserver for tool-card timing (AGENT-LOOP.md design) | Captures actual DOM paint | More complex; `useEffect` is sufficient | Not needed; `useEffect` is adequate |
| Debouncing `ToolCallDelta` updates | Reduces React re-renders for OpenAI | Delays arg preview in UI | Not needed; React 18 batches this |
| Responses API for OpenAI (not chosen) | Newer; better reasoning model support | Different wire format; different decoder | Future: v1.1 if o-models need it |

---

## 10. Phase Split Recommendation

**Phase 6a should NOT be split further.** The scope as defined is coherent:
- All four major components are already implemented.
- The remaining work (Tauri wiring, agentStore, install flow, capability files, E007, tests) is moderate and inter-dependent.
- Splitting would create a sub-phase where the agent loop exists but the UI can't display it, or the UI exists but can't receive events — both are unverifiable states.

The existing 6a/6b split (read-only vs. write tools) is the correct division of risk. Phase 6a is already the "safe" half.

---

## 11. Risks & Unknowns

### Risk 1: `agentStore` does not exist

**Likelihood: HIGH.** The `AgentActivityPanel.tsx` imports `useAgentStore` from `'../state/agentStore'`, but no such file appears in the glob of existing files. If Phase 5 did not create this store, the TypeScript build fails on import.

**Mitigation:** The Phase 6a coder must create `src/state/agentStore.ts` as the first action. The interface is fully specified in this document.

### Risk 2: Ollama `supports_tools` whitelist too narrow

**Likelihood: MED.** The current `list_models` implementation sets `supports_tools: is_gemma4 || is_qwen_coder || is_gemma3`. This marks `llama3.1:8b`, `llama3.2:3b`, `llama3.3:70b`, `phi4`, `qwen3`, and other tool-capable models as `supports_tools: false`, causing the agent toggle to be grayed out when these models are selected.

**Mitigation:** Default to `supports_tools: true` for all models; selectively mark `supports_tools: false` only for models known to not support tools (e.g., embedding models, vision-only models). Alternatively, keep the conservative default and expand the whitelist to include the major tool-capable families. The planner must decide the policy.

**assumption:** for Phase 6a, the simplest correct fix is `supports_tools: true` as the default for all models, with selective `false` for known non-chat models. This is less precise but never incorrectly gates a user out.

### Risk 3: ChatEvent stream to frontend — missing Tauri emit pattern from Phase 5

**Likelihood: MED.** Phase 5 shipped Anthropic chat streaming. The exact pattern for how `ChatEvent` variants are serialized and emitted over Tauri events to the frontend must be consistent. If Phase 5 used a specific event name or payload shape, Phase 6a must match it. The coder must read Phase 5's Execution Notes before authoring the `agent_run` Tauri command.

**Mitigation:** Read the Phase 5 coder's notes in `docs/plan.md` and inspect `src-tauri/src/lib.rs` before implementing `agent_run`.

### Risk 4: Ollama model pull 404 ambiguity

**Likelihood: LOW.** When `ollama pull gemma4:e4b` returns an error, it could mean: (a) Ollama version too old for the tag, (b) network failure, (c) model genuinely not on the registry. The version check via `/api/version` should happen before the pull to distinguish (a). Cases (b) and (c) surface as generic errors and should show `E005` (network) or a new error code.

### Risk 5: `performance.mark` API availability in WebKitGTK 4.1

**Likelihood: LOW.** The `performance.mark` and `performance.measure` Web API are standard and widely supported. WebKitGTK 4.1 (the Tauri backend on Linux) supports these. This is not a concern.

### Risk 6: `agentStore` card accumulation and React re-render performance

**Likelihood: LOW.** For a long agent run with many tool calls (>50 cards), Virtuoso handles the list efficiently. The `ToolCallDelta` events from OpenAI may fire very frequently. The `updateCardArgs(id, delta)` action concatenates a string. React 18 automatic batching will batch rapid state updates within the same event loop tick.

---

## 12. Open Questions for the Planner

**Q1.** Should `supports_tools` in `OllamaProvider::list_models` default to `true` (all models) or remain as a whitelist? The whitelist currently excludes `llama3.1:8b`, `llama3.2:3b`, `phi4`, etc. **Proposed default:** change to `supports_tools: true` for all models; keep the vision-flag (`supports_vision`) as whitelist-only.

**Q2.** Should the Ollama daemon-down error get a new catalogue code (`E019 OllamaDaemonDown`) or be surfaced as `E005` with a specific message? **Proposed default:** add `E019` with recovery action "Start the Ollama daemon with `ollama serve` or install Ollama." This gives the user actionable guidance distinct from generic network errors.

**Q3.** Does the Phase 5 Tauri event emission pattern use a single `"agent:event"` event name carrying a tagged `ChatEvent` payload, or per-event-type names (`"agent:text_delta"`, `"agent:tool_call_start"`, etc.)? **Proposed default:** single event name with tagged payload (simpler; matches the `ChatEvent` serde tag design).

**Q4.** Should read tools be denied for `.env*` and `*.pem` files? CAPABILITIES.md's deny-list applies only to write tools. Allowing reads is the current behavior. **Proposed default:** do not add a read deny-list in Phase 6a; this is a v1.1 security hardening item.

**Q5.** The `ollama-install` shell command uses `sh -c "curl -fsSL https://ollama.com/install.sh | sh"`. Should this require the user to have `curl` installed, or should BiscuitCode check for `curl` before showing the install offer? **Proposed default:** check `which curl` first; if absent, show an error asking the user to install `curl` first (not auto-install it).

**Q6.** The `agentStore` is used by both `AgentActivityPanel` (displays cards) and `ChatPanel` (may need to reference tool calls for the "badge linking card to activity panel" feature). Should `agentStore` be a shared store or kept separate? **Proposed default:** single shared `agentStore`; `ChatPanel` reads `cards` for badge rendering.

**Q7.** The Phase 6a acceptance-criteria demo runs the canonical 3-tool prompt against Ollama with `gemma4:e4b`. In CI, Ollama + a 9.6GB model is not practical. Should the Ollama row of the CI test be optional (skip if Ollama daemon not reachable) or mandatory? **Proposed default:** optional in CI, mandatory for the Phase 10 release smoke test on a machine with the model loaded.

---

## 13. Non-Goals (Phase 6a)

- Write tools (`write_file`, `apply_patch`, `run_shell`) — Phase 6b.
- Snapshot/rewind logic — Phase 6b.
- Inline edit (`Ctrl+K Ctrl+I`) — Phase 6b.
- Confirmation modals — Phase 6b.
- The rewind UI in the conversation header — Phase 6b.
- `@terminal-output`, `@problems`, `@git-diff` context mentions — Phase 7.
- LSP client — Phase 7.
- Git panel — Phase 7.
- Onboarding modal — Phase 8.
- Settings page providers tab (beyond what Phase 5 delivered) — Phase 8.
- Auto-update — Phase 9.
- Error catalogue audit — Phase 9.
- Conversation branching UI — Phase 8.
- The `@` mention fuzzy picker (except `@file`, `@folder`, `@selection` which are Phase 6a per PHASE-6-MERGE.md item 8) — partial scope; the full picker is Phase 7.

---

## Sources

1. [OpenAI Chat Completions API Reference](https://platform.openai.com/docs/api-reference/chat/create)
2. [OpenAI Streaming API Responses](https://developers.openai.com/api/docs/guides/streaming-responses)
3. [OpenAI Chat Completions Streaming Events](https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events)
4. [OpenAI Migrate to Responses API](https://developers.openai.com/api/docs/guides/migrate-to-responses)
5. [OpenAI vs Responses vs Assistants 2026 — PkgPulse](https://www.pkgpulse.com/blog/openai-chat-completions-vs-responses-api-vs-assistants-2026)
6. [Ollama GET /api/version documentation](https://docs.ollama.com/api-reference/get-version)
7. [Ollama Tool Calling documentation](https://docs.ollama.com/capabilities/tool-calling)
8. [Ollama Tool Support blog post](https://ollama.com/blog/tool-support)
9. [Ollama Streaming responses with tool calling blog post](https://ollama.com/blog/streaming-tool)
10. [Ollama API reference (GitHub)](https://github.com/ollama/ollama/blob/main/docs/api.md)
11. [Ollama Tool Calling and Function Execution — DeepWiki](https://deepwiki.com/ollama/ollama/7.2-tool-calling-and-function-execution)
12. [async-openai crate (GitHub)](https://github.com/64bit/async-openai) — v0.35.0 (2026-04-19); not used by BiscuitCode but surveyed for trade-off table.
13. [react-virtuoso](https://virtuoso.dev/react-virtuoso/) — v4.18.5 current.
14. [ollama.com/library/gemma4](https://ollama.com/library/gemma4) — tag verification source.
15. `docs/design/PROVIDER-TRAIT.md` — normalization tables (internal)
16. `docs/design/AGENT-LOOP.md` — executor design (internal)
17. `docs/design/CAPABILITIES.md` — ACL design (internal)
18. `docs/ERROR-CATALOGUE.md` — E007 definition (internal)
19. `docs/PHASE-6-MERGE.md` — Phase 6a TODO list (internal)
20. `tests/fixtures/canonical-tool-prompt.md` — 3-tool fixture (internal)
