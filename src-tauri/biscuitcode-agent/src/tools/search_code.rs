//! `search_code` — substring/regex search across workspace files.
//! Phase 6a deliverable.
//!
//! Uses the `ignore` crate to walk the workspace respecting `.gitignore`,
//! and `grep`-equivalent matching for the query. Returns matches grouped
//! by file with line numbers, up to `ctx.max_result_bytes`.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use biscuitcode_providers::ToolSpec;

use super::{Tool, ToolClass, ToolCtx, ToolError, ToolResult};

pub struct SearchCodeTool;

#[derive(Debug, Deserialize)]
struct Args {
    query: String,
    /// Optional glob (e.g. `"src/**/*.ts"`). Defaults to `**/*` (everything).
    #[serde(default)]
    glob: Option<String>,
    /// Treat `query` as a regex. Default false.
    #[serde(default)]
    regex: Option<bool>,
}

#[async_trait]
impl Tool for SearchCodeTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description:
                "Search workspace files for `query`. Substring match by \
                 default; set `regex: true` for regex. Restrict scope with \
                 `glob` (e.g. `src/**/*.ts`). Respects .gitignore."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "glob":  { "type": "string", "description": "Optional glob to restrict search scope." },
                    "regex": { "type": "boolean", "description": "Treat query as regex (default false)." }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        }
    }

    fn class(&self) -> ToolClass {
        ToolClass::Read
    }

    fn name(&self) -> &'static str {
        "search_code"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: &ToolCtx,
    ) -> Result<ToolResult, ToolError> {
        let _args: Args = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        // ---- Phase 6a coder fills in ----
        //
        // 1. Build globset matcher from args.glob (default = match all).
        // 2. Walk ctx.workspace_root via `ignore::WalkBuilder`
        //    (respects .gitignore, .ignore, hidden-file rules).
        // 3. For each file, scan for query (substring or regex per
        //    args.regex). Skip binary files (heuristic: leading null
        //    byte in first 8 KB).
        // 4. Format matches grouped by file:
        //    `<file>:<line>: <line content trimmed>`
        // 5. Truncate at ctx.max_result_bytes; set truncated=true.

        Ok(ToolResult::text("[search_code: Phase 6a coder fills in implementation]"))
    }
}
