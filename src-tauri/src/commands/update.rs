//! Update commands — Phase 9.
//!
//! Two update paths:
//!
//! 1. **AppImage path** — `tauri-plugin-updater` checks a static JSON manifest
//!    at the GitHub Releases URL. On a newer version: prompt, download, replace,
//!    restart. Wired here via `check_for_update` + `install_update`.
//!
//! 2. **`.deb` path** — "Check for updates" button in Settings → About calls
//!    `check_for_deb_update`. Queries the GitHub Releases API, compares the
//!    tag to the current version, and returns `UpdateInfo` to the frontend.
//!    The frontend shows a modal; the "Download" button opens the release page.
//!    No auto-install of `.deb` (requires sudo — never auto-run).
//!
//! Error codes:
//!   - `E017 UpdateCheckFailed`   — network/API failure on check
//!   - `E018 UpdateDownloadFailed` — AppImage download or signature failure

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// Returned to the frontend for the `.deb` path update check.
#[derive(Clone, Debug, Serialize)]
pub struct UpdateInfo {
    /// Whether a newer version is available.
    pub update_available: bool,
    /// Latest version tag (e.g. "v1.0.1"). `None` if the check returned
    /// the same version or the tag could not be parsed.
    pub latest_version: Option<String>,
    /// URL of the latest GitHub release page (for the Download button).
    pub release_url: Option<String>,
    /// Excerpt from the release body (first 500 chars). Empty if unavailable.
    pub changelog_excerpt: String,
}

/// Check the GitHub Releases API for a newer `.deb` release.
///
/// Returns `UpdateInfo` regardless of whether an update is found; errors
/// map to `E017 UpdateCheckFailed`.
///
/// The frontend opens the release page in the browser on user's "Download"
/// click. This command never installs anything.
#[tauri::command]
pub async fn check_for_deb_update(app: AppHandle) -> Result<UpdateInfo, String> {
    // Read the current app version from the Tauri context.
    let current = app.config().version.clone().unwrap_or_else(|| "0.0.0".to_string());

    let api_url = "https://api.github.com/repos/Coreyalanschmidt-creator/biscuitcode/releases/latest";

    // Use reqwest (already a transitive dep via biscuitcode-providers).
    let client = reqwest::Client::builder()
        .user_agent(format!("biscuitcode/{current}"))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("E017: {e}"))?;

    let resp = client
        .get(api_url)
        .send()
        .await
        .map_err(|e| format!("E017: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("E017: GitHub API returned {}", resp.status()));
    }

    #[derive(serde::Deserialize)]
    struct GhRelease {
        tag_name: String,
        html_url: String,
        body: Option<String>,
    }

    let release: GhRelease = resp
        .json()
        .await
        .map_err(|e| format!("E017: failed to parse release JSON: {e}"))?;

    // Normalise tags: strip leading 'v' for semver comparison.
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let update_available = semver_gt(&latest, current.trim_start_matches('v'));

    let excerpt = release
        .body
        .as_deref()
        .unwrap_or("")
        .chars()
        .take(500)
        .collect();

    Ok(UpdateInfo {
        update_available,
        latest_version: if update_available { Some(release.tag_name.clone()) } else { None },
        release_url: if update_available { Some(release.html_url) } else { None },
        changelog_excerpt: excerpt,
    })
}

/// Check for an AppImage update via `tauri-plugin-updater`.
///
/// On success, returns `true` if an update is available. The frontend
/// should then call `install_appimage_update` after showing the changelog.
/// Errors map to `E017 UpdateCheckFailed`.
#[tauri::command]
pub async fn check_for_appimage_update(app: AppHandle) -> Result<bool, String> {
    let updater = app.updater().map_err(|e| format!("E017: {e}"))?;
    match updater.check().await {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(format!("E017: {e}")),
    }
}

/// Download and install the AppImage update, then restart the app.
///
/// Should only be called after the user accepts the update prompt.
/// Errors map to `E018 UpdateDownloadFailed`.
#[tauri::command]
pub async fn install_appimage_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| format!("E018: {e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("E018: {e}"))?
        .ok_or_else(|| "E018: no update available".to_string())?;

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("E018: {e}"))?;

    app.restart();
}

// ---------- helpers ----------

/// Minimal semver `a > b` comparison for "X.Y.Z" strings.
/// Falls back to false if either string is not parseable.
fn semver_gt(a: &str, b: &str) -> bool {
    fn parse(s: &str) -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = s.splitn(3, '.').collect();
        if parts.len() < 3 { return None; }
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].split('-').next()?.parse().ok()?,
        ))
    }
    match (parse(a), parse(b)) {
        (Some(av), Some(bv)) => av > bv,
        _ => false,
    }
}
