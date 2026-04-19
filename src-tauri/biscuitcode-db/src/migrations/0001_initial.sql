-- Migration 0001 — initial schema.
--
-- Tables:
--   workspaces       — folders the user has opened in BiscuitCode
--   conversations    — top-level chat sessions, bound to a workspace
--   messages         — DAG of messages (parent_id for branching)
--   snapshots        — Phase 6b rewind manifests (one per assistant
--                      message that performed write/shell tools)
--   snapshot_files   — per-file snapshot entries within a manifest
--
-- All timestamps are ISO-8601 UTC strings (chrono::DateTime<Utc>::to_rfc3339).
-- All IDs are ULID-prefixed strings (wks_…, conv_…, msg_…, snap_…) for
-- sortable, globally-unique identifiers without a central coordinator.
--
-- See docs/CONVERSATION-EXPORT-SCHEMA.md for the JSON shape this maps to.

CREATE TABLE workspaces (
    workspace_id   TEXT PRIMARY KEY,
    root_path      TEXT NOT NULL UNIQUE,
    first_seen_at  TEXT NOT NULL,
    label          TEXT
) STRICT;

CREATE TABLE conversations (
    conversation_id           TEXT PRIMARY KEY,
    workspace_id              TEXT NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    title                     TEXT NOT NULL,
    created_at                TEXT NOT NULL,
    updated_at                TEXT NOT NULL,
    active_model              TEXT NOT NULL,
    active_branch_message_id  TEXT     -- nullable; set when first message lands
) STRICT;

CREATE INDEX idx_conversations_workspace_updated
    ON conversations(workspace_id, updated_at DESC);

CREATE TABLE messages (
    message_id       TEXT PRIMARY KEY,
    conversation_id  TEXT NOT NULL REFERENCES conversations(conversation_id) ON DELETE CASCADE,
    parent_id        TEXT REFERENCES messages(message_id) ON DELETE CASCADE,
    role             TEXT NOT NULL CHECK (role IN ('user','assistant','tool','system')),
    created_at       TEXT NOT NULL,
    model            TEXT NOT NULL,

    -- JSON-encoded Vec<ContentBlock>. We treat content as opaque to SQL so
    -- adding a new block type doesn't require a migration.
    content_json     TEXT NOT NULL,

    -- JSON-encoded Vec<ToolCall>. Empty array '[]' for non-tool-calling msgs.
    tool_calls_json  TEXT NOT NULL DEFAULT '[]',

    -- JSON-encoded Vec<ToolResult>. Empty array '[]' for non-tool messages.
    tool_results_json TEXT NOT NULL DEFAULT '[]',

    -- FK to snapshots(snapshot_id). Set for assistant messages that
    -- performed Write/Shell tools (Phase 6b). Null otherwise.
    snapshot_id      TEXT REFERENCES snapshots(snapshot_id) ON DELETE SET NULL,

    -- JSON-encoded Usage. Null for non-assistant messages.
    usage_json       TEXT
) STRICT;

CREATE INDEX idx_messages_conversation_created
    ON messages(conversation_id, created_at);

CREATE INDEX idx_messages_parent
    ON messages(parent_id);

CREATE TABLE snapshots (
    snapshot_id      TEXT PRIMARY KEY,
    conversation_id  TEXT NOT NULL REFERENCES conversations(conversation_id) ON DELETE CASCADE,
    message_id       TEXT NOT NULL,    -- soft FK; the message_id is set
                                       --   AFTER the snapshot row exists
                                       --   so we can't enforce as hard FK
                                       --   without a chicken-and-egg insert
    tool_call_id     TEXT NOT NULL,
    tool_name        TEXT NOT NULL,
    snapshotted_at   TEXT NOT NULL
) STRICT;

CREATE INDEX idx_snapshots_conversation_created
    ON snapshots(conversation_id, snapshotted_at DESC);

CREATE TABLE snapshot_files (
    snapshot_id        TEXT NOT NULL REFERENCES snapshots(snapshot_id) ON DELETE CASCADE,
    abs_path           TEXT NOT NULL,
    snapshot_filename  TEXT,         -- null when pre_existed = 0 (rewind = delete)
    pre_sha256         TEXT,
    pre_size_bytes     INTEGER,
    pre_existed        INTEGER NOT NULL CHECK (pre_existed IN (0,1)),
    PRIMARY KEY (snapshot_id, abs_path)
) STRICT;
