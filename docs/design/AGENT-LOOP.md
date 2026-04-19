# Design — Agent Loop, Tool Surface, Pause/Rewind

> Architecture spec consumed by Phase 6a (read-only tools + executor + Agent Activity UI) and Phase 6b (write tools + inline edit + rewind). Phase 6b's coder must read this end-to-end before touching the snapshot/rewind code — a correctness bug there could delete user files.

## Goals

1. **Predictable agent behavior** — the loop is a plain ReAct cycle, not a chain-of-multiple-agents or a tree-search planner. v1.0 is "the model picks one tool at a time; we run it; we hand back the result; repeat until the model says it's done."
2. **Pausable at human-comprehensible boundaries** — a Pause click stops between iterations; if no tool is currently running, latency under 5s.
3. **Reversible writes** — every write/shell tool snapshots the affected files before running. Rewind = restore the snapshot manifest + truncate later messages.
4. **Workspace as the trust boundary** — every tool's path arguments are validated to be descendants of the open workspace root.
5. **No silent escalation** — write/shell tools always require confirmation unless the user has explicitly opted in to workspace trust.

## Where the code lives

```
biscuitcode-agent/
├── Cargo.toml
└── src/
    ├── lib.rs                  # public re-exports + crate doc
    ├── tools/
    │   ├── mod.rs              # Tool trait, ToolRegistry, ToolSpec
    │   ├── read_file.rs        # Phase 6a
    │   ├── search_code.rs      # Phase 6a
    │   ├── write_file.rs       # Phase 6b
    │   ├── apply_patch.rs      # Phase 6b
    │   └── run_shell.rs        # Phase 6b
    ├── executor/
    │   ├── mod.rs              # ReActExecutor (Phase 6a infrastructure;
    │   │                       #  read-only tool dispatch only)
    │   ├── confirmation.rs     # Phase 6b — write/shell confirmation gate
    │   └── snapshot.rs         # Phase 6b — pre-write file snapshot + rewind
    └── activity/
        └── trace.rs            # performance.mark instrumentation for the
                                # 250ms tool-card-render gate
```

## The `Tool` trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;

    /// Execute the tool. `args` is the JSON the model emitted, already validated
    /// against the schema. `ctx` carries workspace root, conversation id, etc.
    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolCtx,
    ) -> Result<ToolResult, ToolError>;

    /// Side-effect class. Drives the confirmation gate and snapshot policy.
    fn class(&self) -> ToolClass;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolClass {
    Read,        // read_file, search_code — never confirm, never snapshot
    Write,       // write_file, apply_patch — confirm + snapshot before
    Shell,       // run_shell — confirm + snapshot affected paths if knowable
}
```

`ToolSpec` carries the JSON Schema the model sees. We hand-author these — no auto-generation from Rust types — so the schema descriptions are written for the model, not inferred from struct field names.

## The executor (ReAct loop)

```rust
pub struct ReActExecutor {
    registry: Arc<ToolRegistry>,
    pause: Arc<AtomicBool>,
    workspace_root: PathBuf,
    conversation_id: ConversationId,
}

impl ReActExecutor {
    pub async fn run(
        &self,
        provider: Arc<dyn ModelProvider>,
        mut messages: Vec<Message>,
        opts: ChatOptions,
        agent_mode: bool,
    ) -> Result<RunOutcome, ExecutorError> {
        loop {
            // 1. Pause check at iteration boundary.
            if self.pause.load(Ordering::SeqCst) {
                return Ok(RunOutcome::Paused { messages });
            }

            // 2. Stream from provider.
            let tools = if agent_mode { self.registry.specs() } else { vec![] };
            let mut stream = provider.chat_stream(messages.clone(), tools, opts.clone()).await?;
            let assistant_msg = self.consume_stream(&mut stream).await?;
            messages.push(assistant_msg.clone());

            // 3. If no tool calls, we're done.
            if assistant_msg.tool_calls.is_empty() {
                return Ok(RunOutcome::Done { messages });
            }

            // 4. If agent mode is OFF, stop after the first assistant message even
            //    if it contains tool calls. The user must explicitly continue.
            if !agent_mode {
                return Ok(RunOutcome::ToolsAvailable { messages });
            }

            // 5. For each tool call, dispatch (with confirmation if Write/Shell).
            //    Append a `tool` role message with the result.
            for tc in &assistant_msg.tool_calls {
                if self.pause.load(Ordering::SeqCst) {
                    return Ok(RunOutcome::Paused { messages });
                }
                let tool = self.registry.get(&tc.name)
                    .ok_or(ExecutorError::UnknownTool(tc.name.clone()))?;

                let args = serde_json::from_str(&tc.args_json)?;
                let result = self.dispatch(tool.as_ref(), args, tc.id.clone()).await?;
                messages.push(Message::tool_result(tc.id.clone(), result));
            }

            // 6. Loop back — provider will see tool results and may emit more.
        }
    }
}
```

### Pause semantics

- **Single atomic bool** (`Arc<AtomicBool>`) shared with the chat panel's Pause button.
- Checked **at the start of each iteration** AND **between tool dispatches**.
- **Worst-case latency = 5 seconds.** If a tool is mid-execution, we wait for it to finish (we cannot kill arbitrary tool execution mid-flight without risking corrupted state). If no tool is running, the loop's iteration boundary is hit on the next provider event — typically <500ms — but the AC guarantees a 5s upper bound to cover slow networks.
- **Pause is NOT cancellation.** A paused run can be resumed (clears the bool, calls `run` again with the accumulated messages) or stopped (caller drops the future and truncates the conversation).

### Read-only mode (Phase 6a)

Phase 6a ships the executor with ONLY `read_file` and `search_code` registered. Write tools register as stubs that return an error: `"Tool not available in this build (lands in Phase 6b)"`. This way models that try to call `write_file` get a clear signal back, not a 500.

## Confirmation gate (Phase 6b)

```rust
async fn dispatch(
    &self,
    tool: &dyn Tool,
    args: serde_json::Value,
    tool_call_id: String,
) -> Result<ToolResult, ExecutorError> {
    match tool.class() {
        ToolClass::Read => {
            // No confirmation, no snapshot.
            tool.execute(args, &self.ctx()).await.map_err(Into::into)
        }
        ToolClass::Write | ToolClass::Shell => {
            // 1. Workspace-trust shortcut.
            if self.workspace_trusted() {
                return self.dispatch_with_snapshot(tool, args, tool_call_id).await;
            }
            // 2. Build a confirmation prompt (the modal sees this).
            let prompt = build_confirmation(tool, &args, &self.workspace_root)?;
            // 3. Send to frontend, await user decision.
            //    Decision = Approve | Deny | DenyWithFeedback(text).
            let decision = self.frontend.confirm(prompt).await?;
            match decision {
                Decision::Approve => self.dispatch_with_snapshot(tool, args, tool_call_id).await,
                Decision::Deny => Err(ExecutorError::UserDenied),
                Decision::DenyWithFeedback(text) =>
                    Err(ExecutorError::UserDeniedWithFeedback(text)),
            }
        }
    }
}
```

**`run_shell` extra guards** layered before confirmation:
- Reject if first token in `["sudo", "su", "doas"]`. Catalogue `E009`.
- Reject if any arg matches `/curl\s+(?!http:\/\/localhost|https:\/\/api\.(anthropic|openai)\.com)/`. We do NOT permit shell-out HTTP outside the provider scope.
- Reject if any arg contains shell metacharacters (`;`, `&&`, `|`, `>`, backticks) outside of single-quoted strings — we want declarative arg arrays, not shell strings the model assembles.

**`write_file` / `apply_patch` extra guards:**
- Path must be a descendant of `workspace_root`. Catalogue `E002` if not.
- File must NOT match `**/.git/**`, `**/node_modules/**`, `**/target/**`, `**/.cache/**` — the agent never touches these even with workspace-trust on.

## Snapshot + Rewind (Phase 6b)

### Snapshot data model

Snapshots live in `~/.cache/biscuitcode/snapshots/{conversation_id}/{message_id}/`. For each file the tool will modify:

```
~/.cache/biscuitcode/snapshots/conv_<...>/msg_<...>/
├── manifest.json
├── path__src__auth__session.ts.bak    # original contents
└── path__db__migrations__004.sql.bak  # original contents
```

`manifest.json`:

```json
{
  "tool_call_id": "tc_018f9d21...",
  "tool_name": "write_file",
  "snapshotted_at": "2026-04-15T18:11:08.412Z",
  "files": [
    {
      "abs_path": "/home/user/proj/src/auth/session.ts",
      "snapshot_filename": "path__src__auth__session.ts.bak",
      "pre_sha256": "9a1b2c...",
      "pre_size_bytes": 4128,
      "pre_existed": true
    },
    {
      "abs_path": "/home/user/proj/src/auth/new-file.ts",
      "snapshot_filename": null,
      "pre_sha256": null,
      "pre_size_bytes": null,
      "pre_existed": false
    }
  ]
}
```

The `pre_existed: false` case means rewind = delete the file (it didn't exist before).

### Snapshot writing — order matters

```
1. Compute manifest entries for every file the tool will touch.
2. Write each .bak file (copy original contents byte-for-byte, fsync).
3. Write manifest.json (fsync).
4. Only AFTER (3) returns OK: dispatch the tool.
5. If snapshot step (1-3) fails: catalogue E010, do NOT run the tool.
```

The fsync ordering (data files before manifest) ensures that on a crash mid-snapshot, we never have a manifest that references a non-existent .bak file. Worst case is a .bak file with no manifest, which the rewind-cleanup task harmlessly deletes.

### Rewind operation

Triggered by the rewind button on an assistant message that performed write/shell tool calls.

```
1. Load all manifests for messages from the rewind point forward, in reverse
   chronological order (newest first).
2. For each manifest, for each file:
     - If pre_existed: restore from .bak (atomic rename of .bak.tmp → original
       path; verify post-restore sha256 matches pre_sha256; if not → E011).
     - If !pre_existed: delete the file (best-effort — log if missing).
3. Truncate conversation messages from the rewind point forward.
4. Update conversations.active_branch_message_id to the new leaf.
5. Refresh editor models for any open files whose contents changed.
```

If any single restore fails (catalogue `E011 RewindFailed`), the rewind aborts AT THAT POINT — leaves earlier (newer) restores in place, prompts the user with "couldn't fully restore — continue with partial?" yes/no.

### Snapshot retention policy

- Snapshots persist for the conversation's lifetime by default.
- Background cleanup task (Phase 8) deletes snapshot directories whose conversation has been deleted, OR whose snapshots are > 30 days old AND the conversation is closed (no recent activity AND not currently open).
- Setting under `Settings → Conversations → Snapshot retention` to disable cleanup or change the age threshold.
- Disk-space sanity: if `~/.cache/biscuitcode/snapshots/` exceeds 1 GB, the cleanup task runs eagerly regardless of age threshold (oldest snapshots first).

## Tool-card render trace (Phase 6a Global AC)

The 250ms gate `tool_card_visible_<id> - tool_call_start_<id> < 250ms` is enforced via:

1. **Backend emits `performance.mark('tool_call_start_<id>')`** the moment the executor sees a `ToolCallStart` event from the provider. This is BEFORE the tool dispatches — we time the UI render, not the tool execution.
2. **Frontend MutationObserver emits `performance.mark('tool_card_visible_<id>')`** when a card with `data-tool-call-id="<id>"` is added to the Agent Activity DOM.
3. **PerformanceObserver** subscribes to both marks; when both for the same id arrive, computes `performance.measure('tool_card_render', start, visible)`.
4. **e2e test** (`tests/e2e/agent-tool-card-render.spec.ts`) runs the canonical 3-tool prompt (see `tests/fixtures/canonical-tool-prompt.md`) and asserts every measure < 250ms.

The implementation MUST NOT defer card creation to a useEffect that waits on tool result text — the card renders immediately on `ToolCallStart` with a "running" spinner, then updates in place as `ToolCallDelta` and the eventual tool result arrive.

## Conversation persistence (interaction with Phase 5's DB)

Each loop iteration writes to `messages` table:
- The assistant message (after stream completes).
- One `tool` role message per tool call's result.

Snapshot manifests are referenced by foreign key `messages.snapshot_manifest_id` (nullable; only set for messages whose tool calls were Write/Shell class). Manifest rows live in a separate `snapshots` table for efficient cleanup queries.

## Things explicitly NOT in scope for v1

- **Tool parallelism.** Each tool call dispatches sequentially. Future work could parallelize Read-class tools.
- **Tool results > 256KB.** Truncated with a marker; the model is told via the result text. Larger payloads are a v1.1 concern (chunked context, summarization).
- **Cross-conversation context sharing.** Each conversation is independent; the agent loop only sees the current conversation's messages.
- **Mid-stream tool-call cancellation.** Once a tool starts executing, it runs to completion. Pause works between tools, not within them.
- **Multi-step planning** (the model proposing a plan, the user approving/editing, then execution). v1's loop is one-tool-at-a-time. Multi-step plan UX is a v1.1 enhancement.
