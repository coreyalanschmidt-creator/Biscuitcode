//! Git commands for Phase 7.
//!
//! All write operations (`git stage`, `git commit`, `git push`, `git pull`)
//! use `std::process::Command("git")` — they are simpler and don't require
//! libgit2 headers.
//!
//! Read operations (status, branch, log, diff) also use `git` CLI for
//! consistency and to avoid a libgit2 system-library dependency.
//!
//! Error catalogue: `E012 GitPushFailed` when push exits non-zero.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::fs::WorkspaceState;

// ---------- Types ----------

/// A file change in the git working tree.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GitFileStatus {
    /// Workspace-relative path.
    pub path: String,
    /// One of: "staged", "unstaged", "untracked".
    pub bucket: String,
    /// Short status code (e.g. "M", "A", "D", "?").
    pub status_code: String,
}

/// Summary of git status.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GitStatus {
    /// Current branch name.
    pub branch: String,
    /// File-level changes.
    pub files: Vec<GitFileStatus>,
}

/// A single git log entry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GitLogEntry {
    /// Short commit hash (7 chars).
    pub hash: String,
    /// Commit message subject line.
    pub subject: String,
    /// Author name.
    pub author: String,
    /// ISO-8601 date string.
    pub date: String,
}

/// Blame entry for a line range.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GitBlameLine {
    /// 1-indexed line number.
    pub line: u32,
    /// Short hash (7 chars).
    pub hash: String,
    /// Author name.
    pub author: String,
    /// Relative date string (e.g. "3 days ago").
    pub relative_date: String,
}

// ---------- Helpers ----------

/// Run a git command in the workspace root. Returns stdout or stderr error.
fn run_git(workspace: &std::path::Path, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()
        .map_err(|e| format!("git spawn failed: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(stderr)
    }
}

// ---------- Commands ----------

/// Get git status for the workspace.
/// Returns branch name + staged/unstaged/untracked file list.
#[tauri::command]
pub fn git_status(workspace: State<'_, WorkspaceState>) -> Result<GitStatus, String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;

    // Get current branch.
    let branch = run_git(root.as_path(), &["branch", "--show-current"])
        .unwrap_or_else(|_| "HEAD".to_string())
        .trim()
        .to_string();

    // Get status in porcelain v1 format.
    let status_out = run_git(root.as_path(), &["status", "--porcelain=v1"])?;

    let mut files = Vec::new();
    for line in status_out.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = &line[0..2];
        let path = line[3..].trim_matches('"').to_string();

        let x = &xy[0..1]; // index status
        let y = &xy[1..2]; // worktree status

        // Untracked.
        if x == "?" && y == "?" {
            files.push(GitFileStatus {
                path,
                bucket: "untracked".to_string(),
                status_code: "?".to_string(),
            });
            continue;
        }

        // Staged changes (index != ' ' and != '?').
        if x != " " && x != "?" {
            files.push(GitFileStatus {
                path: path.clone(),
                bucket: "staged".to_string(),
                status_code: x.to_string(),
            });
        }

        // Unstaged changes (worktree != ' ' and != '?').
        if y != " " && y != "?" {
            files.push(GitFileStatus {
                path,
                bucket: "unstaged".to_string(),
                status_code: y.to_string(),
            });
        }
    }

    Ok(GitStatus { branch, files })
}

/// Stage a file (or all if path = ".").
#[tauri::command]
pub fn git_stage(path: String, workspace: State<'_, WorkspaceState>) -> Result<(), String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    run_git(root.as_path(), &["add", &path])?;
    Ok(())
}

/// Unstage a file.
#[tauri::command]
pub fn git_unstage(path: String, workspace: State<'_, WorkspaceState>) -> Result<(), String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    run_git(root.as_path(), &["restore", "--staged", &path])?;
    Ok(())
}

/// Commit staged changes with a message.
#[tauri::command]
pub fn git_commit(message: String, workspace: State<'_, WorkspaceState>) -> Result<(), String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    run_git(root.as_path(), &["commit", "-m", &message])?;
    Ok(())
}

/// Push to origin. Returns `E012 GitPushFailed` on non-zero exit.
#[tauri::command]
pub fn git_push(workspace: State<'_, WorkspaceState>) -> Result<String, String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    run_git(root.as_path(), &["push"]).map_err(|stderr| {
        // Emit E012 structured error string for the frontend toast layer.
        format!("E012:{}", stderr)
    })
}

/// Pull from origin.
#[tauri::command]
pub fn git_pull(workspace: State<'_, WorkspaceState>) -> Result<String, String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    run_git(root.as_path(), &["pull"])
}

/// Get recent log entries (default last 20).
#[tauri::command]
pub fn git_log(
    limit: Option<u32>,
    workspace: State<'_, WorkspaceState>,
) -> Result<Vec<GitLogEntry>, String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    let n = limit.unwrap_or(20).to_string();
    let out = run_git(
        &root,
        &[
            "log",
            &format!("-{}", n),
            "--pretty=format:%h|%s|%an|%ar",
        ],
    )?;
    let entries = out
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            GitLogEntry {
                hash: parts.first().unwrap_or(&"").to_string(),
                subject: parts.get(1).unwrap_or(&"").to_string(),
                author: parts.get(2).unwrap_or(&"").to_string(),
                date: parts.get(3).unwrap_or(&"").to_string(),
            }
        })
        .collect();
    Ok(entries)
}

/// List local branches.
#[tauri::command]
pub fn git_branches(workspace: State<'_, WorkspaceState>) -> Result<Vec<String>, String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    let out = run_git(root.as_path(), &["branch", "--format=%(refname:short)"])?;
    Ok(out.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

/// Switch to a branch.
#[tauri::command]
pub fn git_checkout(
    branch: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<(), String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    run_git(root.as_path(), &["checkout", &branch])?;
    Ok(())
}

/// Get unified diff for a file (staged or unstaged depending on `staged` flag).
#[tauri::command]
pub fn git_diff_file(
    path: String,
    staged: bool,
    workspace: State<'_, WorkspaceState>,
) -> Result<String, String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    if staged {
        run_git(root.as_path(), &["diff", "--cached", "--", &path])
    } else {
        run_git(root.as_path(), &["diff", "--", &path])
    }
}

/// Get git blame for a file, returning per-line annotations.
/// `start_line` and `end_line` are 1-indexed inclusive.
#[tauri::command]
pub fn git_blame(
    path: String,
    start_line: u32,
    end_line: u32,
    workspace: State<'_, WorkspaceState>,
) -> Result<Vec<GitBlameLine>, String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    let range = format!("{},{}", start_line, end_line);
    let out = run_git(
        &root,
        &[
            "blame",
            "--porcelain",
            "-L",
            &range,
            "--",
            &path,
        ],
    )?;
    parse_blame_porcelain(&out, start_line)
}

/// Get full git diff (staged + unstaged) for the @git-diff mention.
#[tauri::command]
pub fn git_diff_all(workspace: State<'_, WorkspaceState>) -> Result<String, String> {
    let root = workspace
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no workspace open".to_string())?;
    let staged = run_git(root.as_path(), &["diff", "--cached"]).unwrap_or_default();
    let unstaged = run_git(root.as_path(), &["diff"]).unwrap_or_default();
    Ok(format!("{}{}", staged, unstaged))
}

/// Parse `git blame --porcelain` output into `GitBlameLine` entries.
fn parse_blame_porcelain(output: &str, start_line: u32) -> Result<Vec<GitBlameLine>, String> {
    let mut result = Vec::new();
    let mut current_hash = String::new();
    let mut current_author = String::new();
    let mut current_date = String::new();
    let mut current_line: u32 = start_line;

    for line in output.lines() {
        if line.starts_with('\t') {
            // Source line content — one entry per tab-prefixed line.
            result.push(GitBlameLine {
                line: current_line,
                hash: current_hash[..7.min(current_hash.len())].to_string(),
                author: current_author.clone(),
                relative_date: current_date.clone(),
            });
            current_line += 1;
        } else if line.starts_with("author ") {
            current_author = line["author ".len()..].to_string();
        } else if line.starts_with("author-time ") {
            // Convert unix timestamp to relative string.
            if let Ok(ts) = line["author-time ".len()..].parse::<i64>() {
                current_date = relative_time(ts);
            }
        } else if line.len() >= 40 && line.chars().next().map(|c| c.is_ascii_hexdigit()).unwrap_or(false) {
            // Commit hash line: "<hash> <orig_line> <final_line> <num_lines>"
            current_hash = line.split_whitespace().next().unwrap_or("").to_string();
        }
    }

    Ok(result)
}

/// Convert a Unix timestamp to a human-readable relative time string.
fn relative_time(ts: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let secs = now - ts;
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{} min ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hours ago", secs / 3600)
    } else if secs < 86400 * 30 {
        format!("{} days ago", secs / 86400)
    } else if secs < 86400 * 365 {
        format!("{} months ago", secs / (86400 * 30))
    } else {
        format!("{} years ago", secs / (86400 * 365))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_time_seconds() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(relative_time(now - 30), "just now");
    }

    #[test]
    fn relative_time_minutes() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let s = relative_time(now - 300);
        assert!(s.contains("min ago"), "got: {}", s);
    }

    #[test]
    fn relative_time_days() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let s = relative_time(now - 86400 * 3);
        assert!(s.contains("days ago"), "got: {}", s);
    }

    #[test]
    fn parse_blame_porcelain_empty() {
        let result = parse_blame_porcelain("", 1).unwrap();
        assert!(result.is_empty());
    }
}
