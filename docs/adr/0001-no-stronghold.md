# ADR 0001 — Do not use `tauri-plugin-stronghold` for secrets

**Status:** Accepted (2026-04-18)
**Phase introduced:** Phase 1 (deliverable)
**Source:** research-r2.md "Biggest surprise"

## Context

Several historic Tauri tutorials and the Tauri docs themselves have recommended `tauri-plugin-stronghold` for storing API keys, refresh tokens, and other secrets. A maintainer searching "Tauri secrets" or "Tauri keyring" in 2026 is highly likely to land on Stronghold pages and conclude it is the correct dependency.

It is not.

## Decision

BiscuitCode **shall not depend on, reference, or evaluate `tauri-plugin-stronghold`** as a path to API-key storage. The only secrets path is the **Rust `keyring` crate** wrapping libsecret on Linux (and platform-equivalents on macOS/Windows in the future).

If a future feature appears to require Stronghold-specific functionality (encrypted file vaults, mnemonic-derived keys, etc.), that requirement must be re-examined: in almost every case, the standard Secret Service / libsecret path covers it, and where it doesn't, a small custom solution beats a deprecated dependency.

## Why

Per the **Tauri 2.x → 3.x migration guidance** (verified against the Tauri changelog and plugin maintainer announcements as of early 2026):

- `tauri-plugin-stronghold` is deprecated.
- It will be **removed entirely from Tauri v3**.
- Its IOTA Stronghold backend has its own evolution and is not on the Tauri team's maintenance roadmap.
- Adopting it now would make a future v3 migration require a forced secrets-store rewrite, including a credential-migration step on every existing user's machine.

Meanwhile, the **`keyring` crate** (3.6.x at time of writing):

- Is actively maintained and stable.
- Talks to libsecret on Linux (Mint 22 XFCE ships gnome-keyring + libsecret by default).
- Has equivalent paths for Keychain (macOS) and Credential Manager (Windows) without changes to our `biscuitcode-core::secrets` API.
- Is what BiscuitCode's plan (assumption #5, architecture decision §Secrets) specifies.

## Consequences

- We must accept that BiscuitCode **cannot run** on a Linux session with no Secret Service daemon. This is enforced at onboarding (`busctl --user list | grep org.freedesktop.secrets`) and presented to the user with a specific install command (`sudo apt install gnome-keyring libsecret-1-0 libsecret-tools`) — not a silent plaintext fallback (which the vision explicitly forbids).
- Future maintainer onboarding documentation (this ADR + `docs/DEV-SETUP.md`) must state the no-Stronghold rule explicitly, since web search will mislead.
- If Tauri's official secrets story changes between now and v1.0 release (e.g., a new `tauri-plugin-keyring` materializes), revisit this ADR.

## References

- research-r2.md → "Biggest Surprise"
- Tauri 2.x deprecation notice for `tauri-plugin-stronghold`
- `keyring` crate docs: https://docs.rs/keyring/3.6/keyring/
- Vision §Hard Constraints #7: "Secrets: API keys stored in system keyring via libsecret (`keyring` Rust crate). Never in plaintext config. Never in env vars."
