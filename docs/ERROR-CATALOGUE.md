# BiscuitCode — Error Catalogue

> Phase 9 audited deliverable. Codes are CLAIMED in earlier phases as their failure surfaces are built (see "Phase that registers" column). Phase 9 audits this file for completeness and ensures every entry has a passing test in `tests/error-catalogue.spec.ts`.

This file is the source of truth for every user-facing error in BiscuitCode. The rule is: **users never see a raw stack trace.** Every failure path produces an entry from this catalogue, rendered through `src/errors/ErrorToast.tsx`.

## Conventions

- Codes are `E0NN` where NN is a zero-padded sequence number in the order errors are introduced across phases.
- **Phase that registers**: the phase whose coder must add the typed error variant (Rust `thiserror` enum + TypeScript discriminated-union member).
- **Phase that audits**: always Phase 9 — the audit verifies the catalogue is current, every entry has a trigger test, and every user-facing message is i18n-keyed.
- New codes go at the bottom; codes are NEVER reused or renumbered after they ship.

## Code list (skeleton — coders fill in user-message text and recovery action specifics)

| Code | Phase that registers | Class | Trigger | User message (en bundle key) | Recovery action |
|------|---------------------|-------|---------|------------------------------|-----------------|
| `E001` | Phase 1 | `KeyringMissing` | Secret Service daemon not on user DBus session | `errors.E001.msg` — "BiscuitCode needs a system keyring to store API keys safely. Install gnome-keyring with: `sudo apt install gnome-keyring libsecret-1-0`" | Show install command + Retry button. Block onboarding step 2 until daemon reachable via `busctl --user list`. |
| `E002` | Phase 3 | `OutsideWorkspace` | A file op tried to read/write outside the open workspace root | `errors.E002.msg` — "BiscuitCode can't access files outside your workspace folder. Open a different workspace if you need to work with that file." | Toast only (the operation is silently denied at the capability layer; this notifies the user) |
| `E003` | Phase 4 | `PtyOpenFailed` | `portable-pty` could not open a new PTY (out of FDs, shell binary missing, etc.) | `errors.E003.msg` — "Couldn't open a new terminal. Reason: `<reason from PTY error>`. Try closing other terminals." | Show specific reason. If "no such shell," offer to fall back to `/bin/bash`. |
| `E004` | Phase 5 | `AnthropicAuthInvalid` | Anthropic API rejected the key with 401 | `errors.E004.msg` — "Your Anthropic API key was rejected. Check the key in Settings → Models." | Settings → Models deeplink button. |
| `E005` | Phase 5 | `AnthropicNetworkError` | Network failure talking to api.anthropic.com (DNS, TLS, timeout) | `errors.E005.msg` — "Couldn't reach Anthropic. Check your connection." | Retry button. Honors backoff. |
| `E006` | Phase 5 | `AnthropicRateLimited` | HTTP 429 from Anthropic | `errors.E006.msg` — "Anthropic is rate-limiting your requests. Try again in <retry-after> seconds." | Auto-retry after Retry-After header. Show countdown. |
| `E007` | Phase 6a | `GemmaVersionFallback` | User's Ollama < 0.20.0 doesn't recognize Gemma 4 tags; falling back to Gemma 3 | `errors.E007.msg` — "Gemma 4 isn't available on your Ollama version (need ≥ 0.20.0). Using Gemma 3 instead. Run `curl -fsSL https://ollama.com/install.sh \| sh` to upgrade." | One-time toast (suppressed on subsequent runs). Surface upgrade command verbatim. |
| `E008` | Phase 6b | `WriteToolDenied` | User declined a write-tool confirmation modal | `errors.E008.msg` — "You declined the write to `<path>`. The agent's run was paused." | Resume button (re-prompts) + Stop button (truncates the run). |
| `E009` | Phase 6b | `ShellForbiddenPrefix` | Shell tool tried to run a forbidden command (sudo, network curl, etc.) | `errors.E009.msg` — "BiscuitCode blocked the shell command `<command>` for safety. The agent's run was paused." | Show the verbatim blocked command. Trust toggle deeplink. |
| `E010` | Phase 6b | `SnapshotFailed` | Couldn't snapshot a file before a write tool ran (disk full, permission denied) | `errors.E010.msg` — "Couldn't save an undo point for `<path>`. The write was NOT performed. Free disk space or check file permissions." | Free-space tip + permission-check command. The write does NOT proceed. |
| `E011` | Phase 6b | `RewindFailed` | Snapshot manifest exists but a snapshot file is missing or its hash doesn't match | `errors.E011.msg` — "Can't undo this step — the saved version of `<path>` is missing or corrupted." | Show which file failed; offer Continue (skip restore for that file, keep going with others). |
| `E012` | Phase 7 | `GitPushFailed` | `git push` exited non-zero (auth, conflict, network) | `errors.E012.msg` — "Git push failed: `<git stderr first line>`. Check the Terminal panel for full output." | Open terminal pane to the push output. |
| `E013` | Phase 7 | `LspServerMissing` | LSP binary for the file's language is not on $PATH | `errors.E013.msg` — "The `<language>` language server isn't installed. Install with: `<exact command>`. Click to copy." | Copy-to-clipboard button with the install command. NEVER auto-runs. |
| `E014` | Phase 7 | `LspProtocolError` | LSP server crashed or sent malformed JSON-RPC | `errors.E014.msg` — "The `<language>` language server crashed. Code intelligence is disabled for this language until you restart BiscuitCode." | Restart-language-server button (relaunches just that LSP child). |
| `E015` | Phase 7 | `PreviewRenderFailed` | Markdown / HTML / PDF / image preview threw during render | `errors.E015.msg` — "Couldn't render preview for `<file>`. Reason: `<reason>`." | Show file path + best-effort reason. Editor still works; only the preview pane is affected. |
| `E016` | Phase 8 | `FontLoadFailed` | Self-hosted Inter or JetBrains Mono woff2 didn't load (canary detected fallback metrics) | `errors.E016.msg` — "BiscuitCode's bundled fonts didn't load. Falling back to system fonts. Re-installing the .deb usually fixes this." | One-time toast. Reinstall command. |
| `E017` | Phase 9 | `UpdateCheckFailed` | GitHub Releases API check failed (network, rate limit, 5xx) | `errors.E017.msg` — "Couldn't check for updates. Check your connection." | Retry button. Backs off on subsequent failures. |
| `E018` | Phase 9 | `UpdateDownloadFailed` | AppImage Tauri-updater download failed (network, signature mismatch, disk space) | `errors.E018.msg` — "Couldn't download the update: `<reason>`. Try again later or download manually from the releases page." | Open releases page button. |

## Phase 9 audit checklist

When Phase 9 runs the catalogue audit, verify each row:

- [ ] Has a Rust enum variant in `biscuitcode-core::errors`
- [ ] Has a TypeScript discriminated-union member in `src/errors/types.ts`
- [ ] Has an English bundle key in `src/locales/en.json`
- [ ] Has a passing trigger test in `tests/error-catalogue.spec.ts` that forces the failure and asserts the catalogued toast renders (NOT a stack)
- [ ] Has any documented recovery action implemented (Retry, copy-to-clipboard, deeplink, etc.)
- [ ] Is i18n-keyed throughout (no hardcoded English in source files outside `en.json`)

If any code fails any check, Phase 9 either fixes it or reports `Partial` and surfaces the gap as a blocker for Phase 10.

## Adding new codes mid-project

If a phase's coder discovers a failure surface not in the catalogue:

1. Pick the next unused `E0NN` number.
2. Add a row to this file with the phase number in the "Phase that registers" column.
3. Implement the Rust + TS variants and the i18n bundle key.
4. Add a trigger test.
5. Reference the new code in the Execution Notes for that phase in `plan.md`.
