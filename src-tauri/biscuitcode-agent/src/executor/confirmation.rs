//! Confirmation gate for Write and Shell class tools.
//!
//! The confirmation flow:
//!  1. Backend builds a `ConfirmationRequest` describing the tool + args.
//!  2. Emits a Tauri event `biscuitcode:confirm-request:<conversation_id>`.
//!  3. Frontend renders a modal; user clicks Approve / Deny / Deny-with-feedback.
//!  4. Frontend invokes the `agent_confirm_decision` Tauri command.
//!  5. Backend wakes the waiting `oneshot::Receiver<Decision>`.
//!
//! Deadlock guard: `await_decision` has a 60-second timeout. If the window
//! is closed or the frontend never responds the executor returns `UserDenied`
//! rather than hanging indefinitely (PM-02 fix).
//!
//! This module owns the `PendingConfirmations` shared state that the Tauri
//! command handler writes into.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

/// How the user responded to a confirmation prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Decision {
    /// User approved; proceed with snapshot + tool execution.
    Approve,
    /// User denied; return `ExecutorError::UserDenied`.
    Deny,
    /// User denied and added a message to feed back to the agent.
    DenyWithFeedback { feedback: String },
}

/// Payload the frontend renders in the confirmation modal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationRequest {
    /// Unique ID matching the tool call id — ties the frontend modal to the
    /// pending channel entry so concurrent confirmations work correctly.
    pub request_id: String,
    /// Tool class: `"write"` or `"shell"`.
    pub tool_class: String,
    /// Human-readable summary: for write/apply_patch this is the unified diff
    /// (or "new file: <contents>"); for shell this is the verbatim command.
    pub summary: String,
    /// File paths involved (for display; may be empty for shell).
    pub paths: Vec<String>,
}

/// Shared state map: request_id → sender half of a oneshot channel.
/// The Tauri command handler calls `resolve` to wake the waiting executor.
pub struct PendingConfirmations {
    inner: Mutex<HashMap<String, oneshot::Sender<Decision>>>,
}

impl PendingConfirmations {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Register a pending confirmation. Returns the receiver.
    pub fn register(&self, request_id: String) -> oneshot::Receiver<Decision> {
        let (tx, rx) = oneshot::channel();
        let mut guard = self.inner.lock().unwrap();
        guard.insert(request_id, tx);
        rx
    }

    /// Resolve a pending confirmation with a decision from the frontend.
    /// Returns `false` if the request_id was not found (already resolved or
    /// timed out).
    pub fn resolve(&self, request_id: &str, decision: Decision) -> bool {
        let mut guard = self.inner.lock().unwrap();
        if let Some(tx) = guard.remove(request_id) {
            tx.send(decision).is_ok()
        } else {
            false
        }
    }

    /// Remove a pending confirmation (cleanup on timeout).
    pub fn remove(&self, request_id: &str) {
        let mut guard = self.inner.lock().unwrap();
        guard.remove(request_id);
    }
}

impl Default for PendingConfirmations {
    fn default() -> Self {
        Self::new()
    }
}

/// Maximum time to wait for a user decision before treating as Deny.
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(60);

/// Wait for the user's decision on `rx`. Returns `Decision::Deny` on timeout.
pub async fn await_decision(
    request_id: &str,
    rx: oneshot::Receiver<Decision>,
    pending: &PendingConfirmations,
) -> Decision {
    match timeout(CONFIRMATION_TIMEOUT, rx).await {
        Ok(Ok(decision)) => decision,
        Ok(Err(_)) => {
            // Sender was dropped (should not happen).
            Decision::Deny
        }
        Err(_elapsed) => {
            // 60-second timeout — clean up the entry and deny.
            pending.remove(request_id);
            tracing::warn!(
                request_id = %request_id,
                "confirmation timed out after 60s — treating as Deny"
            );
            Decision::Deny
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn approve_resolves_receiver() {
        let pending = PendingConfirmations::new();
        let rx = pending.register("req-1".to_string());
        let resolved = pending.resolve("req-1", Decision::Approve);
        assert!(resolved, "resolve should succeed");
        let decision = rx.await.unwrap();
        assert!(matches!(decision, Decision::Approve));
    }

    #[tokio::test]
    async fn deny_resolves_receiver() {
        let pending = PendingConfirmations::new();
        let rx = pending.register("req-2".to_string());
        pending.resolve("req-2", Decision::Deny);
        let decision = rx.await.unwrap();
        assert!(matches!(decision, Decision::Deny));
    }

    #[tokio::test]
    async fn unknown_request_id_returns_false() {
        let pending = PendingConfirmations::new();
        let resolved = pending.resolve("nonexistent", Decision::Approve);
        assert!(!resolved);
    }

    #[tokio::test]
    async fn await_decision_returns_decision_when_resolved() {
        let pending = PendingConfirmations::new();
        let rx = pending.register("req-3".to_string());
        // Resolve immediately in a separate task.
        let pending2 = std::sync::Arc::new(PendingConfirmations::new());
        let p2 = pending2.clone();
        // Use the original pending to send.
        pending.resolve(
            "req-3",
            Decision::DenyWithFeedback {
                feedback: "Try a different approach".to_string(),
            },
        );
        let decision = await_decision("req-3", rx, &pending).await;
        assert!(matches!(decision, Decision::DenyWithFeedback { .. }));
        let _ = p2; // suppress unused warning
    }
}
