# Changelog

All notable changes to BiscuitCode are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — 2026-04-21

Initial public release.

### Added

- **Editor** — Monaco editor with file tree, multi-tab workspace, cross-file
  find & replace (`Ctrl+Shift+F`), split-view (`Ctrl+\`), quick-open
  (`Ctrl+P`), per-file blame.
- **Integrated terminal** — xterm.js frontend + `portable-pty` Rust backend;
  multi-tab; `path:line[:col]` links routed to the editor via
  `biscuitcode:open-file-at`; `Ctrl+`` focus.
- **AI chat panel** — virtualised message list (react-virtuoso),
  Markdown + math + code-highlight rendering, SQLite-persisted
  conversations, streaming tokens via Tauri events.
- **Providers** — Anthropic (Claude Opus 4.7 / Sonnet 4.6 / Haiku 4.5,
  legacy 4.6); OpenAI (GPT 5.4 family + legacy 5.2-thinking); Ollama
  (Gemma 4 `e2b`/`e4b`/`26b`/`31b`, XML-tag fallback for Gemma 3
  community fine-tunes). Prompt caching on by default for Anthropic
  system + tool blocks.
- **Agent loop** — ReAct executor with read-only tools (`read_file`,
  `search_code`) always available; write tools (`write_file`,
  `apply_patch`, `run_shell`) gated behind per-action confirmation
  unless workspace trust is enabled.
- **Confirmation + rewind** — every write/shell action pre-snapshots
  affected files with SHA-256 verification and fsync ordering;
  per-message rewind restores snapshots and truncates the conversation.
- **Inline AI edit** — `Ctrl+K Ctrl+I` opens a Zed-style split-diff
  pane with streaming provider output and Accept/Reject/Regenerate.
- **Git panel** — branch picker, staged/unstaged/untracked groups with
  hunk-level stage, commit composer, blame trigger.
- **LSP backend** — `biscuitcode-lsp` crate spawns and frames
  rust-analyzer / typescript-language-server / pyright / gopls / clangd
  child processes with proper Content-Length handling and shutdown.
- **Preview panel** — Markdown + HTML (sandboxed iframe) + image + PDF
  (`react-pdf`) + Jupyter notebook, routed by `biscuitcode:preview-file`.
- **Onboarding + settings** — 3-screen onboarding (Welcome → Models →
  Open Folder); 8-section settings page (general, editor, terminal,
  appearance, models, security, conversations, about) with three themes
  (BiscuitCode Warm, BiscuitCode Cream, High Contrast).
- **Secrets** — API keys stored in libsecret via the `keyring` crate;
  pre-flight DBus name-check (`busctl --user list`) never activates the
  daemon with a credential; `keyring::Entry` calls wrapped in
  `spawn_blocking` to avoid zbus runtime panics under Tokio.
- **Error catalogue** — E001–E019 wired end-to-end (Rust enum, TS
  discriminated union, i18n, toast UI); each code has a catalogued
  message and recovery action. See `docs/ERROR-CATALOGUE.md`.
- **i18n** — 244 static translation keys; `scripts/check-i18n.js` gate
  on every PR.
- **Packaging** — signed `.deb` + AppImage; Tauri minisign updater
  (`latest.json` at release root); GitHub Releases workflow with GPG
  detach-sign + SHA256SUMS.
- **CI** — lint (clippy + eslint), typecheck, test (cargo + vitest +
  i18n), e2e (Vitest via second config), audit (cargo + pnpm), license
  compatibility.

### Known limitations in 1.0

- **LSP hover / go-to-definition not wired to the editor.** The Rust
  LSP backend is complete and tested, but the `monaco-languageclient`
  frontend adapter is deferred post-v1 (Open Question #19 in
  `docs/plan.md`) due to Vite ESM issues with
  `@codingame/monaco-vscode-api` transitive deps.
- **Wayland-XFCE not supported.** XFCE 4.18 has no Wayland session;
  the `.deb` targets X11-XFCE on Mint 22. Cinnamon-Wayland is
  best-effort.
- **Linux only.** macOS / Windows targets are v2 material.
- **e2e tests run under Vitest**, not Playwright. The `@playwright/test`
  dev dependency is installed and `playwright.config.ts` exists as a
  forward hook, but the specs themselves use React Testing Library;
  swapping to real browser automation is a v1.1 item.

### Security

- API keys never touch config files, environment variables, crash
  reports, or logs.
- Tauri capabilities are deny-by-default. `fs` permissions are scoped
  to `$APPCONFIG`/`$APPDATA`/`$APPCACHE`; `http` has no webview
  allow-list (Rust `reqwest` handles provider HTTP).
- `run_shell` rejects `sudo`/`su`/`doas` prefixes (E009), blocks
  shell metacharacters outside single-quoted strings, and allowlists
  `curl` to provider hosts + localhost only.
- Release `.deb` and AppImage are GPG-detach-signed; `SHA256SUMS.txt`
  is published alongside. GPG fingerprint:
  `58FA85F45D785B47641EFE725F30C6E47B7D2DE4`.

### Architecture notes

- Internal Rust crates are prefixed `biscuitcode-*` to avoid the
  existing `biscuit` and `biscuit-auth` crates on crates.io.
- Debian package name is `biscuit-code` (kebab-case). Tauri 2.x has no
  override for the auto-derived package name; all docs and CI scripts
  use `biscuit-code` (`apt remove biscuit-code`, `dpkg -s biscuit-code`).
- `tauri-plugin-stronghold` is not used. See
  `docs/adr/0001-no-stronghold.md` for why.

[1.0.0]: https://github.com/coreyalanschmidt-creator/biscuitcode/releases/tag/v1.0.0
