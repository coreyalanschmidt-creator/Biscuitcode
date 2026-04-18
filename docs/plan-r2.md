# Implementation Plan: BiscuitCode (Round 2)

> Round 2 of 2. An independent plan drawing on both research-r1 and research-r2.
> A synthesis pass will merge with plan-r1.md into final docs/plan.md.

## Review Log

_Empty — reviewer round 2 will fill._

## Vision Summary

BiscuitCode is a Tauri 2.10.x + React 18 + TypeScript 5 desktop AI coding environment targeting Linux Mint 22 XFCE (Ubuntu 24.04 / WebKitGTK 4.1 / kernel 6.8) with VS Code parity: Monaco editor, xterm.js over `portable-pty`, LSP client, git panel, preview pane, and a four-region resizable shell. Three AI providers (Anthropic, OpenAI, Ollama) sit behind a unified `ModelProvider` trait emitting a flattened `ChatEvent` stream; a ReAct agent loop calls workspace-scoped tools; API keys live in libsecret via the Rust `keyring` crate with no plaintext fallback. "Done" = a GPG-signed `biscuitcode_1.0.0_amd64.deb` that installs clean on a stock Mint 22 XFCE VM, cold-launches in under 2 s on i5-8xxx / 8 GB hardware, completes 3-screen onboarding in under 2 minutes, and survives `apt remove biscuitcode` cleanly.

## Assumptions

Carried from both research rounds plus planning-specific assumptions, flagged by confidence.

1. **[HIGH]** Canonical target is Mint 22.1 Xia (kernel 6.8, XFCE 4.18). Smoke matrix also covers 22.0 and 22.2. Ubuntu 24.04 "noble" is the Debian base. (r1 §2; r2 C1)
2. **[HIGH]** Tauri pin: v2.10.x (`tauri` 2.10.3, `tauri-cli` 2.10.1, `@tauri-apps/api` 2.10.1). Capability files hand-authored, never `tauri migrate`-generated. (r1 §1, Best Practice #1)
3. **[HIGH]** Linux webview is `libwebkit2gtk-4.1-0`; declared in `.deb` `Depends`. Ubuntu 24.04 does **not** ship webkit2gtk-4.0. (r1 §1)
4. **[HIGH]** `@xterm/*` scoped packages only. Old `xterm-addon-*` are deprecated. (r1 §5; r2 reinforced)
5. **[HIGH]** `keyring` 3.6.x with features `linux-native-async-persistent + async-secret-service + crypto-rust + tokio`. **No plaintext fallback; block onboarding if Secret Service unavailable.** (r1 §6) Detection is read-only via `busctl list --user`, never a keyring probe. (r2 D6)
6. **[HIGH]** `tauri-plugin-stronghold` is **deprecated and slated for removal in Tauri v3**. Do not evaluate, reference, or mention it outside the ADR warning. (r2 A7)
7. **[HIGH]** Provider defaults corrected from vision: Anthropic `claude-opus-4-7` (**omit** temperature/top_p/top_k or request returns 400); OpenAI `gpt-5.4-mini` (not retired `gpt-4o`); Ollama `gemma3:4b` default with RAM tiering, `qwen2.5-coder:7b` for agent use, `gemma4:*` preferred when present for native tool calls. (r1 §7, Risks #3 #4; r2 D3 D5)
8. **[HIGH]** Anthropic SSE streaming: `message_start → content_block_{start,delta,stop} → message_delta → message_stop`. `input_json_delta` deltas are partial strings; full `input` object is only safe at `content_block_stop`. (r1 §7; r2 D3)
9. **[HIGH]** Prompt caching matters. `cache_control: {type: "ephemeral"}` on the system prompt and tool definitions gives ~5x cost reduction on long conversations. (r2 New Risks #1; r1 missed this)
10. **[HIGH]** Monaco loads via `@monaco-editor/react` pinned locally (no CDN), `vite-plugin-monaco-editor` for workers, **explicit `languageWorkers: []`** (no default languages loaded) to keep the cold bundle lean. TS worker silenced when LSP connects. (r1 §4; r2 D2)
11. **[HIGH]** SQLite via `rusqlite` direct (no `plugin-sql`), WAL mode, `PRAGMA user_version` migrations. r2 flagged this is closer to a coin-flip than r1 made it; the deciding factor is "no frontend DB access needed" — which holds. (r1 §10; r2 C2)
12. **[HIGH]** Git: `git2-rs` for reads (status, diff, blame), shell-out for writes (commit, push, pull). Swap to `gix` is a post-v1 target. (r1 §Best-Practice-8; r2 C3)
13. **[HIGH]** LSP: Rust spawns language servers, proxies stdio via Tauri events keyed by `session_id`; frontend wires `monaco-languageclient` with custom `MessageTransports`. No auto-install of LSP binaries — copy-to-clipboard only. (r1 §9)
14. **[HIGH]** All code-phase work runs from WSL2 Ubuntu 24.04 with the project rooted in `~/` (never `/mnt/c/`). A coder invoked without WSL2 must stop and report. (CLAUDE.md §Cross-platform; r1 §3)
15. **[MED]** **Wayland-XFCE is NOT reachable on any Mint 22 release.** XFCE 4.18 lacks Wayland; 22.2's XFCE edition stays on 4.18, not 4.20. Drop Wayland-XFCE smoke testing from the release matrix. Document Wayland as "Cinnamon only; XFCE v2 or later." (r2 C1 — r1 got this wrong.)
16. **[MED]** GitHub Actions runner is `ubuntu-24.04` (pinned, not `-latest`). Release builds GPG-signed via `GPG_PRIVATE_KEY` secret; SHA256 via `sha256sum`. (r1 §12)
17. **[MED]** Auto-update is **in scope for v1** but minimal: Tauri updater plugin for AppImage; GitHub Releases API check-for-updates button for `.deb` (manual re-install). No apt repo hosting in v1. (r2 G3)
18. **[MED]** Chat and Agent Activity panels use `react-virtuoso` for message virtualization from the first panel that streams content (Phase 5). Shared abstraction avoids rewriting Phase 6. (r2 D8)
19. **[MED]** Inline edit UX is **Zed-style split-diff** via `monaco.editor.createDiffEditor` — simpler and uses Monaco-native primitives. Cursor-style in-place decorations is a v1.1 polish. (r2 D7)
20. **[MED]** i18n scaffolding is in scope for v1 (all user-facing strings wrapped in `t('key')`; English-only bundle). Extra cost ≈ 1 hour in Phase 2; saves v1.1 find-and-replace sweep. (r2 G1)
21. **[MED]** Accessibility is "reasonable posture" for v1: keyboard-only navigation, ARIA labels on icon buttons, `aria-live="polite"` on streaming chat, focus rings. Full WCAG AA is post-v1. (r2 G2)
22. **[LOW]** Arm64 is NOT a v1 target.
23. **[LOW]** VS Code theme import is a disabled placeholder in v1.
24. **[LOW]** Notebook preview is read-only render; execution deferred to v2.

## Architecture Decisions

Each decision cites the research section. Decisions marked **(diverges from r1)** depart from plan-r1.md's choice.

- **Tauri v2.10.x, hand-authored capability ACL files** in `src-tauri/capabilities/{core,fs,shell,http}.json`, deny-by-default scopes. Workspace-root fs scope patched at runtime via `FsScope::allow_directory`. (r1 §13, r2 D1)
- **Brand tokens verbatim** in Tailwind theme + Rust palette constants + CSS custom properties. No `system-ui` in visible chrome; self-hosted Inter + JetBrains Mono in `src-tauri/fonts/`. (Vision; r2 G9)
- **Font fallback chain**: `'Inter', 'Ubuntu', sans-serif` for UI; `'JetBrains Mono', 'Ubuntu Mono', 'DejaVu Sans Mono', monospace` for code. Ubuntu fonts ship on Mint 22 by default — a named-system fallback, not `system-ui`. (r2 G9)
- **React 18 + Zustand** for state; `react-resizable-panels` for layout; `react-virtuoso` for both chat and Agent Activity lists. (r1 Trade-offs; r2 A2, D8)
- **Monaco single-instance, one `ITextModel` per tab**. Languages registered on-demand, not at startup. `createDiffEditor` for the inline-edit split diff. TS worker silenced via `setDiagnosticsOptions({ noSemanticValidation: true, noSyntaxValidation: true })` when LSP is active. (r1 §4; r2 D2)
- **`ModelProvider` trait → flattened `ChatEvent` enum**: `TextDelta`, `ThinkingDelta`, `ToolCallStart`, `ToolCallDelta`, `ToolCallEnd`, `Done { stop_reason, usage }`, `Error`. Provider quirks live in each impl. (r1 §7)
- **Prompt caching** on Anthropic: `cache_control: {type: "ephemeral"}` on system prompt + tool definitions; default on. (r2 New Risks #1) **(diverges from r1)**
- **ReAct loop with a read-only tool surface in v1.0 and writes gated behind explicit per-tool confirmation UX in v1.0** — the compromise between r1's "ship everything" and r2's "ship reads only." Split across Phases 6a (read-only infra) and 6b (write tools + rewind + inline edit). **(diverges from r1 — r1 bundled all five tools into one Phase 6)**
- **Ordering: providers-then-agent**. OpenAI and Ollama ship in Phase 6a *before* the agent loop's tool surface lands, so the `ChatEvent` contract is validated against three real providers before anything depends on it. **(diverges from r1, which put other providers in Phase 7 after the agent loop)**
- **Inline edit = Zed-style split-diff** via `createDiffEditor`. Accept/reject whole diff in v1; per-hunk in v1.1. (r2 D7) **(diverges from r1 which did not specify)**
- **LSP via `monaco-languageclient` + Rust stdio proxy** over Tauri events. One LSP child per (language, workspace) pair. Copy-to-clipboard install commands only. (r1 §9)
- **SQLite via `rusqlite` direct**, WAL mode, hand-rolled `PRAGMA user_version` migrations, DAG schema with `parent_id` for branching. (r1 §10) — noted as a defensible-but-close call per r2 C2.
- **Git: `git2-rs` for reads, shell-out for writes.** `gix` is v1.1+ swap target. (r1 Best Practice #8; r2 C3)
- **Theming: `xfconf-query -c xsettings -p /Net/ThemeName`** with `gsettings` fallback; dark heuristic via `-dark$` regex. (r1 §11)
- **Secret Service detection via `busctl list --user`** (read-only, no daemon activation), *before* any keyring call. (r2 D6) **(diverges from r1 which probed via the keyring API)**
- **Auto-update: dual path in v1.** AppImage users get the Tauri updater plugin (v2.10.x); `.deb` users get a "Check for updates" button backed by GitHub Releases API. No apt repo in v1. (r2 G3) **(diverges from r1 which was silent)**
- **Error taxonomy scaffolded in Phase 1** (`src/errors/types.ts` + `src/errors/ErrorToast.tsx` + Rust `thiserror` enum in `biscuitcode-core`). Each feature phase **adds its own codes as it touches a failure surface**. Phase 9 **audits** the catalogue rather than building it from zero. (r2 G6) **(diverges from r1 which deferred the full catalogue to Phase 9)**
- **Internal Rust crates prefixed `biscuitcode-*`** (`-core`, `-agent`, `-providers`, `-lsp`, `-pty`, `-db`). Defensively claim `biscuitcode` on crates.io day 1. Avoid `biscuit`, `biscuit-auth`, `biscuit-cli`, `CodeBiscuits`. (r1 §15)
- **Stronghold plugin explicitly forbidden** — ADR records this so a future maintainer searching "Tauri secrets" does not land on deprecated docs. (r2 A7)
- **Wayland-XFCE drop**. Mint 22 XFCE ships 4.18 (no Wayland). Smoke matrix drops the Wayland-XFCE row; Cinnamon-Wayland 22.2 is a best-effort test. (r2 C1) **(diverges from r1)**
- **Reasoning-model TTFT exemption**. `gpt-5.4-pro` and other reasoning-only models emit no output until reasoning finishes (3–30 s). The p50-under-500ms TTFT gate applies only to non-reasoning models; reasoning runs show a "Thinking..." state. (r2 New Risks #2) **(diverges from r1 which applied one TTFT gate to everything)**

## Phase Index

| # | Phase | Status | Complexity | Depends on |
|---|-------|--------|------------|------------|
| 0 | Dev Environment Bootstrap (WSL2 + toolchain) | Not Started | Low | — |
| 1 | Scaffold + Brand Tokens + Capability Skeleton + Error Infra | Not Started | Medium | 0 |
| 2 | Four-Region Layout + Shortcuts + i18n Scaffold + Installable .deb | Not Started | Medium | 1 |
| 3 | Editor + File Tree + Find/Replace | Not Started | Medium | 2 |
| 4 | Terminal (xterm.js + portable-pty) | Not Started | Medium | 2 |
| 5 | Keyring + Anthropic Provider + Chat Panel (virtualized E2E) | Not Started | Medium | 2 |
| 6a | All Providers + Read-Only Tool Surface + Agent Activity UI | Not Started | Medium | 5 |
| 6b | Write Tools + Inline Edit (split-diff) + Rewind | Not Started | High | 3, 6a |
| 7 | Git Panel + Preview Panel | Not Started | Medium | 3 |
| 8 | LSP Client (5 servers) | Not Started | Medium | 3 |
| 9 | Onboarding + Settings UI + Theming + Icon + Data/Persistence Polish | Not Started | Medium | 5, 6a |
| 10 | Auto-Update + a11y Audit + Error Catalogue Consolidation | Not Started | Low | 9 |
| 11 | Packaging + CI + GPG Signing + Release Smoke Test | Not Started | Medium | 10 |

Total: **13 phases** (0 through 11, where Phase 6 is split 6a/6b). Estimated calendar: Phase 0 half day; Phases 1/2/4/5/7/8/10 ≈ 1 day each; Phases 3/6a/9/11 ≈ 2 days each; Phase 6b ≈ 2 days. **Total ≈ 17 focused working days** for a solo maintainer — roughly aligned with the vision's 21-day sketch, slightly looser than r1's 15 because Phase 6 is split and auto-update/a11y/error-consolidation are explicit.

## Phases

### Phase 0 — Dev Environment Bootstrap (WSL2 + toolchain)

**Goal:** Windows maintainer reaches a working WSL2 Ubuntu 24.04 dev environment — `cargo tauri --version` succeeds, project lives in `~/biscuitcode/`, all apt dependencies present — before any code phase runs.

**Deliverables:**
- `scripts/bootstrap-wsl.sh`: idempotent `apt` install of the full Tauri prereq list: `pkg-config libdbus-1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf libfuse2t64 file build-essential curl gnome-keyring libsecret-1-0 libsecret-tools busctl`.
- `scripts/bootstrap-toolchain.sh`: installs rustup (stable 1.85+), `cargo-tauri-cli@2.10.1`, Node.js 20+ via nvm, `pnpm@9+`.
- `docs/DEV-SETUP.md` (new, short): WSL2 install, why the project must live in `$HOME`, bootstrap instructions, `pnpm tauri dev` launching into WSLg.
- PR description: output of `cargo tauri --version`, `node --version`, `pnpm --version`, `rustc --version`, `busctl --user list | head`.

**Acceptance criteria:**
- [ ] `bash scripts/bootstrap-wsl.sh` on fresh WSL2 Ubuntu 24.04 exits `0`.
- [ ] `cargo tauri --version` prints `tauri-cli 2.10.x`.
- [ ] `pnpm --version` prints `9.x` or higher.
- [ ] `apt list --installed 2>/dev/null | grep -c libwebkit2gtk-4.1-dev` returns `1`.
- [ ] `busctl --user list 2>/dev/null | grep -c org.freedesktop.secrets` returns `1` on a session where `gnome-keyring-daemon` is running (documents the happy path).
- [ ] `realpath .` in the script asserts the path does not begin with `/mnt/`.
- [ ] `README.md` links to `docs/DEV-SETUP.md`.

**Dependencies:** None.
**Complexity:** Low.
**Split rationale:** r1 has the same phase. Keeping it as a named phase (rather than "prerequisite chatter") enforces that Phase 1 starts from a known-good environment. r2 adds `busctl` install to the apt list, reinforcing the read-only Secret Service probe used later in Phase 5.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 1 — Scaffold + Brand Tokens + Capability Skeleton + Error Infra

**Goal:** Empty Tauri project, brand tokens wired into Tailwind + Rust, capability ACL files authored deny-by-default, and a thin but real error-code scaffolding (enum + toast component) that feature phases will extend.

**Deliverables:**
- `pnpm create tauri-app` scaffold: React + TS + Vite + Tailwind, app name `biscuitcode`, bundle ID `io.github.Coreyalanschmidt-creator.biscuitcode`.
- Workspace with **one crate only this phase**: `biscuitcode-core`. Sibling crates (`-agent`, `-providers`, `-db`, `-pty`, `-lsp`) are created in the phase that first uses them (matches r1's deferred-creation rule).
- `tauri.conf.json`: `bundle.active: true`, `bundle.identifier`, Linux section with `webkitVersion: "4.1"`, `deb.depends: ["libwebkit2gtk-4.1-0", "libgtk-3-0"]`, `deb.recommends: ["gnome-keyring", "ollama"]`.
- `tailwind.config.ts` with brand tokens verbatim: `biscuit-50..900`, `cocoa-50..900`, semantic `ok/warn/error`. CSS custom properties defined in `src/theme/tokens.css`.
- Self-hosted fonts: `src-tauri/fonts/Inter-{Regular,Medium,SemiBold}.woff2` + `JetBrainsMono-{Regular,Medium}.woff2`. `@font-face` rules in `src/theme/fonts.css` with fallback `'Ubuntu', sans-serif` (for Inter) and `'Ubuntu Mono', 'DejaVu Sans Mono', monospace` (for JetBrains Mono).
- `src-tauri/capabilities/{core,fs,shell,http}.json` hand-authored, deny-by-default. Initial `fs` scope = `$APPCONFIG`, `$APPDATA`, `$APPCACHE` only. `shell` and `http` capabilities have **no** allow rules yet; features add rules in their phases.
- `src/theme/tokens.ts` TS constants and `biscuitcode-core::palette` Rust constants (mirror).
- Rust window chrome: default decorations off, custom titlebar showing `BiscuitCode` in Inter 14px on cocoa-700.
- **Error infrastructure (new — diverges from r1):**
  - `biscuitcode-core::error::Error` — `thiserror`-derived enum with a `code()` method returning `&'static str` like `"E005"`. Initial variants: `NetworkUnreachable (E001)`, `InternalCapabilityDenied (E018)`. Other codes are added in the phase that can emit them.
  - `src/errors/types.ts` — TS enum matching every Rust code. Build gate: a unit test asserts the two sides stay in sync via a generated JSON file.
  - `src/errors/ErrorToast.tsx` — a single toast component keyed by code, reads a message table loaded from locales (Phase 2 wires i18n; Phase 1 hardcodes English). Recovery action = optional callback.
- CI workflow skeleton at `.github/workflows/ci.yml` — `lint`, `typecheck`, `test` jobs on PR; jobs run even with minimal content so later phases extend, not invent.
- `LICENSE` (MIT), `.gitattributes`, `.editorconfig`.

**Acceptance criteria:**
- [ ] `pnpm install && pnpm tauri dev` opens a WSLg window in under 2s on the dev machine.
- [ ] Document background is exactly `#1C1610`; a visible `--biscuit-500` (`#E8B04C`) accent strip renders on the sidebar placeholder.
- [ ] `grep -c 'fonts.googleapis\|cdn\.jsdelivr' dist/index.html` returns `0` (no external font/CDN loaded).
- [ ] `src-tauri/capabilities/fs.json` declares `fs:allow-read-text-file` scoped **only** to `$APPCONFIG`; `grep -c '"identifier": "fs:allow-write"' src-tauri/capabilities/fs.json` returns `0`.
- [ ] `grep -cE '^(biscuit|biscuit-auth|biscuit-cli)\s*=' src-tauri/Cargo.toml` returns `0`.
- [ ] `cargo build -p biscuitcode-core -- -D warnings` succeeds.
- [ ] `cargo test -p biscuitcode-core error_codes_match_typescript` passes (validates error-code parity between Rust enum and TS enum).
- [ ] `grep -rn 'font-family' src/theme/fonts.css | grep -v 'system-ui'` returns all lines (no `system-ui` in the chrome fallback chain).
- [ ] A PR touching only `README.md` triggers CI and the `lint` job exits `0`.
- [ ] `biscuitcode-core::error::Error::NetworkUnreachable.code() == "E001"` via inline doctest.

**Dependencies:** Phase 0.
**Complexity:** Medium.
**Split rationale:** Identical surface to r1's Phase 1 except for two additions: (a) the font-fallback chain is explicit (r2 G9 flagged r1's silence about what happens when Inter fails to load), (b) the error-infrastructure scaffold lands here rather than at Phase 9. I prefer distributed error-code ownership — each feature phase owns its own codes — because centralizing in Phase 9 means Phase 9 has to re-audit all feature paths, which is more work than extending as we go.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 2 — Four-Region Layout + Shortcuts + i18n Scaffold + Installable .deb

**Goal:** Render the full layout (Activity Bar / Side Panel / Editor Area / Bottom Panel / Chat Panel / Status Bar), wire every vision shortcut (real where implemented, toast-placeholder otherwise), scaffold i18n so all chrome strings are `t('key')`-wrapped from day 1, and produce the first installable `.deb`.

**Deliverables:**
- `src/layout/WorkspaceGrid.tsx` using `react-resizable-panels`. Panel sizes persisted via a **Zustand + localStorage bridge** (one record per panel). **Outer window geometry** (position, maximized) handled separately by `plugin-window-state`. These are two different concerns.
- Empty component shells (1–2 lines each, each renders a labelled placeholder): `ActivityBar`, `SidePanel`, `EditorArea`, `TerminalPanel`, `ChatPanel`, `AgentActivityPanel`, `PreviewPanel`, `StatusBar`.
- `ActivityBar` 48 px with `lucide-react` icons (Files, Search, Git, Chats, Settings). Active icon has a 2 px `--biscuit-500` left-edge bar.
- Shortcut layer in `src/shortcuts/global.ts` handling **every** vision shortcut, with chord support (`Ctrl+K Ctrl+I`). Real implementations: `Ctrl+B`, `Ctrl+J`, `Ctrl+Alt+C`. Placeholders fire a toast `"<shortcut> registered; lands in Phase <n>"` so verification is honest.
- Command palette (`Ctrl+Shift+P`) with registered commands: `View: Toggle Side Panel`, `View: Toggle Bottom Panel`, `View: Toggle Chat Panel`.
- Status bar renders placeholders: `git:main`, `0 errors`, `claude-opus-4-7`, `Ln 1 C1`.
- **i18n scaffold (new — diverges from r1):** `react-i18next` with a single English bundle at `src/locales/en.json`. Every string in chrome components is wrapped in `t('key')`. `useTranslation()` hook initialised at app root. Settings → Appearance → Language dropdown with only "English" enabled; disabled tooltip "Additional languages in v1.1".
- **Focus management scaffold (new — supports a11y in Phase 10):** `F6` cycles focus between the four regions; interactive components use `focus-visible:ring-2 focus-visible:ring-biscuit-500` Tailwind utilities.
- Icon assets: placeholder `biscuitcode.png` (single 64x64 for now) in `packaging/icons/` so `.deb` has a non-broken icon; final icon lands in Phase 9.
- `cargo tauri build --target x86_64-unknown-linux-gnu` produces `biscuitcode_0.1.0_amd64.deb`.

**Acceptance criteria:**
- [ ] Every region in the vision's ASCII layout renders at spec default size (Activity 48 px, Side 260 px, Bottom 240 px, Chat 380 px).
- [ ] `Ctrl+B` toggles side panel; after re-open the previous width is restored (localStorage).
- [ ] `Ctrl+Shift+P` opens palette; typing "toggle bottom" + Enter toggles the bottom panel.
- [ ] Every shortcut in the vision table is registered — each either performs or shows the "registered; lands in Phase N" toast; none silently no-op.
- [ ] `F6` cycles focus through the four regions in a fixed order (assertion via Playwright or a keyboard test).
- [ ] `pnpm tauri build` produces `src-tauri/target/release/bundle/deb/biscuitcode_0.1.0_amd64.deb`.
- [ ] On fresh Mint 22 XFCE VM: `sudo dpkg -i biscuitcode_0.1.0_amd64.deb` then `dpkg -s biscuitcode | grep -F 'Version: 0.1.0'` returns exactly one line.
- [ ] Whisker menu → Development → **BiscuitCode** appears and launches.
- [ ] `sudo apt remove biscuitcode` removes binary, desktop entry, and icon; `ls /usr/share/applications/biscuitcode.desktop 2>/dev/null` returns empty.
- [ ] `grep -rnE '">\s*[A-Z][a-z]' src/components/` returns zero hits (every UI string is routed through `t(...)`, so no bare capitalized strings in JSX children).
- [ ] `cat src/locales/en.json | jq '.chrome | length'` returns ≥ 20 (all chrome labels present).

**Dependencies:** Phase 1.
**Complexity:** Medium.
**Split rationale:** This is the first point where the app becomes installable, matching vision Phase 1's runnable checkpoint. Bundling i18n here (rather than bolt-on later) is r2 G1's advice: 1 hour now avoids a sweep later. Focus management is a 30-minute addition that saves Phase 10 from retrofitting accessibility. The `.deb` landing here de-risks Phase 11 — packaging becomes incremental polish, not a from-scratch build.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 3 — Editor + File Tree + Find/Replace

**Goal:** Working Monaco multi-tab editor, live file tree with real filesystem ops scoped to the workspace, in-file and cross-file find/replace.

**Deliverables:**
- `@monaco-editor/react` pinned locally (no CDN), `vite-plugin-monaco-editor` with **`languageWorkers: []` explicit empty** — no default language workers at boot. Languages (`typescript`, `javascript`, `json`, `css`, `html`, `rust`, `python`, `go`, `markdown`) registered on-demand when a matching file opens. Per r2 D2.
- `EditorArea.tsx`: tab bar (dirty dot, middle-click close, `Ctrl+W`, `Ctrl+Shift+T` reopen), single Monaco instance, one `ITextModel` per tab, language autodetection from extension, JetBrains Mono 14px, ligatures on by default.
- `Ctrl+\` implements horizontal split via a second Monaco instance — two panes each with their own tab bar; both share the same `ITextModel` cache.
- Diff view stub: `createDiffEditor` can be constructed and given two models (no UI yet).
- `SidePanel: Files` tree with lazy `FileTreeNode`. Workspace opens via `plugin-dialog`. Context menu: New File, New Folder, Rename, Delete, Reveal in File Manager (`xdg-open`), Copy Path, Open in Terminal (emits event for Phase 4).
- Rust commands in `src-tauri/src/commands/fs.rs`: `fs_list`, `fs_read`, `fs_write`, `fs_rename`, `fs_delete`, `fs_create_dir`, `fs_open_folder`. Each validates the path is a descendant of the workspace root or returns error code `E008: FsOutsideWorkspace`.
- Capability `fs.json` amended: `fs:allow-read-text-file`, `fs:allow-write-text-file`, binary counterparts, scoped dynamically via `FsScope::allow_directory(workspace_root, recursive=true)` on `fs_open_folder`, revoked on workspace close. **`Arc<RwLock<WorkspacePath>>` guards coherent reads during the allow/revoke window** (r2 D1 race note).
- Find in file (`Ctrl+F`): Monaco built-in, exposed.
- Find across files (`Ctrl+Shift+F`): Side Panel pane with regex / case / whole-word toggles. Backend uses `ignore` + `grep` crates over workspace root.
- File tree git-status colouring placeholder (hook; real git in Phase 7).
- Error code adoption: Rust variants added for `E007 (FsPermissionDenied)`, `E008 (FsOutsideWorkspace)`, mirrored in TS enum.
- **Monaco lazy-load proof**: `performance.measure` confirms Monaco bundle fetched after initial paint (name: `monaco-first-fetch`).

**Acceptance criteria:**
- [ ] Open a `.ts` file: syntax highlighting active; JetBrains Mono rendering confirmed; ligatures on.
- [ ] Ctrl+W closes current tab; middle-click closes.
- [ ] New File via tree creates disk file; rename updates disk name; delete asks confirm and removes.
- [ ] `fs_read` on a path outside the workspace returns `Error::FsOutsideWorkspace` (code `"E008"`), verifiable via devtools manual invoke.
- [ ] `Ctrl+Shift+F` for `TODO` across a 1k-file workspace returns results in under 2 s.
- [ ] `pnpm tauri build && dpkg-deb -c biscuitcode_*.deb | grep -c monacoeditorwork` returns ≥ 3 (workers bundled).
- [ ] Cold-launch to shell (no file open) under 2 s on i5-8xxx: `time (biscuitcode & sleep 3 ; wmctrl -l | grep -q BiscuitCode)` shows window within 2000ms.
- [ ] `performance.getEntriesByName('monaco-first-fetch')[0].startTime > 200` (assertion in an e2e test — proves Monaco did not block first paint).
- [ ] `Ctrl+\` splits the editor into two horizontal panes each bound to the same `ITextModel`.
- [ ] `fs.json` capability contains a runtime-patched scope matching the opened workspace (assertion via a debug command that returns the current scope list).

**Dependencies:** Phase 2.
**Complexity:** Medium (edging High because of worker wiring + scoped-fs runtime patching).
**Split rationale:** Editor + tree + find-in-files belong together — they share the `fs` capability patch logic, Monaco is useless without a file source, and find-in-files reuses the same scope guard. Git-status colouring deliberately deferred to Phase 7 with the rest of git. `Ctrl+\` split is kept here (not later) because its implementation is inside the editor component, and the vision treats it as core navigation.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 4 — Terminal (xterm.js + portable-pty)

**Goal:** Multi-tab integrated terminal with real PTY-backed shells, clickable URLs and file paths, wired to "Open in Terminal" from Phase 3.

**Deliverables:**
- **Create workspace crate `biscuitcode-pty` here.**
- `TerminalPanel.tsx` with tabbed `@xterm/xterm` 5.x instances, addons `fit`, `web-links`, `search`, `webgl` (canvas fallback if WebGL2 absent).
- Rust `biscuitcode-pty` crate exposing commands `terminal_open(shell, cwd, rows, cols) -> SessionId`, `terminal_input(session_id, bytes)`, `terminal_resize(session_id, rows, cols)`, `terminal_close(session_id)`.
- Two Tokio tasks per session: reader (PTY master → `terminal_data_{session_id}` event), writer (consumes queued input). Sessions in `Arc<RwLock<HashMap<SessionId, PtySession>>>`.
- Shell detection: `$SHELL` → `getent passwd $UID` → `/bin/bash`.
- Custom link provider for `path/to/file:line[:col]` patterns → emits `open_file_at` event (consumed by editor).
- `Ctrl+\`` real implementation (placeholder from Phase 2 now lights up): focuses terminal panel; `+` button opens new tabs.
- On tab close, PTY master/slave dropped and child killed.
- Error codes added: none specific to PTY; errors flow through `biscuitcode-core::Error` as `Internal`.

**Acceptance criteria:**
- [ ] Open terminal → prompt appears within 500 ms; `echo $SHELL` returns the user's actual shell.
- [ ] Resizing the terminal panel resizes the PTY (assertion: `tput lines` and `tput cols` in terminal match rendered rows/cols after resize).
- [ ] Click a URL in terminal output → opens in default browser via `plugin-shell` (allow-list-restricted).
- [ ] Click `src/main.rs:12` in terminal output → opens `src/main.rs` at line 12 in the editor pane.
- [ ] Close a terminal tab → `pgrep -f 'biscuitcode.*bash'` returns no orphans after 2 s.
- [ ] Five concurrent terminals each running `yes > /dev/null`: total CPU under one core; no crash over 60 s.
- [ ] WebGL renderer active by default (confirmable via `document.querySelector('.xterm-screen canvas')` absent; WebGL textures present in `performance.getEntries()`).

**Dependencies:** Phase 2.
**Complexity:** Medium.
**Split rationale:** Terminal is a one-day self-contained unit and unblocks the LSP Tokio-stream pattern (Phase 8) and the tool result streaming in Phase 6. Keeping it before Phase 5 gives an early OS-integration win even if the chat providers hit a snag.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 5 — Keyring + Anthropic Provider + Chat Panel (virtualized E2E)

**Goal:** User adds an Anthropic API key in settings (stored in libsecret), opens the chat panel, picks `claude-opus-4-7`, types a message, and watches streaming tokens render into a `react-virtuoso`-backed list — no tools, no agent loop.

**Deliverables:**
- **Create workspace crates `biscuitcode-providers` and `biscuitcode-db` here.**
- `biscuitcode-core::secrets`: wraps `keyring` 3.6 with features `linux-native-async-persistent + async-secret-service + crypto-rust + tokio`. API: `async fn set(service, key, value)`, `async fn get`, `async fn delete`.
- **Secret Service read-only probe (`busctl list --user`-based)** at startup; emits event `secret_service_available: bool`. If false, onboarding Step 2 blocks with `E005 KeyringMissing` toast + install command. **Never probes via `keyring::get` (r2 D6: avoids accidental daemon activation).**
- `biscuitcode-providers::anthropic::AnthropicProvider` implementing `ModelProvider`:
  - `reqwest` with HTTP/2 keep-alive, optional prewarm on app start (speculative HEAD).
  - Full SSE parse of Anthropic streaming envelope.
  - Delta routing: `text_delta → TextDelta`, `thinking_delta → ThinkingDelta`, `input_json_delta → ToolCallDelta` (accumulated + finalized on `content_block_stop`).
  - **Sampling params stripped for Opus 4.7**: `ChatOptions { temperature, top_p, top_k }` are `Option`, and the impl unconditionally `None`s them when `model.starts_with("claude-opus-4-7")`. Unit test `anthropic_provider::requests_strip_sampling_for_opus_47` asserts the outgoing JSON lacks those keys.
  - **Prompt caching (new — diverges from r1)**: `cache_control: {type: "ephemeral"}` on the system prompt **and** on the tool-definition block (tools ship empty initially; the cache point is there so Phase 6a doesn't re-invalidate caches). Default-on. Test: unit asserts `cache_control` present in request body for a message older than the 5th turn.
  - Model list: `claude-opus-4-7` (default), `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`, `claude-opus-4-6` (legacy).
- `ChatPanel.tsx` using `VirtuosoMessageList` from `react-virtuoso`:
  - Message list with markdown rendering (`react-markdown` + `remark-gfm`), code blocks with copy button (apply/run deferred to Phase 6b).
  - Model picker sourced from provider registry.
  - `aria-live="polite"` on the messages container (Phase 10 a11y audit gates this).
  - Send button; streaming token render.
- `biscuitcode-db` using `rusqlite` with WAL mode, `PRAGMA user_version=1` migrations. Schema: `workspaces`, `conversations`, `messages` per r1 §10 plus a `content_json` column for storing text/tool/thinking blocks. Migration script embedded as Rust const string.
- `http.json` capability: fetch allowlist `https://api.anthropic.com/**`.
- Settings provider page stub (`SettingsProviders.tsx`): list providers, badges (green/yellow/red), test-connection button.
- Error codes adopted: `E001 (NetworkUnreachable)`, `E002 (AuthInvalid)`, `E003 (ProviderDown)`, `E004 (ProviderRateLimit)`, `E005 (KeyringMissing)`, `E006 (KeyringLocked)`, `E017 (DbCorrupt)`.
- First-token latency telemetry-scaffold event (no wire send).

**Acceptance criteria:**
- [ ] `settings → Models → Anthropic → Add key` stores key in libsecret. `secret-tool search service biscuitcode` returns the value. `grep -r 'ANTHROPIC_API_KEY\|sk-ant' ~/.config/biscuitcode/` returns zero matches.
- [ ] Typing "say hi in three words" → assistant tokens render within 500 ms of send-button click, **p50 measured over 20 prompts after a 1-minute prewarm**. p95 under 1200 ms.
- [ ] Sending with Opus 4.7 and `temperature: 0.7` attempted via devtools → HTTP 200 (the provider filtered the field). Unit test `anthropic_provider::requests_strip_sampling_for_opus_47` passes.
- [ ] Conversation persisted — app restart shows prior message; `messages` row exists with correct `conversation_id`, `role`, `content_json`.
- [ ] On fresh VM without `gnome-keyring`: add-key flow shows `E005` toast with install command. No plaintext file is created under `~/.config/biscuitcode/`.
- [ ] Secret Service probe uses `busctl list --user` (assertion: a debug command exposes the probe method, asserts it is not `keyring::get`).
- [ ] `ChatPanel` virtualized: with 500 synthetic messages in the conversation, scroll jank is absent (measured: frame time under 16 ms during scroll; `performance.mark`-based assertion).
- [ ] Unit test `anthropic_provider::system_prompt_is_cached` asserts `cache_control: {type: "ephemeral"}` is present on the system block in the outgoing request.

**Dependencies:** Phase 2.
**Complexity:** Medium.
**Split rationale:** Matches r1's Phase 5 "one provider E2E" checkpoint closely but adds three things: (a) `react-virtuoso` wired from the start so Phase 6a's Agent Activity reuses the same abstraction, (b) prompt caching from day one (r2 flagged r1's omission), (c) Secret Service probe via `busctl` not `keyring`. These are each tiny additions that cost almost nothing now and save rework later.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 6a — All Providers + Read-Only Tool Surface + Agent Activity UI

**Goal:** Ship OpenAI and Ollama providers behind the same `ModelProvider` trait, a read-only tool surface (`read_file`, `search_code`), a ReAct executor loop that can chain N tool calls, and the Agent Activity UI rendering tool cards in real time. **No writes, no shell, no rewind, no inline edit yet** — those land in Phase 6b.

**Deliverables:**
- **Create workspace crate `biscuitcode-agent` here.**
- `biscuitcode-providers::openai::OpenAIProvider`:
  - Chat Completions API only (not Responses API; r2 D4 — Responses is a separate second decoder we defer).
  - Per-index `tool_calls` argument accumulation across chunks; emit `ToolCallStart + ToolCallDelta* + ToolCallEnd` on `finish_reason === "tool_calls"`.
  - Default `gpt-5.4-mini`; picker exposes `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5.4-pro` (flagged "reasoning; slower first-token"), legacy `gpt-5.2 Thinking` until 2026-06-05.
  - **Reasoning-mode TTFT exemption**: `ChatPanel` UI displays "Thinking..." state when `model.startsWith('gpt-5.4-pro')` with no tokens observed after 1.5s. The p50-under-500ms gate excludes reasoning models.
- `biscuitcode-providers::ollama::OllamaProvider`:
  - NDJSON parsing of `/api/chat`.
  - `tools` passthrough (OpenAI-function-call format); extract `message.tool_calls` from the final non-done chunk.
  - **Malformed-tool-call fallback (r2 D5)**: if model emits `<tool_call>...</tool_call>` XML in `message.content` or a JSON-ish blob in prose, the executor regex-extracts and emits it as a `ToolCallStart + End` pair; on unrecoverable parse error, loops back with a "your last tool call had invalid JSON, please retry" user message.
  - Model picker from `GET /api/tags`; default `gemma3:4b` with RAM tiering (`<6 GB → gemma3:1b`; `6–12 → gemma3:4b`; `12+ → qwen2.5-coder:7b` if agent mode else `gemma3:4b`; `gemma4:*` preferred when present).
  - `ollama_install()` command: probe via `curl -sSfm 1 http://localhost:11434/api/version` + `which ollama`; if absent, confirm dialog shows verbatim `curl -fsSL https://ollama.com/install.sh | sh` command before running via `plugin-shell`.
  - `ollama_pull(model)` piped to a progress bar.
  - RAM detection via `sysinfo` crate.
- `http.json` capability: add `http://localhost:11434/**` and `https://api.openai.com/**`.
- `shell.json` capability: add `ollama {pull <model>, list, show <model>, serve}` with argument regex. No wildcard args.
- Per-conversation model switch: dropdown persisted to `conversations.active_model`.
- **Read-only tool registry** (`biscuitcode-agent::tools`): two tools only.
  - `read_file({path: string}) -> {content: string, truncated: bool}` — validates path is inside workspace root; returns `E008` otherwise; 256 KB cap, truncated flag set if exceeded.
  - `search_code({pattern: string, glob?: string}) -> {matches: [{path, line, text}]}` — wraps `ignore` + `grep` crates, workspace-scoped.
  - JSON Schema definitions + side-effect class tag (`read` for both) + confirmation policy (none for reads, matches Cursor/Zed behavior).
- **`biscuitcode-agent::executor`** ReAct loop:
  - Accepts a conversation and streams from the selected provider.
  - On `ToolCallEnd`: decodes accumulated args JSON, checks side-effect class, **auto-runs if `read`**. Writes and shell are rejected here (land in Phase 6b).
  - Appends `ToolResult` to the conversation, continues the loop until `stop_reason: end_turn` or pause flag set.
  - Atomic pause flag; checked at loop iteration boundaries and **at least every 5 seconds** if stuck on a long streaming block.
  - `Esc` (or Cancel button) flips the pause flag.
- **`AgentActivityPanel.tsx`** using `VirtuosoMessageList` (same abstraction as ChatPanel):
  - Each tool call renders as a collapsible card: tool name, pretty-JSON args, streamed result, timing, status icon (`running` / `ok` / `error`).
  - Badge on the chat message links to its activity cards.
  - `aria-live="polite"` on results (screen-reader friendly).
  - **Tool-card render trace**: on every `ToolCallStart` event, executor emits `performance.mark('tool_call_start_<id>')`; when card first paints, a MutationObserver emits `performance.mark('tool_card_visible_<id>')`. Gate: all deltas under 250 ms.
- Agent-mode toggle in chat panel (default off). Off = single-turn (no tool calls at all). On = ReAct loop active with read-only tools.
- Chat context mentions — editor-local subset: `@file`, `@folder`, `@selection` (all wired to Phase 3's tree + editor). `@terminal-output`, `@problems`, `@git-diff` arrive in Phases 7 (git-diff), 8 (problems), 4 (terminal-output retroactively in Phase 7 alongside git work). Drag-file-into-chat inserts the same `@file:<path>` token.
- Error codes adopted: `E011 (OllamaDown)`, `E012 (OllamaModelMissing)`, `E013 (OllamaPullFailed)`, `E015 (ToolArgsInvalid)`.

**Acceptance criteria:**
- [ ] With OpenAI key set, send via `gpt-5.4-mini` → streams text; trigger a read-tool call → matching card appears.
- [ ] Switching the same conversation to `claude-opus-4-7` mid-thread preserves prior messages; assistant sees prior tool results.
- [ ] On fresh VM without Ollama: "Install Ollama" shows confirm dialog with `curl | sh` verbatim; declining does nothing; accepting installs.
- [ ] After Ollama install: `ollama_pull("gemma3:4b")` progress bar updates at least once per second until complete.
- [ ] 8 GB VM → RAM detector picks `gemma3:4b`; 4 GB VM → picks `gemma3:1b`.
- [ ] Local `gemma3:4b` streams text in under 3 s on i5-8xxx (warm model). `gemma4:*` when available emits structured `tool_calls`.
- [ ] Ollama model that emits `<tool_call>...</tool_call>` XML in content: executor extracts, emits `ToolCallStart+End`, card renders correctly. Test `ollama_malformed_tool_call_recovery` passes.
- [ ] `ChatEvent` stream is byte-for-byte identical across all three providers for an equivalent "hello" prompt (snapshot test).
- [ ] Agent mode on, "list files in src/" → Agent Activity shows `search_code` card with streaming args + result → assistant summarizes.
- [ ] Write-tool attempt (e.g., assistant calls a fictional `write_file`) → rejected with `E015` or "tool not found" error; **not** executed (writes are Phase 6b).
- [ ] Pause during a long agent run: stops before next tool call, and within 5 s of click even without a pending tool.
- [ ] Tool-card render latency gate: for a 3-tool prompt, all `tool_card_visible_<id> - tool_call_start_<id>` measures under 250 ms (e2e test `agent_tool_card_visible_within_250ms`).
- [ ] Reasoning-mode test: with `gpt-5.4-pro` and a prompt requiring reasoning, `ChatPanel` shows "Thinking..." state within 1.5 s of send, then streams the result. First-token-latency gate is **not** applied.

**Dependencies:** Phase 5 (chat panel, provider trait, keyring).
**Complexity:** Medium. (Kept Medium by excluding writes/rewind/inline-edit.)
**Split rationale:** r1 bundled all three providers, the full tool set, agent UI, inline edit, and rewind into one Phase 6 (High complexity). I split for two reasons: (a) r2 A4 argued read-only-in-v1 is a serious alternative — I take the middle path of "read-only lands first, writes land after" within v1, (b) the `ChatEvent` trait's cross-provider parity is unverifiable if only Anthropic implements it; shipping all three before writing the tool registry means Phase 6a's snapshot test is real. The cost is one extra phase boundary; the benefit is a risk split that matches r2 A4's scope concerns while still shipping the full vision.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 6b — Write Tools + Inline Edit (split-diff) + Rewind

**Goal:** Extend Phase 6a's read-only agent with `write_file`, `run_shell`, `apply_patch` tools behind confirmation UX; ship inline AI edit on selection (Zed-style split-diff via `createDiffEditor`); implement per-action snapshot rewind.

**Deliverables:**
- **Write tools** in `biscuitcode-agent::tools`:
  - `write_file({path: string, content: string}) -> {bytes_written: u64}` — workspace-scoped, returns `E008` otherwise, **requires confirmation unless workspace trust is enabled**.
  - `apply_patch({path: string, unified_diff: string}) -> {hunks_applied: u32, conflicts: []}` — validates patch applies; returns conflict list if not; requires confirmation.
  - `run_shell({command: string, args: string[]}) -> {exit: i32, stdout, stderr}` — **denies `sudo`, `su`, `rm -rf /` prefix-matched**; restricted to binaries in `PATH`; confirmation required; executed via `plugin-shell` with per-call registry injection.
- **Confirmation UX**: when agent requests a write/shell tool, Agent Activity card enters `pending` state; modal shows the pretty-printed operation (diff for writes, command for shell); Accept / Reject / "Trust this workspace forever" buttons. Auto-approved if workspace trust toggle is on.
- **Workspace-trust toggle** (stored in per-workspace settings under `~/.config/biscuitcode/workspaces.json`).
- **Per-action snapshot rewind**:
  - Before each write/shell tool: snapshot affected files (writes: pre-write contents; shell: none — shell is idempotent-by-the-user's-own-risk) into `~/.cache/biscuitcode/snapshots/{conversation_id}/{message_id}/...`. Manifest recorded in `messages.snapshot_manifest_json`.
  - Conversation header: per-assistant-message rewind button. Click → restores snapshots (via the fs commands, respecting workspace scope) + truncates messages past that point (SQL `DELETE` with cascade on `parent_id`).
- **Inline edit (`Ctrl+K Ctrl+I`) — Zed-style split-diff (r2 D7)**:
  - Selection + shortcut → popover input at cursor location → on submit, open a transient Monaco `createDiffEditor` in a right-side split pane with the selection on the left and a streamed pending buffer on the right.
  - Whole-diff Accept/Reject (not per-hunk — per-hunk is v1.1).
  - Regenerate button re-streams from the same prompt.
  - Popover input uses the provider selected for the active conversation; request includes the selection + surrounding file + prompt.
- **`apply` button** on chat code blocks: opens the affected file (if mentioned via `@file`) and applies the patch using Monaco's model diff; fallback = user picks target file.
- **`run` button** on chat code blocks: pushes the selected code into a new terminal tab (no auto-exec; user hits Enter).
- Error codes adopted: `E014 (ShellForbidden)`, `E018 (CapabilityDenied)`.

**Acceptance criteria:**
- [ ] With workspace-trust off: `write_file` call triggers the confirmation modal showing a diff; declining cancels; accepting writes.
- [ ] With workspace-trust on: same call auto-approves; `write_file` executes without modal.
- [ ] `run_shell` called with `sudo rm -rf /` rejected with `E014 ShellForbidden`; no execution.
- [ ] Rewind on the assistant message that created `hi.txt` restores pre-create state (file absent) and truncates later messages from `messages` table; `SELECT COUNT(*) FROM messages WHERE conversation_id=? AND created_at > ?` returns `0` after rewind.
- [ ] `Ctrl+K Ctrl+I` on a selected function: popover appears; submitting streams a diff into the right-split pane; Accept replaces the selection; Reject discards; Regenerate re-streams.
- [ ] Applying a patch via the `apply` button on a chat code block opens the file and applies the patch atomically (a single undo in Monaco reverts the whole patch).
- [ ] `run_shell` with `ls` runs and returns output; shell capability allow-list contains `ls` (registered just-in-time when the agent requests it, with user confirm).
- [ ] Pause during a long agent run mid-snapshot: the snapshot is flushed to disk before the pause lands, so rewind still works — assertion: pause + rewind + inspect `snapshots/` directory shows complete manifest.
- [ ] Snapshot corruption scenario (manually corrupt `manifest_json` in DB): rewind surfaces `E017 DbCorrupt` and does not touch files.

**Dependencies:** Phase 3 (fs commands, editor + diff editor), Phase 6a (executor, tool registry, Agent Activity UI).
**Complexity:** High.
**Split rationale:** This is the single highest-risk phase in the plan and sits alone. Split from 6a so that a bad week on write-tool UX or rewind semantics does not block the read-only agent (which is still shippable as a "v1 minus writes" if Phase 6b blows up). r2 A4 argues for deferring all writes to v1.1; I take a middle path — they're in v1 but isolated so a bad phase can be replanned without unraveling Phase 6a.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 7 — Git Panel + Preview Panel

**Goal:** VS Code-parity git (stage/unstage/commit/push/pull, gutter blame, status colours) + preview pane (Markdown, HTML, images, PDF, read-only notebook).

**Deliverables:**
- **Git** via `git2-rs` (reads) + `std::process::Command("git")` (writes):
  - Side Panel Git pane: files grouped `staged` / `unstaged` / `untracked`, hunk-level stage/unstage (Monaco inline diff buttons), commit message input, commit button, push/pull buttons streaming stdout to Terminal panel.
  - Branch name in status bar, clickable → branch switcher dropdown.
  - **Gutter blame**: off by default; settings toggle `editor.blame.gutter = true`. Uses `git2::BlameOptions` per visible line range; re-blames on `git commit` or file save; shows `hash[0..7] · author · relative-date`; blame column 180 px.
  - File-tree git-status colour hooks (from Phase 3) now wired.
- **Preview Panel** (split pane in editor area, never a new window):
  - Markdown: `react-markdown` + `remark-gfm` + `rehype-highlight` + `mermaid` + `rehype-katex`, live-update on editor changes (300 ms debounce).
  - HTML: sandboxed iframe with `sandbox="allow-scripts"` (no forms, no top-navigation); live-reload on save; devtools button uses `plugin-window`.
  - Images: `img` with CSS zoom/pan.
  - PDF: `pdf.js` via `react-pdf` (single-page view, next/prev).
  - Notebook `.ipynb`: read-only cell-by-cell render (markdown cells as markdown, code cells in JetBrains Mono, outputs as text/mime-typed blocks). No execution controls. No "Run" button.
  - Auto-open rule: AI-edited `.md` / `.html` / `.svg` / image files → open preview in split pane.
- **Non-editor chat mentions land here**: `@terminal-output` (active terminal tab's visible buffer), `@git-diff` (output of `git diff` for staged + unstaged). Picker surfaces these only when the relevant subsystem has data; disabled otherwise.
- Error codes adopted: `E016 (GitAuthRequired)`.

**Acceptance criteria:**
- [ ] In a repo: stage a hunk via inline diff button → status changes from `unstaged` to `staged`; commit with message; `git log -1` shows it.
- [ ] Branch switcher shows all local branches; switching updates status bar within 500 ms.
- [ ] Push/pull stream stdout to Terminal panel; auth failure yields `E016 GitAuthRequired` toast with "open credential helper" CTA.
- [ ] Gutter blame off by default; enabling it in settings shows `hash · author · relative-date` in gutter; disabling removes.
- [ ] Opening `README.md` → preview shows rendered markdown; typing updates preview within 200 ms.
- [ ] `.ipynb` with 3 cells renders read-only with cell borders; no run controls visible.
- [ ] HTML preview iframe: `window.top.location` blocked (assertion via console); `<form>` rejected by sandbox.
- [ ] Typing `@` in chat with a terminal open and in a git-repo surfaces `@terminal-output` and `@git-diff` options.
- [ ] File tree M/U/A/D badges reflect git status; modifying a file via the editor updates the badge within 500 ms.

**Dependencies:** Phase 3 (editor, file tree, diff editor).
**Complexity:** Medium.
**Split rationale:** r1 bundled Git + LSP + Preview as one "VS Code parity" Phase 8 (High). I split LSP into Phase 8 because LSP is a sufficiently different risk profile (child-process management, stdio proxying, per-language installation UX) that bundling with git/preview creates a two-day High-complexity blob. Splitting makes each Medium-complexity and reviewable.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 8 — LSP Client (5 servers)

**Goal:** Working LSP client for rust-analyzer, typescript-language-server, pyright, gopls, clangd, via `monaco-languageclient` + Rust stdio proxy.

**Deliverables:**
- **Create workspace crate `biscuitcode-lsp` here.**
- `biscuitcode-lsp` crate:
  - Per-language child process spawner based on detected project files: `Cargo.toml` → `rust-analyzer`; `package.json` | `tsconfig.json` → `typescript-language-server --stdio`; `pyproject.toml` | `requirements.txt` → `pyright-langserver --stdio`; `go.mod` → `gopls`; `CMakeLists.txt` | `compile_commands.json` → `clangd`.
  - One LSP child per (language, workspace) pair.
  - Tauri events `lsp-msg-in-{session_id}` + command `lsp_write(session_id, msg)`.
  - `stderr` piped to an "Output" tab in the bottom panel.
- Frontend:
  - `monaco-languageclient` with custom `MessageTransports` pair (reads from Tauri events, writes via invoke).
  - **TS worker silenced when LSP connects** (r2 D2): `monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({ noSemanticValidation: true, noSyntaxValidation: true })`.
  - Diagnostics rendered as Monaco squigglies; problem count in status bar clickable → opens Problems pane in bottom panel.
  - Missing-server dialog: copy-to-clipboard install command per language; **no auto-install**.
- `shell.json` capability: add `which <binary>` and the LSP binary names to the command registry; no wildcard args.
- **`@problems` chat mention** (deferred from Phase 6a) lands here: picker option for "all LSP diagnostics in current workspace."
- Error codes adopted: `E009 (LspMissing)`, `E010 (LspCrashed)`.

**Acceptance criteria:**
- [ ] Open a Rust file in a `Cargo.toml` project → `rust-analyzer` starts (verifiable via a "Language Servers" debug view); hover shows type; go-to-definition jumps correctly; diagnostics appear as squigglies.
- [ ] Missing `clangd`: opening a `.cpp` file shows `E009 LspMissing` toast with `sudo apt install clangd` in clipboard; clicking "Copy" puts it in clipboard (verify via `xsel --clipboard`); no auto-install.
- [ ] Killing a running `rust-analyzer` process: `E010 LspCrashed` toast appears with "Restart" button; click restarts.
- [ ] TS Monaco worker silenced when `typescript-language-server` connects: `monaco.languages.typescript.getTypeScriptWorker` returns a worker but `setDiagnosticsOptions({noSemanticValidation: true})` confirmed in a unit test.
- [ ] Typing `@` in chat with an LSP diagnostic present: `@problems` option available; selecting inserts diagnostic content into the message.
- [ ] Problems pane shows all diagnostics across open files; clicking a diagnostic jumps to the location in the editor.
- [ ] Stderr from `gopls` appears in the Output tab when the server starts.

**Dependencies:** Phase 3 (editor + problem count infrastructure), Phase 4 (Output tab hosting pattern from terminal panel).
**Complexity:** Medium.
**Split rationale:** Split from the git/preview phase because LSP's risk profile is unique — Tauri IPC proxying of stdio, one child per language, five different language-server UX stories. Keeping LSP alone makes its day-of-work focused and reviewable. Sequencing *after* git/preview is intentional: LSP has the lowest "user-visible first-impression" weight — a rough LSP hookup is forgivable; a rough git commit flow is not.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 9 — Onboarding + Settings UI + Theming + Icon + Data/Persistence Polish

**Goal:** Ship 3-screen onboarding, full settings UI, three themes with live preview, final icon (Concept A), plus data-polish items: corrupt-DB recovery, first-run recovery, conversation export.

**Deliverables:**
- **Onboarding** (`OnboardingModal.tsx`) — 3 screens:
  1. **Welcome**: BiscuitCode logo + tagline + Next.
  2. **Pick models**: Anthropic / OpenAI / Ollama cards. Each: add-key UI or Install Ollama button. At least one required before Next. Keyring-absent check runs via `busctl list --user`; on miss, step 2 blocks with install prompt.
  3. **Open a folder**: file picker; "Continue without a folder" secondary button for explore mode.
- **Idempotent onboarding (r2 G4)**: progress stored in `settings.json` under `onboarding: { version: 1, completed_steps: ["welcome", "provider"] }`. If step incomplete on launch, resume from that step. Corrupt settings → treat as fresh. Settings → About → "Re-run onboarding" button.
- **Settings page** (`SettingsPage.tsx`) with sections: General, Editor, Models, Terminal, Appearance, Security, Data, About. Raw JSON editor button opens `~/.config/biscuitcode/settings.json` in Monaco for power users.
- **Three themes**: `BiscuitCode Warm` (dark default), `BiscuitCode Cream` (light), `High Contrast`. CSS variable overrides in `src/theme/themes.ts`. Live preview on hover in Appearance pane.
- **GTK theme detection** at startup: Rust `detect_gtk_theme()` via `xfconf-query -c xsettings -p /Net/ThemeName`, fallback `gsettings get org.gnome.desktop.interface gtk-theme`. `-dark$` regex → dark; else light. On first run with a light GTK theme, offer to switch to Cream. Offer does not reappear on later launches.
- **Icon**: `packaging/icons/biscuitcode.svg` authored as Concept A (biscuit-gold `>_` glyph on cocoa-dark rounded-square #1C1610, 22% corner radius). Render via `rsvg-convert` to `biscuitcode-{16,32,48,64,128,256,512}.png`. `.ico` for Windows future.
- **16×16 legibility check**: CI step validates `biscuitcode-16.png` pixel-distinct `>_` shape; visual diff against checked-in reference.
- VS Code theme import: placeholder entry in Appearance, disabled, tooltip "Coming in v1.1".
- **Data/Persistence Polish (r2 G4, G7)**:
  - Settings → Data → "Export all conversations" → JSON file per workspace (`{conversation, messages[]}` per r2 G7 schema). "Open data folder" button reveals `~/.local/share/biscuitcode/` via `xdg-open`.
  - Per-conversation right-click → "Export as Markdown" in the Chats sidebar.
  - Corrupt DB recovery: on `PRAGMA integrity_check` failure at startup, rename `conversations.db` to `conversations.db.corrupt.<timestamp>`, create fresh. Show toast: "Previous conversation history was corrupted; starting fresh. Old file preserved at [path]."
- **Font-load canary** (r2 G9): in a one-time startup effect, check `document.fonts.check('14px Inter')`. If false, emit a debug log line and a Settings → About badge "Font fallback active." Non-blocking.
- **Telemetry toggle stub** (off by default): Settings → Privacy → "Send anonymous crash reports" off. On flip: show exact schema in dialog. Toggle stored in keyring. No wire implementation in v1.

**Acceptance criteria:**
- [ ] Fresh install → first launch shows onboarding; no way to reach main UI without either setting a provider or clicking "Skip" in step 2 (skip leaves badges red).
- [ ] On keyring-absent VM: step 2 shows exact install command (`sudo apt install gnome-keyring libsecret-1-0 libsecret-tools`); retry resumes once installed.
- [ ] Partial onboarding interrupted by crash: next launch resumes from the incomplete step (not the start).
- [ ] Settings → Appearance → hover Cream → preview shows cocoa-50 bg, biscuit-900 text. Select Cream → persists across restart.
- [ ] GTK theme `Mint-Xia-Light`: first run offers Cream; no offer on later launches (assertion: `onboarding.gtk_theme_offer_shown = true` in settings.json).
- [ ] 16×16 icon CI legibility check passes; visual diff within tolerance.
- [ ] `desktop-file-validate packaging/deb/biscuitcode.desktop` exits 0.
- [ ] `grep -rn 'font-family' src/` shows no `system-ui` in primary chrome paths (monospace fallbacks OK).
- [ ] Settings → Data → Export → produces a JSON file matching the schema; `jq '.conversations | length' export.json` returns the conversation count.
- [ ] Per-conversation "Export as Markdown" produces readable `.md` file with message roles and timestamps.
- [ ] Corrupt DB test: manually corrupt `conversations.db`, launch → toast appears, file renamed, fresh DB created, app usable.
- [ ] Font-load canary: remove `Inter-Regular.woff2` from the bundle → Settings → About shows "Font fallback active"; app remains usable on Ubuntu font.
- [ ] Telemetry toggle off persists across restart; on flip, schema dialog shown; flipping on stores `telemetry_opt_in=true` in keyring.

**Dependencies:** Phase 5 (keyring + Anthropic provider for onboarding), Phase 6a (Ollama onboarding path). Not dependent on Phase 6b, 7, or 8.
**Complexity:** Medium.
**Split rationale:** r1 collapsed onboarding + settings + theming + icon into Phase 9. I keep that but **add data/persistence polish** (export, corrupt-DB recovery, first-run recovery) because these are small-but-critical items that r1 deferred to "cross-cutting" and could easily be forgotten. Each is half an hour; together they're 4–5 hours and belong with settings where they're surfaced.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 10 — Auto-Update + a11y Audit + Error Catalogue Consolidation

**Goal:** Wire the auto-update paths (AppImage via Tauri updater; `.deb` via GitHub Releases JSON check), run a complete a11y audit, and consolidate the distributed error-code enum into a user-facing documented catalogue.

**Deliverables:**
- **Auto-update (r2 G3)**:
  - AppImage: `tauri-plugin-updater` configured with `pubkey` (Tauri updater key) and endpoint `https://github.com/Coreyalanschmidt-creator/biscuitcode/releases/latest.json`. Updater fetches signed `.tar.gz` patch.
  - `.deb`: Settings → About → "Check for updates" button. Backend fetches `https://api.github.com/repos/Coreyalanschmidt-creator/biscuitcode/releases/latest`. If newer, show modal with changelog + download link + instructions `sudo dpkg -i ~/Downloads/biscuitcode_X.Y.Z_amd64.deb`. **Do not auto-download or auto-install the `.deb`** (would require sudo; outside app scope).
  - HTTP scope `https://api.github.com/**` and `https://github.com/**` added to `http.json` capability.
  - "Check for updates on app start" setting (off by default).
- **a11y audit** (from r2 G2):
  - Keyboard-only navigation test: every panel reachable via Tab + `F6`; chat input, editor, file tree, terminal all operable from keyboard. Documented in `docs/RELEASE.md` as a checklist.
  - ARIA labels on every icon-only button (Activity Bar, chat send, tool-card expand/collapse, inline-edit popover buttons). Grep gate: `grep -rnE 'aria-label|aria-labelledby' src/components/ | wc -l` ≥ 40.
  - `aria-live="polite"` on chat messages container (verified already in Phase 5); `aria-live="assertive"` on tool-result completion.
  - Focus rings: every focused element has 2 px `--biscuit-500` outline. Grep gate: `grep -rnE 'focus-visible:ring' src/` ≥ 30.
  - `axe-core` run once in CI (integrated in Phase 11) — zero critical violations.
- **Error Catalogue Consolidation**:
  - `docs/ERRORS.md` enumerates every error code (E001–E018 per r2 G6), each with: code, class, cause, user message, recovery action, link to troubleshooting docs. Generated from the Rust enum at build time via a `build.rs` step — if an error code is added in code without a doc entry, build fails.
  - Toast component (scaffolded in Phase 1) connects to the generated catalogue.
  - Per-code forcing test: for each of the 18 codes, a manual forcing procedure in `docs/RELEASE.md` (e.g., "disconnect network to force E001; revoke key to force E002; stop `ollama serve` to force E011"). A VM smoke checklist in Phase 11 runs all 18.
- **Capability-upgrade handling (r2 New Risks #3)**: on app version bump where capability files changed (hash compared against last-seen version), show "Permissions updated; please reopen your workspace" toast on first launch after upgrade.

**Acceptance criteria:**
- [ ] AppImage: with updater configured pointing at a test GitHub Releases endpoint with a newer version, app shows update prompt on startup.
- [ ] `.deb`: Settings → About → Check for updates shows newer version + download link when a newer release is published.
- [ ] Auto-update setting off by default; flipping on enables background check on startup.
- [ ] Keyboard-only test: fresh install → navigate through onboarding → add Anthropic key → open folder → send chat message → all via keyboard, no mouse. Documented pass in `docs/RELEASE.md`.
- [ ] `axe-core` run against the running app yields zero critical violations.
- [ ] `docs/ERRORS.md` contains 18 entries (E001–E018); `build.rs` test asserts each Rust `Error` variant has a catalogue entry.
- [ ] Forcing each of the 18 errors shows the catalogued toast, never a raw stack trace. Verification checklist in `docs/RELEASE.md`.
- [ ] Simulate capability-file change across versions: with a test build, upgrading shows the "reopen workspace" toast on first launch.

**Dependencies:** Phase 9 (Settings, About page host the updater and toggle).
**Complexity:** Low (mostly wiring + documentation; no new subsystems).
**Split rationale:** This phase is net-new relative to r1. It groups three distinct small concerns (auto-update, a11y, error consolidation) that each would have been uncomfortably shoved into Phase 9 or Phase 10 (packaging) in r1. Keeping it as a thin dedicated phase gives focus without blocking packaging. Low complexity because no new subsystems — just wiring existing ones and documentation.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 11 — Packaging + CI + GPG Signing + Release Smoke Test

**Goal:** Build signed `biscuitcode_1.0.0_amd64.deb` + `BiscuitCode-1.0.0-x86_64.AppImage` in GitHub Actions on `ubuntu-24.04`, publish SHA256, and run the full release smoke test on a fresh Mint 22 XFCE VM.

**Deliverables:**
- `tauri.conf.json` `bundle` finalized: `targets: ["deb", "appimage"]`, `deb.depends: ["libwebkit2gtk-4.1-0", "libgtk-3-0"]`, `deb.recommends: ["gnome-keyring", "ollama"]`, `deb.suggests: ["rust-analyzer", "typescript-language-server", "pyright", "gopls", "clangd"]`, `deb.section: "devel"`.
- `.github/workflows/release.yml` on tag `v*`:
  - Runner `ubuntu-24.04`.
  - Linux deps install step (full r1 §12 list).
  - `pnpm install --frozen-lockfile`.
  - `tauri-apps/tauri-action@v0` with `args: "--target x86_64-unknown-linux-gnu"`, tag + release name.
  - GPG import via `GPG_PRIVATE_KEY` secret; `gpg --detach-sign --armor` both artifacts.
  - `sha256sum biscuitcode_*.deb BiscuitCode-*.AppImage > SHA256SUMS.txt`.
  - Upload `.deb`, `.AppImage`, both `.asc`, `SHA256SUMS.txt` to the release.
  - `linuxdeploy` retry wrapper on the AppImage step (r1 Risks #9).
- `.github/workflows/ci.yml` (scaffolded in Phase 1, fully populated here): lint (`cargo clippy -D warnings`, `pnpm lint`), typecheck (`tsc --noEmit`), tests (`cargo test --workspace`, `pnpm test`), security audits (`cargo audit`, `pnpm audit --prod`), `axe-core` run.
- AppImage `libfuse2t64` handling: README banner + AppImage wrapper script that checks for `libfuse2t64` and prompts install if missing.
- Release smoke-test checklist in `docs/RELEASE.md` — **pointer to Global Acceptance Criteria** rather than restatement. VM matrix explicit: three X11 sessions (22.0, 22.1, 22.2); **no Wayland-XFCE row** (XFCE 4.18 no Wayland — r2 C1); Cinnamon-Wayland on 22.2 as "best effort only."
- Three README screenshots using `BiscuitCode Warm`: main editor with chat, Agent Activity mid-run, preview split pane.
- `README.md` finalized: install instructions, screenshots, license, link to `docs/DEV-SETUP.md`, auto-update note.

**Acceptance criteria:**
- [ ] Pushing `v1.0.0` tag triggers CI; within ~15 min the release page has `.deb`, `.AppImage`, both `.asc`, and `SHA256SUMS.txt`.
- [ ] `gpg --verify biscuitcode_1.0.0_amd64.deb.asc biscuitcode_1.0.0_amd64.deb` returns "Good signature".
- [ ] `sha256sum -c SHA256SUMS.txt` passes.
- [ ] Fresh Mint 22 XFCE VM (X11 — 22.0, 22.1, 22.2): Global Acceptance Criteria checklist passes 100%.
- [ ] `time (biscuitcode & sleep 3 ; wmctrl -l | grep -q BiscuitCode)` — window within 2000ms.
- [ ] `apt remove biscuitcode` removes binary, desktop entry, all 7 icon sizes, `/usr/bin/biscuitcode` symlink.
- [ ] README screenshots contain no `lorem ipsum` / `TODO` / `placeholder`.
- [ ] `cargo audit` clean; `pnpm audit --prod` clean.
- [ ] Cinnamon-Wayland session on 22.2 (best effort): cold-launch succeeds; failure is logged but not release-blocking.
- [ ] `axe-core` CI job exits 0 (no critical a11y violations).

**Dependencies:** Phase 10.
**Complexity:** Medium.
**Split rationale:** Last phase, packaging. Same shape as r1's Phase 10 with one change: the Wayland row is dropped per r2 C1; Cinnamon-Wayland is a soft best-effort test rather than a gated row. This is the "prove it ships" phase.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

## Where This Plan Differs From R1

| Aspect | R1 choice | R2 choice | Rationale |
|---|---|---|---|
| Phase count | 11 (0–10) | 13 (0–11 with 6a/6b) | Phase 6 split isolates highest-risk work (writes + rewind) from read-only agent surface |
| Phase 6 scope | One phase: all tools, inline edit, rewind (High) | Split 6a (read-only tools + 3 providers + agent UI) + 6b (writes + rewind + inline edit) | r2 A4 flagged read-only-only alternative; I take the middle: read first, then writes |
| OpenAI + Ollama ordering | Phase 7 (after agent) | Phase 6a (before writes) | `ChatEvent` trait parity must be testable against 3 providers before tool surface depends on it |
| Inline edit UX | Unspecified | Zed-style split-diff via `createDiffEditor` in Phase 6b | r2 D7 notes split-diff is simpler; Monaco has the primitive built-in |
| Auto-update | Not addressed | Phase 10: Tauri updater for AppImage + GitHub Releases JSON for .deb | r2 G3 provided the architecture; v1 with no update path is embarrassing |
| Error taxonomy | Full catalogue in Phase 9 | Scaffolded Phase 1, codes adopted per feature phase, audit/consolidation in Phase 10 | Distributed ownership is cheaper than a Phase-9 all-at-once retrospective audit |
| i18n | Not mentioned | Phase 2 scaffolding (react-i18next, `t('key')` wrapping) | r2 G1: 1 hour now saves v1.1 sweep |
| a11y | Vision's High Contrast theme only | Phase 2 focus management + Phase 10 a11y audit (ARIA, keyboard-only, aria-live, axe-core) | r2 G2: reasonable posture cheap; full WCAG post-v1 |
| Conversation export | Not mentioned | Phase 9 Data section | r2 G7 user-owned-data principle; 30 min |
| Corrupt-DB recovery | Not mentioned | Phase 9 | r2 G4; trivial to add |
| First-run recovery | Not mentioned | Phase 9 idempotent onboarding | r2 G4 |
| Font-load canary | Not mentioned | Phase 9 Settings → About badge | r2 G9 |
| Multi-window | Not addressed | ADR: deliberate v1.1 defer | r2 G8: single-window precedent (Slack, Figma) |
| Virtualization library | Not specified | `react-virtuoso` from Phase 5 (chat) and reused in Phase 6a (Agent Activity) | r2 D8; front-loading means shared abstraction |
| Prompt caching | Not mentioned | Phase 5 Anthropic impl with `cache_control: ephemeral` | r2 New Risks #1: 5x cost savings on long conversations |
| Secret Service detection | `keyring::get` probe | `busctl list --user` read-only probe | r2 D6: avoids inadvertent daemon activation |
| OpenAI API flavor | Unspecified | Chat Completions only; Responses API is v1.1 | r2 D4: one decoder; reasoning-model TTFT exempted |
| Reasoning-mode TTFT | Single 500ms gate for all | Exempted for `gpt-5.4-pro`; "Thinking..." UI state | r2 New Risks #2 |
| Ollama malformed tool-call | Not mentioned | Phase 6a executor regex-extracts `<tool_call>` XML; invalid JSON → retry loop | r2 D5: common failure mode for Gemma 3 base |
| LSP split | Bundled with Git + Preview (Phase 8, High) | Own phase (Phase 8) separate from Git + Preview (Phase 7) | Different risk profile; Medium each beats a bundled High |
| Git + Preview | With LSP in Phase 8 (High) | Phase 7 (Medium) | Keeps LSP isolated; makes both phases reviewable |
| Wayland-XFCE smoke | Required row in Phase 10 | Dropped; Cinnamon-Wayland is best-effort only | r2 C1: XFCE 4.18 has no Wayland on any Mint 22 release |
| Stronghold plugin | Not mentioned | Explicit ADR warning: deprecated, will be removed in Tauri v3 | r2 A7 + New Risks #5: prevents future-maintainer rabbit hole |
| Capability upgrade handling | Not mentioned | Phase 10 "reopen workspace" toast on capability-file hash change | r2 New Risks #3 |
| Monaco TS worker + LSP conflict | Not mentioned | Phase 8 silences TS worker on LSP connect | r2 D2: prevents duplicate diagnostics |
| Font fallback chain | Not specified | Phase 1 explicit `'Inter', 'Ubuntu', sans-serif` (not `system-ui`) | r2 G9: vision forbids system-ui; Ubuntu is named-system fallback |
| Monaco language workers | Subset at boot | `languageWorkers: []` boot-empty; on-demand registration | r2 D2: 30–40% cold-bundle savings |

## Global Acceptance Criteria

Same vision quality bar as r1, translated into testable bullets. Some items carried over verbatim; many new items reflect r2 additions.

- [ ] `sudo dpkg -i biscuitcode_1.0.0_amd64.deb` installs clean on fresh Mint 22 XFCE (22.0, 22.1, 22.2) VMs; `sudo apt remove biscuitcode` removes everything installed.
- [ ] Cold-launch budget: `time (biscuitcode & sleep 3 ; wmctrl -l | grep -q BiscuitCode)` — window present within 2000ms on i5-8xxx / 8 GB.
- [ ] No console errors in devtools or Rust logs during a 5-minute normal session. `journalctl --user -t biscuitcode --since '5m ago' | grep -iE 'error|panic' | wc -l` returns `0`.
- [ ] All keyboard shortcuts in the vision's table work as specified (manual checklist in `docs/RELEASE.md`).
- [ ] `grep -rnE 'lorem|TODO|FIXME|placeholder|XXX' src/ src-tauri/src/` returns zero user-visible hits.
- [ ] Typography audit: `grep -rn 'system-ui' src/` returns zero hits in primary chrome (monospace fallbacks in code-editor surfaces are exempt).
- [ ] Dark theme uses Cocoa scale: `grep -rn '#000000\|#fff\b\|#ffffff' src/theme/` returns zero hits.
- [ ] Every one of the 18 error codes in `docs/ERRORS.md` is reachable and each forcing procedure shows the catalogued toast (not a raw stack).
- [ ] First-token latency on Claude streaming (non-reasoning): p50 under 500ms, p95 under 1200ms, over 20 prompts on warm connection.
- [ ] Reasoning models (`gpt-5.4-pro`): "Thinking..." state visible within 1.5 s; first real token may arrive anywhere from 3 s to 30 s; this exempts TTFT gate.
- [ ] Tool-card render latency: `tool_card_visible_<id> - tool_call_start_<id>` under 250 ms for each of 3 tools in an e2e test.
- [ ] `cargo audit` and `pnpm audit --prod` return zero critical vulnerabilities.
- [ ] `desktop-file-validate packaging/deb/biscuitcode.desktop` exits 0.
- [ ] All dependencies MIT / Apache-2.0 / BSD — `cargo-license` + `license-checker-rseidelsohn` clean.
- [ ] Icon legible at 16x16 in XFCE system tray (CI pixel check + manual confirm).
- [ ] `cargo test --workspace` and `pnpm test` both exit `0`.
- [ ] **axe-core** CI run: zero critical a11y violations.
- [ ] **Keyboard-only navigation**: fresh install → onboarding → add key → open folder → send chat → via keyboard alone.
- [ ] **Capability file forward-compat**: upgrading from vN to vN+1 where capabilities changed shows "reopen workspace" toast on first launch (simulated with staged build).
- [ ] **Secret Service pre-flight**: on VM without `gnome-keyring`, probe via `busctl list --user` returns empty match; app surfaces install prompt within 1 s of launch; no keyring API call was made.
- [ ] **Font fallback active**: if `Inter-Regular.woff2` is missing from the bundle, `document.fonts.check('14px Inter')` returns false, Settings → About shows "Font fallback active," and UI remains readable.
- [ ] Auto-update: AppImage users see "Update available" on newer release; .deb users see Settings → About → Check for updates button which correctly shows newer version.
- [ ] Conversation export: JSON export file validates against schema; per-conversation Markdown export renders readably.
- [ ] Corrupt DB scenario: with deliberately corrupted `conversations.db`, launch rename-recovers to `conversations.db.corrupt.<ts>` and creates fresh DB; app remains usable.
- [ ] README screenshots look like a real product, not a prototype (manual eye-check).
- [ ] Cinnamon-Wayland on Mint 22.2 (best-effort): cold-launch succeeds; any single-item failure is logged but not release-blocking. **XFCE-Wayland is not tested** per r2 C1.

## Open Questions

1. **Telemetry backend.** v1 ships the toggle + keyring storage, no wire implementation. Is Sentry acceptable in v1.1, or must the endpoint be self-hosted? Recommendation: v1 toggle-only; v1.1 decides transport based on user scale.
2. **AppImage `libfuse2t64` wrapper.** r1 Q2 — do we ship a wrapper script (`.AppImage.sh`) or README-only documentation, or both? Default: both (wrapper auto-prompts install on missing dep).
3. **Icon Concept D spike.** Vision allows D if it renders better at 16×16. Plan ships A and gates on the 16×16 pixel check in Phase 9. Should Phase 9 include a 2-hour A/B spike? Default: skip unless Concept A's 16×16 check fails.
4. **Arm64 build.** `ubuntu-24.04-arm` runners exist. v1 goal or v1.1 defer? Default: defer.
5. **Debian repo (`apt.biscuitcode.io`).** Shipping signed `.deb` via GitHub Releases in v1. Repo hosting is v1.1+ pending adoption metrics. Default: defer confirmed.
6. **LSP install auto-run.** Vision + research agree: no auto-run, copy-to-clipboard only. Confirmed for v1? Default: confirmed.
7. **Preview notebook execution.** Read-only v1. Placeholder "Run all cells" button in v1 (disabled)? Default: no hint; render-only with no run controls.
8. **DB growth.** `content_json` stores image base64 if vision models are used. Cap or lazy blob table? Default: defer; surface "Clear old conversations" in Phase 9 Data section.
9. **Chat mention resolution.** Substring or semantic (LSP-symbol)? Default: substring in v1; semantic in v1.1.
10. **Split-editor behavior.** Phase 3 ships `Ctrl+\` as a horizontal split. Does the plan also need vertical split (`Ctrl+Shift+\`)? Default: horizontal only for v1; vertical in v1.1.
11. **Ollama malformed tool-call retry cap.** When executor detects `<tool_call>` XML and the model keeps emitting malformed calls, how many retries before giving up? Default: 3; then surface `E015 ToolArgsInvalid` to user.
12. **Workspace trust UI granularity.** Boolean trust toggle (match Cursor/Zed) or per-tool allow-lists (finer-grained)? Default: boolean for v1; finer in v1.1.
13. **Update-check frequency.** When auto-update-on-startup is on, how often does the app check? On every launch? Weekly? Default: on every launch, with a 1-minute response timeout.
14. **Reasoning-mode timeout.** If `gpt-5.4-pro` returns no token after 60 s, do we abort with a specific error? Default: yes; `E003 ProviderDown` with a note that reasoning runs may take longer and providing a "Wait longer" button.
15. **Non-blocking adjacent work noticed during planning (Law 3 surfacing):** none silently added. Candidates that would be separate additions if the reviewer wanted them: AI-generated git commit message, crash-reporter privacy-strip layer, `@clipboard` chat mention, Monaco extension API stub. All explicit v1.1+.
