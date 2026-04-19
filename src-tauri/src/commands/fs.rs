//! Workspace-scoped filesystem commands — Phase 3.
//!
//! Every command validates that the target path is a descendant of the
//! current workspace root before performing any I/O. Paths outside the
//! workspace return `E002 OutsideWorkspace` rather than a raw OS error.
//!
//! # Path validation
//!
//! Both the incoming path AND the stored workspace root are canonicalized
//! via `std::fs::canonicalize` before the prefix check. This prevents
//! symlink-traversal escapes and normalises trailing-slash differences.
//!
//! # Workspace state
//!
//! The active workspace root is stored in a `Mutex<Option<PathBuf>>` held
//! in Tauri's managed state. `fs_open_folder` sets it; all other commands
//! read it. There is exactly one workspace per window in v1.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::State;

use biscuitcode_core::errors::CatalogueError;

// ---------- Managed state ----------

/// Single workspace root for the running app instance.
pub struct WorkspaceState(pub Mutex<Option<PathBuf>>);

// ---------- Wire types ----------

/// A single entry returned by `fs_list`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    /// Absolute path to this entry.
    pub path: String,
    /// File name (last component only).
    pub name: String,
    /// `true` if the entry is a directory.
    pub is_dir: bool,
}

/// Result type for all fs commands.
type FsResult<T> = Result<T, FsError>;

/// Serialisable wrapper around the errors that fs commands can emit.
///
/// We don't use `CatalogueError` directly over IPC because `thiserror`
/// enums don't automatically derive `Serialize` in a shape that `tauri::command`
/// can convey; instead we flatten to a simple `{code, message}` object.
#[derive(Debug, Serialize)]
pub struct FsError {
    code: &'static str,
    message: String,
}

impl From<CatalogueError> for FsError {
    fn from(e: CatalogueError) -> Self {
        FsError {
            code: e.code(),
            message: e.to_string(),
        }
    }
}

impl From<std::io::Error> for FsError {
    fn from(e: std::io::Error) -> Self {
        FsError {
            code: "E000",
            message: e.to_string(),
        }
    }
}

// ---------- Path helpers ----------

/// Canonicalize `path` and assert it is a descendant of `workspace_root`.
///
/// Returns the canonical `PathBuf` on success or `E002 OutsideWorkspace`
/// if the check fails.
fn assert_inside_workspace(
    workspace_root: &Path,
    path: &str,
) -> Result<PathBuf, CatalogueError> {
    let canonical_root = std::fs::canonicalize(workspace_root).map_err(|_| {
        CatalogueError::OutsideWorkspace {
            path: path.to_string(),
        }
    })?;

    let canonical_path = std::fs::canonicalize(path).map_err(|_| {
        // Path may not exist yet (e.g. new file). Fall back to lexical check
        // on the parent directory.
        let p = PathBuf::from(path);
        let parent = p.parent().unwrap_or(&p);
        match std::fs::canonicalize(parent) {
            Ok(cp) if cp.starts_with(&canonical_root) => {
                // Parent is inside; the path itself will be inside once created.
                // We return the un-canonicalized path so callers can create it.
                return CatalogueError::OutsideWorkspace {
                    path: "__PARENT_OK__".to_string(),
                };
            }
            _ => CatalogueError::OutsideWorkspace {
                path: path.to_string(),
            },
        }
    });

    // If the parent-OK sentinel came back, reconstruct a best-effort path.
    if let Err(CatalogueError::OutsideWorkspace { path: ref p }) = canonical_path {
        if p == "__PARENT_OK__" {
            return Ok(PathBuf::from(path));
        }
    }

    let canonical_path = canonical_path?;

    if canonical_path.starts_with(&canonical_root) {
        Ok(canonical_path)
    } else {
        Err(CatalogueError::OutsideWorkspace {
            path: path.to_string(),
        })
    }
}

/// Get the current workspace root from managed state, returning a human error
/// if no folder has been opened.
fn get_workspace_root(state: &State<WorkspaceState>) -> Result<PathBuf, FsError> {
    state
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| FsError {
            code: "E000",
            message: "No workspace folder is open. Use Open Folder first.".to_string(),
        })
}

// ---------- Commands ----------

/// Open a folder via system dialog and store it as the active workspace root.
///
/// Returns the canonical path string so the frontend can display it.
#[tauri::command]
pub fn fs_open_folder(
    app: tauri::AppHandle,
    state: State<'_, WorkspaceState>,
) -> FsResult<String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app
        .dialog()
        .file()
        .blocking_pick_folder()
        .ok_or_else(|| FsError {
            code: "E000",
            message: "No folder selected.".to_string(),
        })?;

    let path = folder.into_path().map_err(|e| FsError {
        code: "E000",
        message: e.to_string(),
    })?;

    let canonical = std::fs::canonicalize(&path)?;
    *state.0.lock().unwrap() = Some(canonical.clone());

    Ok(canonical.to_string_lossy().to_string())
}

/// List the immediate children of a directory inside the workspace.
#[tauri::command]
pub fn fs_list(path: String, state: State<'_, WorkspaceState>) -> FsResult<Vec<DirEntry>> {
    let root = get_workspace_root(&state)?;
    let target = assert_inside_workspace(&root, &path).map_err(FsError::from)?;

    let mut entries: Vec<DirEntry> = std::fs::read_dir(&target)?
        .filter_map(|e| e.ok())
        .map(|e| {
            let p = e.path();
            let is_dir = p.is_dir();
            DirEntry {
                name: e.file_name().to_string_lossy().to_string(),
                path: p.to_string_lossy().to_string(),
                is_dir,
            }
        })
        .collect();

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

/// Read a UTF-8 text file inside the workspace.
#[tauri::command]
pub fn fs_read(path: String, state: State<'_, WorkspaceState>) -> FsResult<String> {
    let root = get_workspace_root(&state)?;
    let target = assert_inside_workspace(&root, &path).map_err(FsError::from)?;
    let content = std::fs::read_to_string(&target)?;
    Ok(content)
}

/// Write UTF-8 text to a file inside the workspace (creates if not exists).
#[tauri::command]
pub fn fs_write(path: String, content: String, state: State<'_, WorkspaceState>) -> FsResult<()> {
    let root = get_workspace_root(&state)?;
    // For new files the path may not exist yet; parent validation is done
    // inside `assert_inside_workspace`.
    let _ = assert_inside_workspace(&root, &path).map_err(FsError::from)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Rename (move) a file or directory inside the workspace.
#[tauri::command]
pub fn fs_rename(from: String, to: String, state: State<'_, WorkspaceState>) -> FsResult<()> {
    let root = get_workspace_root(&state)?;
    let _ = assert_inside_workspace(&root, &from).map_err(FsError::from)?;
    let _ = assert_inside_workspace(&root, &to).map_err(FsError::from)?;
    std::fs::rename(&from, &to)?;
    Ok(())
}

/// Delete a file or empty directory inside the workspace.
///
/// Directories are removed recursively.
#[tauri::command]
pub fn fs_delete(path: String, state: State<'_, WorkspaceState>) -> FsResult<()> {
    let root = get_workspace_root(&state)?;
    let target = assert_inside_workspace(&root, &path).map_err(FsError::from)?;

    if target.is_dir() {
        std::fs::remove_dir_all(&target)?;
    } else {
        std::fs::remove_file(&target)?;
    }
    Ok(())
}

/// Create a directory (and any missing parents) inside the workspace.
#[tauri::command]
pub fn fs_create_dir(path: String, state: State<'_, WorkspaceState>) -> FsResult<()> {
    let root = get_workspace_root(&state)?;
    let _ = assert_inside_workspace(&root, &path).map_err(FsError::from)?;
    std::fs::create_dir_all(path)?;
    Ok(())
}

/// Fuzzy-search file paths in the workspace for the quick-open palette.
///
/// Returns up to `limit` paths whose names contain `query` as a
/// case-insensitive substring. Simple but fast enough for <10k file trees.
#[tauri::command]
pub fn fs_search_files(
    query: String,
    limit: usize,
    state: State<'_, WorkspaceState>,
) -> FsResult<Vec<String>> {
    let root = get_workspace_root(&state)?;
    let root_str = root.to_string_lossy().to_string();
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    walk_dir_for_files(&root, &root_str, &query_lower, limit, &mut results);
    Ok(results)
}

fn walk_dir_for_files(
    dir: &Path,
    root_str: &str,
    query: &str,
    limit: usize,
    results: &mut Vec<String>,
) {
    if results.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        if results.len() >= limit {
            return;
        }
        let path = entry.path();
        // Skip hidden dirs and common noise dirs.
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules" {
            continue;
        }
        if path.is_dir() {
            walk_dir_for_files(&path, root_str, query, limit, results);
        } else if name_str.to_lowercase().contains(query) {
            let relative = path
                .strip_prefix(root_str)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            results.push(relative);
        }
    }
}

/// Search file contents in the workspace (cross-file find).
///
/// Returns a list of `{path, line, col, text}` matches. Respects
/// `.gitignore` via the `ignore` crate. Capped at 500 results.
#[tauri::command]
pub fn fs_search_content(
    query: String,
    use_regex: bool,
    case_sensitive: bool,
    state: State<'_, WorkspaceState>,
) -> FsResult<Vec<SearchMatch>> {
    let root = get_workspace_root(&state)?;

    let pattern: Box<dyn Fn(&str) -> bool + Send> = if use_regex {
        let re = regex::Regex::new(&if case_sensitive {
            query.clone()
        } else {
            format!("(?i){}", query)
        })
        .map_err(|e| FsError {
            code: "E000",
            message: format!("Invalid regex: {}", e),
        })?;
        Box::new(move |s: &str| re.is_match(s))
    } else if case_sensitive {
        let q = query.clone();
        Box::new(move |s: &str| s.contains(q.as_str()))
    } else {
        let q = query.to_lowercase();
        Box::new(move |s: &str| s.to_lowercase().contains(q.as_str()))
    };

    let mut matches = Vec::new();
    let walker = ignore::WalkBuilder::new(&root)
        .hidden(true)
        .git_ignore(true)
        .build();

    'outer: for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else { continue };
        for (line_idx, line_text) in content.lines().enumerate() {
            if pattern(line_text) {
                let relative = path
                    .strip_prefix(&root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                matches.push(SearchMatch {
                    path: relative,
                    line: line_idx + 1,
                    text: line_text.to_string(),
                });
                if matches.len() >= 500 {
                    break 'outer;
                }
            }
        }
    }

    Ok(matches)
}

/// A single content-search result.
#[derive(Debug, Serialize)]
pub struct SearchMatch {
    /// Workspace-relative path.
    pub path: String,
    /// 1-based line number.
    pub line: usize,
    /// Full text of the matching line.
    pub text: String,
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_temp_workspace() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    /// PM-02 falsification: canonicalize-based check must block /etc/passwd.
    #[test]
    fn outside_workspace_returns_e002() {
        let dir = make_temp_workspace();
        let root = dir.path().to_path_buf();
        let err = assert_inside_workspace(&root, "/etc/passwd").unwrap_err();
        assert_eq!(err.code(), "E002");
        assert!(matches!(err, CatalogueError::OutsideWorkspace { .. }));
    }

    /// PM-02 falsification: a real file inside the workspace resolves OK.
    #[test]
    fn inside_workspace_resolves_ok() {
        let dir = make_temp_workspace();
        let root = dir.path();
        let file = root.join("hello.txt");
        fs::write(&file, "hi").unwrap();
        let result = assert_inside_workspace(root, file.to_str().unwrap());
        assert!(result.is_ok(), "{:?}", result);
    }

    /// Canonical form: a different path outside the workspace must also be blocked.
    #[test]
    fn outside_in_tmp_returns_e002() {
        let dir = make_temp_workspace();
        let root = dir.path();
        let outside = tempfile::NamedTempFile::new_in("/tmp").unwrap();
        let err = assert_inside_workspace(root, outside.path().to_str().unwrap()).unwrap_err();
        assert_eq!(err.code(), "E002");
    }

    /// Verify that a sub-directory inside the workspace passes.
    #[test]
    fn subdir_inside_workspace_ok() {
        let dir = make_temp_workspace();
        let root = dir.path();
        let sub = root.join("a").join("b");
        fs::create_dir_all(&sub).unwrap();
        let result = assert_inside_workspace(root, sub.to_str().unwrap());
        assert!(result.is_ok(), "{:?}", result);
    }

    /// Test the file-path fuzzy search helper directly (avoids Tauri runtime).
    #[test]
    fn walk_dir_finds_matching_file() {
        let dir = make_temp_workspace();
        let root = dir.path();
        fs::write(root.join("main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("lib.rs"), "").unwrap();

        let mut results = Vec::new();
        walk_dir_for_files(root, root.to_str().unwrap(), "main", 10, &mut results);
        assert!(
            results.iter().any(|r| r.contains("main.rs")),
            "expected main.rs in results: {:?}",
            results
        );
    }

    /// Test that node_modules and .git are skipped.
    #[test]
    fn walk_dir_skips_noise_dirs() {
        let dir = make_temp_workspace();
        let root = dir.path();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules").join("pkg.js"), "").unwrap();

        let mut results = Vec::new();
        walk_dir_for_files(root, root.to_str().unwrap(), "pkg", 10, &mut results);
        assert!(
            results.is_empty(),
            "node_modules should be skipped, got: {:?}",
            results
        );
    }
}
