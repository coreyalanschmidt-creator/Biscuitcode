//! `search_code` — substring/regex search across workspace files.
//! Phase 6a deliverable.
//!
//! Uses the `ignore` crate to walk the workspace respecting `.gitignore`,
//! and `globset` for glob filtering. Returns matches grouped by file
//! with line numbers, up to `ctx.max_result_bytes`.
//!
//! PM-03 prevention: glob patterns (including brace expansion like
//! `{src,tests}/**/*.ts`) are compiled through `globset::GlobSetBuilder`
//! which correctly handles brace expansion. We do NOT use the `ignore`
//! crate's built-in glob filter for user-supplied patterns.

use async_trait::async_trait;
use globset::{Glob, GlobSetBuilder};
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

use biscuitcode_providers::ToolSpec;

use super::{Tool, ToolClass, ToolCtx, ToolError, ToolResult};

pub struct SearchCodeTool;

#[derive(Debug, Deserialize)]
struct Args {
    query: String,
    /// Optional glob (e.g. `"src/**/*.ts"` or `"{src,tests}/**/*.ts"`).
    /// Defaults to match all files.
    #[serde(default)]
    glob: Option<String>,
    /// Treat `query` as a regex. Default false (substring match).
    #[serde(default)]
    regex: Option<bool>,
}

#[async_trait]
impl Tool for SearchCodeTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: "Search workspace files for `query`. Substring match by \
                 default; set `regex: true` for regex. Restrict scope with \
                 `glob` (e.g. `src/**/*.ts` or `{src,tests}/**/*.ts`). \
                 Respects .gitignore."
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
        ctx: &ToolCtx,
    ) -> Result<ToolResult, ToolError> {
        let args: Args =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        // Build the matcher. PM-03 fix: use globset::GlobSetBuilder so brace
        // expansion works correctly.
        let glob_set = if let Some(ref pattern) = args.glob {
            let glob = Glob::new(pattern)
                .map_err(|e| ToolError::InvalidArgs(format!("invalid glob {pattern:?}: {e}")))?;
            let mut builder = GlobSetBuilder::new();
            builder.add(glob);
            Some(
                builder
                    .build()
                    .map_err(|e| ToolError::InvalidArgs(e.to_string()))?,
            )
        } else {
            None
        };

        // Build the query matcher.
        let use_regex = args.regex.unwrap_or(false);
        let re = if use_regex {
            Some(
                Regex::new(&args.query)
                    .map_err(|e| ToolError::InvalidArgs(format!("invalid regex: {e}")))?,
            )
        } else {
            None
        };

        let workspace = ctx.workspace_root.clone();
        let max_bytes = ctx.max_result_bytes;

        // Run blocking I/O on a blocking thread (ignore::Walk is sync).
        let query = args.query.clone();
        let result = tokio::task::spawn_blocking(move || {
            search_sync(
                &workspace,
                glob_set.as_ref(),
                re.as_ref(),
                &query,
                max_bytes,
            )
        })
        .await
        .map_err(|e| ToolError::Other(e.to_string()))??;

        Ok(result)
    }
}

fn search_sync(
    root: &std::path::Path,
    glob_set: Option<&globset::GlobSet>,
    re: Option<&Regex>,
    query: &str,
    max_bytes: usize,
) -> Result<ToolResult, ToolError> {
    let mut output = String::new();
    let mut truncated = false;

    let walker = ignore::WalkBuilder::new(root)
        .hidden(false) // include hidden dirs (user may want .github etc.)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Glob filter: match path relative to workspace root.
        if let Some(gs) = glob_set {
            let rel = path.strip_prefix(root).unwrap_or(path);
            if !gs.is_match(rel) {
                continue;
            }
        }

        // Read file; skip on I/O error (e.g. permission denied).
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        // Binary heuristic: if first 8KB contains a null byte, skip.
        let probe = &bytes[..bytes.len().min(8192)];
        if probe.contains(&0u8) {
            continue;
        }

        let text = String::from_utf8_lossy(&bytes);
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();

        for (line_no, line) in text.lines().enumerate() {
            let matches = if let Some(r) = re {
                r.is_match(line)
            } else {
                line.contains(query)
            };

            if matches {
                let entry_str = format!("{}:{}: {}\n", rel_path, line_no + 1, line.trim_end());

                if output.len() + entry_str.len() > max_bytes {
                    truncated = true;
                    break;
                }
                output.push_str(&entry_str);
            }
        }

        if truncated {
            break;
        }
    }

    if output.is_empty() && !truncated {
        output = format!("No matches found for {:?}", query);
    }

    Ok(ToolResult {
        result: output,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuitcode_db::ConversationId;
    use tempfile::TempDir;

    fn make_ctx(dir: &TempDir) -> ToolCtx {
        ToolCtx {
            workspace_root: dir.path().to_path_buf(),
            conversation_id: ConversationId::new(),
            max_result_bytes: 256 * 1024,
        }
    }

    fn write(dir: &TempDir, path: &str, content: &str) {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, content).unwrap();
    }

    #[tokio::test]
    async fn finds_substring_matches_with_line_numbers() {
        let dir = TempDir::new().unwrap();
        write(&dir, "src/alpha.ts", "// TODO: implement alpha\n// done\n");
        write(&dir, "src/beta.ts", "// no marker here\n");

        let tool = SearchCodeTool;
        let ctx = make_ctx(&dir);
        let result = tool
            .execute(json!({ "query": "TODO" }), &ctx)
            .await
            .unwrap();

        assert!(
            result.result.contains("alpha.ts"),
            "result: {}",
            result.result
        );
        assert!(
            result.result.contains("1:"),
            "expected line 1; result: {}",
            result.result
        );
        assert!(
            !result.result.contains("beta.ts"),
            "beta.ts should not match"
        );
        assert!(!result.truncated);
    }

    /// PM-03 falsification: brace-expansion glob `{src,tests}/**/*.ts`
    /// should match files in both src/ and tests/.
    #[tokio::test]
    async fn glob_brace_expansion_matches_both_dirs() {
        let dir = TempDir::new().unwrap();
        write(&dir, "src/alpha.ts", "// TODO: implement alpha\n");
        write(&dir, "tests/alpha.test.ts", "// TODO: cover alpha\n");
        write(&dir, "lib/gamma.ts", "// TODO: something in lib\n");

        let tool = SearchCodeTool;
        let ctx = make_ctx(&dir);
        let result = tool
            .execute(
                json!({ "query": "TODO", "glob": "{src,tests}/**/*.ts" }),
                &ctx,
            )
            .await
            .unwrap();

        assert!(
            result.result.contains("alpha.ts"),
            "src match missing: {}",
            result.result
        );
        assert!(
            result.result.contains("alpha.test.ts"),
            "tests match missing: {}",
            result.result
        );
        assert!(
            !result.result.contains("gamma.ts"),
            "lib/gamma.ts must be excluded: {}",
            result.result
        );
    }

    #[tokio::test]
    async fn regex_mode_matches_pattern() {
        let dir = TempDir::new().unwrap();
        write(&dir, "a.ts", "const x = 42;\nconst y = 100;\n");

        let tool = SearchCodeTool;
        let ctx = make_ctx(&dir);
        let result = tool
            .execute(json!({ "query": "const x = \\d+", "regex": true }), &ctx)
            .await
            .unwrap();

        assert!(result.result.contains("a.ts"), "result: {}", result.result);
        assert!(result.result.contains("const x = 42"));
    }

    #[tokio::test]
    async fn no_matches_returns_no_matches_message() {
        let dir = TempDir::new().unwrap();
        write(&dir, "a.ts", "hello world\n");

        let tool = SearchCodeTool;
        let ctx = make_ctx(&dir);
        let result = tool
            .execute(json!({ "query": "NONEXISTENT_UNIQUE_STRING" }), &ctx)
            .await
            .unwrap();
        assert!(
            result.result.contains("No matches"),
            "result: {}",
            result.result
        );
    }

    #[tokio::test]
    async fn invalid_regex_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = SearchCodeTool;
        let ctx = make_ctx(&dir);
        let err = tool
            .execute(json!({ "query": "[invalid regex", "regex": true }), &ctx)
            .await;
        assert!(err.is_err(), "expected error for invalid regex");
    }

    #[tokio::test]
    async fn glob_without_brace_expansion_still_works() {
        let dir = TempDir::new().unwrap();
        write(&dir, "src/alpha.ts", "// TODO: alpha\n");
        write(&dir, "src/main.rs", "// TODO: not typescript\n");

        let tool = SearchCodeTool;
        let ctx = make_ctx(&dir);
        let result = tool
            .execute(json!({ "query": "TODO", "glob": "src/**/*.ts" }), &ctx)
            .await
            .unwrap();

        assert!(
            result.result.contains("alpha.ts"),
            "result: {}",
            result.result
        );
        assert!(
            !result.result.contains("main.rs"),
            "result: {}",
            result.result
        );
    }

    #[tokio::test]
    async fn truncation_flag_set_when_result_exceeds_limit() {
        let dir = TempDir::new().unwrap();
        // Write enough matching lines to exceed a tiny limit.
        let content = "TODO marker\n".repeat(100);
        write(&dir, "big.ts", &content);

        let tool = SearchCodeTool;
        let mut ctx = make_ctx(&dir);
        ctx.max_result_bytes = 50; // very small limit
        let result = tool
            .execute(json!({ "query": "TODO" }), &ctx)
            .await
            .unwrap();
        assert!(
            result.truncated,
            "expected truncated=true; result len={}",
            result.result.len()
        );
    }
}
