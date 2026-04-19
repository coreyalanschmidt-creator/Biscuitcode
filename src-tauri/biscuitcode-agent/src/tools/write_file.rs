//! `write_file` — write (or create) a workspace file.
//!
//! Phase 6b deliverable. Always `ToolClass::Write` — requires confirmation
//! unless workspace-trust is on. Snapshot happens before write.
//!
//! Writes UTF-8 content to `path`. Creates parent directories if they don't
//! exist. If the file exists, it is overwritten.
//!
//! Protected paths (`**/.git/**`, `**/node_modules/**`, `**/target/**`,
//! `**/.cache/**`) are rejected even with workspace-trust.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use biscuitcode_providers::ToolSpec;

use super::{Tool, ToolClass, ToolCtx, ToolError, ToolResult};

// Protected path segments — agent never writes these.
const PROTECTED_SEGMENTS: &[&str] = &[".git", "node_modules", "target", ".cache"];

pub struct WriteFileTool;

#[derive(Debug, Deserialize)]
struct Args {
    path: String,
    contents: String,
}

fn is_protected(path: &std::path::Path) -> bool {
    path.components().any(|c| {
        if let std::path::Component::Normal(s) = c {
            PROTECTED_SEGMENTS.contains(&s.to_string_lossy().as_ref())
        } else {
            false
        }
    })
}

/// Resolve a path that may not exist yet (can't canonicalize).
/// For write_file we need to check workspace containment without the file
/// existing, so we resolve relative paths against the workspace root and
/// check the resulting path is under the root (without following symlinks
/// that might escape).
fn resolve_write_path(root: &std::path::Path, p: &str) -> PathBuf {
    let candidate = std::path::Path::new(p);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}

/// Check workspace containment without requiring the file to exist.
/// Uses lexical normalization (remove `.` and `..` components).
fn is_inside_workspace_lexical(root: &std::path::Path, path: &std::path::Path) -> bool {
    // Try canonicalize first (works when file exists).
    if let Ok(canon) = std::fs::canonicalize(path) {
        return canon.starts_with(root);
    }
    // Lexical fallback for new files: normalize the path.
    let norm = normalize_path(path);
    norm.starts_with(root)
}

/// Normalize a path by resolving `.` and `..` components lexically.
fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => { out.pop(); }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[async_trait]
impl Tool for WriteFileTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description:
                "Write (create or overwrite) a file in the workspace. Requires \
                 user confirmation unless workspace trust is enabled."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Workspace-relative or absolute path to write."
                    },
                    "contents": {
                        "type": "string",
                        "description": "UTF-8 contents of the file."
                    }
                },
                "required": ["path", "contents"],
                "additionalProperties": false
            }),
        }
    }

    fn class(&self) -> ToolClass {
        ToolClass::Write
    }

    fn name(&self) -> &'static str {
        "write_file"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolCtx,
    ) -> Result<ToolResult, ToolError> {
        let args: Args = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let path = resolve_write_path(&ctx.workspace_root, &args.path);

        if !is_inside_workspace_lexical(&ctx.workspace_root, &path) {
            return Err(ToolError::OutsideWorkspace { path: args.path });
        }

        if is_protected(&path) {
            return Err(ToolError::Forbidden(format!(
                "path {} is in a protected directory ({})",
                path.display(),
                PROTECTED_SEGMENTS.join(", ")
            )));
        }

        // Create parent directories if needed.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(ToolError::Io)?;
        }

        fs::write(&path, args.contents.as_bytes())
            .await
            .map_err(ToolError::Io)?;

        Ok(ToolResult::text(format!(
            "Wrote {} bytes to {}",
            args.contents.len(),
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuitcode_db::ConversationId;
    use serde_json::json;
    use tempfile::TempDir;

    fn make_ctx(dir: &TempDir) -> ToolCtx {
        ToolCtx {
            workspace_root: dir.path().to_path_buf(),
            conversation_id: ConversationId::new(),
            max_result_bytes: 256 * 1024,
        }
    }

    #[tokio::test]
    async fn creates_new_file() {
        let dir = TempDir::new().unwrap();
        let tool = WriteFileTool;
        let ctx = make_ctx(&dir);
        let result = tool
            .execute(
                json!({ "path": "hello.txt", "contents": "hello world" }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.result.contains("bytes"));
        let written = std::fs::read_to_string(dir.path().join("hello.txt")).unwrap();
        assert_eq!(written, "hello world");
    }

    #[tokio::test]
    async fn overwrites_existing_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("f.txt"), "old content").unwrap();
        let tool = WriteFileTool;
        let ctx = make_ctx(&dir);
        tool.execute(json!({ "path": "f.txt", "contents": "new content" }), &ctx)
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("f.txt")).unwrap(), "new content");
    }

    #[tokio::test]
    async fn creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let tool = WriteFileTool;
        let ctx = make_ctx(&dir);
        tool.execute(json!({ "path": "a/b/c.txt", "contents": "deep" }), &ctx)
            .await
            .unwrap();
        assert!(dir.path().join("a/b/c.txt").exists());
    }

    #[tokio::test]
    async fn rejects_path_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let tool = WriteFileTool;
        let ctx = make_ctx(&dir);
        let err = tool
            .execute(json!({ "path": "/etc/passwd", "contents": "evil" }), &ctx)
            .await;
        assert!(matches!(err, Err(ToolError::OutsideWorkspace { .. })));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        let tool = WriteFileTool;
        let ctx = make_ctx(&dir);
        let err = tool
            .execute(json!({ "path": "../../etc/passwd", "contents": "evil" }), &ctx)
            .await;
        assert!(
            matches!(err, Err(ToolError::OutsideWorkspace { .. })),
            "expected OutsideWorkspace for traversal, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn rejects_git_directory() {
        let dir = TempDir::new().unwrap();
        let tool = WriteFileTool;
        let ctx = make_ctx(&dir);
        let err = tool
            .execute(json!({ "path": ".git/config", "contents": "evil" }), &ctx)
            .await;
        assert!(matches!(err, Err(ToolError::Forbidden(_))));
    }

    #[tokio::test]
    async fn rejects_node_modules() {
        let dir = TempDir::new().unwrap();
        let tool = WriteFileTool;
        let ctx = make_ctx(&dir);
        let err = tool
            .execute(json!({ "path": "node_modules/evil.js", "contents": "x" }), &ctx)
            .await;
        assert!(matches!(err, Err(ToolError::Forbidden(_))));
    }
}
