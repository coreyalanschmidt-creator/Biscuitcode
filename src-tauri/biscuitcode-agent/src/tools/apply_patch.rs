//! `apply_patch` — apply a unified-diff patch to a workspace file.
//!
//! Phase 6b deliverable. Always `ToolClass::Write`.
//!
//! Takes a unified-diff patch string and applies it to `path`. The patch
//! must apply cleanly; if it does not, the tool returns an error (the file
//! is NOT modified).
//!
//! PM-03 (line ending mismatch) mitigation: we normalize the target file's
//! line endings to LF before applying the patch, then restore to the
//! original line ending style after applying. This covers the common case
//! where a Windows-origin file has CRLF endings.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use biscuitcode_providers::ToolSpec;

use super::{Tool, ToolClass, ToolCtx, ToolError, ToolResult};

const PROTECTED_SEGMENTS: &[&str] = &[".git", "node_modules", "target", ".cache"];

pub struct ApplyPatchTool;

#[derive(Debug, Deserialize)]
struct Args {
    path: String,
    patch: String,
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

fn resolve_path(root: &std::path::Path, p: &str) -> PathBuf {
    let candidate = std::path::Path::new(p);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}

fn is_inside_workspace(root: &std::path::Path, path: &std::path::Path) -> bool {
    if let Ok(canon) = std::fs::canonicalize(path) {
        return canon.starts_with(root);
    }
    // Lexical check for paths that may not exist.
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out.starts_with(root)
}

/// Detect whether a byte slice uses CRLF line endings (majority vote).
fn uses_crlf(bytes: &[u8]) -> bool {
    let crlf_count = bytes.windows(2).filter(|w| w == b"\r\n").count();
    let lf_only_count = bytes
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        .saturating_sub(crlf_count);
    crlf_count > lf_only_count
}

/// Apply a unified diff patch to `original` text.
/// Returns the patched text or an error string.
fn apply_patch_text(original: &str, patch: &str) -> Result<String, String> {
    // We implement a minimal unified-diff apply:
    // Parse hunks and apply them sequentially to the original lines.
    let original_lines: Vec<&str> = original.lines().collect();
    let mut result = original_lines.clone();
    let mut offset: i64 = 0;

    let mut in_hunk = false;
    let mut hunk_old_start = 0usize;
    let mut hunk_lines: Vec<(char, &str)> = Vec::new();

    let patch_lines: Vec<&str> = patch.lines().collect();
    let mut i = 0;

    while i < patch_lines.len() {
        let line = patch_lines[i];

        if line.starts_with("@@") {
            // Apply previous hunk if any.
            if in_hunk && !hunk_lines.is_empty() {
                apply_hunk(&mut result, hunk_old_start, &hunk_lines, &mut offset)?;
                hunk_lines.clear();
            }
            // Parse: @@ -old_start,old_count +new_start,new_count @@
            hunk_old_start = parse_hunk_header(line)?;
            in_hunk = true;
        } else if in_hunk {
            if let Some(rest) = line.strip_prefix('+') {
                hunk_lines.push(('+', rest));
            } else if let Some(rest) = line.strip_prefix('-') {
                hunk_lines.push(('-', rest));
            } else if let Some(rest) = line.strip_prefix(' ') {
                hunk_lines.push((' ', rest));
            } else if line.is_empty() {
                // Blank continuation line treated as context.
                hunk_lines.push((' ', ""));
            }
            // Lines starting with `\` (no newline) or file headers — skip.
        }
        i += 1;
    }

    // Apply final hunk.
    if in_hunk && !hunk_lines.is_empty() {
        apply_hunk(&mut result, hunk_old_start, &hunk_lines, &mut offset)?;
    }

    Ok(result.join("\n"))
}

fn parse_hunk_header(line: &str) -> Result<usize, String> {
    // @@ -old_start[,old_count] +new_start[,new_count] @@
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(format!("malformed hunk header: {line}"));
    }
    let old_part = parts[1]; // e.g. "-5,7" or "-5"
    let without_minus = old_part.trim_start_matches('-');
    let start_str = without_minus.split(',').next().unwrap_or("1");
    let start: usize = start_str
        .parse()
        .map_err(|_| format!("bad hunk old_start in: {line}"))?;
    // Convert 1-based to 0-based.
    Ok(if start == 0 { 0 } else { start - 1 })
}

fn apply_hunk<'a>(
    result: &mut Vec<&'a str>,
    old_start: usize,
    hunk: &[(char, &'a str)],
    offset: &mut i64,
) -> Result<(), String> {
    let actual_start = (old_start as i64 + *offset) as usize;

    // Find context match. The context lines in the hunk must match the file.
    let context_lines: Vec<&str> = hunk
        .iter()
        .filter(|(op, _)| *op == ' ' || *op == '-')
        .map(|(_, l)| *l)
        .collect();

    // Verify context.
    if actual_start + context_lines.len() > result.len() && !context_lines.is_empty() {
        return Err(format!(
            "hunk context extends past end of file (start={actual_start}, context={}, file_len={})",
            context_lines.len(),
            result.len()
        ));
    }

    // Build the replacement.
    let mut new_lines: Vec<&str> = Vec::new();
    let mut old_idx = actual_start;
    for (op, line) in hunk {
        match op {
            ' ' => {
                // Verify context line matches.
                if old_idx >= result.len() || result[old_idx] != *line {
                    return Err(format!(
                        "context mismatch at line {}: expected {:?}, found {:?}",
                        old_idx + 1,
                        line,
                        result.get(old_idx).copied().unwrap_or("<eof>")
                    ));
                }
                new_lines.push(line);
                old_idx += 1;
            }
            '-' => {
                if old_idx >= result.len() || result[old_idx] != *line {
                    return Err(format!(
                        "remove mismatch at line {}: expected {:?}, found {:?}",
                        old_idx + 1,
                        line,
                        result.get(old_idx).copied().unwrap_or("<eof>")
                    ));
                }
                old_idx += 1;
            }
            '+' => {
                new_lines.push(line);
            }
            _ => {}
        }
    }

    let old_len = old_idx - actual_start;
    let new_len = new_lines.len();
    result.splice(actual_start..actual_start + old_len, new_lines);
    *offset += new_len as i64 - old_len as i64;

    Ok(())
}

#[async_trait]
impl Tool for ApplyPatchTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description:
                "Apply a unified-diff patch to a workspace file. Requires user confirmation. \
                 The patch must apply cleanly — the file is not modified if it fails."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Workspace-relative or absolute path to patch."
                    },
                    "patch": {
                        "type": "string",
                        "description": "Unified-diff patch string (output of `diff -u`)."
                    }
                },
                "required": ["path", "patch"],
                "additionalProperties": false
            }),
        }
    }

    fn class(&self) -> ToolClass {
        ToolClass::Write
    }

    fn name(&self) -> &'static str {
        "apply_patch"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolCtx,
    ) -> Result<ToolResult, ToolError> {
        let args: Args =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let path = resolve_path(&ctx.workspace_root, &args.path);

        if !is_inside_workspace(&ctx.workspace_root, &path) {
            return Err(ToolError::OutsideWorkspace { path: args.path });
        }

        if is_protected(&path) {
            return Err(ToolError::Forbidden(format!(
                "path {} is in a protected directory",
                path.display()
            )));
        }

        let original_bytes = fs::read(&path).await?;
        let had_crlf = uses_crlf(&original_bytes);

        // Normalize to LF for patching (PM-03 fix).
        let original_lf = String::from_utf8_lossy(&original_bytes).replace("\r\n", "\n");

        let patched = apply_patch_text(&original_lf, &args.patch)
            .map_err(|e| ToolError::Other(format!("patch apply failed: {e}")))?;

        // Restore CRLF if original used it.
        let final_content = if had_crlf {
            patched.replace('\n', "\r\n")
        } else {
            patched
        };

        fs::write(&path, final_content.as_bytes()).await?;

        Ok(ToolResult::text(format!(
            "Applied patch to {}",
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
    async fn applies_simple_patch() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("f.txt"), "line1\nline2\nline3\n").unwrap();
        let tool = ApplyPatchTool;
        let ctx = make_ctx(&dir);
        let patch = "@@ -1,3 +1,3 @@\n line1\n-line2\n+LINE2\n line3\n";
        let r = tool
            .execute(json!({ "path": "f.txt", "patch": patch }), &ctx)
            .await
            .unwrap();
        assert!(r.result.contains("Applied"));
        let content = std::fs::read_to_string(dir.path().join("f.txt")).unwrap();
        assert_eq!(content, "line1\nLINE2\nline3");
    }

    /// PM-03 falsification: CRLF file patched with LF-only patch should work.
    #[tokio::test]
    async fn applies_patch_to_crlf_file() {
        let dir = TempDir::new().unwrap();
        // Write a CRLF file.
        std::fs::write(dir.path().join("win.txt"), "line1\r\nline2\r\nline3\r\n").unwrap();
        let tool = ApplyPatchTool;
        let ctx = make_ctx(&dir);
        // LF-only patch.
        let patch = "@@ -1,3 +1,3 @@\n line1\n-line2\n+LINE2\n line3\n";
        let r = tool
            .execute(json!({ "path": "win.txt", "patch": patch }), &ctx)
            .await
            .unwrap();
        assert!(r.result.contains("Applied"), "result: {}", r.result);
        let content = std::fs::read(dir.path().join("win.txt")).unwrap();
        // Should still use CRLF endings and have the change.
        assert!(
            content.windows(2).any(|w| w == b"\r\n"),
            "CRLF should be preserved"
        );
        let text = String::from_utf8(content).unwrap();
        assert!(text.contains("LINE2"), "patch change should be applied");
    }

    #[tokio::test]
    async fn rejects_path_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let tool = ApplyPatchTool;
        let ctx = make_ctx(&dir);
        let err = tool
            .execute(json!({ "path": "/etc/passwd", "patch": "" }), &ctx)
            .await;
        assert!(matches!(err, Err(ToolError::OutsideWorkspace { .. })));
    }

    #[tokio::test]
    async fn fails_if_file_does_not_exist() {
        let dir = TempDir::new().unwrap();
        let tool = ApplyPatchTool;
        let ctx = make_ctx(&dir);
        let err = tool
            .execute(
                json!({ "path": "nonexistent.txt", "patch": "@@ -1,1 +1,1 @@\n-x\n+y\n" }),
                &ctx,
            )
            .await;
        assert!(err.is_err());
    }
}
