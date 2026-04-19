//! Pre-write file snapshots + rewind.
//!
//! Design contract: docs/design/AGENT-LOOP.md §Snapshot + Rewind.
//!
//! ## Fsync ordering (PM-01 fix)
//!
//! For each snapshot we:
//!   1. Write .bak files — `File::sync_all()` on each.
//!   2. Write manifest.json — `File::sync_all()`.
//!   3. Only AFTER manifest is durably flushed: return OK to the caller.
//!
//! This ensures that on crash, we never have a manifest pointing to missing
//! .bak files. The worst case is an orphaned .bak with no manifest, which
//! the Phase 8 cleanup task harmlessly removes.
//!
//! ## Rewind
//!
//! Rewind loads manifests in reverse-chronological order and restores each
//! file. If any restore fails the rewind aborts at that point and returns
//! `Err(RewindError::FileFailed { path, reason })`.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;

// ---------- Snapshot data model ----------

/// Per-file entry within a snapshot manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFileEntry {
    pub abs_path: String,
    /// Filename of the .bak copy. `None` when `pre_existed == false`.
    pub snapshot_filename: Option<String>,
    pub pre_sha256: Option<String>,
    pub pre_size_bytes: Option<u64>,
    pub pre_existed: bool,
}

/// On-disk manifest.json format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub tool_call_id: String,
    pub tool_name: String,
    pub snapshotted_at: String,
    pub files: Vec<SnapshotFileEntry>,
}

/// Directory for snapshots: `~/.cache/biscuitcode/snapshots/{conv_id}/{msg_id}/`
pub fn snapshot_dir(
    cache_root: &Path,
    conversation_id: &str,
    message_id: &str,
) -> PathBuf {
    cache_root
        .join("snapshots")
        .join(conversation_id)
        .join(message_id)
}

/// Encode a file's absolute path into a safe .bak filename.
/// Uses `__` as separator so `/home/user/src/main.rs` →
/// `path__home__user__src__main.rs.bak`.
pub fn bak_filename(abs_path: &str) -> String {
    let safe = abs_path.replace('/', "__").replace('\\', "__");
    format!("{}.bak", safe)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Error returned by snapshot / rewind operations.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("io error on {path}: {reason}")]
    Io { path: String, reason: String },
    #[error("manifest write failed: {0}")]
    ManifestWrite(String),
}

/// Error returned by rewind.
#[derive(Debug, thiserror::Error)]
pub enum RewindError {
    #[error("failed to restore {path}: {reason}")]
    FileFailed { path: String, reason: String },
    #[error("sha256 mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch { path: String, expected: String, actual: String },
    #[error("manifest load failed: {0}")]
    ManifestLoad(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

// ---------- Snapshot creation ----------

/// Take a snapshot of `paths` before the tool runs.
///
/// `paths` is the list of absolute paths the tool will write. Non-existent
/// paths are recorded as `pre_existed: false` (rewind = delete them).
///
/// Returns the manifest for storage in the DB.
pub async fn take(
    snapshot_dir: &Path,
    paths: &[PathBuf],
    tool_call_id: &str,
    tool_name: &str,
) -> Result<SnapshotManifest, SnapshotError> {
    // Create snapshot directory.
    fs::create_dir_all(snapshot_dir)
        .await
        .map_err(|e| SnapshotError::Io {
            path: snapshot_dir.display().to_string(),
            reason: e.to_string(),
        })?;

    let mut entries = Vec::with_capacity(paths.len());

    // Step 1: Write .bak files BEFORE the manifest. (PM-01 fix)
    for path in paths {
        let abs_path = path.display().to_string();

        let exists = path.exists();
        if exists {
            let bytes = fs::read(path).await.map_err(|e| SnapshotError::Io {
                path: abs_path.clone(),
                reason: e.to_string(),
            })?;

            let sha = hex_sha256(&bytes);
            let size = bytes.len() as u64;
            let bak_name = bak_filename(&abs_path);
            let bak_path = snapshot_dir.join(&bak_name);

            // Write .bak and fsync (data before manifest).
            {
                let mut f = fs::File::create(&bak_path)
                    .await
                    .map_err(|e| SnapshotError::Io {
                        path: bak_path.display().to_string(),
                        reason: e.to_string(),
                    })?;
                f.write_all(&bytes).await.map_err(|e| SnapshotError::Io {
                    path: bak_path.display().to_string(),
                    reason: e.to_string(),
                })?;
                f.sync_all().await.map_err(|e| SnapshotError::Io {
                    path: bak_path.display().to_string(),
                    reason: e.to_string(),
                })?;
            }

            entries.push(SnapshotFileEntry {
                abs_path,
                snapshot_filename: Some(bak_name),
                pre_sha256: Some(sha),
                pre_size_bytes: Some(size),
                pre_existed: true,
            });
        } else {
            entries.push(SnapshotFileEntry {
                abs_path,
                snapshot_filename: None,
                pre_sha256: None,
                pre_size_bytes: None,
                pre_existed: false,
            });
        }
    }

    // Step 2: Write manifest AFTER all .bak files are durable. (PM-01)
    let manifest = SnapshotManifest {
        tool_call_id: tool_call_id.to_string(),
        tool_name: tool_name.to_string(),
        snapshotted_at: Utc::now().to_rfc3339(),
        files: entries,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| SnapshotError::ManifestWrite(e.to_string()))?;

    let manifest_path = snapshot_dir.join("manifest.json");
    {
        let mut f = fs::File::create(&manifest_path)
            .await
            .map_err(|e| SnapshotError::ManifestWrite(e.to_string()))?;
        f.write_all(manifest_json.as_bytes())
            .await
            .map_err(|e| SnapshotError::ManifestWrite(e.to_string()))?;
        f.sync_all()
            .await
            .map_err(|e| SnapshotError::ManifestWrite(e.to_string()))?;
    }

    Ok(manifest)
}

// ---------- Rewind ----------

/// Load a manifest from a snapshot directory.
pub async fn load_manifest(snapshot_dir: &Path) -> Result<SnapshotManifest, RewindError> {
    let path = snapshot_dir.join("manifest.json");
    let bytes = fs::read(&path).await.map_err(|e| {
        RewindError::ManifestLoad(format!("{}: {}", path.display(), e))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|e| RewindError::ManifestLoad(e.to_string()))
}

/// Restore all files referenced by a manifest.
///
/// Restoration order: each file is restored by atomic rename of a temp copy,
/// with sha256 verification after restore.
///
/// On first failure, returns `Err(RewindError::FileFailed {...})` and leaves
/// any already-restored files in their restored state.
pub async fn restore(
    snapshot_dir: &Path,
    manifest: &SnapshotManifest,
) -> Result<(), RewindError> {
    for entry in &manifest.files {
        let target = Path::new(&entry.abs_path);

        if !entry.pre_existed {
            // File was created by the tool — delete it.
            if target.exists() {
                fs::remove_file(target)
                    .await
                    .map_err(|e| RewindError::FileFailed {
                        path: entry.abs_path.clone(),
                        reason: format!("delete failed: {e}"),
                    })?;
            }
            // If already gone, that's fine.
            continue;
        }

        // File existed before — restore from .bak.
        let bak_name = entry.snapshot_filename.as_deref().ok_or_else(|| {
            RewindError::FileFailed {
                path: entry.abs_path.clone(),
                reason: "pre_existed=true but snapshot_filename is null".to_string(),
            }
        })?;

        let bak_path = snapshot_dir.join(bak_name);
        if !bak_path.exists() {
            return Err(RewindError::FileFailed {
                path: entry.abs_path.clone(),
                reason: format!("backup file {} is missing", bak_path.display()),
            });
        }

        let bak_bytes = fs::read(&bak_path).await.map_err(|e| RewindError::FileFailed {
            path: entry.abs_path.clone(),
            reason: format!("reading backup: {e}"),
        })?;

        // Verify sha256 of the backup matches what we recorded.
        if let Some(expected_sha) = &entry.pre_sha256 {
            let actual_sha = hex_sha256(&bak_bytes);
            if actual_sha != *expected_sha {
                return Err(RewindError::HashMismatch {
                    path: entry.abs_path.clone(),
                    expected: expected_sha.clone(),
                    actual: actual_sha,
                });
            }
        }

        // Atomic rename: write to a .tmp sibling, then rename to target.
        // Ensure the parent directory exists.
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await.map_err(|e| RewindError::FileFailed {
                path: entry.abs_path.clone(),
                reason: format!("create parent dirs: {e}"),
            })?;
        }

        let tmp_path = {
            let mut p = target.to_path_buf();
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            p.set_file_name(format!("{}.bak_restore_tmp", name));
            p
        };

        fs::write(&tmp_path, &bak_bytes)
            .await
            .map_err(|e| RewindError::FileFailed {
                path: entry.abs_path.clone(),
                reason: format!("write tmp: {e}"),
            })?;

        fs::rename(&tmp_path, target)
            .await
            .map_err(|e| RewindError::FileFailed {
                path: entry.abs_path.clone(),
                reason: format!("atomic rename: {e}"),
            })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_cache_and_snap(dir: &TempDir, conv: &str, msg: &str) -> PathBuf {
        let sd = snapshot_dir(dir.path(), conv, msg);
        sd
    }

    #[tokio::test]
    async fn snapshot_existing_file_creates_bak_and_manifest() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        let file = workspace.join("hello.txt");
        std::fs::write(&file, b"hello world").unwrap();

        let sd = make_cache_and_snap(&dir, "conv_1", "msg_1");

        let manifest = take(&sd, &[file.clone()], "tc_001", "write_file")
            .await
            .expect("snapshot should succeed");

        assert_eq!(manifest.files.len(), 1);
        assert!(manifest.files[0].pre_existed);
        assert!(manifest.files[0].pre_sha256.is_some());

        // .bak file exists
        let bak_name = manifest.files[0].snapshot_filename.as_deref().unwrap();
        assert!(sd.join(bak_name).exists(), "bak file should exist");
        // manifest.json exists
        assert!(sd.join("manifest.json").exists());
    }

    #[tokio::test]
    async fn snapshot_nonexistent_file_records_pre_existed_false() {
        let dir = TempDir::new().unwrap();
        let sd = make_cache_and_snap(&dir, "conv_2", "msg_2");
        let fake_path = dir.path().join("new_file.txt");

        let manifest = take(&sd, &[fake_path], "tc_002", "write_file")
            .await
            .unwrap();

        assert!(!manifest.files[0].pre_existed);
        assert!(manifest.files[0].snapshot_filename.is_none());
    }

    /// PM-01 falsification: manifest must exist AFTER bak files.
    /// We verify that the manifest references a .bak that actually exists,
    /// meaning data was written before the manifest.
    #[tokio::test]
    async fn manifest_written_after_bak_files() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("data.txt");
        std::fs::write(&file, b"important data").unwrap();

        let sd = make_cache_and_snap(&dir, "conv_3", "msg_3");
        let manifest = take(&sd, &[file], "tc_003", "write_file").await.unwrap();

        // Load manifest from disk and verify the referenced .bak file exists.
        let loaded = load_manifest(&sd).await.unwrap();
        for entry in &loaded.files {
            if let Some(bak) = &entry.snapshot_filename {
                assert!(sd.join(bak).exists(), "manifest references nonexistent bak: {bak}");
            }
        }
        let _ = manifest;
    }

    #[tokio::test]
    async fn restore_puts_file_back() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        let file = workspace.join("edit_me.rs");
        std::fs::write(&file, b"fn original() {}").unwrap();

        let sd = make_cache_and_snap(&dir, "conv_4", "msg_4");
        let _manifest = take(&sd, &[file.clone()], "tc_004", "write_file")
            .await
            .unwrap();

        // Simulate what the tool would have done.
        std::fs::write(&file, b"fn modified() {}").unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"fn modified() {}");

        // Now rewind.
        let loaded = load_manifest(&sd).await.unwrap();
        restore(&sd, &loaded).await.unwrap();

        assert_eq!(std::fs::read(&file).unwrap(), b"fn original() {}");
    }

    #[tokio::test]
    async fn restore_deletes_new_file() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        let new_file = workspace.join("new_file.txt");

        let sd = make_cache_and_snap(&dir, "conv_5", "msg_5");
        // Snapshot before: file doesn't exist.
        let _manifest = take(&sd, &[new_file.clone()], "tc_005", "write_file")
            .await
            .unwrap();

        // Tool created it.
        std::fs::write(&new_file, b"I was created by the agent").unwrap();
        assert!(new_file.exists());

        // Rewind should delete it.
        let loaded = load_manifest(&sd).await.unwrap();
        restore(&sd, &loaded).await.unwrap();
        assert!(!new_file.exists(), "file should be deleted on rewind");
    }

    #[tokio::test]
    async fn restore_detects_hash_mismatch() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("tampered.txt");
        std::fs::write(&file, b"original").unwrap();

        let sd = make_cache_and_snap(&dir, "conv_6", "msg_6");
        let manifest = take(&sd, &[file.clone()], "tc_006", "write_file")
            .await
            .unwrap();

        // Tamper with the .bak file.
        let bak_name = manifest.files[0].snapshot_filename.as_deref().unwrap();
        std::fs::write(sd.join(bak_name), b"tampered backup").unwrap();

        // Rewind should detect the mismatch.
        let loaded = load_manifest(&sd).await.unwrap();
        let result = restore(&sd, &loaded).await;
        assert!(
            matches!(result, Err(RewindError::HashMismatch { .. })),
            "expected HashMismatch, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn multi_file_snapshot_and_restore() {
        let dir = TempDir::new().unwrap();
        let ws = dir.path().join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        let a = ws.join("a.txt");
        let b = ws.join("b.txt");
        let c = ws.join("c.txt"); // doesn't exist yet

        std::fs::write(&a, b"aaa").unwrap();
        std::fs::write(&b, b"bbb").unwrap();

        let sd = make_cache_and_snap(&dir, "conv_7", "msg_7");
        let _m = take(&sd, &[a.clone(), b.clone(), c.clone()], "tc_007", "write_file")
            .await
            .unwrap();

        // Tool modifies a and b, creates c.
        std::fs::write(&a, b"AAA").unwrap();
        std::fs::write(&b, b"BBB").unwrap();
        std::fs::write(&c, b"CCC").unwrap();

        // Rewind should restore all three.
        let loaded = load_manifest(&sd).await.unwrap();
        restore(&sd, &loaded).await.unwrap();

        assert_eq!(std::fs::read(&a).unwrap(), b"aaa", "a.txt restored");
        assert_eq!(std::fs::read(&b).unwrap(), b"bbb", "b.txt restored");
        assert!(!c.exists(), "c.txt should be deleted on rewind");
    }
}
