//! Row types + ID newtypes.
//!
//! Keep in sync with `docs/CONVERSATION-EXPORT-SCHEMA.md` — the schema
//! the export/import code uses must match what these structs serialize to.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

use biscuitcode_providers::{ContentBlock, MessageRole, ToolCall, ToolResult, Usage};

// ---------- ID newtypes ----------

/// `wks_<ULID>` — globally unique, sortable workspace identifier.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

impl WorkspaceId {
    pub fn new() -> Self {
        Self(format!("wks_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Default for WorkspaceId {
    fn default() -> Self {
        Self::new()
    }
}

/// `conv_<ULID>` — globally unique, sortable conversation identifier.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub String);

impl ConversationId {
    pub fn new() -> Self {
        Self(format!("conv_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Default for ConversationId {
    fn default() -> Self {
        Self::new()
    }
}

/// `msg_<ULID>` — globally unique, sortable message identifier.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl MessageId {
    pub fn new() -> Self {
        Self(format!("msg_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// `snap_<ULID>` — Phase 6b snapshot manifest identifier.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub String);

impl SnapshotId {
    pub fn new() -> Self {
        Self(format!("snap_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Default for SnapshotId {
    fn default() -> Self {
        Self::new()
    }
}

// ---------- Row types ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub workspace_id: WorkspaceId,
    /// Absolute path to the workspace root on disk.
    pub root_path: String,
    pub first_seen_at: DateTime<Utc>,
    /// Optional user-given label (e.g. "Work — backend").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conversation {
    pub conversation_id: ConversationId,
    pub workspace_id: WorkspaceId,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Last selected model — may reference an unavailable provider on
    /// import; UI shows a "select another" notice in that case.
    pub active_model: String,
    /// Leaf message id of the currently-displayed branch in the DAG.
    pub active_branch_message_id: Option<MessageId>,
}

/// Message as stored in the `messages` table. The DAG-shape is encoded
/// via `parent_id` — multiple messages can share a parent (branches
/// happen when the user edits a past user message).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredMessage {
    pub message_id: MessageId,
    pub conversation_id: ConversationId,
    /// None for the conversation's root user message; Some(parent) for all others.
    pub parent_id: Option<MessageId>,
    pub role: MessageRole,
    pub created_at: DateTime<Utc>,
    /// Model name as known at the time this message was generated. May
    /// differ across messages within one conversation if the user
    /// switched models mid-thread.
    pub model: String,
    /// Content blocks — text + mentions + images + thinking. Stored as
    /// a JSON column so the schema doesn't grow with every new block type.
    pub content: Vec<ContentBlock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_results: Vec<ToolResult>,
    /// Foreign key to `snapshots.snapshot_id` for messages whose tool
    /// calls performed Write/Shell tools (Phase 6b).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<SnapshotId>,
    /// Provider-reported usage. None for non-assistant messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: SnapshotId,
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub tool_call_id: String,
    pub tool_name: String,
    pub snapshotted_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub snapshot_id: SnapshotId,
    pub abs_path: String,
    /// None means the file did not exist before the tool ran (rewind = delete).
    pub snapshot_filename: Option<String>,
    pub pre_sha256: Option<String>,
    pub pre_size_bytes: Option<u64>,
    pub pre_existed: bool,
}

// ---------- Errors ----------

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration {version} failed: {reason}")]
    Migration { version: u32, reason: String },

    #[error("schema version {found} ahead of supported max {max} — upgrade BiscuitCode")]
    SchemaTooNew { found: u32, max: u32 },

    #[error("serde_json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("not found: {0}")]
    NotFound(String),
}
