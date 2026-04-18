# Implementation Plan: BiscuitCode (Round 1)

> Round 1 of 2. A synthesis pass will merge with plan-r2.md into final docs/plan.md.

## Review Log

_Empty — reviewer round 1 will fill._

## Vision Summary

BiscuitCode is a Tauri 2.x + React + TypeScript desktop AI coding environment targeting Linux Mint 22 XFCE (Ubuntu 24.04 / WebKitGTK 4.1 / kernel 6.8+) with VS Code parity: Monaco editor, xterm.js over `portable-pty`, LSP, git, preview pane, and a four-region resizable shell. It ships three AI providers (Anthropic, OpenAI, Ollama) behind a unified `ModelProvider` trait, a ReAct agent loop with per-action rewind, API keys in libsecret via the `keyring` crate (no plaintext fallback), and a persistent conversation DAG in SQLite. "Done" means a signed `biscuitcode_1.0.0_amd64.deb` that installs cleanly on a stock Mint 22 XFCE VM, appears in the Whisker menu under Development, cold-launches in under 2s on i5-8xxx / 8 GB hardware, completes the 3-screen onboarding inside 2 minutes, and survives `apt remove biscuitcode` cleanly — with all brand tokens, Inter/JetBrains Mono typography, and security posture matching the vision verbatim.

## Assumptions

Carried from research-r1.md, then extended with planning-specific assumptions. All flagged by confidence.

1. **[HIGH]** Mint 22.1 Xia is the canonical target; 22.0 and 22.2 are compatibility goals. CI runs on `ubuntu-24.04` which matches the noble base.
2. **[HIGH]** Tauri `v2.10.x` is the build pin. Capability ACL files are hand-authored, not `migrate`-generated.
3. **[HIGH]** Linux webview is `libwebkit2gtk-4.1-0`; the `.deb` declares it as a `Depends`.
4. **[HIGH]** `@xterm/*` scoped packages only; no legacy `xterm-addon-*`.
5. **[HIGH]** `keyring` 3.6 features: `linux-native-async-persistent + async-secret-service + crypto-rust + tokio`. Plaintext fallback is prohibited — block onboarding instead.
6. **[HIGH]** The editor uses a single Monaco instance with one `ITextModel` per open tab; Monaco loads lazily after first paint to meet the 2s cold-launch budget.
7. **[HIGH]** Rust spawns language servers and proxies stdio via Tauri events; frontend wires `monaco-languageclient` with custom `MessageTransports`.
8. **[HIGH]** SQLite uses `rusqlite` directly (no `plugin-sql`), WAL mode, `PRAGMA user_version` migrations.
9. **[HIGH]** Provider model defaults — **updated from vision**: Anthropic `claude-opus-4-7` (omit `temperature`/`top_p`/`top_k`), OpenAI `gpt-5.4-mini` (NOT `gpt-4o` — retired 2026-04-03), Ollama `gemma3:4b` default with RAM-tiered fallbacks (`gemma3:1b` under 6 GB, `qwen2.5-coder:7b` at 12 GB+, `gemma4:*` preferred when available for native tool calls).
10. **[HIGH]** All code-phase work runs from WSL2 Ubuntu 24.04 with the project rooted in `~/`, never `/mnt/c/`. A coder invoked from Windows without WSL2 must stop and report.
11. **[HIGH]** Debian packaging uses `cargo-tauri-bundle`; AppImage needs `libfuse2t64` on noble — we ship the AppImage but the `.deb` is primary.
12. **[MED]** GitHub Actions runner is `ubuntu-24.04`, not `-latest`. GPG-signing via `GPG_PRIVATE_KEY` secret; SHA256 via `sha256sum`.
13. **[MED]** Icon Concept A ("The Prompt") ships in v1. A 16x16 render check happens inside Phase 9 before the icon is declared done; Concept D is deferred unless A fails the legibility test.
14. **[MED]** Telemetry is scaffolded as an off-by-default setting in v1 but no wire implementation — deferred pending the maintainer's choice of endpoint (Open Question).
15. **[MED]** Secret Service presence is verified at onboarding. On bare XFCE sessions where `gnome-keyring` is missing, we show a specific install prompt and block API-key entry. We also declare `Recommends: gnome-keyring, ollama` in the `.deb` control file.
16. **[MED]** Notebook preview is **read-only render in v1** (per vision); execution is explicitly v2.
17. **[LOW]** Arm64 is NOT a v1 target. `.deb` ships x86_64 only.
18. **[LOW]** VS Code theme import is a UI placeholder only in v1 (per vision).

## Architecture Decisions

Each decision cites the research section that justifies it.

- **Tauri v2.10.x with hand-authored capability files** under `src-tauri/capabilities/{fs,shell,http,core}.json`, deny-by-default scopes. No `tauri migrate` output. (research §1, §13, Best Practice #1)
- **Linux webview pin to WebKitGTK 4.1** (`libwebkit2gtk-4.1-0`); all CSS must render correctly in 4.1 patch levels across 22.0/22.1/22.2 — smoke test includes the `:has()` + container-query check. (research §1, Risks #1)
- **React frontend with Zustand stores** for UI state; `react-resizable-panels` for the four-region layout. No Dockview — we don't need floating panels in v1. (research Trade-offs: Layout)
- **Monaco via `@monaco-editor/react`** pinned locally with `vite-plugin-monaco-editor`; single editor instance + per-tab `ITextModel`; diff view via `createDiffEditor`; dynamic import deferred to after first paint or first file-open. (research §4, Best Practice #5)
- **xterm.js 5.x (`@xterm/xterm`) with addons `fit`, `web-links`, `search`, `webgl` (canvas fallback)**; Rust `portable-pty` 0.8+, one PTY per tab, two Tokio tasks per stream direction keyed by `session_id`. (research §5)
- **Rust `ModelProvider` trait → flattened `ChatEvent` enum** (`TextDelta`, `ThinkingDelta`, `ToolCallStart/Delta/End`, `Done`, `Error`). Provider quirks live inside each impl; the trait stays minimal. (research §7, Best Practice #4)
- **Updated model defaults**: Anthropic `claude-opus-4-7` (no sampling params); OpenAI `gpt-5.4-mini`; Ollama `gemma3:4b` with RAM-tiered fallbacks. Legacy IDs excluded from defaults but kept available in the picker. (research §7, Risks #3, #4, Assumption #9)
- **ReAct agent loop with pause checks between iterations**. Rewind = per-action snapshot of modified-file contents keyed by `(conversation_id, message_id)`. Trust boundary = workspace root. (research §8)
- **LSP via `monaco-languageclient` + Rust stdio proxy** over Tauri events. One language-server child process per (language, workspace) pair. (research §9)
- **SQLite via `rusqlite`** (no plugin), WAL mode, `PRAGMA user_version` migrations, DAG message schema with `parent_id` for branching. (research §10, Best Practice #7)
- **Theme detection via `xfconf-query -c xsettings -p /Net/ThemeName`** with `gsettings` fallback; dark/light heuristic = `-dark$` regex. (research §11)
- **Git reads via `git2-rs`, writes via shelling out to `git`** — inherits user's `.gitconfig`, credential helpers, signing, LFS. (Best Practice #8)
- **Packaging**: primary `.deb` built by `cargo-tauri-bundle`; AppImage is secondary with documented `libfuse2t64` caveat. Both produced by `tauri-apps/tauri-action` on `ubuntu-24.04` runners, GPG-signed, SHA256 published. (research §2, §12, Best Practice #10)
- **Security posture**: deny-by-default Tauri capabilities scoped to workspace root + app config/data dirs; HTTP scope limited to three provider hosts; shell execute limited to a pre-registered command registry; API keys strictly in keyring. (research §13)
- **Internal Rust crate names all prefixed `biscuitcode-*`** (`-core`, `-agent`, `-providers`, `-lsp`, `-pty`, `-db`). Defensive claim of `biscuitcode` on crates.io on day 1. (research §15, Best Practice namespace)

## Phase Index

| # | Phase | Status | Complexity | Depends on |
|---|-------|--------|------------|------------|
| 0 | Dev Environment Bootstrap (WSL2 + toolchain) | Not Started | Low | — |
| 1 | Scaffold + Brand Tokens + Capability Skeleton | Not Started | Medium | 0 |
| 2 | Four-Region Layout + Shortcuts + Installable .deb | Not Started | Medium | 1 |
| 3 | Editor + File Tree + Find/Replace | Not Started | Medium | 2 |
| 4 | Terminal (xterm.js + portable-pty) | Not Started | Medium | 2 |
| 5 | Keyring + Anthropic Provider + Chat Panel (E2E text-only) | Not Started | Medium | 2 |
| 6 | Agent Loop + Tool Registry + Inline Edit + Rewind | Not Started | High | 3, 5 |
| 7 | OpenAI + Ollama Providers + Ollama Detection/Install | Not Started | Medium | 5 |
| 8 | Git Panel + LSP Client + Preview Panel | Not Started | High | 3 |
| 9 | Onboarding + Settings UI + Theming + Icon | Not Started | Medium | 5, 7, 8 |
| 10 | Packaging + CI + GPG Signing + Release Smoke Test | Not Started | Medium | 9 |

Total: **11 phases** (0 through 10). Estimated calendar: Phase 0 half day, Phases 1/2/4/7/9/10 ~1 day each, Phases 3/5/8 ~2 days each, Phase 6 ~3 days. Total ~15 focused working days for a solo maintainer, aligned with the vision's 21-day sketch after WSL2 bootstrap + mid-phase re-plans.

## Phases

### Phase 0 — Dev Environment Bootstrap (WSL2 + toolchain)
**Goal:** Bring the Windows-host maintainer to a working WSL2 Ubuntu 24.04 dev environment with `cargo tauri --version` succeeding, project living in `~/biscuitcode/`, and all apt deps installed, **before any code phase runs**.

**Deliverables:**
- `scripts/bootstrap-wsl.sh` — idempotent apt install of the full Tauri prereq list (`pkg-config libdbus-1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf libfuse2t64 file build-essential curl gnome-keyring libsecret-1-0 libsecret-tools`).
- `scripts/bootstrap-toolchain.sh` — installs rustup (stable 1.85+), `cargo-tauri-cli@2.10.1`, Node.js 20+ via nvm, `pnpm@9+`.
- `docs/DEV-SETUP.md` (new, short) — WSL2 install, why not `/mnt/c/`, how to run the bootstrap scripts, how to launch `pnpm tauri dev` into WSLg.
- Sanity output committed to PR description: `cargo tauri --version`, `node --version`, `pnpm --version`, `rustc --version`, `apt list --installed | grep webkit2gtk-4.1-dev`.

**Acceptance criteria:**
- [ ] Running `bash scripts/bootstrap-wsl.sh` on a fresh WSL2 Ubuntu 24.04 completes without errors and exits 0.
- [ ] `cargo tauri --version` prints `tauri-cli 2.10.x`.
- [ ] `pnpm --version` prints `9.x` or higher.
- [ ] `apt list --installed 2>/dev/null | grep libwebkit2gtk-4.1-dev` returns a line.
- [ ] `systemctl --user status gnome-keyring-daemon` shows an active unit OR the script documents the PAM-start workaround.
- [ ] Project working directory resolves under `$HOME` (not `/mnt/c/`); `realpath .` is asserted in the script.
- [ ] `docs/DEV-SETUP.md` exists and is linked from `README.md`.

**Dependencies:** None.
**Complexity:** Low.
**Split rationale:** The vision assumes a working dev env. Research-r1 §3 documents multiple WSL2 gotchas (inotify on `/mnt/c`, webkit-4.0 vs 4.1 rename, `libfuse2t64` confusion) that are each single-sentence fixes only if the environment is correct from minute one. Making bootstrap a named phase enforces "Phase 1 can actually build" rather than discovering missing libs mid-scaffold. It is deliberately Low complexity and half-a-day — short on purpose so it doesn't inflate the real phases.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 1 — Scaffold + Brand Tokens + Capability Skeleton
**Goal:** Create the empty BiscuitCode Tauri project, wire brand tokens into Tailwind and Rust palette constants, author the capability ACL files, and ship a window that paints on cocoa-700 with the biscuit accent.

**Deliverables:**
- `pnpm create tauri-app` output scaffolded with React + TS + Vite + Tailwind, app name `biscuitcode`, bundle ID `io.github.Coreyalanschmidt-creator.biscuitcode`.
- Internal workspace crates: `biscuitcode-core`, with placeholder `biscuitcode-agent`, `biscuitcode-providers`, `biscuitcode-db`, `biscuitcode-pty`, `biscuitcode-lsp` (empty lib.rs files ready for later phases).
- `tauri.conf.json` with `bundle.active: true`, `bundle.identifier`, Linux section declaring `webkitVersion: "4.1"` and `deb.depends: ["libwebkit2gtk-4.1-0", "libgtk-3-0"]`, `deb.recommends: ["gnome-keyring", "ollama"]`.
- `tailwind.config.ts` with brand tokens *verbatim* as CSS custom properties + Tailwind theme extension (`biscuit-50..900`, `cocoa-50..900`, semantic `ok/warn/error`).
- Self-hosted fonts: `src-tauri/fonts/Inter-{Regular,Medium,SemiBold}.woff2`, `JetBrainsMono-{Regular,Medium}.woff2`. `@font-face` rules in `src/theme/fonts.css`; **no `system-ui` fallback** for primary UI text.
- `src-tauri/capabilities/{core,fs,shell,http}.json` — hand-authored, deny-by-default. Core capability grants only `core:default`. `fs` allows `$APPCONFIG`, `$APPDATA`, `$APPCACHE` and nothing else yet. `shell` and `http` have no allow rules in this phase (added per feature).
- `src/theme/tokens.ts` exporting the palette as TS constants for JS-only colour math.
- Rust `biscuitcode-core::palette` module exposing the same values.
- Window chrome: default decorations off, custom titlebar showing `BiscuitCode` in Inter 14px, cocoa-700 bg.

**Acceptance criteria:**
- [ ] `pnpm install && pnpm tauri dev` opens a WSLg window in under 2s on the dev machine.
- [ ] Document background is `#1C1610`; a single `--biscuit-500` (`#E8B04C`) accent bar renders on the sidebar placeholder.
- [ ] `curl -sS http://localhost:1420/` (Vite dev) returns HTML with `Inter` loaded from `/fonts/`, not CDN (`grep -v 'fonts.googleapis' /index.html`).
- [ ] `src-tauri/capabilities/fs.json` contains `"permissions"` with `fs:allow-read-text-file` scoped to `$APPCONFIG` only; `grep -c '"identifier": "fs:allow-write"' src-tauri/capabilities/fs.json` returns `0`.
- [ ] `cargo tree -p biscuitcode-core` lists `biscuitcode-core` and `biscuitcode-agent` as workspace members.
- [ ] `cargo build -p biscuitcode-core` succeeds with `-D warnings`.
- [ ] `grep -r 'biscuit-auth\|^biscuit ' src-tauri/Cargo.toml` returns nothing (no crate name collision).

**Dependencies:** Phase 0.
**Complexity:** Medium.
**Split rationale:** The vision lumps "scaffold + layout" into Phase 1. I split those because the scaffold ALSO owns the brand-token + capability-skeleton choices that are load-bearing for every later phase — getting them wrong means retrofits. Keeping this phase scoped to "project exists, brand is wired, capabilities deny-by-default" gives a clean mergeable commit without also committing to react-resizable-panels + shortcut wiring. The layout work is Phase 2, where it's the whole focus.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 2 — Four-Region Layout + Shortcuts + Installable .deb
**Goal:** Render the Activity Bar / Side Panel / Editor Area / Bottom Panel / Chat Panel / Status Bar layout with `react-resizable-panels`, wire every toggle shortcut, and produce the first installable-to-VM `.deb`.

**Deliverables:**
- `src/layout/WorkspaceGrid.tsx` using `react-resizable-panels` with persisted sizes via `plugin-window-state` or a Zustand-backed localStorage bridge.
- Components (empty shells, 1-2 lines each): `ActivityBar`, `SidePanel`, `EditorArea`, `TerminalPanel`, `ChatPanel`, `AgentActivityPanel`, `PreviewPanel`, `StatusBar`. Each renders a labelled placeholder so the region is visible.
- `ActivityBar` 48 px, icons via `lucide-react` (Files, Search, Git, Chats, Settings). Active icon gets a 2 px `--biscuit-500` left-edge bar.
- Shortcut layer in `src/shortcuts/global.ts` handling: `Ctrl+B` (side), `Ctrl+J` (bottom), `Ctrl+Alt+C` (chat), `Ctrl+Shift+P` (palette placeholder), `Ctrl+\`` (terminal focus placeholder), `Ctrl+P` (quick-open placeholder), `F1` (help placeholder), `Ctrl+Shift+L` (new chat placeholder), `Ctrl+K Ctrl+I` (inline edit placeholder). Chord support via a two-stage handler.
- Command palette (`Ctrl+Shift+P`) with registered commands: `View: Toggle Side Panel`, `View: Toggle Bottom Panel`, `View: Toggle Chat Panel`. Enough to prove the registry works.
- Status bar renders `git:main`, `0 errors`, `claude-opus-4-7`, `Ln 0 C0` — all static placeholders this phase.
- `cargo tauri build --target x86_64-unknown-linux-gnu` produces `biscuitcode_0.1.0_amd64.deb`.

**Acceptance criteria:**
- [ ] Every region in the vision's ASCII layout renders with the correct default size (Activity 48px, Side 260px, Bottom 240px, Chat 380px).
- [ ] Pressing `Ctrl+B` toggles side panel visibility; after re-open the previous width is restored.
- [ ] Pressing `Ctrl+Shift+P`, typing "toggle bottom", pressing Enter toggles the bottom panel.
- [ ] `pnpm tauri build` produces `src-tauri/target/release/bundle/deb/biscuitcode_0.1.0_amd64.deb`.
- [ ] On a Mint 22 XFCE VM: `sudo dpkg -i biscuitcode_0.1.0_amd64.deb` then `biscuitcode --version` prints `0.1.0`.
- [ ] After install, Whisker menu → Development → **BiscuitCode** exists with the placeholder icon and launches the app.
- [ ] `sudo apt remove biscuitcode` removes the binary, desktop entry, and icon; `ls /usr/share/applications/biscuitcode.desktop` returns no such file.

**Dependencies:** Phase 1.
**Complexity:** Medium.
**Split rationale:** This is the phase where the app first becomes a thing a user could install, which is the vision's Phase 1 runnable checkpoint. Bundling the shortcut layer in here (rather than deferring to phase 9 polish) avoids a late-stage "oh wait, Ctrl+B was never actually global" scramble. The `.deb` being producible here also de-risks Phase 10 — packaging is now an incremental tightening rather than a from-scratch build.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 3 — Editor + File Tree + Find/Replace
**Goal:** Working Monaco multi-tab editor, live file tree with real filesystem ops scoped to the workspace, and in-file + cross-file find/replace.

**Deliverables:**
- `@monaco-editor/react` pinned locally (no CDN), `vite-plugin-monaco-editor` configured with languages `typescript`, `javascript`, `json`, `css`, `html`, `rust`, `python`, `go`, `markdown`. Workers emitted under `monacoeditorwork/`.
- `EditorArea.tsx`: tab bar (dirty dot, middle-click close, `Ctrl+W`, `Ctrl+Shift+T` reopen), one Monaco instance, `ITextModel` per tab, language autodetection from extension, JetBrains Mono 14px, ligatures on by default.
- Diff view stub (`monaco.editor.createDiffEditor`) instantiable but not yet wired — just assert it can construct.
- `SidePanel: Files` tree using a lazy `FileTreeNode` component. Initial workspace = `open-folder` dialog (via `plugin-dialog`). Context menu: New File, New Folder, Rename, Delete, Reveal in File Manager (`xdg-open`), Copy Path, Open in Terminal (emits an event for Phase 4).
- Rust commands in `src-tauri/src/commands/fs.rs`: `fs_list(path)`, `fs_read(path)`, `fs_write(path, bytes)`, `fs_rename(from, to)`, `fs_delete(path)`, `fs_create_dir(path)`, `fs_open_folder() -> WorkspaceId`. Each validates the path is a descendant of the open workspace root or denies with a typed error.
- `fs.json` capability amended: `fs:allow-read-text-file`, `fs:allow-write-text-file`, `fs:allow-read-binary-file`, `fs:allow-write-binary-file` each scoped dynamically via `fs:scope` updated per workspace-open (runtime patch via `tauri::scope::Scopes`).
- Find in file (`Ctrl+F`) — Monaco built-in, just unhidden.
- Find in files (`Ctrl+Shift+F`) — a Side Panel pane with regex/case/whole-word toggles. Backend implemented with `ignore` + `grep` crates over the workspace root.
- File-tree git status colouring placeholder (hook exists; real git in Phase 8).
- **Monaco lazy-load proof**: `performance.measure` instrumentation confirms Monaco bundle is fetched after initial paint.

**Acceptance criteria:**
- [ ] Open a TypeScript file; syntax highlight correct; JetBrains Mono renders; ligatures toggle in settings placeholder.
- [ ] Ctrl+W closes current tab; middle-click does the same.
- [ ] New File via tree creates the file on disk; rename updates disk name; delete asks a confirm dialog and removes.
- [ ] `fs_read` on a path outside the workspace root returns the typed `OutsideWorkspace` error — verified via devtools invoking manually.
- [ ] `Ctrl+Shift+F` for "TODO" across a 1k-file workspace returns results in under 2s.
- [ ] `pnpm tauri build && dpkg-deb -c biscuitcode_*.deb | grep -c monacoeditorwork` ≥ 5 (workers packaged).
- [ ] Cold-launch time to shell (before opening a file) is under 2s on i5-8xxx: `time (biscuitcode &) ; sleep 3 ; wmctrl -l | grep BiscuitCode` shows the window within 2000ms.

**Dependencies:** Phase 2.
**Complexity:** Medium (edging into High because of Monaco worker wiring + scoped-fs runtime patching).
**Split rationale:** Editor + file tree belong together — neither is useful alone, and the file-scope capability work needs both. Find/replace is bundled because Monaco gives Ctrl+F essentially for free, and the cross-file find uses the same `fs` scope validation we're already adding. Git-status colouring is deliberately NOT here; it's in Phase 8 with the rest of git.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 4 — Terminal (xterm.js + portable-pty)
**Goal:** Multi-tab integrated terminal with `xterm.js`, real PTY-backed shells, clickable links and paths, wired to the "Open in Terminal" action from Phase 3.

**Deliverables:**
- `TerminalPanel.tsx` with tabbed `xterm.js` instances, `@xterm/addon-fit`, `@xterm/addon-web-links`, `@xterm/addon-search`, `@xterm/addon-webgl` (with canvas fallback).
- Rust `biscuitcode-pty` crate exposing commands `terminal_open(shell, cwd, rows, cols) -> SessionId`, `terminal_input(session_id, bytes)`, `terminal_resize(session_id, rows, cols)`, `terminal_close(session_id)`.
- Two Tokio tasks per session: reader (PTY master → `terminal_data_{session_id}` event), writer (consumes queued input). Hash-map of sessions under `Arc<RwLock<HashMap<SessionId, PtySession>>>`.
- Shell detection: read `$SHELL`, else `getent passwd $UID`, else `/bin/bash`.
- Custom link provider matching `path/to/file:line[:col]` → emits `open_file_at` event (consumed by editor).
- `Ctrl+\`` focuses the terminal panel; a `+` button opens new tabs.
- Tab close drops the PTY master/slave and kills the child.

**Acceptance criteria:**
- [ ] Open terminal → prompt appears in under 500ms; `echo $SHELL` returns the user's shell.
- [ ] Resizing the terminal panel resizes the PTY (run `tput lines` and `tput cols` after resize — values match).
- [ ] Click a URL in terminal output → opens in browser via `plugin-shell` (allow-listed only).
- [ ] Click `src/main.rs:12` in terminal output → opens `src/main.rs` at line 12 in the editor.
- [ ] Close a terminal tab → `pgrep -f 'biscuitcode.*bash'` returns no orphans after 2s.
- [ ] Five concurrent terminals each running `yes > /dev/null` → total CPU under one core's worth on the test machine; no crash over 60s.

**Dependencies:** Phase 2 (needs layout + shortcuts).
**Complexity:** Medium.
**Split rationale:** Terminal is small enough to stand alone — the vision allocates it one day. Sequencing it *before* Phase 5 (chat) is intentional because it doesn't need providers and provides an early OS-integration win; it also de-risks the Tokio stream-task pattern that Phase 5 and Phase 8 (LSP) will reuse. Parallelism with Phase 3 is tempting but blocked by both needing Phase 2's shortcut layer.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 5 — Keyring + Anthropic Provider + Chat Panel (E2E text-only)
**Goal:** User can add an Anthropic API key in settings (stored in libsecret), open the chat panel, pick `claude-opus-4-7`, type a message, and watch streaming text render — no tools, no agent loop yet.

**Deliverables:**
- `biscuitcode-core::secrets` module wrapping `keyring` 3.6 with features `linux-native-async-persistent + async-secret-service + crypto-rust + tokio`. API: `async fn set(service, key, value)`, `async fn get(service, key)`, `async fn delete(service, key)`.
- Startup check `secret_service_available()` that pings `org.freedesktop.secrets` over DBus; if absent, emits an event that blocks API-key entry and shows the install prompt in onboarding (scaffolded now, full onboarding Phase 9).
- `biscuitcode-providers::anthropic::AnthropicProvider` implementing the `ModelProvider` trait:
  - `reqwest` with HTTP/2 keep-alive, optional prewarm on app start.
  - SSE parsing of `message_start → content_block_{start,delta,stop} → message_delta → message_stop`.
  - Delta-type handling: `text_delta` → `TextDelta`, `thinking_delta` → `ThinkingDelta`, `input_json_delta` → `ToolCallDelta` (accumulated on `content_block_stop`).
  - **Critical gotcha**: `ChatOptions` sampling fields (temp/top_p/top_k) are `Option`, and the Anthropic impl unconditionally omits them when the model is Opus 4.7. A unit test asserts the request JSON does not contain those keys.
  - Models list: `claude-opus-4-7` (default), `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`, `claude-opus-4-6` (marked legacy in UI).
- `ChatPanel.tsx` with message list (markdown via `react-markdown` + `remark-gfm`), code blocks with copy button (apply/run deferred to Phase 6), model picker reading from provider list, send button, streaming token rendering.
- `biscuitcode-db` crate using `rusqlite` with WAL mode, `PRAGMA user_version` migrations, initial schema: `workspaces`, `conversations`, `messages` as in research §10. Migration file embedded as a Rust const string.
- `http.json` capability: fetch allowlist `https://api.anthropic.com/**`.
- Settings page stub (`SettingsProviders.tsx`): list providers, status badges (`green` = key valid via test request, `yellow` = key present but untested, `red` = no key / invalid), test-connection button.
- First-token-latency measurement emitted as a telemetry-scaffold event (no wire send) for future measurement.

**Acceptance criteria:**
- [ ] `settings → Models → Anthropic → Add key` stores the key in libsecret. Verified with `secret-tool search service biscuitcode` — the value is returned from the daemon, not from any file under `~/.config/biscuitcode/`.
- [ ] `grep -r 'ANTHROPIC_API_KEY\|sk-ant' ~/.config/biscuitcode/` returns nothing after key entry.
- [ ] Typing "say hi in three words" → assistant tokens render in under 500ms from send button press (p50 on warm connection).
- [ ] Sending the same message with `claude-opus-4-7` selected and `temperature: 0.7` attempted via devtools shim returns HTTP 200 (the provider filtered the field).
- [ ] The conversation is persisted — reopen app, prior message visible, messages table populated.
- [ ] On a VM without `gnome-keyring`, add-key flow shows the exact install command (`sudo apt install gnome-keyring libsecret-1-0`); no plaintext file created.
- [ ] Unit test `anthropic_provider::requests_strip_sampling_for_opus_47` passes.

**Dependencies:** Phase 2.
**Complexity:** Medium (high on the keyring edge cases).
**Split rationale:** Combining keyring + Anthropic + chat panel into one phase matches the vision's "one provider E2E" checkpoint. Keyring alone is too small; provider alone has no UI; chat panel alone has nothing to call. Bundling them produces a real runnable milestone ("chat with Claude works") in 2 days. Other providers are explicitly Phase 7 because adding two more providers before tools exist would stall the more valuable agent-loop work in Phase 6.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 6 — Agent Loop + Tool Registry + Inline Edit + Rewind
**Goal:** Full ReAct-style agent executor with tool calls streaming to Agent Activity, workspace-scoped tools (`read_file`, `write_file`, `run_shell`, `search_code`, `apply_patch`), inline AI edit on selection (`Ctrl+K Ctrl+I`), and per-action rewind.

**Deliverables:**
- `biscuitcode-agent::tools` module with the five tool handlers, each declaring JSON Schema input, side-effect class (`read`/`write`/`shell`), and confirmation policy. All file tools respect workspace-scope. `run_shell` has an explicit sandbox: no `sudo`, no network calls except via the provider HTTP scope.
- `biscuitcode-agent::executor` implementing ReAct loop:
  - Accepts a conversation and streams from the provider.
  - On `ToolCallEnd`, decodes accumulated args JSON, consults confirmation policy, either prompts (writes/shell) or auto-runs (reads), then appends `ToolResult` to the conversation and loops.
  - Pause flag checked at loop boundaries (single atomic bool).
  - Per-action snapshot: before each write/shell tool, snapshot the affected file(s) to `~/.cache/biscuitcode/snapshots/{conversation_id}/{message_id}/...` and record the manifest in the messages table.
- `AgentActivityPanel.tsx` rendering tool calls as collapsible cards (running/ok/error status, timing, pretty-JSON args, streamed result). Badge on chat message links to the card.
- Agent mode toggle in chat panel (default off). When off, the loop stops after the first assistant message; when on, it auto-continues on tool calls.
- Workspace trust toggle (stored in settings). When on, write/shell tools auto-approve.
- Inline edit (`Ctrl+K Ctrl+I`): select code → popover input → backend calls provider with an edit prompt + selection + file path → diff streamed into a transient Monaco diff decoration → user accepts/rejects/regenerates.
- Rewind UI: conversation header shows a rewind button per assistant message; clicking it restores snapshots + truncates messages past that point.
- `apply/run` buttons on code blocks in chat: `apply` opens the affected file and applies the patch using Monaco's model diff; `run` pushes the selected code into a new terminal tab (no auto-exec — user hits Enter).

**Acceptance criteria:**
- [ ] With agent mode on: asking "list files in src/" → Agent Activity shows `search_code` card → result appears → assistant continues with a natural-language summary.
- [ ] Write-tool call ("create a file hi.txt with contents 'hello'") triggers a confirmation modal showing the diff; decline prevents file creation; accept creates it.
- [ ] Rewind on the assistant message that created `hi.txt` restores its pre-create state (file removed) and removes messages after.
- [ ] `Ctrl+K Ctrl+I` on a selected function inside Monaco streams a diff inline; accept applies, reject discards, regenerate re-streams.
- [ ] Pause button during a long agent run stops before the next tool call (verified by timing — pause arrives within one tool-call boundary).
- [ ] `run_shell` called with `sudo rm -rf /` is rejected before execution with a specific error (`ShellForbiddenPrefix`).
- [ ] All workspace-trust-off runs prompt; with workspace-trust-on the same runs do not prompt.

**Dependencies:** Phase 3 (file system, tabs), Phase 5 (provider stream, conversation persistence).
**Complexity:** High.
**Split rationale:** This is the single largest phase because every subtopic (tool registry, executor, activity UI, inline edit, rewind) is tightly coupled to the others — splitting leaves orphans. Inline edit is in this phase rather than Phase 3 because it depends on the provider (Phase 5) and on the confirmation/diff UX this phase defines. Rewind sits here too because its snapshots are a side-effect of the write tool's execution, not a later add-on. This phase is the riskiest in the plan, flagged in Return Message.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 7 — OpenAI + Ollama Providers + Ollama Detection/Install
**Goal:** Ship the remaining two providers behind the same `ModelProvider` trait, with Ollama onboarding detection, one-click install, RAM-aware model auto-pull, and per-conversation model switching.

**Deliverables:**
- `biscuitcode-providers::openai::OpenAIProvider`:
  - SSE parsing of Chat Completions deltas.
  - Per-index `tool_calls` argument accumulation until `finish_reason === "tool_calls"`, then emit `ToolCallStart + ToolCallDelta* + ToolCallEnd` into the same `ChatEvent` stream.
  - Default model `gpt-5.4-mini`. Picker exposes `gpt-5.4`, `gpt-5.4-pro` (reasoning-only), `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5.3 Instant`. Legacy `gpt-5.2 Thinking` shown but tagged legacy until 2026-06-05.
  - `reasoning.effort` surfaced as an optional per-conversation setting.
- `biscuitcode-providers::ollama::OllamaProvider`:
  - NDJSON parsing of `/api/chat` responses (line-delimited JSON, one object per line).
  - `tools` passthrough in OpenAI-function-call format; extract `message.tool_calls` from the final non-done chunk.
  - Model picker pulls from `GET /api/tags` (local models); default `gemma3:4b` with RAM-tiered fallback. `gemma4:*` preferred when present; `qwen2.5-coder:7b` as agent-oriented alternative.
  - `ollama_install()` command: detects absence via `curl -sSfm 1 http://localhost:11434/api/version` and `which ollama`. On missing, shows a confirm dialog with the verbatim command `curl -fsSL https://ollama.com/install.sh | sh` and runs via `plugin-shell` *only after* user confirms.
  - `ollama_pull(model)` command with progress events piped from `ollama pull` stdout to a progress bar in the model picker.
  - RAM detection via `sysinfo` crate → chooses default per table in research §7 (`<6GB → gemma3:1b`, `6-12 → gemma3:4b`, `12-24 → qwen2.5-coder:7b + gemma3:12b`, `≥32 → gemma3:27b`).
- `http.json` capability: add `http://localhost:11434/**` and `https://api.openai.com/**` to fetch allowlist.
- `shell.json` capability: add `ollama` to the command registry, with argument regex limiting to `pull <model>`, `list`, `show <model>`, `serve`.
- Per-conversation model switch: chat panel model dropdown is conversation-scoped, persisted to `conversations.active_model`.
- Provider status badges go live (green/yellow/red) for all three providers.

**Acceptance criteria:**
- [ ] With an OpenAI key set, send a message using `gpt-5.4-mini` → streams text; tool call for "get weather" (stub tool) returns valid JSON args and completes.
- [ ] Switching the same conversation to `claude-opus-4-7` mid-thread preserves prior messages; Claude sees the OpenAI tool result in its input.
- [ ] On a VM without Ollama: clicking "Install Ollama" shows the confirm dialog with the full `curl | sh` command before executing; declining does nothing.
- [ ] After install, `ollama_pull("gemma3:4b")` shows a progress bar updating at least every second until complete.
- [ ] On an 8 GB VM, the RAM detector picks `gemma3:4b` by default; on a 4 GB VM it picks `gemma3:1b`.
- [ ] Sending a chat through Ollama on a local gemma3:4b streams text tokens in under 3s on the test machine; using `gemma4:*` the same prompt returns a tool call for a registered tool.
- [ ] A single `ChatEvent` stream is identical in shape across all three providers for an equivalent "hello" prompt (verified by snapshot test).

**Dependencies:** Phase 5 (trait exists, chat panel, keyring).
**Complexity:** Medium.
**Split rationale:** Providers 2 and 3 must ship together because the Ollama path is materially more user-visible (detect/install/pull UI) than OpenAI, but both share the `ChatEvent` translation plumbing. Doing only one here would leave the other unevenly wired. This is placed after Phase 6 (agent loop) so the tool-calling path across all three providers is exercised end-to-end with the real executor, not a stub. The alternative — shipping this before Phase 6 — tempted me but the agent loop's risk is much higher, so Phase 6 gets the prior slot for blast-radius reasons.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 8 — Git Panel + LSP Client + Preview Panel
**Goal:** VS Code parity features: a git panel with stage/unstage/commit/push/pull, a working LSP client for five languages, and a preview panel covering Markdown, HTML, images, and PDF.

**Deliverables:**
- **Git** via `git2-rs` (reads) + `std::process::Command('git')` (writes):
  - Side Panel Git pane: files grouped by `staged`/`unstaged`/`untracked`, hunk-level stage/unstage (Monaco inline diff buttons), commit message input, commit button, push/pull buttons that stream stdout to the Terminal panel.
  - Branch name in status bar, clickable → branch switcher dropdown; optional gutter blame toggle in settings.
  - File tree git status colours (M/U/A/D) now live.
- **LSP** via `biscuitcode-lsp` crate + `monaco-languageclient` frontend:
  - Rust spawns `rust-analyzer`, `typescript-language-server --stdio`, `pyright-langserver --stdio`, `gopls`, `clangd` based on detected project files (presence of `Cargo.toml`, `package.json`/`tsconfig.json`, `pyproject.toml`/`requirements.txt`, `go.mod`, `CMakeLists.txt`/`compile_commands.json`).
  - Tauri events `lsp-msg-in-{session_id}` + `lsp_write` command as proxy; frontend `MessageTransports` adapter.
  - Missing-server dialog: copy-to-clipboard install command (per research §9 table), no auto-run.
  - Diagnostics rendered as Monaco squigglies + problem count in status bar.
- **Preview Panel** (split pane in editor area, not a new window):
  - Markdown: `react-markdown` + `remark-gfm` + `rehype-highlight` + `mermaid` + `rehype-katex`, live update.
  - HTML: sandboxed iframe with `sandbox="allow-scripts"` (no forms, no top-navigation), live-reload on save, devtools button via `plugin-window`.
  - Images: `img` with CSS zoom/pan (simple, no external viewer).
  - PDF: `pdf.js` via `react-pdf`, single-page view with next/prev.
  - Notebook (`.ipynb`): read-only render — parse cells, render markdown cells as markdown, code cells as JetBrains Mono, outputs as text/mime-typed blocks. No execution.
  - Auto-open rule: AI-edited `.md`, `.html`, `.svg`, image → open preview as split pane.
- `shell.json` capability: add `which <binary>` and the LSP binary paths to the registry; no wildcard args.

**Acceptance criteria:**
- [ ] Open a Rust file → `rust-analyzer` starts → hover shows type; go-to-definition jumps correctly; diagnostics appear.
- [ ] In a repo: stage a hunk via the inline diff button; status changes from `unstaged` to `staged`; commit with a message; `git log -1` shows it.
- [ ] Branch switcher shows all local branches; switching updates the status bar within 500ms.
- [ ] Opening `README.md` and hitting preview shows rendered markdown side-by-side; typing updates the preview within 200ms.
- [ ] A `.ipynb` with 3 cells renders read-only with cell borders.
- [ ] Missing language server (e.g., `clangd` absent) triggers a toast with a copy-to-clipboard `sudo apt install clangd` command; the app does not auto-run it.
- [ ] HTML preview iframe cannot navigate away (`window.top.location` attempts blocked by sandbox).

**Dependencies:** Phase 3 (editor, file tree).
**Complexity:** High.
**Split rationale:** Git + LSP + Preview are three distinct subsystems, but each alone is a half-day and they all share Phase 3's editor. Splitting them into three phases would create thrash (three times the PR overhead, three times the VM smoke test). They're independent enough that a parallel coder could tackle them internally, but the plan treats them as one coherent "VS Code parity" phase to keep the phase count honest. This is a deliberate choice to group related work; if a coder finds the scope too wide at execution time, they may flag `Needs Replanning` and we'll split in round 2.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 9 — Onboarding + Settings UI + Theming + Icon
**Goal:** Ship the 3-screen onboarding, full settings UI, three themes with live preview, and the final icon set rendered from `packaging/icons/biscuitcode.svg` (Concept A).

**Deliverables:**
- Onboarding flow (`OnboardingModal.tsx`) — 3 screens:
  1. **Welcome**: BiscuitCode logo + tagline + Next.
  2. **Pick models**: provider cards (Anthropic, OpenAI, Ollama). Each: add-key UI or Install Ollama button. Must set at least one before Next.
  3. **Open a folder**: file picker; also offers "Continue without a folder" for just-exploring mode.
- Keyring absence check in onboarding Step 2: if Secret Service is unavailable, step 2 shows a blocking dialog with the exact `sudo apt install gnome-keyring libsecret-1-0 libsecret-tools` command and a retry button.
- Settings page (`SettingsPage.tsx`) with sections: General, Editor, Models, Terminal, Appearance, Security, About. Raw JSON editor button opens `~/.config/biscuitcode/settings.json` in the Monaco editor for power-users.
- Three themes: `BiscuitCode Warm` (dark, default), `BiscuitCode Cream` (light), `High Contrast`. Each defined as CSS variable overrides in `src/theme/themes.ts`. Live preview on hover in the settings Appearance pane.
- GTK theme detection at startup: Rust `detect_gtk_theme()` via `xfconf-query -c xsettings -p /Net/ThemeName`, fallback `gsettings get org.gnome.desktop.interface gtk-theme`. Regex `-dark$` (case-insensitive) → dark; otherwise light. On first run with a light GTK theme, offer to switch to Cream.
- Icon: `packaging/icons/biscuitcode.svg` authored as Concept A — biscuit-gold `>_` glyph on cocoa-dark rounded-square (#1C1610, 22% corner radius). Render with `rsvg-convert` to `biscuitcode-{16,32,48,64,128,256,512}.png`. `.ico` for Windows future.
- **16x16 render verification**: CI step asserts `biscuitcode-16.png` pixel-level legibility: at least 2 distinct pixels forming a `>` shape and 3 pixels for `_`. Visual diff against a checked-in reference.
- VS Code theme import: placeholder entry under Appearance, disabled, tooltip "Coming in v1.1".

**Acceptance criteria:**
- [ ] Fresh install → first launch shows onboarding; no way to reach the main UI without either setting a provider or clicking "Skip" in step 2 (skip leaves all badges red).
- [ ] Onboarding step 2 on a keyring-absent VM shows the install command; retry progresses once `gnome-keyring` is installed.
- [ ] Settings → Appearance → hover Cream → preview shows cocoa-50 bg, biscuit-900 text; select Cream → theme persists across restart.
- [ ] With GTK theme `Mint-Xia-Light` set, first run offers to switch to Cream; the offer does not appear on later launches.
- [ ] 16x16 icon renders a readable `>_` at launcher-grid size — CI pixel-check passes.
- [ ] `desktop-file-validate packaging/deb/biscuitcode.desktop` exits 0.
- [ ] No `system-ui` in `grep -rn 'font-family' src/` output (Inter only in primary chrome).

**Dependencies:** Phase 5 (onboarding needs keyring + providers), Phase 7 (Ollama onboarding path), Phase 8 (settings page uses theming decisions that depend on the preview split-pane).
**Complexity:** Medium.
**Split rationale:** Onboarding + settings + theming + icon cluster naturally because they're all user-chrome work that has no functional blockers from earlier phases except the provider setup in Phase 5/7. Doing this before Phase 10 (packaging) is critical because the icon PNGs and `.desktop` file have to be in the bundle. The vision keeps onboarding/settings/icon in Phase 8; I promoted it to its own phase ahead of packaging to give it the focused polish pass the vision's quality bar demands ("no placeholder text, no lorem ipsum").
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 10 — Packaging + CI + GPG Signing + Release Smoke Test
**Goal:** Build `biscuitcode_1.0.0_amd64.deb` + `BiscuitCode-1.0.0-x86_64.AppImage` in GitHub Actions on `ubuntu-24.04` runners, GPG-sign, publish SHA256, and smoke-test on a fresh Mint 22 XFCE VM.

**Deliverables:**
- `tauri.conf.json` `bundle` section finalised: `targets: ["deb", "appimage"]`, `deb.depends: ["libwebkit2gtk-4.1-0", "libgtk-3-0"]`, `deb.recommends: ["gnome-keyring", "ollama"]`, `deb.suggests: ["rust-analyzer", "typescript-language-server", "pyright", "gopls", "clangd"]`, `deb.section: "devel"`, correct `maintainer`, `description`.
- `.github/workflows/release.yml` — on tag `v*`:
  - Runner `ubuntu-24.04`.
  - Linux deps install step (full list from research §12).
  - `pnpm install --frozen-lockfile`.
  - `tauri-apps/tauri-action@v0` with `args: "--target x86_64-unknown-linux-gnu"`, `tagName: v__VERSION__`, `releaseName: BiscuitCode v__VERSION__`.
  - GPG import step using `GPG_PRIVATE_KEY` secret; `gpg --detach-sign --armor` both artifacts.
  - `sha256sum biscuitcode_*.deb BiscuitCode-*.AppImage > SHA256SUMS.txt`.
  - Upload `.deb`, `.AppImage`, `.deb.asc`, `.AppImage.asc`, `SHA256SUMS.txt` to the release.
  - `linuxdeploy` retry wrapper for AppImage step (research §12 flake).
- `.github/workflows/ci.yml` — on PR: lint (`cargo clippy -D warnings`, `pnpm lint`), typecheck, unit tests.
- AppImage `libfuse2t64` handling: README banner + a postinstall check in the AppImage wrapper that prompts install if missing.
- Release smoke-test checklist in `docs/RELEASE.md`:
  1. Download `.deb` from GitHub release.
  2. On a fresh Mint 22 XFCE VM: `sudo dpkg -i biscuitcode_1.0.0_amd64.deb`.
  3. Whisker menu → Development → BiscuitCode.
  4. Complete onboarding in under 2 minutes.
  5. Ctrl+L on a selection, Agent mode refactor, accept diff, commit + push.
  6. `sudo apt remove biscuitcode` clean.
- Three screenshots for README, using `BiscuitCode Warm` theme: main editor with chat, Agent Activity mid-run, preview split pane.
- README: install instructions, screenshots, license, link to `docs/DEV-SETUP.md`.

**Acceptance criteria:**
- [ ] Pushing a `v1.0.0` tag triggers CI; within ~15 min the release page has both artifacts, both `.asc` signatures, and `SHA256SUMS.txt`.
- [ ] `gpg --verify biscuitcode_1.0.0_amd64.deb.asc biscuitcode_1.0.0_amd64.deb` returns "Good signature".
- [ ] `sha256sum -c SHA256SUMS.txt` passes.
- [ ] On fresh Mint 22 XFCE VM: full 6-step smoke-test checklist passes.
- [ ] `time (biscuitcode & sleep 3 ; wmctrl -l | grep BiscuitCode)` — the window title appears within 2000ms.
- [ ] `apt remove biscuitcode` removes binary, desktop entry, icons across all 7 sizes, and the `/usr/bin/biscuitcode` symlink.
- [ ] README screenshots render without `lorem ipsum` or any `TODO` strings.
- [ ] `cargo audit` clean; `pnpm audit --prod` clean.

**Dependencies:** Phase 9 (needs icon, onboarding, final UI).
**Complexity:** Medium.
**Split rationale:** Packaging is the "prove it's shippable" phase. It deliberately lands last because the `.deb` has been producible since Phase 2 — this phase is about signing, CI, the AppImage caveat, and the release checklist rather than packaging-from-scratch. Splitting CI and release smoke-test into two phases would be a waste: they're a single continuous "tag → artifacts → VM test" loop.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

## Global Acceptance Criteria

These span the whole project and are checked at Phase 10 against the signed `v1.0.0` `.deb`.

- [ ] `sudo dpkg -i biscuitcode_1.0.0_amd64.deb` installs clean on fresh Mint 22 XFCE (22.0, 22.1, 22.2) VMs; `sudo apt remove biscuitcode` removes everything it installed.
- [ ] Cold-launch budget: `time (biscuitcode & sleep 3 ; wmctrl -l | grep -q BiscuitCode)` — window present within 2000ms on i5-8xxx / 8GB hardware.
- [ ] No console errors in devtools or Rust logs during a normal 5-minute session: open folder, edit file, chat, run agent tool, commit via git panel. (`journalctl --user -t biscuitcode --since '5m ago' | grep -iE 'error|panic' | wc -l` returns `0`.)
- [ ] All keyboard shortcuts in the vision's table work as specified (manual checklist in `docs/RELEASE.md`).
- [ ] `grep -rnE 'lorem|TODO|FIXME|placeholder|XXX' src/ src-tauri/src/` returns zero user-visible hits (excluding internal type-inference placeholders in comments).
- [ ] Typography audit: `grep -rn 'system-ui\|sans-serif' src/` returns only intentional code-related rules (mono-font fallbacks are OK; primary chrome must not fall back to system-ui).
- [ ] Dark theme uses Cocoa scale exclusively: `grep -rn '#000000\|#fff\b\|#ffffff' src/theme/` returns zero hits.
- [ ] Every failure path has an actionable error: verified by the error-path checklist (no network, bad key, Ollama down, permission denied, keyring missing) — each shows a specific message, not a stack trace.
- [ ] First-token-latency on Claude streaming: p50 under 500ms, p95 under 1200ms, measured over 20 prompts on a warm connection.
- [ ] Provider tool calls render as Agent Activity cards at start time, not completion time (visual verification: card appears within 250ms of `tool_use` `content_block_start`).
- [ ] `cargo audit` and `pnpm audit --prod` return zero critical vulnerabilities.
- [ ] `desktop-file-validate packaging/deb/biscuitcode.desktop` exits 0.
- [ ] All dependencies MIT/Apache-2.0/BSD compatible — `cargo-license` + `license-checker-rseidelsohn` reports clean.
- [ ] Icon legible at 16x16 in the XFCE system tray (manual visual check).
- [ ] Release smoke-test checklist in `docs/RELEASE.md` passes 100% on a fresh VM.

## Open Questions

1. **Telemetry backend.** The vision allows opt-in anonymous crashes. Do we wire Sentry (vendor dep), run our own ingestion endpoint, or ship the UI toggle in v1 with no server and implement the endpoint in v1.1? Affects Phase 9 (setting surface) and Phase 10 (privacy disclosure in README). Recommendation: ship toggle only in v1, no wire; surface in Phase 9 as a clearly-disabled control.
2. **AppImage `libfuse2t64` UX.** Do we (a) ship a wrapper `.AppImage.sh` that `apt install`s `libfuse2t64` on first launch, (b) just document the dependency in README, or (c) both? Option (c) is safest but adds a shell-script artifact. Planner's default: go with (c); defer to reviewer if (a) is too heavy.
3. **Icon Concept D spike.** The vision prefers Concept A but allows D if D renders better at 16x16. My plan ships A and defers D. Should Phase 9 include a 2-hour spike to render both and pick, or do we trust A without the A/B? Default: trust A, fall back to D only if CI 16x16 check fails.
4. **Arm64 build.** Mint runs on arm64 Pi-class hardware occasionally. `ubuntu-24.04-arm` runners exist. Is a `linux/arm64` `.deb` a v1 goal or a v1.1 defer? Default: defer to v1.1 (not in this plan).
5. **Debian repo (`apt.biscuitcode.io`).** Currently we ship a signed `.deb` via GitHub releases. Hosting a repo is ~1-2 days of work + ongoing cost. Explicit defer or plan? Default: defer — GitHub release `.deb` is the v1 distribution channel.
6. **Secret Service fallback on truly broken sessions.** If a user runs `startxfce4` via `xinit` bypassing PAM, `gnome-keyring-daemon` may not auto-start even when installed. My plan blocks onboarding with an install prompt. Is there an alternative "start the daemon ourselves via `gnome-keyring-daemon --replace`" recovery we should try automatically before blocking? Default: no, be conservative — block with a clear message.
7. **LSP install auto-run.** Research §9 and the vision agree: we do NOT auto-run `rustup component add`, `npm i -g`, etc. — copy-to-clipboard only. Confirm that's the final call? Default: confirmed; no auto-install of LSPs.
8. **Preview notebook deferred-execution scope.** v1 is read-only render. Does the plan need a placeholder "Run all cells" disabled button in v1, or do we not hint at it at all? Default: do not hint; render-only with no run controls in v1.
9. **Conversation DAG storage overhead.** `content_json` stores all content blocks including base64-encoded images if vision is used. For large sessions, DB growth could be MBs/conversation. Should Phase 5 include a size cap or lazy blob table, or is that a v1.1 problem? Default: defer; surface in settings as "Clear old conversations" in Phase 9.
10. **Non-blocking adjacent work noticed during planning** (surfaced per Law 3, not silently added): none beyond what's listed above. If the reviewer wants to expand, candidates include: file-drag-drop into chat (vision says yes, plan puts it in Phase 5), AI git commit message generation (vision does not mention), crash-reporter with privacy-stripped fields (telemetry-adjacent).
