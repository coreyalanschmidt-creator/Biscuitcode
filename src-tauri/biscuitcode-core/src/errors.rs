//! Catalogued errors — Rust source of truth.
//!
//! Every error that can reach the user is one of these variants. Backends
//! convert their internal failures into a `CatalogueError` before sending
//! to the frontend, where `ErrorToast.tsx` renders the catalogued payload.
//!
//! Mirrors `src/errors/types.ts` and `docs/ERROR-CATALOGUE.md`. When adding
//! a new variant, update all three.
//!
//! Phase 1 ships ONE variant fully wired (`E001 KeyringMissing`) as the
//! proof-of-concept; subsequent phases register their own variants in this
//! enum and add corresponding TS types + i18n keys.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Top-level error type. Every variant has a stable `E0NN` code that maps
/// 1:1 to a row in `docs/ERROR-CATALOGUE.md`.
///
/// Codes are NEVER reused after they ship — adding a new failure surface
/// always claims the next unused E0NN.
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "UPPERCASE")]
pub enum CatalogueError {
    /// Phase 1 — Secret Service daemon unavailable.
    #[serde(rename = "E001")]
    #[error("E001 KeyringMissing: Secret Service daemon (gnome-keyring or equivalent) is not reachable on the user DBus session.")]
    KeyringMissing,

    // ----- Variants below are CLAIMED but not yet IMPLEMENTED. -----
    // Each owning phase fills in fields + helper constructors as it lands.

    /// Phase 3 — File op attempted outside the open workspace root.
    #[serde(rename = "E002")]
    #[error("E002 OutsideWorkspace: path {path} is not a descendant of the workspace root")]
    OutsideWorkspace { path: String },

    /// Phase 4 — `portable-pty` could not open a new PTY.
    #[serde(rename = "E003")]
    #[error("E003 PtyOpenFailed: {reason}")]
    PtyOpenFailed { reason: String },

    /// Phase 5 — Anthropic returned 401.
    #[serde(rename = "E004")]
    #[error("E004 AnthropicAuthInvalid")]
    AnthropicAuthInvalid,

    /// Phase 5 — Network failure to api.anthropic.com.
    #[serde(rename = "E005")]
    #[error("E005 AnthropicNetworkError: {reason}")]
    AnthropicNetworkError { reason: String },

    /// Phase 5 — Anthropic 429.
    #[serde(rename = "E006")]
    #[error("E006 AnthropicRateLimited (retry after {retry_after_seconds}s)")]
    AnthropicRateLimited { retry_after_seconds: u64 },

    /// Phase 6a — Ollama < 0.20.0; falling back to Gemma 3.
    #[serde(rename = "E007")]
    #[error("E007 GemmaVersionFallback: Gemma 4 unavailable on Ollama {ollama_version}; using {fallback_model}")]
    GemmaVersionFallback {
        ollama_version: String,
        fallback_model: String,
    },

    /// Phase 6b — User declined a write-tool confirmation.
    #[serde(rename = "E008")]
    #[error("E008 WriteToolDenied: user declined {tool_name} on {path}")]
    WriteToolDenied { tool_name: String, path: String },

    /// Phase 6b — Shell tool tried a forbidden command.
    #[serde(rename = "E009")]
    #[error("E009 ShellForbiddenPrefix: blocked `{command}`")]
    ShellForbiddenPrefix { command: String },

    /// Phase 6b — Pre-write snapshot failed; write was NOT performed.
    #[serde(rename = "E010")]
    #[error("E010 SnapshotFailed: couldn't snapshot {path}: {reason}")]
    SnapshotFailed { path: String, reason: String },

    /// Phase 6b — Snapshot manifest references a missing/corrupt .bak file.
    #[serde(rename = "E011")]
    #[error("E011 RewindFailed: can't restore {path}: {reason}")]
    RewindFailed { path: String, reason: String },

    /// Phase 7 — git push exited non-zero.
    #[serde(rename = "E012")]
    #[error("E012 GitPushFailed: {git_stderr}")]
    GitPushFailed { git_stderr: String },

    /// Phase 7 — LSP binary not on PATH.
    #[serde(rename = "E013")]
    #[error("E013 LspServerMissing: {language} ({install_command})")]
    LspServerMissing {
        language: String,
        install_command: String,
    },

    /// Phase 7 — LSP server crash or malformed JSON-RPC.
    #[serde(rename = "E014")]
    #[error("E014 LspProtocolError: {language}: {reason}")]
    LspProtocolError { language: String, reason: String },

    /// Phase 7 — Preview render threw.
    #[serde(rename = "E015")]
    #[error("E015 PreviewRenderFailed: {file}: {reason}")]
    PreviewRenderFailed { file: String, reason: String },

    /// Phase 8 — Self-hosted font failed to load (canary detected).
    #[serde(rename = "E016")]
    #[error("E016 FontLoadFailed: {font_family}")]
    FontLoadFailed { font_family: String },

    /// Phase 9 — Update check failed.
    #[serde(rename = "E017")]
    #[error("E017 UpdateCheckFailed: {reason}")]
    UpdateCheckFailed { reason: String },

    /// Phase 9 — Update download failed.
    #[serde(rename = "E018")]
    #[error("E018 UpdateDownloadFailed: {reason}")]
    UpdateDownloadFailed { reason: String },
}

impl CatalogueError {
    /// Stable `E0NN` code string. Used for telemetry tagging and the
    /// front-end discriminated-union dispatch.
    pub fn code(&self) -> &'static str {
        match self {
            Self::KeyringMissing             => "E001",
            Self::OutsideWorkspace { .. }    => "E002",
            Self::PtyOpenFailed { .. }       => "E003",
            Self::AnthropicAuthInvalid       => "E004",
            Self::AnthropicNetworkError { .. } => "E005",
            Self::AnthropicRateLimited { .. } => "E006",
            Self::GemmaVersionFallback { .. } => "E007",
            Self::WriteToolDenied { .. }     => "E008",
            Self::ShellForbiddenPrefix { .. } => "E009",
            Self::SnapshotFailed { .. }      => "E010",
            Self::RewindFailed { .. }        => "E011",
            Self::GitPushFailed { .. }       => "E012",
            Self::LspServerMissing { .. }    => "E013",
            Self::LspProtocolError { .. }    => "E014",
            Self::PreviewRenderFailed { .. } => "E015",
            Self::FontLoadFailed { .. }      => "E016",
            Self::UpdateCheckFailed { .. }   => "E017",
            Self::UpdateDownloadFailed { .. } => "E018",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_codes_are_distinct_and_match_format() {
        // Sanity: every variant returns a unique E0NN string.
        let codes = [
            CatalogueError::KeyringMissing.code(),
            CatalogueError::OutsideWorkspace { path: "/x".into() }.code(),
            CatalogueError::PtyOpenFailed { reason: "x".into() }.code(),
            CatalogueError::AnthropicAuthInvalid.code(),
            CatalogueError::AnthropicNetworkError { reason: "x".into() }.code(),
            CatalogueError::AnthropicRateLimited { retry_after_seconds: 1 }.code(),
            CatalogueError::GemmaVersionFallback {
                ollama_version: "0.19".into(),
                fallback_model: "gemma3:4b".into(),
            }.code(),
            CatalogueError::WriteToolDenied {
                tool_name: "write_file".into(),
                path: "/x".into(),
            }.code(),
            CatalogueError::ShellForbiddenPrefix { command: "sudo".into() }.code(),
            CatalogueError::SnapshotFailed {
                path: "/x".into(), reason: "x".into(),
            }.code(),
            CatalogueError::RewindFailed {
                path: "/x".into(), reason: "x".into(),
            }.code(),
            CatalogueError::GitPushFailed { git_stderr: "x".into() }.code(),
            CatalogueError::LspServerMissing {
                language: "rust".into(),
                install_command: "rustup".into(),
            }.code(),
            CatalogueError::LspProtocolError {
                language: "rust".into(), reason: "x".into(),
            }.code(),
            CatalogueError::PreviewRenderFailed {
                file: "x.md".into(), reason: "x".into(),
            }.code(),
            CatalogueError::FontLoadFailed { font_family: "Inter".into() }.code(),
            CatalogueError::UpdateCheckFailed { reason: "x".into() }.code(),
            CatalogueError::UpdateDownloadFailed { reason: "x".into() }.code(),
        ];

        // Distinct
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len(), "duplicate error codes detected");

        // Format
        for c in &codes {
            assert_eq!(c.len(), 4, "code {} is not 4 chars", c);
            assert!(c.starts_with('E'), "code {} doesn't start with E", c);
            assert!(c[1..].chars().all(|ch| ch.is_ascii_digit()),
                "code {} non-digit suffix", c);
        }
    }

    #[test]
    fn phase_1_keyring_missing_works() {
        let e = CatalogueError::KeyringMissing;
        assert_eq!(e.code(), "E001");
        assert!(format!("{}", e).contains("KeyringMissing"));
    }
}
