//! Secret storage — `keyring` crate wrapper over libsecret on Linux.
//!
//! Phase 5 deliverable. The vision + plan + `docs/adr/0001-no-stronghold.md`
//! are emphatic: API keys live in libsecret ONLY. No plaintext fallback.
//! No env vars. No stronghold. No anything else.
//!
//! **Pre-flight contract (docs/design/CAPABILITIES.md; synthesis log):**
//! every call path that might touch the Secret Service must first call
//! [`secret_service_available`]. That function uses a **read-only
//! DBus name-check** (`busctl --user list`) — it NEVER activates the
//! daemon with a known credential, which `keyring::Entry::get_password`
//! would do as a side-effect on some Linux distributions.
//!
//! **Implementation note:** these `async fn` wrappers use
//! `tokio::task::spawn_blocking` to move the synchronous `keyring::Entry`
//! calls off the Tokio worker thread. Without this, when called from a
//! Tauri command (which runs on a Tokio worker), the sync `keyring` crate
//! drives `zbus` which internally calls `block_on` and panics with
//! "Cannot start a runtime from within a runtime" on `tokio-rt-worker`.
//! The `async` signature is preserved so callers don't have to change.

use crate::CatalogueError;
use std::process::Command;

/// The service name used for all BiscuitCode secrets. Per-user, scoped
/// so `secret-tool search service biscuitcode` surfaces exactly what
/// we've stored.
pub const SERVICE: &str = "biscuitcode";

/// Probe whether the user's DBus session has `org.freedesktop.secrets`
/// available. Runs `busctl --user list` and checks for the name.
///
/// Returns:
/// - `Ok(true)`  — Secret Service reachable; it's safe to call keyring ops.
/// - `Ok(false)` — NOT reachable; onboarding must block and surface E001.
/// - `Err(...)`  — `busctl` itself failed (should not happen — it's part
///   of systemd and verified by `bootstrap-wsl.sh`).
pub fn secret_service_available() -> Result<bool, CatalogueError> {
    let output = Command::new("busctl")
        .args(["--user", "list", "--no-pager"])
        .output();

    let stdout = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        // If busctl fails outright, we can't verify the service. Rather
        // than assume available, treat as not-available — the user will
        // see the E001 install prompt, which is the correct safe path.
        Ok(_) | Err(_) => return Ok(false),
    };

    Ok(stdout.contains("org.freedesktop.secrets"))
}

/// Store a secret for (service, key).
///
/// Requires [`secret_service_available`] returned `Ok(true)` for the
/// current session. Call that BEFORE this; if false, surface E001.
pub async fn set(service: &str, key: &str, value: &str) -> Result<(), CatalogueError> {
    let service = service.to_string();
    let key = key.to_string();
    let value = value.to_string();
    tokio::task::spawn_blocking(move || {
        let entry =
            keyring::Entry::new(&service, &key).map_err(|_| CatalogueError::KeyringMissing)?;
        entry.set_password(&value).map_err(keyring_err_to_catalogue)
    })
    .await
    .map_err(keyring_join_to_catalogue)?
}

/// Retrieve a secret. Returns `Ok(None)` when the key is absent;
/// `Err(KeyringMissing)` when the Secret Service itself is down.
pub async fn get(service: &str, key: &str) -> Result<Option<String>, CatalogueError> {
    let service = service.to_string();
    let key = key.to_string();
    tokio::task::spawn_blocking(move || {
        let entry =
            keyring::Entry::new(&service, &key).map_err(|_| CatalogueError::KeyringMissing)?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(keyring_err_to_catalogue(e)),
        }
    })
    .await
    .map_err(keyring_join_to_catalogue)?
}

/// Delete a secret. No-op if the key didn't exist (idempotent).
pub async fn delete(service: &str, key: &str) -> Result<(), CatalogueError> {
    let service = service.to_string();
    let key = key.to_string();
    tokio::task::spawn_blocking(move || {
        let entry =
            keyring::Entry::new(&service, &key).map_err(|_| CatalogueError::KeyringMissing)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(keyring_err_to_catalogue(e)),
        }
    })
    .await
    .map_err(keyring_join_to_catalogue)?
}

fn keyring_join_to_catalogue(e: tokio::task::JoinError) -> CatalogueError {
    CatalogueError::AnthropicNetworkError {
        reason: format!("keyring task: {e}"),
    }
}

fn keyring_err_to_catalogue(e: keyring::Error) -> CatalogueError {
    match e {
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => {
            CatalogueError::KeyringMissing
        }
        other => CatalogueError::AnthropicNetworkError {
            reason: format!("keyring: {other}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_constant_matches_vision() {
        // Vision locks this to "biscuitcode" so that `secret-tool
        // search service biscuitcode` is the canonical query.
        assert_eq!(SERVICE, "biscuitcode");
    }

    #[test]
    fn secret_service_available_doesnt_panic() {
        // Tolerant test — just asserts the probe doesn't panic regardless
        // of the CI environment. The real semantic test (gnome-keyring
        // on vs. off) belongs on the VM smoke-test matrix (PHASE-5 ACs).
        let _ = secret_service_available();
    }

    #[tokio::test]
    async fn set_from_tokio_runtime_does_not_panic() {
        // Regression: without spawn_blocking, keyring's sync API drives zbus
        // which calls block_on from inside a tokio worker thread and panics
        // with "Cannot start a runtime from within a runtime" on Linux.
        // On CI / machines without a Secret Service the call returns an Err
        // (KeyringMissing). Either outcome is acceptable — the point is
        // that the call completes without panicking the runtime.
        let _ = set("biscuitcode-test", "regression-runtime-panic", "v").await;
    }
}
