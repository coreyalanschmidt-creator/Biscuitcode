//! `read_file` — read a workspace file. Phase 6a deliverable.
//!
//! Returns the file's contents up to `ctx.max_result_bytes`. Sets
//! `truncated: true` if the file was larger.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use biscuitcode_providers::ToolSpec;

use super::{Tool, ToolClass, ToolCtx, ToolError, ToolResult};

pub struct ReadFileTool;

#[derive(Debug, Deserialize)]
struct Args {
    path: String,
}

#[async_trait]
impl Tool for ReadFileTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description:
                "Read the contents of a file in the workspace. Returns text \
                 (UTF-8). Files larger than 256 KB are truncated."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Workspace-relative or absolute path to the file."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        }
    }

    fn class(&self) -> ToolClass {
        ToolClass::Read
    }

    fn name(&self) -> &'static str {
        "read_file"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolCtx,
    ) -> Result<ToolResult, ToolError> {
        let args: Args = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let p = resolve(&ctx.workspace_root, &args.path);
        if !ctx.is_inside_workspace(&p) {
            return Err(ToolError::OutsideWorkspace { path: args.path });
        }

        let mut bytes = fs::read(&p).await?;
        let truncated = bytes.len() > ctx.max_result_bytes;
        if truncated {
            bytes.truncate(ctx.max_result_bytes);
        }

        // Lossy UTF-8 — binary files become un-displayable but the model
        // gets a clear signal rather than crashing.
        let text = String::from_utf8_lossy(&bytes).into_owned();

        Ok(ToolResult { result: text, truncated })
    }
}

fn resolve(root: &std::path::Path, p: &str) -> PathBuf {
    let candidate = std::path::Path::new(p);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}
