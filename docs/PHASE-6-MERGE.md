# Phase 6 — Merging the agent crate + OpenAI/Ollama providers

> Read this when starting Phase 6a (OpenAI + Ollama providers, read-only tool surface, Agent Activity UI). Phase 6b extends the same `biscuitcode-agent` crate with write tools, snapshot/rewind, and inline edit — its TODO blocks are marked inline in the source. **Read `docs/design/AGENT-LOOP.md` BEFORE 6b** — a correctness bug in rewind could delete user files.

## Pre-staged files (Phase 6 foundation)

The `biscuitcode-agent` crate is already created in the workspace (skeletoned alongside the Phase 5 foundation). The OpenAI and Ollama providers are skeletons inside the existing `biscuitcode-providers` crate.

| Path | What | Phase |
|---|---|---|
| `src-tauri/biscuitcode-agent/Cargo.toml` | Member crate: depends on core + providers + db, async-trait, tokio (fs), futures, globset, ignore, sha2, chrono | 6a |
| `src-tauri/biscuitcode-agent/src/lib.rs` | Re-exports `executor` + `tools` modules | 6a |
| `src-tauri/biscuitcode-agent/src/tools/mod.rs` | `Tool` trait + `ToolClass` enum + `ToolCtx` (with `is_inside_workspace` impl'd) + `ToolRegistry` (with `read_only_default()`) + `ToolError` (with `NotYetAvailable` variant for write-tool stubs) | 6a |
| `src-tauri/biscuitcode-agent/src/tools/read_file.rs` | **Fully implemented** including `ToolSpec`, args parsing, workspace-scope check, 256KB truncation, lossy UTF-8. | 6a |
| `src-tauri/biscuitcode-agent/src/tools/search_code.rs` | `ToolSpec` + args struct done; `execute` body is a TODO block (see `// ---- Phase 6a coder fills in ----`) | 6a |
| `src-tauri/biscuitcode-agent/src/executor/mod.rs` | `ReActExecutor` with full `run` loop, pause flag (5s no-tool-running latency budget enforced), `consume_stream` parses every `ChatEvent` variant. Read-class dispatch works; Write/Shell dispatch returns `NotYetAvailable` (Phase 6b fills in). | 6a (read path) / 6b (write path) |
| `src-tauri/biscuitcode-providers/src/openai/mod.rs` | `OpenAIProvider` skeleton with `list_models()` curated set; `chat_stream` returns `not_implemented` until filled in | 6a |
| `src-tauri/biscuitcode-providers/src/ollama/mod.rs` | `OllamaProvider` skeleton; `list_models` and `chat_stream` are TODO blocks; doc-comment locks the verified Gemma 4 tag list | 6a |

**Already added to the workspace** as part of the Phase 5 foundation merge — no new `[workspace] members` change needed for `biscuitcode-providers`. Add `biscuitcode-agent` if Phase 5 didn't already:

```toml
[workspace]
members = [
    ".",
    "biscuitcode-core",
    "biscuitcode-providers",
    "biscuitcode-db",
    "biscuitcode-pty",
    "biscuitcode-agent",          # add if not present
]
```

In top-level `src-tauri/Cargo.toml`'s `[dependencies]`:

```toml
biscuitcode-agent = { path = "biscuitcode-agent" }
```

## What's still TODO for the Phase 6a coder

### 1. Implement `OpenAIProvider::chat_stream`

Per the doc-comment in `src-tauri/biscuitcode-providers/src/openai/mod.rs`:

- POST `https://api.openai.com/v1/chat/completions` with `stream: true`. Auth: `Authorization: Bearer <api_key>`.
- SSE parsing: each `data: {...}` chunk has `choices[0].delta.{content, tool_calls}`.
- **Per-index `tool_calls[i].index` accumulation** — the OpenAI quirk. A single delta may carry partial args for multiple tool calls; key by `index`, accumulate `arguments` strings until `finish_reason === "tool_calls"`, then emit `ToolCallStart + ToolCallDelta* + ToolCallEnd` into the `ChatEvent` stream.
- **Reasoning models** (`gpt-5.4-pro`): emit `ThinkingDelta` events while reasoning, no `TextDelta` until reasoning finishes (3–30 s). UI shows `Thinking…`. **Exempt from TTFT < 500 ms gate.**
- Map errors per the same table as Anthropic: 401 → `AuthInvalid`, 429 → `RateLimited`, etc.

### 2. Implement `OllamaProvider::list_models` + `chat_stream`

`list_models`:
- `GET <base_url>/api/tags`. Translate each entry to `ModelInfo`.
- Mark `gemma3:*` as legacy when any `gemma4:*` is also present.
- Mark `gemma4:*` with `supports_vision = true`, `is_reasoning_model = false`.
- Mark `qwen2.5-coder:*` with `supports_tools = true` (proven tool-calling stability).
- On daemon-down: `ProviderError::OllamaDaemonDown { endpoint }`.

`chat_stream`:
- POST `<base_url>/api/chat` with `stream: true` (NDJSON, **not SSE** — line-delimited JSON, one object per line).
- `tools` passthrough in OpenAI-function-call format.
- Emit `ChatEvent::TextDelta` for each `message.content` chunk.
- On the final non-done chunk, extract `message.tool_calls` and emit `ToolCallStart/End` pairs.
- **XML-tag fallback:** if `tool_calls` is empty AND `message.content` contains `<tool_call>...</tool_call>`, regex-extract and synthesize a `ToolCallStart/End` pair. (Common with Gemma 3 community fine-tunes; **not needed for Gemma 4** — defensive parsing only.)

### 3. Implement `SearchCodeTool::execute`

In `src-tauri/biscuitcode-agent/src/tools/search_code.rs`. Steps in the inline comment:

1. Build a `globset::GlobMatcher` from `args.glob` (default = match all).
2. Walk `ctx.workspace_root` via `ignore::WalkBuilder` (respects `.gitignore`, `.ignore`, hidden-file rules).
3. For each file, scan for `args.query` (substring or regex per `args.regex`). Skip binary files (heuristic: leading null byte in first 8 KB).
4. Format matches grouped by file: `<file>:<line>: <line content trimmed>`.
5. Truncate at `ctx.max_result_bytes`; set `truncated: true` if hit.

### 4. Ollama install flow + RAM-tier auto-pull

Per plan deliverables:

- `ollama_install()`: detect via `curl -sSfm 1 http://localhost:11434/api/version` AND `which ollama`. On missing, show confirm dialog with the verbatim command `curl -fsSL https://ollama.com/install.sh | sh`; run via `plugin-shell` **only after user confirms**.
- `ollama_pull(model)`: stream stdout from `ollama pull` to a frontend progress bar.
- RAM detection via `sysinfo` crate; pick the tier from the verified Gemma 4 table (`docs/plan.md` Phase 6a deliverables).
- On `ollama pull` 404 (Gemma 4 tag unrecognized → Ollama < 0.20.0): fall back to Gemma 3 ladder, fire `E007 GemmaVersionFallback` toast with the upgrade-Ollama install command.

### 5. Wire Tauri commands for the agent loop

`src-tauri/src/main.rs`:

```rust
#[tauri::command]
async fn agent_run(
    registry: tauri::State<'_, Arc<ToolRegistry>>,
    /* db, providers, conversation_id, agent_mode, ... */
) -> Result<RunOutcome, String> { ... }

#[tauri::command]
fn agent_pause(pause: tauri::State<'_, Arc<AtomicBool>>) {
    pause.store(true, Ordering::SeqCst);
}
```

Manage `Arc::new(ToolRegistry::read_only_default())` and the pause flag as Tauri state.

### 6. Capabilities updates

`src-tauri/capabilities/http.json` — add to fetch allowlist:
- `https://api.openai.com/**`
- `http://localhost:11434/**`

`src-tauri/capabilities/shell.json` — add `ollama` to the command registry, argument regex limited to `pull <model>`, `list`, `show <model>`, `serve`, `--version` (NEVER wildcard args).

### 7. Replace Phase 2 `AgentActivityPanel.tsx` shell with the real virtualized panel

Per plan: collapsible cards (running/ok/error status, timing, pretty-JSON args, streamed result), `react-virtuoso` for virtualization, badge on chat message links to the card.

**Tool-card render trace instrumentation:**
- On `ToolCallStart`: `performance.mark('tool_call_start_<id>')`.
- When the card's MutationObserver fires for first paint: `performance.mark('tool_card_visible_<id>')`.
- Persist measures in a debug log; the gate test (`tests/e2e/agent-tool-card-render.spec.ts`) asserts `< 250 ms` over the canonical 3-tool fixture.

### 8. Chat context mentions (editor-local subset)

Typing `@` in chat input opens picker for `@file` (fuzzy over workspace tree), `@folder`, `@selection`. Each resolves to a structured context block in the user message. Drag-file-into-chat: dropping a file from the tree onto the chat input inserts an `@file:<path>` token.

(Non-editor mentions — `@terminal-output`, `@problems`, `@git-diff` — land in Phase 7.)

### 9. Register error code `E007 GemmaVersionFallback`

Rust enum + TS union + en bundle key + trigger test. The trigger forces the Ollama version-too-old fallback path and asserts the catalogued toast renders.

### 10. Run the Phase 6a ACs

The agent-mode demo is the gate: **with agent mode ON, sending the prompt `"List every file under src/ that contains the string TODO and summarize each TODO in one sentence"` to Anthropic produces (1) a `search_code` tool call, (2) a `read_file` per match, (3) a final summary message.** Repeat against Ollama with a Gemma 4 model — same tool sequence, possibly different timing. Test file: `tests/e2e/agent-mode-demo.spec.ts`.

## Phase 6b — what's pre-staged

The Phase 6b coder reads `docs/design/AGENT-LOOP.md` first (snapshot manifest fsync ordering, rewind correctness, workspace-trust shortcut). The crate already has:

- `ToolClass::Write` and `ToolClass::Shell` variants
- `ToolError::NotYetAvailable` variant returned by the executor for write/shell calls
- `executor::dispatch` has a `Phase 6b coder fills in` block at the Write/Shell match arm — confirm + snapshot + execute + persist manifest

What 6b adds that does NOT exist yet:
- `tools/write_file.rs`, `tools/apply_patch.rs`, `tools/run_shell.rs`
- `executor/confirmation.rs` (per-tool gate + workspace-trust shortcut)
- `executor/snapshot.rs` (pre-write snapshot, manifest fsync, rewind)
- Frontend confirmation modals + diff preview
- Inline edit (`Ctrl+K Ctrl+I`) using `monaco.editor.createDiffEditor`
- Rewind UI in conversation header
- New error codes: `E008` `E009` `E010` `E011`
