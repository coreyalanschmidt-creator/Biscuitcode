//! High-level query helpers used by the Tauri chat commands.
//!
//! Phase 5 deliverable. Low-level rusqlite calls; no ORM.
//! All JSON columns are encoded/decoded via serde_json.

use chrono::Utc;
use rusqlite::{params, Row};

use crate::types::{
    Conversation, ConversationId, DbError, MessageId, Snapshot, SnapshotFile, SnapshotId,
    StoredMessage, WorkspaceId,
};
use crate::Database;
use biscuitcode_providers::{ContentBlock, MessageRole, ToolCall, ToolResult, Usage};

// ---------- Workspace helpers ----------

impl Database {
    /// Upsert a workspace by root_path. Returns the existing id if already known.
    pub fn upsert_workspace(&mut self, root_path: &str) -> Result<WorkspaceId, DbError> {
        // Check if it exists.
        let existing: Option<WorkspaceId> = self
            .conn
            .query_row(
                "SELECT workspace_id FROM workspaces WHERE root_path = ?1",
                params![root_path],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(WorkspaceId);

        if let Some(id) = existing {
            return Ok(id);
        }

        let id = WorkspaceId::new();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, first_seen_at) VALUES (?1, ?2, ?3)",
            params![id.0, root_path, now],
        )?;
        Ok(id)
    }
}

// ---------- Conversation helpers ----------

impl Database {
    /// Create a new conversation. Returns the created `Conversation`.
    pub fn create_conversation(
        &mut self,
        workspace_id: &WorkspaceId,
        title: &str,
        active_model: &str,
    ) -> Result<Conversation, DbError> {
        let id = ConversationId::new();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO conversations \
             (conversation_id, workspace_id, title, created_at, updated_at, active_model) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id.0, workspace_id.0, title, now, now, active_model],
        )?;
        Ok(Conversation {
            conversation_id: id,
            workspace_id: workspace_id.clone(),
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            active_model: active_model.to_string(),
            active_branch_message_id: None,
        })
    }

    /// List conversations for a workspace, ordered by `updated_at DESC`.
    pub fn list_conversations(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Conversation>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT conversation_id, workspace_id, title, created_at, updated_at, \
             active_model, active_branch_message_id \
             FROM conversations WHERE workspace_id = ?1 ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![workspace_id.0], row_to_conversation)?;
        rows.map(|r| r.map_err(DbError::from)).collect()
    }

    /// Update `active_model` and bump `updated_at`.
    pub fn update_conversation_model(
        &self,
        conversation_id: &ConversationId,
        active_model: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE conversations SET active_model = ?1, updated_at = ?2 \
             WHERE conversation_id = ?3",
            params![active_model, now, conversation_id.0],
        )?;
        Ok(())
    }

    /// Bump `updated_at` and optionally set `active_branch_message_id`.
    pub fn touch_conversation(
        &self,
        conversation_id: &ConversationId,
        active_branch_message_id: Option<&MessageId>,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let leaf_id = active_branch_message_id.map(|m| m.0.as_str());
        self.conn.execute(
            "UPDATE conversations SET updated_at = ?1, active_branch_message_id = ?2 \
             WHERE conversation_id = ?3",
            params![now, leaf_id, conversation_id.0],
        )?;
        Ok(())
    }
}

// ---------- Message helpers ----------

impl Database {
    /// Append a message to a conversation. Returns the stored message.
    #[allow(clippy::too_many_arguments)]
    pub fn append_message(
        &mut self,
        conversation_id: &ConversationId,
        parent_id: Option<&MessageId>,
        role: MessageRole,
        model: &str,
        content: &[ContentBlock],
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
        usage: Option<&Usage>,
    ) -> Result<StoredMessage, DbError> {
        let id = MessageId::new();
        let now = Utc::now().to_rfc3339();
        let content_json = serde_json::to_string(content)?;
        let tool_calls_json = serde_json::to_string(tool_calls)?;
        let tool_results_json = serde_json::to_string(tool_results)?;
        let usage_json = usage.map(serde_json::to_string).transpose()?;
        let parent_str = parent_id.map(|m| m.0.as_str());

        self.conn.execute(
            "INSERT INTO messages \
             (message_id, conversation_id, parent_id, role, created_at, model, \
              content_json, tool_calls_json, tool_results_json, usage_json) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id.0,
                conversation_id.0,
                parent_str,
                role_to_str(role),
                now,
                model,
                content_json,
                tool_calls_json,
                tool_results_json,
                usage_json,
            ],
        )?;

        // Bump conversation leaf.
        self.touch_conversation(conversation_id, Some(&id))?;

        Ok(StoredMessage {
            message_id: id,
            conversation_id: conversation_id.clone(),
            parent_id: parent_id.cloned(),
            role,
            created_at: Utc::now(),
            model: model.to_string(),
            content: content.to_vec(),
            tool_calls: tool_calls.to_vec(),
            tool_results: tool_results.to_vec(),
            snapshot_id: None,
            usage: usage.cloned(),
        })
    }

    /// Load all messages for a conversation, ordered by `created_at`.
    pub fn list_messages(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<Vec<StoredMessage>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT message_id, conversation_id, parent_id, role, created_at, model, \
             content_json, tool_calls_json, tool_results_json, snapshot_id, usage_json \
             FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![conversation_id.0], row_to_message)?;
        rows.map(|r| r.map_err(DbError::from)).collect()
    }
}

// ---------- Row decoders ----------

fn row_to_conversation(row: &Row) -> rusqlite::Result<Conversation> {
    use chrono::DateTime;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;
    let abmi: Option<String> = row.get(6)?;
    Ok(Conversation {
        conversation_id: ConversationId(row.get(0)?),
        workspace_id: WorkspaceId(row.get(1)?),
        title: row.get(2)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        active_model: row.get(5)?,
        active_branch_message_id: abmi.map(MessageId),
    })
}

fn row_to_message(row: &Row) -> rusqlite::Result<StoredMessage> {
    use crate::types::{MessageId, SnapshotId};
    use chrono::DateTime;

    let created_at_str: String = row.get(4)?;
    let content_json: String = row.get(6)?;
    let tc_json: String = row.get(7)?;
    let tr_json: String = row.get(8)?;
    let snap_id: Option<String> = row.get(9)?;
    let usage_json: Option<String> = row.get(10)?;
    let role_str: String = row.get(3)?;
    let parent_str: Option<String> = row.get(2)?;

    Ok(StoredMessage {
        message_id: MessageId(row.get(0)?),
        conversation_id: ConversationId(row.get(1)?),
        parent_id: parent_str.map(MessageId),
        role: str_to_role(&role_str),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        model: row.get(5)?,
        content: serde_json::from_str(&content_json).unwrap_or_default(),
        tool_calls: serde_json::from_str(&tc_json).unwrap_or_default(),
        tool_results: serde_json::from_str(&tr_json).unwrap_or_default(),
        snapshot_id: snap_id.map(SnapshotId),
        usage: usage_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok()),
    })
}

fn role_to_str(role: MessageRole) -> &'static str {
    match role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
        MessageRole::System => "system",
    }
}

fn str_to_role(s: &str) -> MessageRole {
    match s {
        "assistant" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        "system" => MessageRole::System,
        _ => MessageRole::User,
    }
}

// ---------- Snapshot helpers (Phase 6b) ----------

impl Database {
    /// Insert a snapshot manifest row and its file entries.
    /// Returns the generated `SnapshotId`.
    pub fn insert_snapshot(
        &mut self,
        conversation_id: &ConversationId,
        message_id: &MessageId,
        tool_call_id: &str,
        tool_name: &str,
        files: &[SnapshotFile],
    ) -> Result<SnapshotId, DbError> {
        let id = SnapshotId::new();
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO snapshots \
             (snapshot_id, conversation_id, message_id, tool_call_id, tool_name, snapshotted_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id.0,
                conversation_id.0,
                message_id.0,
                tool_call_id,
                tool_name,
                now,
            ],
        )?;

        for f in files {
            self.conn.execute(
                "INSERT INTO snapshot_files \
                 (snapshot_id, abs_path, snapshot_filename, pre_sha256, pre_size_bytes, pre_existed) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    id.0,
                    f.abs_path,
                    f.snapshot_filename,
                    f.pre_sha256,
                    f.pre_size_bytes.map(|s| s as i64),
                    i64::from(f.pre_existed),
                ],
            )?;
        }

        Ok(id)
    }

    /// Link a snapshot to an assistant message (set `messages.snapshot_id`).
    pub fn link_snapshot_to_message(
        &self,
        message_id: &MessageId,
        snapshot_id: &SnapshotId,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE messages SET snapshot_id = ?1 WHERE message_id = ?2",
            params![snapshot_id.0, message_id.0],
        )?;
        Ok(())
    }

    /// Load snapshots for a conversation ordered newest-first (for rewind).
    pub fn list_snapshots(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<Vec<Snapshot>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT snapshot_id, conversation_id, message_id, tool_call_id, tool_name, snapshotted_at \
             FROM snapshots WHERE conversation_id = ?1 ORDER BY snapshotted_at DESC",
        )?;
        let rows = stmt.query_map(params![conversation_id.0], |row| {
            use chrono::DateTime;
            let snapshotted_at_str: String = row.get(5)?;
            Ok(Snapshot {
                snapshot_id: SnapshotId(row.get(0)?),
                conversation_id: ConversationId(row.get(1)?),
                message_id: MessageId(row.get(2)?),
                tool_call_id: row.get(3)?,
                tool_name: row.get(4)?,
                snapshotted_at: DateTime::parse_from_rfc3339(&snapshotted_at_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.map(|r| r.map_err(DbError::from)).collect()
    }

    /// Load snapshots for a conversation from a given message's timestamp forward
    /// (i.e., all snapshots at or after the rewind point), newest-first.
    pub fn list_snapshots_from_message(
        &self,
        conversation_id: &ConversationId,
        from_message_id: &MessageId,
    ) -> Result<Vec<Snapshot>, DbError> {
        // Find the `created_at` of the from_message.
        let from_created: Option<String> = self
            .conn
            .query_row(
                "SELECT created_at FROM messages WHERE message_id = ?1",
                params![from_message_id.0],
                |row| row.get(0),
            )
            .optional()?;

        let from_created = match from_created {
            Some(t) => t,
            None => return Ok(vec![]),
        };

        let mut stmt = self.conn.prepare(
            "SELECT snapshot_id, conversation_id, message_id, tool_call_id, tool_name, snapshotted_at \
             FROM snapshots WHERE conversation_id = ?1 AND snapshotted_at >= ?2 \
             ORDER BY snapshotted_at DESC",
        )?;
        let rows = stmt.query_map(params![conversation_id.0, from_created], |row| {
            use chrono::DateTime;
            let snapshotted_at_str: String = row.get(5)?;
            Ok(Snapshot {
                snapshot_id: SnapshotId(row.get(0)?),
                conversation_id: ConversationId(row.get(1)?),
                message_id: MessageId(row.get(2)?),
                tool_call_id: row.get(3)?,
                tool_name: row.get(4)?,
                snapshotted_at: DateTime::parse_from_rfc3339(&snapshotted_at_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.map(|r| r.map_err(DbError::from)).collect()
    }

    /// Delete a snapshot row and its file entries (CASCADE in schema).
    pub fn delete_snapshot(&self, snapshot_id: &SnapshotId) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM snapshots WHERE snapshot_id = ?1",
            params![snapshot_id.0],
        )?;
        Ok(())
    }

    /// Truncate messages after (not including) `from_message_id` in a conversation.
    /// Deletes messages whose `parent_id` chain leads through `from_message_id` —
    /// simpler implementation: delete all messages created after `from_message_id`.
    pub fn truncate_messages_after(
        &mut self,
        conversation_id: &ConversationId,
        from_message_id: &MessageId,
    ) -> Result<(), DbError> {
        // Get created_at of from_message.
        let from_created: String = self.conn.query_row(
            "SELECT created_at FROM messages WHERE message_id = ?1",
            params![from_message_id.0],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1 AND created_at > ?2",
            params![conversation_id.0, from_created],
        )?;

        // Update active_branch_message_id to point to from_message.
        self.touch_conversation(conversation_id, Some(from_message_id))?;

        Ok(())
    }
}

// Extension trait to make `optional()` available on single-value queries.
trait OptionalExt<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalExt<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuitcode_providers::ContentBlock;

    #[test]
    fn workspace_upsert_is_idempotent() {
        let mut db = Database::open_in_memory().unwrap();
        let id1 = db.upsert_workspace("/home/user/project").unwrap();
        let id2 = db.upsert_workspace("/home/user/project").unwrap();
        assert_eq!(id1.0, id2.0, "upsert should return same id on second call");
    }

    #[test]
    fn create_and_list_conversation() {
        let mut db = Database::open_in_memory().unwrap();
        let ws = db.upsert_workspace("/tmp/myproject").unwrap();
        let conv = db
            .create_conversation(&ws, "Test conversation", "claude-sonnet-4-6")
            .unwrap();
        let list = db.list_conversations(&ws).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].conversation_id.0, conv.conversation_id.0);
        assert_eq!(list[0].active_model, "claude-sonnet-4-6");
    }

    #[test]
    fn append_and_list_messages() {
        let mut db = Database::open_in_memory().unwrap();
        let ws = db.upsert_workspace("/tmp/msg_test").unwrap();
        let conv = db
            .create_conversation(&ws, "Chat", "claude-opus-4-7")
            .unwrap();

        let user_msg = db
            .append_message(
                &conv.conversation_id,
                None,
                MessageRole::User,
                "",
                &[ContentBlock::Text {
                    text: "Hello".to_string(),
                }],
                &[],
                &[],
                None,
            )
            .unwrap();

        let usage = Usage {
            input_tokens: 5,
            output_tokens: 3,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
        };
        let _asst_msg = db
            .append_message(
                &conv.conversation_id,
                Some(&user_msg.message_id),
                MessageRole::Assistant,
                "claude-opus-4-7",
                &[ContentBlock::Text {
                    text: "Hi there!".to_string(),
                }],
                &[],
                &[],
                Some(&usage),
            )
            .unwrap();

        let msgs = db.list_messages(&conv.conversation_id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, MessageRole::User);
        assert_eq!(msgs[1].role, MessageRole::Assistant);
        assert_eq!(msgs[1].usage.as_ref().unwrap().input_tokens, 5);
        // parent linkage
        assert_eq!(msgs[1].parent_id.as_ref().unwrap().0, user_msg.message_id.0);
    }

    #[test]
    fn conversation_touch_updates_leaf() {
        let mut db = Database::open_in_memory().unwrap();
        let ws = db.upsert_workspace("/tmp/touch_test").unwrap();
        let conv = db
            .create_conversation(&ws, "Conv", "claude-haiku-4-5-20251001")
            .unwrap();

        let msg = db
            .append_message(
                &conv.conversation_id,
                None,
                MessageRole::User,
                "",
                &[ContentBlock::Text { text: "msg".into() }],
                &[],
                &[],
                None,
            )
            .unwrap();

        let list = db.list_conversations(&ws).unwrap();
        assert_eq!(
            list[0].active_branch_message_id.as_ref().unwrap().0,
            msg.message_id.0,
            "active_branch_message_id should be the last appended message"
        );
    }

    #[test]
    fn insert_and_list_snapshot() {
        use crate::types::{SnapshotFile, SnapshotId};

        let mut db = Database::open_in_memory().unwrap();
        let ws = db.upsert_workspace("/tmp/snap_test").unwrap();
        let conv = db
            .create_conversation(&ws, "SnapConv", "claude-opus-4-7")
            .unwrap();
        let msg = db
            .append_message(
                &conv.conversation_id,
                None,
                MessageRole::User,
                "",
                &[ContentBlock::Text {
                    text: "edit file".into(),
                }],
                &[],
                &[],
                None,
            )
            .unwrap();

        let files = vec![SnapshotFile {
            snapshot_id: SnapshotId::new(), // will be overwritten by insert_snapshot
            abs_path: "/tmp/snap_test/hello.txt".to_string(),
            snapshot_filename: Some("path__tmp__snap_test__hello.txt.bak".to_string()),
            pre_sha256: Some("abc123".to_string()),
            pre_size_bytes: Some(11),
            pre_existed: true,
        }];

        let snap_id = db
            .insert_snapshot(
                &conv.conversation_id,
                &msg.message_id,
                "tc_001",
                "write_file",
                &files,
            )
            .unwrap();

        let snaps = db.list_snapshots(&conv.conversation_id).unwrap();
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].snapshot_id.0, snap_id.0);
        assert_eq!(snaps[0].tool_name, "write_file");
    }

    #[test]
    fn truncate_messages_after_removes_later_messages() {
        let mut db = Database::open_in_memory().unwrap();
        let ws = db.upsert_workspace("/tmp/truncate_test").unwrap();
        let conv = db
            .create_conversation(&ws, "TruncConv", "claude-opus-4-7")
            .unwrap();

        let m1 = db
            .append_message(
                &conv.conversation_id,
                None,
                MessageRole::User,
                "",
                &[ContentBlock::Text {
                    text: "msg1".into(),
                }],
                &[],
                &[],
                None,
            )
            .unwrap();

        // Small sleep to ensure distinct created_at timestamps.
        std::thread::sleep(std::time::Duration::from_millis(5));

        let _m2 = db
            .append_message(
                &conv.conversation_id,
                Some(&m1.message_id),
                MessageRole::Assistant,
                "claude-opus-4-7",
                &[ContentBlock::Text {
                    text: "msg2".into(),
                }],
                &[],
                &[],
                None,
            )
            .unwrap();

        let msgs_before = db.list_messages(&conv.conversation_id).unwrap();
        assert_eq!(msgs_before.len(), 2);

        db.truncate_messages_after(&conv.conversation_id, &m1.message_id)
            .unwrap();

        let msgs_after = db.list_messages(&conv.conversation_id).unwrap();
        assert_eq!(
            msgs_after.len(),
            1,
            "only m1 should remain after truncation"
        );
        assert_eq!(msgs_after[0].message_id.0, m1.message_id.0);

        // active_branch_message_id should point to m1.
        let convs = db.list_conversations(&ws).unwrap();
        assert_eq!(
            convs[0].active_branch_message_id.as_ref().unwrap().0,
            m1.message_id.0
        );
    }
}
