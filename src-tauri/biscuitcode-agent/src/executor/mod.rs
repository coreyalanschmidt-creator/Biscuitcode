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

    #[error("user denied with feedback")]
    UserDeniedWithFeedback(String),
}

pub struct ReActExecutor {
    pub registry: Arc<ToolRegistry>,
    pub pause: Arc<AtomicBool>,
    pub workspace_root: PathBuf,
    pub conversation_id: ConversationId,
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
        }
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
            for tc in assistant_msg.tool_calls.iter() {
                if self.pause.load(Ordering::SeqCst) {
                    return Ok(RunOutcome::Paused { messages });
                }

                let result = self.dispatch(tc).await?;
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

        Ok(Message {
            role: Role::Assistant,
            content: vec![biscuitcode_providers::ContentBlock::Text { text }],
            tool_calls,
            tool_results: vec![],
        })
    }

    async fn dispatch(&self, tc: &ToolCall) -> Result<crate::tools::ToolResult, ExecutorError> {
        let tool = self
            .registry
            .get(&tc.name)
            .ok_or_else(|| ExecutorError::UnknownTool(tc.name.clone()))?;

        let args: serde_json::Value = serde_json::from_str(&tc.args_json)
            .map_err(|e| ExecutorError::ToolArgsParse(e.to_string()))?;

        let ctx = ToolCtx {
            workspace_root: self.workspace_root.clone(),
            conversation_id: self.conversation_id.clone(),
            max_result_bytes: 256 * 1024,
        };

        match tool.class() {
            ToolClass::Read => Ok(tool.execute(args, &ctx).await?),

            ToolClass::Write | ToolClass::Shell => {
                // ---- Phase 6b coder fills in ----
                // 1. confirmation::ask(...) -> Decision (Approve | Deny | DenyWithFeedback)
                // 2. if Approve -> snapshot::take(...) before tool.execute(...)
                // 3. After execute, persist snapshot manifest + link to assistant msg
                // 4. on Deny -> return UserDenied; do NOT execute
                Err(ExecutorError::Tool(ToolError::NotYetAvailable {
                    tool: "write/shell tools land in Phase 6b",
                }))
            }
        }
    }
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
