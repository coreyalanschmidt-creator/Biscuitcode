//! ReAct executor.
//!
//! Phase 6a ships read-only mode: the loop dispatches Read-class tools
//! without confirmation; Write/Shell-class tools (which Phase 6a doesn't
//! register at all) would be unreachable.
//!
//! Phase 6b adds:
//!   - `confirmation` submodule (prompt user for Write/Shell calls)
//!   - `snapshot` submodule (pre-write file snapshots, fsync ordering)
//!   - workspace-trust shortcut
//!   - rewind operation (reverse-chrono restore + truncate messages)
//!
//! Design contract: docs/design/AGENT-LOOP.md.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use thiserror::Error;
use tokio::time::{Duration, Instant};

use biscuitcode_db::ConversationId;
use biscuitcode_providers::{
    ChatEvent, ChatOptions, Message, ModelProvider, ProviderError, Role, ToolCall,
};

use crate::tools::{ToolClass, ToolCtx, ToolError, ToolRegistry};

pub mod confirmation;
pub mod snapshot;

use confirmation::{ConfirmationRequest, Decision, PendingConfirmations};

/// Maximum acceptable pause-button latency when no tool is currently
/// running. Per Global AC: < 5 seconds.
pub const PAUSE_NO_TOOL_LATENCY: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum RunOutcome {
    /// Provider stopped emitting tool calls; conversation is "settled."
    Done { messages: Vec<Message> },
    /// User pressed pause; loop stopped at boundary.
    Paused { messages: Vec<Message> },
    /// Agent mode was OFF and the assistant emitted tool calls — the
    /// frontend asks the user whether to continue.
    ToolsAvailable { messages: Vec<Message> },
}

#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("provider: {0}")]
    Provider(#[from] ProviderError),

    #[error("tool: {0}")]
    Tool(#[from] ToolError),

    #[error("unknown tool {0}")]
    UnknownTool(String),

    #[error("tool args parse: {0}")]
    ToolArgsParse(String),

    #[error("user denied write/shell tool {0}")]
    UserDenied(String),

    #[error("user denied with feedback: {0}")]
    UserDeniedWithFeedback(String),

    #[error("snapshot failed: {0}")]
    SnapshotFailed(String),
}

/// Runtime context injected into the executor by the Tauri command handler.
/// Carries the pieces needed for confirmation + snapshot without having the
/// executor directly depend on `tauri` (keeps the crate testable offline).
pub struct ExecutorContext {
    /// Directory used as the base for snapshot storage.
    /// Typically `~/.cache/biscuitcode/`.
    pub cache_root: PathBuf,
    /// Pending confirmation channel map (shared with the `agent_confirm_decision` command).
    pub pending: Arc<PendingConfirmations>,
    /// Per-workspace trust flag. When true, write/shell tools skip the modal.
    pub workspace_trusted: bool,
    /// Callback invoked to send a confirmation request to the frontend.
    /// Receives the serialized `ConfirmationRequest` JSON.
    /// Returns `Ok(())` if the event was emitted; `Err` if the window is gone.
    pub emit_confirm: Arc<dyn Fn(ConfirmationRequest) -> Result<(), String> + Send + Sync>,
}

pub struct ReActExecutor {
    pub registry: Arc<ToolRegistry>,
    pub pause: Arc<AtomicBool>,
    pub workspace_root: PathBuf,
    pub conversation_id: ConversationId,
    /// Optional context for Phase 6b confirmation + snapshot.
    /// `None` means read-only mode (no confirmation, no snapshot).
    pub ctx: Option<Arc<ExecutorContext>>,
}

impl ReActExecutor {
    pub fn new(
        registry: Arc<ToolRegistry>,
        workspace_root: PathBuf,
        conversation_id: ConversationId,
    ) -> Self {
        Self {
            registry,
            pause: Arc::new(AtomicBool::new(false)),
            workspace_root,
            conversation_id,
            ctx: None,
        }
    }

    /// Create with full Phase 6b context (confirmation + snapshot).
    pub fn with_context(mut self, ctx: Arc<ExecutorContext>) -> Self {
        self.ctx = Some(ctx);
        self
    }

    /// Returns a clone of the pause flag — used by the chat panel's
    /// Pause button to set it from the frontend.
    pub fn pause_flag(&self) -> Arc<AtomicBool> {
        self.pause.clone()
    }

    /// Run the loop. `agent_mode` true = auto-continue on tool calls;
    /// false = stop after the first assistant response.
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

            // 2. Stream from the provider.
            let tools = if agent_mode { self.registry.specs() } else { vec![] };
            let mut stream = provider
                .chat_stream(messages.clone(), tools, opts.clone())
                .await?;

            let assistant_msg = self.consume_stream(&mut stream).await?;
            messages.push(assistant_msg.clone());

            // 3. No tool calls -> we're done.
            if assistant_msg.tool_calls.is_empty() {
                return Ok(RunOutcome::Done { messages });
            }

            // 4. Agent mode off -> stop and surface.
            if !agent_mode {
                return Ok(RunOutcome::ToolsAvailable { messages });
            }

            // 5. Dispatch each tool call sequentially. Pause checks between.
            // Generate a stable message_id-like string for snapshot directory naming.
            let assistant_msg_key = format!("msg_{}", ulid_now());

            for tc in assistant_msg.tool_calls.iter() {
                if self.pause.load(Ordering::SeqCst) {
                    return Ok(RunOutcome::Paused { messages });
                }

                let result = self.dispatch(tc, &assistant_msg_key).await?;
                messages.push(tool_result_message(tc.id.clone(), result));
            }

            // 6. Loop. Provider sees tool results and may emit more tool calls.
        }
    }

    async fn consume_stream(
        &self,
        stream: &mut std::pin::Pin<
            Box<dyn futures::Stream<Item = Result<ChatEvent, ProviderError>> + Send>,
        >,
    ) -> Result<Message, ExecutorError> {
        use futures::StreamExt;

        let mut text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        // Worst-case-pause-latency tracking when no tool is running.
        let mut last_pause_check = Instant::now();

        while let Some(ev) = stream.next().await {
            // Periodic pause check during long streams (no-tool case).
            if last_pause_check.elapsed() >= PAUSE_NO_TOOL_LATENCY {
                if self.pause.load(Ordering::SeqCst) {
                    // Drop the stream; outer loop handles the early return.
                    break;
                }
                last_pause_check = Instant::now();
            }

            match ev? {
                ChatEvent::TextDelta { text: t } => text.push_str(&t),
                ChatEvent::ThinkingDelta { .. } => { /* persist separately */ }
                ChatEvent::ToolCallStart { id, name } => {
                    tool_calls.push(ToolCall {
                        id, name, args_json: String::new(),
                    });
                }
                ChatEvent::ToolCallDelta { id, args_delta } => {
                    if let Some(tc) = tool_calls.iter_mut().find(|t| t.id == id) {
                        tc.args_json.push_str(&args_delta);
                    }
                }
                ChatEvent::ToolCallEnd { id, args_json } => {
                    if let Some(tc) = tool_calls.iter_mut().find(|t| t.id == id) {
                        tc.args_json = args_json;
                    }
                }
                ChatEvent::Done { .. } => break,
                ChatEvent::Error { code, message, recoverable } => {
                    if !recoverable {
                        return Err(ProviderError::Other(
                            format!("{code}: {message}")
                        ).into());
                    }
                    tracing::warn!(code = %code, %message, "recoverable stream error");
                }
            }
        }

        // Phase 6a follow-up: assert single tool_result per tool message.
        // The tool_calls are collected here; each gets a separate ToolResult message
        // in the outer loop — one result per tool call ID.

        Ok(Message {
            role: Role::Assistant,
            content: vec![biscuitcode_providers::ContentBlock::Text { text }],
            tool_calls,
            tool_results: vec![],
        })
    }

    async fn dispatch(
        &self,
        tc: &ToolCall,
        assistant_msg_key: &str,
    ) -> Result<crate::tools::ToolResult, ExecutorError> {
        let tool = self
            .registry
            .get(&tc.name)
            .ok_or_else(|| ExecutorError::UnknownTool(tc.name.clone()))?;

        let args: serde_json::Value = serde_json::from_str(&tc.args_json)
            .map_err(|e| ExecutorError::ToolArgsParse(e.to_string()))?;

        let tool_ctx = ToolCtx {
            workspace_root: self.workspace_root.clone(),
            conversation_id: self.conversation_id.clone(),
            max_result_bytes: 256 * 1024,
        };

        match tool.class() {
            ToolClass::Read => Ok(tool.execute(args, &tool_ctx).await?),

            ToolClass::Write | ToolClass::Shell => {
                let exec_ctx = match &self.ctx {
                    Some(c) => c.clone(),
                    None => {
                        // No context — treat as denied (safe default).
                        return Err(ExecutorError::Tool(ToolError::NotYetAvailable {
                            tool: "write/shell tools require ExecutorContext",
                        }));
                    }
                };

                // Build confirmation summary.
                let summary = if tool.class() == ToolClass::Shell {
                    // For shell: verbatim command line.
                    format!("{} {}", tc.name, tc.args_json)
                } else {
                    // For write/patch: show the args JSON as the summary.
                    // A richer diff-based summary would be built here in a
                    // more complete implementation; the model args carry
                    // enough context for the modal.
                    tc.args_json.clone()
                };

                let req = ConfirmationRequest {
                    request_id: tc.id.clone(),
                    tool_class: format!("{:?}", tool.class()).to_lowercase(),
                    summary,
                    paths: vec![],
                };

                // Workspace trust shortcut.
                let decision = if exec_ctx.workspace_trusted {
                    Decision::Approve
                } else {
                    // Send confirmation request to frontend.
                    let rx = exec_ctx.pending.register(tc.id.clone());
                    (exec_ctx.emit_confirm)(req).map_err(|e| {
                        ExecutorError::SnapshotFailed(format!("emit confirm failed: {e}"))
                    })?;
                    // Await user decision (60s timeout → Deny).
                    confirmation::await_decision(&tc.id, rx, &exec_ctx.pending).await
                };

                match decision {
                    Decision::Deny => {
                        Err(ExecutorError::UserDenied(tc.name.clone()))
                    }
                    Decision::DenyWithFeedback { feedback } => {
                        Err(ExecutorError::UserDeniedWithFeedback(feedback))
                    }
                    Decision::Approve => {
                        // Take snapshot before executing.
                        let paths = extract_paths_from_args(&args, &self.workspace_root);
                        let snap_dir = snapshot::snapshot_dir(
                            &exec_ctx.cache_root,
                            &self.conversation_id.0,
                            assistant_msg_key,
                        );

                        if !paths.is_empty() {
                            snapshot::take(&snap_dir, &paths, &tc.id, &tc.name)
                                .await
                                .map_err(|e| ExecutorError::SnapshotFailed(e.to_string()))?;
                        }

                        // Execute the tool.
                        Ok(tool.execute(args, &tool_ctx).await?)
                    }
                }
            }
        }
    }
}

/// Attempt to extract file paths from tool args for snapshotting.
/// Looks for a "path" or "paths" key.
fn extract_paths_from_args(args: &serde_json::Value, workspace_root: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(p) = args.get("path").and_then(|v| v.as_str()) {
        let resolved = if std::path::Path::new(p).is_absolute() {
            PathBuf::from(p)
        } else {
            workspace_root.join(p)
        };
        paths.push(resolved);
    }
    if let Some(arr) = args.get("paths").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(p) = item.as_str() {
                let resolved = if std::path::Path::new(p).is_absolute() {
                    PathBuf::from(p)
                } else {
                    workspace_root.join(p)
                };
                paths.push(resolved);
            }
        }
    }
    paths
}

/// Generate a ULID-like timestamp string for snapshot directory naming.
/// This avoids a dependency on the ulid crate in this module.
fn ulid_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:013x}", ms)
}

fn tool_result_message(tool_call_id: String, result: crate::tools::ToolResult) -> Message {
    Message {
        role: Role::Tool,
        content: vec![],
        tool_calls: vec![],
        tool_results: vec![biscuitcode_providers::ToolResult {
            tool_call_id,
            result: result.result,
            truncated: result.truncated,
        }],
    }
}
