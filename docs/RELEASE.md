# BiscuitCode — Release Smoke Test Checklist

> Phase 10 deliverable. Run this before tagging any `v*` release. The Global Acceptance Criteria in `docs/plan.md` is the source of truth — this file points at it and adds the human-judgment items that don't fit a CI gate (icon legibility on a real tray, screenshots, etc.).

## When to run

- Before every `v*` tag push (CI auto-runs the gates; this file covers the manual judgment).
- After any change to `tauri.conf.json`, `.deb` packaging, capability files, icon assets, or auto-update wiring (these can break in ways CI doesn't fully catch).

## Required environment

Three fresh VMs (or fresh user accounts on a clean machine), one each of:

- **Mint 22.0 (Ubuntu 24.04 base, kernel 6.8, XFCE 4.18)** — minimum supported
- **Mint 22.1 Xia (kernel 6.8, XFCE 4.18)** — primary target
- **Mint 22.2 (kernel 6.14 fresh-install OR 6.8 upgrade-from-22.1, XFCE 4.18)** — newest supported

Plus optional:
- **Mint 22.2 Cinnamon-Wayland session** — best-effort row, does not block release.

**Wayland-XFCE is NOT in the matrix.** XFCE 4.18 has no Wayland support; r2 verified Mint 22.2 does NOT ship XFCE 4.20.

## Phase 1 — install

For each VM in the matrix:

- [ ] Download `BiscuitCode_<VERSION>_amd64.deb` and `BiscuitCode_<VERSION>_amd64.deb.asc` from the release page.
- [ ] `gpg --verify BiscuitCode_<VERSION>_amd64.deb.asc BiscuitCode_<VERSION>_amd64.deb` — signature is "Good".
- [ ] `sha256sum -c SHA256SUMS.txt` (downloaded from the same release) — both .deb and .AppImage check out.
- [ ] Install via GDebi (double-click): GUI installer opens, dependencies install cleanly, no "broken dependencies" message.
- [ ] Alternatively: `sudo dpkg -i BiscuitCode_<VERSION>_amd64.deb` succeeds; if any deps are missing `sudo apt -f install` resolves them.
- [ ] Verify install: `dpkg -s biscuit-code | grep -F "Version: <VERSION>"` returns one line.
- [ ] **Whisker menu shows BiscuitCode under Development** with the BiscuitCode icon (NOT a generic gear or question mark).

## Phase 2 — first launch + onboarding

- [ ] Launch from the Whisker menu. Window opens within 2s on test hardware.
- [ ] **Onboarding screen 1** (Welcome): logo + tagline + Next button rendered correctly.
- [ ] **Onboarding screen 2** (Pick models): all three provider cards (Anthropic, OpenAI, Ollama). On a VM with no `gnome-keyring-daemon`, this step shows the install command (catalogue `E001`) and blocks Next.
- [ ] Set an Anthropic API key. Status badge goes green within 3s of Save.
- [ ] **Onboarding screen 3** (Open a folder): file picker works; selecting a folder advances to main UI within 1s.
- [ ] First-message latency: typing `say hi in three words` and pressing Send → first token visible within 500ms (p50; one-shot is good enough for manual smoke).

## Phase 3 — core features

- [ ] **Editor**: open a TypeScript file from the file tree → syntax highlights with JetBrains Mono; ligatures render; multi-cursor (Alt+Click) works; minimap visible on the right edge.
- [ ] **Terminal** (`Ctrl+\``): opens; `echo $SHELL` returns user's shell; clicking `src/main.rs:12` in terminal output opens the editor at that line.
- [ ] **Find in files** (`Ctrl+Shift+F`): searching "TODO" returns results in under 2s on a 1k-file workspace.
- [ ] **Chat** with `claude-opus-4-7`: send a message; tokens stream live; conversation persists after restart.
- [ ] **Agent mode**: prompt `list every file in src/ that contains TODO and summarize` → Agent Activity shows tool cards (search_code, then read_file each) appearing within 250ms of the model's `tool_call_start` event.
- [ ] **Inline edit** (`Ctrl+K Ctrl+I`): select a function, describe a refactor, accept the diff in the split-diff editor.
- [ ] **Rewind**: edit a file via agent, then click rewind on the assistant message → file restored byte-identical (`sha256sum` matches pre-edit state).
- [ ] **Git panel**: stage a hunk, commit with a message; `git log -1` shows the commit.
- [ ] **LSP**: open a Rust file → rust-analyzer starts; hover shows type; go-to-definition jumps.
- [ ] **Preview pane**: open `README.md` → preview shows rendered markdown side-by-side.
- [ ] **Theme switching**: Settings → Appearance → switch to Cream → light theme persists across restart.

## Phase 4 — error paths

For each catalogue code in `docs/ERROR-CATALOGUE.md`, force the failure and verify the catalogued toast renders (NOT a stack trace). The CI test `tests/error-catalogue.spec.ts` covers most automatically; this list covers the ones that need human VM judgment:

- [ ] **`E001 KeyringMissing`**: `pkill gnome-keyring-daemon` then try to add an API key — install command surfaces.
- [ ] **`E007 GemmaVersionFallback`**: install Ollama < 0.20.0 OR mock the version check — toast appears with upgrade command.
- [ ] **`E013 LspServerMissing`**: `apt remove clangd` then open a `.c` file — toast with `sudo apt install clangd` copy button.

## Phase 5 — auto-update

For the AppImage path:

- [ ] Run an older AppImage (e.g., v0.9.0) with a published v1.0.0 manifest available — update prompt appears with changelog excerpt; accept downloads + replaces; relaunch shows v1.0.0.

For the `.deb` path:

- [ ] On a v0.9.0 install, click Settings → About → "Check for updates" — modal appears with v1.0.0 changelog and a "Download .deb" button that opens the release page.
- [ ] No path attempts `sudo` or auto-installs the `.deb`.

## Phase 6 — uninstall

- [ ] `sudo apt remove biscuit-code` removes the binary, the `.desktop` file, all 7 icon sizes (`/usr/share/icons/hicolor/{16,32,48,64,128,256,512}x{same}/apps/biscuitcode.png`), and `/usr/bin/biscuitcode`.
- [ ] Whisker menu no longer shows the entry.
- [ ] User config under `~/.config/biscuitcode/`, `~/.local/share/biscuitcode/`, and `~/.cache/biscuitcode/` is preserved (apt remove is not purge).
- [ ] `sudo apt purge biscuit-code` additionally removes nothing user-data-related (we DON'T delete user data on purge — confirm via `ls ~/.config/biscuitcode/`).

## Phase 7 — visual polish

These judgment calls don't fit CI:

- [ ] **16x16 icon legibility** in the XFCE system tray (if we add a tray icon — currently not in v1, but the same `.png` is used for the Whisker menu entry which IS visible at 16-22px). The `>_` glyph reads as two distinct shapes.
- [ ] **README screenshots** show rendered, real-data UI — no `lorem ipsum`, no `TODO`, no debug overlays, no missing icons.
- [ ] **Dark theme actually dark**: no element renders in pure `#000` or pure `#fff`. Eyeball check on every panel.
- [ ] **Typography**: every primary-chrome element uses Inter (NOT system-ui). Open devtools, inspect a button, computed `font-family` starts with `Inter`.
- [ ] **No console errors** in devtools after 5 minutes of normal use (open folder, edit file, chat, run agent tool, commit via git panel).

## Phase 8 — Cinnamon-Wayland (best-effort, does NOT block release)

On Mint 22.2 with a Cinnamon-Wayland session:

- [ ] Cold-launch succeeds (window appears within 3s — Wayland adds startup overhead).
- [ ] Clipboard copy/paste in terminal works.
- [ ] Drag-file-into-chat works.
- [ ] Window decorations render (no missing titlebar).

Failures here are logged in the release notes ("Wayland-Cinnamon: <known issue>") but do not block tagging the release.

## Sign-off

- [ ] All Phase 1-7 boxes green on all three VMs (22.0, 22.1, 22.2 X11).
- [ ] CI workflow green (lint, typecheck, test, audit, license-scan all passing on the release commit).
- [ ] Release notes drafted with: highlights, breaking changes (if any), known issues, contributor list.
- [ ] GPG signatures published.
- [ ] SHA256SUMS published.
- [ ] `latest.json` for the Tauri AppImage updater published.

If any of the above is not green, **do not tag the release**. Open issues for the failures, fix, re-run.

## Known limitations for v1.0

These are documented deferrals — do NOT block the release on them, but DO include them in the release notes:

- **LSP hover / go-to-definition not implemented** (Open Question #19). The `monaco-languageclient` adapter was evaluated during Phase 9 and deferred: it pulls in the full VS Code extension host (~20 `@codingame/monaco-vscode-api` packages) with known Vite ESM issues. The LSP backend (`biscuitcode-lsp`) is wired for stdio transport but the frontend adapter is absent. Smoke-test item "Phase 3 — LSP" row will show rust-analyzer starting (backend process launches) but hover and go-to-definition will not respond. Mark that row as **known-fail / deferred** in the release notes; it does not block release.

- **Wayland-XFCE not in smoke matrix.** XFCE 4.18 has no Wayland support; the Wayland-XFCE row is unreachable on Mint 22. Cinnamon-Wayland is best-effort only.

## After release

- [ ] Push the release announcement to wherever (TBD — Open Question Q1 or future).
- [ ] Bump `version` in `package.json` and `src-tauri/Cargo.toml` to the next minor or patch (depending on the next change set).
- [ ] Open the milestone for the next version.
