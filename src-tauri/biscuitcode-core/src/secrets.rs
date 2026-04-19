//! Secret storage — `keyring` crate wrapper over libsecret on Linux.
//!
//! Phase 5 deliverable. The vision + plan + `docs/adr/0001-no-stronghold.md`
//! are emphatic: API keys live in libsecret ONLY. No plaintext fallback.
//! No env vars. No stronghold. No anything else.
//!
//! Phase 5 coder fills in the actual `keyring::Entry` calls once the
//! exact 3.6.x feature-flag incantation is confirmed against the crate's
//! current docs. The API surface here is locked so the settings UI and
//! provider layer can be written against it before the impl lands.
//!
//! **Pre-flight contract (docs/design/CAPABILITIES.md; synthesis log):**
//! every call path that might touch the Secret Service must first call
//! [`secret_service_available`]. That function uses a **read-only
//! DBus name-check** (`busctl --user list`) — it NEVER activates the
//! daemon with a known credential, which `keyring::Entry::get_password`
//! would do as a side-effect on some Linux distributions.

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
///   - `Ok(true)`  — Secret Service reachable; it's safe to call keyring ops.
///   - `Ok(false)` — NOT reachable; onboarding must block and surface E001.
///   - `Err(...)`  — `busctl` itself failed (should not happen — it's part
///                   of systemd and verified by `bootstrap-wsl.sh`).
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

/// Store a secret for (service, key). Phase 5 coder fills in with
/// `keyring::Entry::new(service, key).set_password(value)`.
///
/// Requires [`secret_service_available`] returned `Ok(true)` for the
/// current session. Call that BEFORE this; if false, surface E001.
pub async fn set(service: &str, key: &str, _value: &str) -> Result<(), CatalogueError> {
    let _ = (service, key, _value);
    // ---- Phase 5 coder fills in ----
    // The exact keyring::Entry::new signature depends on the feature-flag
    // set in biscuitcode-core/Cargo.toml (see plan: "linux-native-async-
    // persistent + async-secret-service + crypto-rust + tokio"). Phase 5
    // coder's job to pick the flags that match the installed keyring 3.6.x.
    Err(CatalogueError::KeyringMissing)
}

/// Retrieve a secret. Returns `Ok(None)` when the key is absent;
/// `Err(KeyringMissing)` when the Secret Service itself is down.
pub async fn get(service: &str, key: &str) -> Result<Option<String>, CatalogueError> {
    let _ = (service, key);
    Err(CatalogueError::KeyringMissing)
}

/// Delete a secret. No-op if the key didn't exist (idempotent).
pub async fn delete(service: &str, key: &str) -> Result<(), CatalogueError> {
    let _ = (service, key);
    Err(CatalogueError::KeyringMissing)
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
}
