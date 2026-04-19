# BiscuitCode Conversation Export Schema

> Phase 8 deliverable. Defines the JSON shape produced by `Settings → Conversations → "Export all"` and consumed by `Import`. Versioned via `schema_version` so future format changes can migrate forward without breaking older exports.

## File format

`biscuitcode-conversations-<ISO-date>.json` — one JSON object containing an array of conversations and their full DAG of messages, plus the workspace bindings the conversations belong to.

## Top-level structure

```json
{
  "schema_version": 1,
  "exported_at": "2026-04-18T03:42:00Z",
  "exported_by": "biscuitcode 1.0.0",
  "workspaces": [
    {
      "workspace_id": "wks_018f3e2a...",
      "root_path": "/home/user/projects/foo",
      "first_seen_at": "2026-03-12T11:02:14Z",
      "label": null
    }
  ],
  "conversations": [
    {
      "conversation_id": "conv_018f8c10...",
      "workspace_id": "wks_018f3e2a...",
      "title": "Refactor the auth module to use the new session API",
      "created_at": "2026-04-15T18:11:02Z",
      "updated_at": "2026-04-17T22:48:11Z",
      "active_model": "claude-opus-4-7",
      "active_branch_message_id": "msg_018f9d20...",
      "messages": [ /* see Message schema below */ ]
    }
  ]
}
```

### Field rules

| Field | Type | Notes |
|---|---|---|
| `schema_version` | integer | Currently `1`. Importer rejects unknown versions with error code `E018b SchemaVersionUnsupported` (added to the catalogue here). |
| `exported_at` | ISO-8601 UTC string | When the export was generated. |
| `exported_by` | string | Always `"biscuitcode <version>"`. |
| `workspaces` | array | Every workspace referenced by an exported conversation. Reference-only — import does NOT recreate workspace folders. |
| `conversations` | array | The conversation tree. |
| `workspace_id` | string | ULID-based, prefix `wks_`. Globally unique. |
| `conversation_id` | string | ULID-based, prefix `conv_`. Globally unique. |
| `active_model` | string | Last model selected for the conversation. May reference a model not currently available on import; the importer notes but does not block. |
| `active_branch_message_id` | string | The leaf message ID the user was viewing when the conversation was last seen. Importer uses this to set the initially-displayed branch. |

## Message schema (DAG node)

Each message is a node in a conversation DAG (parent_id pointing back to the previous message in its chain; multiple messages can share a parent for branching).

```json
{
  "message_id": "msg_018f9d20...",
  "parent_id": "msg_018f9d1f...",
  "role": "user" | "assistant" | "tool",
  "created_at": "2026-04-15T18:11:05Z",
  "model": "claude-opus-4-7",
  "content": [
    {
      "type": "text",
      "text": "Refactor the auth module to use the new session API."
    }
  ],
  "tool_calls": [],
  "tool_results": [],
  "snapshots": [],
  "usage": {
    "input_tokens": 142,
    "output_tokens": 0,
    "cache_read_input_tokens": 0,
    "cache_creation_input_tokens": 142
  }
}
```

### `content` is an array of typed blocks

| Block type | Shape | Used by |
|---|---|---|
| `text` | `{ type: "text", text: string }` | All roles |
| `mention` | `{ type: "mention", mention_kind: "file"\|"folder"\|"selection"\|"terminal-output"\|"problems"\|"git-diff", value: string \| object }` | `user` only |
| `image` | `{ type: "image", media_type: "image/png"\|..., data_b64: string }` | `user` only (vision input); not used in v1 unless vision model selected |
| `thinking` | `{ type: "thinking", text: string }` | `assistant` only (Anthropic thinking blocks; OpenAI reasoning if exposed) |

### `tool_calls` (assistant role)

```json
[
  {
    "tool_call_id": "tc_018f9d21...",
    "tool": "read_file",
    "args": { "path": "src/auth/session.ts" },
    "started_at": "2026-04-15T18:11:08.412Z",
    "ended_at": "2026-04-15T18:11:08.591Z",
    "status": "ok" | "error" | "rejected_by_user",
    "confirmation_required": false,
    "confirmation_decision": null
  }
]
```

### `tool_results` (tool role, one per tool call)

```json
[
  {
    "tool_call_id": "tc_018f9d21...",
    "result": "<the tool's return value, JSON-encoded if structured, base64 if binary>",
    "result_is_truncated": false,
    "error": null
  }
]
```

### `snapshots` (assistant messages that performed write/shell tool calls — Phase 6b)

```json
[
  {
    "snapshot_id": "snap_018f9d22...",
    "tool_call_id": "tc_018f9d21...",
    "files": [
      {
        "path": "src/auth/session.ts",
        "pre_sha256": "9a1b...",
        "pre_size_bytes": 4128,
        "snapshot_path_relative_to_cache": "snap_018f9d22.../src/auth/session.ts.bak"
      }
    ]
  }
]
```

**Note:** the actual snapshot file contents (`.bak` files) are NOT bundled in the export — they live in `~/.cache/biscuitcode/snapshots/...` and are subject to the 30-day cleanup. Export includes the manifest (paths + hashes) so imported conversations can show "rewind unavailable: snapshot expired" rather than crashing on a rewind attempt.

### `usage` (assistant role only)

Provider-reported token counts. `cache_read_input_tokens` and `cache_creation_input_tokens` are Anthropic-specific (prompt-caching, Phase 5 deliverable). Other providers may emit only `input_tokens` and `output_tokens`.

## Importer behavior

- **Duplicate detection**: by `(conversation_id, message_id)` tuple. Re-importing the same file is a no-op.
- **Workspace mismatch**: if an imported conversation's `workspace_id` doesn't match any local workspace's `root_path`, the importer creates a placeholder workspace entry with `root_path: null` so the conversation can still be opened (read-only — file mentions resolve to "unavailable").
- **Schema version mismatch**: if `schema_version > 1` (older app importing newer export), the importer aborts with a clear error pointing the user to the latest BiscuitCode release. If `schema_version < 1`, the importer attempts in-place migration (none needed at v1).
- **Model unavailable**: imported conversations whose `active_model` is not configured on the importing instance display a notice in the conversation header offering to select an alternative. They do NOT auto-switch.

## Privacy considerations

- The export contains every message body, including any text the user typed and every assistant response. Treat it as sensitive.
- API keys are NEVER in the export (they live only in libsecret).
- Prompt-cache identifiers, internal request IDs, and provider-side billing tokens are NOT included.
- `image` blocks are base64-encoded inline. For very large vision inputs the export file can grow quickly; the export UI warns above 100 MB.

## Backward compatibility commitment

`schema_version: 1` is frozen at the BiscuitCode v1.0 release. Future changes:
- **Additive fields**: allowed in v1.x without bumping `schema_version` (importer ignores unknown keys).
- **Breaking changes**: bump `schema_version` to `2`; importer must support both `1` and `2` for at least one major version after the bump.
