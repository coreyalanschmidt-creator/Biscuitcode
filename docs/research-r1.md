# BiscuitCode — Research Dossier

> Feeds `docs/plan.md`. Authored by the research stage of the C.Alan pipeline. Cites primary sources where possible. Generated 2026-04-18.

---

## Topic & Scope

**Plain-English restatement.** Build a VS Code-class AI coding environment, called **BiscuitCode**, targeting Linux Mint 22 XFCE (Xia) as the primary install target. Stack is locked: **Tauri 2.x + Rust + React 18 + TypeScript 5 + Vite + Tailwind CSS 3**. Editor is **Monaco** via `@monaco-editor/react`. Terminal is **xterm.js** over a **Rust `portable-pty`** backend. Secrets live in **libsecret via the Rust `keyring` crate**. Providers are **Anthropic Messages API**, **OpenAI**, and **Ollama**. Distribution is **.deb** (primary) and **AppImage** (secondary), built in **GitHub Actions**, signed, checksummed.

**In scope for this research.** The 15 domains listed in the brief: Tauri state, Mint/XFCE targeting, WSL2 dev flow, Monaco, xterm.js+PTY, keyring, provider APIs, agent loop UX, LSP client, SQLite persistence, GTK theme detection, packaging/CI, security posture, performance targets, namespace collisions.

**Out of scope.** Icon design critique beyond collision checks; marketing copy; the vision's brand-token hex values (locked); v1.1 features (VS Code theme import, notebook execution). No implementation code or `docs/plan.md` content in this document.

---

## Assumptions

Flagged [LOW] / [MED] / [HIGH] by confidence, where HIGH = I'm sure and LOW = the maintainer should sanity-check before planning.

1. **[HIGH]** "Linux Mint 22" in the vision means the 22.x series (22, 22.1 "Xia", 22.2 "Zara" per release notes), all of which share the Ubuntu 24.04 "noble" base and kernel 6.8/6.14. The vision explicitly says Xia, so I treat **22.1 Xia** as the canonical target and 22.0/22.2 as in-family compatibility goals. [1][2]
2. **[HIGH]** "Tauri 2.x" means the stable 2.x line. As of April 2026 the current stable is `tauri` **v2.10.3**, `tauri-cli` v2.10.1, `@tauri-apps/api` v2.10.1, `@tauri-apps/cli` v2.10.1. [3]
3. **[HIGH]** Target webview on Linux is **WebKitGTK 4.1** (the `libwebkit2gtk-4.1-0` package), which is what Ubuntu 24.04 ships. WebKit2GTK 4.0 is dropped from noble, so Tauri v1 is not viable for this target. [4][5]
4. **[HIGH]** XFCE on Mint 22 is still **X11 primary**; Wayland is explicitly marked experimental only on the **Cinnamon** edition, not XFCE, and XFCE 4.20 Wayland work is far behind. Plan for X11 first, design nothing that breaks if Wayland happens later. [6][7]
5. **[HIGH]** The vision's Claude model IDs were current at drafting but are slightly stale. Per Anthropic docs (April 2026): `claude-opus-4-7`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001` (alias `claude-haiku-4-5`) are current; **`claude-opus-4-6` is now a *legacy* model** (still on the API, but not the default choice). [8]
6. **[MED]** OpenAI's `gpt-4o` (vision default) **was retired on April 3, 2026**. Current frontier is `gpt-5.4` / `gpt-5.4-pro` / `gpt-5.4-mini` / `gpt-5.4-nano`, with `gpt-5.2 Thinking` in legacy until June 5, 2026. The planner should re-default OpenAI to `gpt-5.4` or `gpt-5.4-mini`. [9][10]
7. **[HIGH]** "Gemma2" in the vision should be read as "the current Gemma family"; current Ollama library has **Gemma 3** (multimodal, 128K context, sizes 270M / 1B / 4B / 12B / 27B) and a community `gemma3-tools` variant that exposes tool calling; **Gemma 4** was released 2026-04-02 with native tool support. [11][12]
8. **[MED]** Development environment is **Windows with WSL2 Ubuntu 24.04** (inferred from the cwd `C:\Users\super\...` in the brief and the Mint target). Flagging because cross-compiling Windows→Linux from WSL2 is the *easy* direction; cross-compiling Linux→Windows from WSL2 is experimental. For BiscuitCode's *Linux-first* v1, this is fine.
9. **[MED]** Keyring crate version: **`keyring` 3.6.x** (current). The crate requires feature-flag selection — default features are off. [13]
10. **[LOW]** "Secret Service available out-of-the-box on Mint 22 XFCE" — the Mint XFCE edition does install `gnome-keyring` and `libsecret` by default (common across Mint editions since 20), but this is worth verifying on a clean install before calling the keyring path the happy path. A fallback must exist for the session where no keyring daemon is running.
11. **[HIGH]** The app is MIT-licensed and must ship dependencies compatible with MIT/Apache-2.0/BSD only. GTK/WebKitGTK (LGPL) is dynamically linked on Linux — OK.

---

## Background & Landscape

### 1. Tauri 2.x current state (early 2026)

Tauri 2.0 stable shipped in late 2024. In April 2026 the framework is at **v2.10.x**, has added stable iOS/Android support, and is used in production by Hoppscotch, Spacedrive, AppFlowy, Padloc, and Firezone. [3][14][15]

Architecturally: the frontend runs in the OS-native webview (WebView2 on Windows, WKWebView on macOS, **WebKitGTK 4.1 on Linux**). Backend is Rust. IPC crosses via a postMessage-style channel that in v2 supports raw bytes and custom serialization (big win over v1's JSON-only path). [4][16]

**The single biggest v1 → v2 change** is the replacement of the global `tauri > allowlist` config block with a **per-file Access Control List (ACL)** system: capability files under `src-tauri/capabilities/*.json`, permissions either bundled by plugin or defined inline, scopes as first-class concepts on fs/shell/http. `migrate` CLI auto-generates a `migrated.json` capability from a v1 allowlist. [17][18]

**Sidecars in v2.** Declared in `tauri.conf.json` under `bundle.externalBin` as `binaries/<name>` (Tauri appends `-$TARGET_TRIPLE`). Spawning from Rust gives you an async channel of `CommandEvent::Stdout/Stderr/Terminated` events; spawning from the frontend requires `shell:allow-execute` or `shell:allow-spawn`. Communication is just another local process — HTTP or stdio. [19][20]

**Plugin ecosystem** (official, v2): `plugin-fs`, `plugin-shell`, `plugin-sql` (sqlx-based), `plugin-dialog`, `plugin-http`, `plugin-clipboard-manager`, `plugin-os`, `plugin-process`, `plugin-store`, `plugin-updater`, `plugin-notification`, `plugin-deep-link`, `plugin-global-shortcut`, `plugin-window-state`. Community: `tauri-plugin-pty` for terminals. [21][22]

**Known gotchas** (vs. v1):
- **Breaking config rename**: `tauri > allowlist` gone; `tauri > bundle` → `bundle`; `tauri > pattern` → `app.security.pattern`. [17]
- **WebKit bump**: v2 links against `libwebkit2gtk-4.1`; v1 linked against 4.0. v1 apps literally will not install on Ubuntu 24+. [4][5]
- **linuxdeploy + AppImage under Ubuntu 24.04**: AppImages still need **FUSE 2**, but Ubuntu 24.04 renamed `libfuse2` to `libfuse2t64`. Users must install it manually or the AppImage refuses to launch. This is a user-visible UX problem unless we ship an installer script or document it. [23][24]

### 2. Linux Mint 22 XFCE targeting

Mint 22 is Ubuntu 24.04 "noble" based, kernel **6.8** on 22.0/22.1, kernel **6.14** on 22.2. Package base matches noble. [1][2]

**Desktop environment details for XFCE edition:**
- **XFCE 4.18** on Mint 22 / 22.1; **XFCE 4.20** available in 22.2. [2]
- **GTK 3 dominant**. Mint has actively *downgraded* apps from GTK 4/libadwaita back to GTK 3 (Celluloid, Calculator, File Roller, etc.) to preserve theme coherence, which tells us the platform is a GTK 3 deployment target. WebKitGTK 4.1 itself is GTK 3-based. [1]
- **X11 only in practice**; Wayland on XFCE is tech-preview and not offered at login on Mint 22 as of 22.2. Cinnamon has experimental Wayland; XFCE does not. [6][7]
- **Secret Service daemon**: `gnome-keyring-daemon` is installed by default on Mint editions and is auto-started by the session; on XFCE it typically works out-of-the-box but may not start in some configurations (historically an issue when running gajim/etc under XFCE). [25][26]

**Packaging required artifacts on freedesktop:**
- `.desktop` file at `/usr/share/applications/biscuitcode.desktop` (Name, Comment, Exec, Icon, Categories=Development;IDE;, MimeType list for associated files)
- Icons at `/usr/share/icons/hicolor/{16,32,48,64,128,256,512}x{same}/apps/biscuitcode.png`
- Binary / opt install at `/opt/biscuitcode/` with a symlink at `/usr/bin/biscuitcode`
- **`postinst` must run**: `update-desktop-database /usr/share/applications`, `gtk-update-icon-cache -q -t /usr/share/icons/hicolor`, `update-mime-database /usr/share/mime` (if custom MIME types), `desktop-file-validate` for validation during build. [27]
- **`postrm` must reverse**: remove symlinks, re-run `update-desktop-database` and `gtk-update-icon-cache`.
- Debian control fields required: `Package`, `Version`, `Architecture`, `Maintainer`, `Depends: libwebkit2gtk-4.1-0, libgtk-3-0, libappindicator3-1` (the last only if tray), `Section: devel` or `editors`, `Priority: optional`, `Description`. Tauri's bundler generates this but you may want to add custom fields like `Recommends: ollama, gnome-keyring`. [4]

### 3. Cross-platform dev from Windows (WSL2)

For a Linux-target Tauri app, the recommended dev flow from Windows is **WSL2 Ubuntu 24.04** with WSLg. [28][29]

**Prerequisite apt packages on Ubuntu 24.04:**
```
pkg-config libdbus-1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf libfuse2t64 file build-essential curl
```
(The `-4.1` suffixes differ from most older tutorials — check everything twice.) [4][30]

**WSLg gotchas:**
- GUI works; DISPLAY is auto-set by WSLg. Avoid 3rd-party X servers (VcXsrv, Xming). [29]
- **Inotify watchers on `/mnt/c` are broken / slow** — place the project in the Linux home (`~/projects/biscuitcode`) not `/mnt/c/...` for fast HMR and correct file-watch behavior. This is a known, persistent WSL limitation.
- GPU acceleration in WebKitGTK works via WSLg's GPU passthrough on recent drivers; without it WebKit falls back to software rendering (works, just slower).
- `cargo tauri dev` launches a live-reloading Vite dev server + webview. The webview opens in a WSLg window on the Windows desktop.

**Cross-compilation constraint.** WSL2-from-Ubuntu → Linux artifacts (.deb, .AppImage): natural and supported. WSL2 → Windows .msi: experimental via `cargo-xwin`; not the recommended path for production. v1 plans Linux-first and may produce Windows artifacts later on native Windows runners (or GH Actions). [31]

### 4. Monaco Editor integration

Monaco is Microsoft's VS Code editor core shipped as a browser library. **`@monaco-editor/react`** (maintained by Suren Atoyan) is the canonical React wrapper; current as of April 2026. It lazy-loads Monaco from CDN by default, which is **not** what a Tauri offline-capable app wants — you pin the loader to a local Vite-bundled copy. [32][33]

**Worker loading in Vite.** Monaco has separate web workers for editor, JSON, CSS, HTML, and TypeScript. Under Vite the two clean options are:
1. `vite-plugin-monaco-editor` by vdesjs — esbuild-bundles workers into `node_modules/.monaco` and serves via middleware. Configure a subset of language workers; the defaults `['editorWorkerService', 'css', 'html', 'json', 'typescript']` are a good minimum. [34]
2. Hand-rolled `?worker` imports — `import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'` plus `self.MonacoEnvironment = { getWorker(_, label) { /* dispatch by label */ } }`. More control, more boilerplate. [35]

For Tauri specifically: workers must be same-origin; Vite serves them locally, which works. Under `tauri://localhost` (production), workers resolve relative to the app bundle. There is no CSP issue unless you tighten CSP manually.

**Bundle size reality.** Full Monaco is **~15 MB unminified**; ~5 MB gzipped after tree-shaking unused languages. For a Guitar-Hero-scale app that would be fine; for BiscuitCode it's the dominant frontend payload. Aggressive language trimming + dynamic import to defer Monaco until the editor pane is shown brings cold-launch well under budget. [36][37]

**Keybinding integration with host app.** Monaco exposes `editor.addAction({ id, label, keybindings, run })`. Keybindings in Monaco live in a separate keybinding service from the host app's global shortcuts, so **Ctrl+K Ctrl+I** (inline edit) must be registered in *both*:
- Monaco (with `monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK` chord logic) so it fires with an editor in focus.
- The Tauri/React host (via `plugin-global-shortcut` or a document-level listener) so it fires from outside the editor.

Monaco's F1 command palette is a separate surface; we want our own Ctrl+Shift+P that covers non-editor commands too, so Monaco's F1 can either stay (as an editor-scoped palette) or be intercepted. [38]

**Multi-tab architecture.** Standard pattern: one `monaco.editor.ITextModel` per open file, a single editor instance whose `setModel()` swaps as tabs change. Keeps memory flat, preserves undo history per model, and is what VS Code does. For diff view, `monaco.editor.createDiffEditor` is a sibling component; it accepts two models.

### 5. xterm.js + portable-pty

This is a well-trodden path with several reference implementations (tauri-terminal, Terminon, rustpty, tauri-plugin-pty). [39][40][41]

**Flow:**
1. Frontend creates `xterm.js` terminal, calls Tauri command `terminal_open({ shell, cwd, rows, cols })`.
2. Rust `portable-pty` spawns the shell (`portable_pty::native_pty_system().openpty(...)`), returns a session ID.
3. Two Tokio tasks: one reads from PTY master → emits `terminal_data_<id>` Tauri events; one consumes frontend input events (`terminal_input`) → writes to PTY master.
4. On resize: frontend emits `terminal_resize`, backend calls `master.resize(PtySize { ... })`.
5. On tab close: frontend emits `terminal_close`, backend kills child and drops the pty.

**Current xterm.js ecosystem** (as of April 2026; **note the new `@xterm/*` scope — old `xterm-addon-*` packages are deprecated**):
- Core: `@xterm/xterm` v5.x.
- `@xterm/addon-fit` — auto-resize to container (essential).
- `@xterm/addon-web-links` v0.12.0 — clickable URLs. [42]
- `@xterm/addon-search` — find in terminal.
- `@xterm/addon-webgl` v0.19.0 — GPU rendering (best perf; requires WebGL2). [43]
- `@xterm/addon-canvas` v0.7.0 — fallback. [44]

**Clickable file paths** (not URLs) are *not* built-in; custom matcher via `registerLinkProvider` with a regex over typical Linux paths + stat check before activation. Same hook handles terminal output like `src/foo.rs:12:4` → open file at line.

**Shell detection:** read `$SHELL`, fall back to `/bin/bash`. Honor `getent passwd $UID` for the user's configured login shell. Don't hardcode; on Mint users do run zsh/fish.

**Multi-tab cleanup**: critical to drop the PtyMaster/PtySlave on tab close — otherwise file descriptors leak and processes linger.

### 6. Rust `keyring` crate + libsecret

Current crate: **`keyring` 3.6.3**. No default features — must explicitly enable platform + async runtime + crypto backend. [13]

**Recommended feature set for Linux-primary:**
```
keyring = { version = "3", features = ["linux-native-async-persistent", "async-secret-service", "crypto-rust", "tokio"] }
```
This gives:
- Secret Service over DBus (the standard, works with gnome-keyring, kwallet5/kwallet6).
- Async API (matches Tauri/Tokio).
- Pure-Rust crypto (no OpenSSL link).
- `keyutils` fallback persistent layer for headless/session-broken cases.

**What happens on a barebones XFCE session with no keyring daemon?** The Secret Service DBus call fails. The `keyutils` layer is in-memory only (kernel session keyring) — good for the current session but lost at logout. Any production design must:
1. Detect Secret Service availability at startup (`org.freedesktop.secrets` on DBus).
2. If missing, **prompt the user** to install `gnome-keyring` + `libsecret` (`sudo apt install gnome-keyring libsecret-1-0 libsecret-tools`) rather than silently storing secrets in an inferior location.
3. Absolutely never fall back to plaintext disk storage. The vision forbids it.

**Mint 22 XFCE factory default**: `gnome-keyring` is installed; the daemon starts via PAM on desktop login. This means the happy path works. Clean headless cases and weird `Exec=` launches that bypass PAM can still break. [25]

### 7. Model provider integration

**Anthropic Messages API** — `POST https://api.anthropic.com/v1/messages`. Headers: `x-api-key`, `anthropic-version: 2023-06-01`, optionally `anthropic-beta: prompt-caching-2024-07-31`. Set `"stream": true` for SSE.

SSE event types in order on a streamed response: `message_start` → for each content block `content_block_start` → N × `content_block_delta` → `content_block_stop` → `message_delta` (carries `stop_reason`) → `message_stop`. [44][45]

Delta types inside `content_block_delta`:
- `text_delta` — normal text tokens.
- `thinking_delta` — extended-thinking tokens (Sonnet 4.6 / Haiku 4.5 when enabled).
- `input_json_delta` — partial JSON string for a `tool_use` block. **The tool arguments only arrive across these deltas**; a `content_block_stop` is your cue that the JSON is complete.
- `signature_delta` — thinking signatures.

**Stop reasons:** `end_turn`, `max_tokens`, `stop_sequence`, `tool_use`, `pause_turn`.

**Tool use request shape**: tools are an array of `{ name, description, input_schema }` where `input_schema` is JSON Schema. When the model wants to call a tool, you see `content_block_start` with `{ type: "tool_use", id, name }`, then JSON input streamed across `input_json_delta`s.

**Tool result shape**: on the next request, user turn includes `{ type: "tool_result", tool_use_id, content }` where content is a string or structured blocks.

**Rate limits**: per-org, per-model, visible in response headers `anthropic-ratelimit-*`. 429 responses carry `retry-after`. Exponential backoff + jitter is standard.

**Current Claude models (April 2026)**:

| Model | API ID | Context | Max Out | Notes |
|-------|--------|---------|---------|-------|
| Claude Opus 4.7 | `claude-opus-4-7` | 1M | 128k | Frontier; adaptive thinking only (no extended-thinking toggle) |
| Claude Sonnet 4.6 | `claude-sonnet-4-6` | 1M | 64k | Balanced; extended thinking available |
| Claude Haiku 4.5 | `claude-haiku-4-5-20251001` (alias `claude-haiku-4-5`) | 200k | 64k | Fast; extended thinking available |
| Claude Opus 4.6 (legacy) | `claude-opus-4-6` | 1M | 128k | Still on API but not primary |

**Important Opus 4.7 gotcha**: setting `temperature`, `top_p`, or `top_k` to any non-default value **returns HTTP 400**. The migration-safe path is to omit these params entirely. The `ModelProvider` trait must allow per-provider sampling opts or omit them. [46]

---

**OpenAI Chat Completions API** — `POST https://api.openai.com/v1/chat/completions`. Headers: `Authorization: Bearer sk-...`.

Streaming deltas differ meaningfully from Anthropic:
- Each SSE `data:` line holds a `ChatCompletionChunk` with `choices[0].delta.content` (string, nullable) and `choices[0].delta.tool_calls` (array of partial `{index, id, type, function:{name, arguments}}` objects).
- The **tool_call arguments string is streamed concatenated across chunks**, so you accumulate `tool_calls[i].function.arguments` by index until `finish_reason === "tool_calls"`.
- `finish_reason` lives on the final chunk's `choices[0].finish_reason`.
- No `message_start`/`message_stop` framing; no content-block abstraction. [47][48]

**Current OpenAI models (April 2026)**: GPT-4o **retired April 3, 2026**. Current default frontier: `gpt-5.4` (with `reasoning.effort` ∈ {none, low, medium, high, xhigh}), `gpt-5.4-pro` (reasoning-only, medium/high/xhigh), `gpt-5.4-mini`, `gpt-5.4-nano`. `gpt-5.3 Instant` for everyday workhorse; `gpt-5.2 Thinking` in legacy until 2026-06-05. [9][10]

**Migration note for the vision**: `gpt-4o` in the vision must be replaced. Recommendation below.

---

**Ollama API** — `POST http://localhost:11434/api/chat`. No auth.

Request: `{ model, messages: [{role, content}], stream: true|false, tools?: [...], options?: {...}, keep_alive?: string }`.

Response stream format: **NDJSON (newline-delimited JSON)**, one object per line, *not* SSE:
```json
{"model":"gemma3:4b","created_at":"...","message":{"role":"assistant","content":"Hel"},"done":false}
{"model":"gemma3:4b","created_at":"...","message":{"role":"assistant","content":"lo"},"done":false}
{"model":"gemma3:4b","created_at":"...","message":{"role":"assistant","content":"","tool_calls":[{...}]},"done":false}
{"model":"...","done":true,"total_duration":123456,"prompt_eval_count":42,"eval_count":17}
```

**Tool calling in Ollama**: the `/api/chat` endpoint accepts a `tools` array in OpenAI-function-call format and, when a model supports it, returns `message.tool_calls: [{function: {name, arguments: {...}}}]`. Tool-calling support varies by model:

| Model | Tool calls | Notes |
|-------|-----------|-------|
| `gemma3:*` | Community variant `orieg/gemma3-tools` adds it | Base `gemma3` does *not* reliably tool-call [11][12] |
| `gemma4:*` (April 2026) | Native | Recommended default for local agents [12] |
| `qwen2.5-coder:7b` | Yes (native) | Best local coding model in 8GB RAM tier |
| `llama3.1:8b` | Yes (native) | Stable baseline |
| `mistral-nemo:12b` | Yes | Strong for tool use |
| `phi-4` | Yes | 14B, strong reasoning for size |

**Ollama install detection** (Linux):
- Probe: `curl -sSfm 1 http://localhost:11434/api/version` (fast timeout, 200 on running).
- If connection refused, try `which ollama` via the shell plugin to check binary presence.
- If absent, offer one-click install: `curl -fsSL https://ollama.com/install.sh | sh`. **Always show the full URL and command in a confirm dialog** before running — the user must consent to a root-level install script. [49]
- After install, `ollama serve` is registered as a systemd user service on recent installs.

**RAM-aware default**:
- `free -b` or `sysinfo` crate for total RAM.
- Rule of thumb: **~0.6 GB per billion parameters at q4_K_M**. Pull budget tables:
  - <6GB RAM → `gemma3:1b` or `qwen2.5-coder:1.5b`.
  - 6–12GB → `gemma3:4b` or `qwen2.5-coder:3b` (new defaults; "2b" ≈ 270M–4B tier).
  - 12–24GB → `qwen2.5-coder:7b`, `gemma3:12b`, `llama3.1:8b`.
  - ≥32GB → `qwen2.5-coder:32b`, `gemma3:27b`. [50]

**`ModelProvider` trait abstraction (sketch).** The common denominator is:

```rust
enum ChatEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart { id: String, name: String },
    ToolCallDelta { id: String, partial_args_json: String },
    ToolCallEnd { id: String },
    // ToolResult is produced by the executor, not the provider.
    Done { stop_reason: StopReason, usage: Usage },
    Error(Box<dyn std::error::Error + Send + Sync>),
}
```
Anthropic's content-block framing must be flattened into `TextDelta`/`ToolCallStart`/`ToolCallDelta`/`ToolCallEnd`. OpenAI's per-chunk delta-by-index tool accumulation must be de-multiplexed into the same events. Ollama's NDJSON must be buffered per line and its `tool_calls` (which arrive only on the final non-done chunk in many models) emitted as a single `ToolCallStart` + `ToolCallEnd` pair. Keep the trait minimal; put provider quirks in each impl.

### 8. Agent loop / tool execution

**ReAct-style loop**: the textbook agent pattern is "generate → parse tool call → execute → observe → repeat". Modern providers simplify this because the model emits structured tool calls natively; the parse step is JSON decoding of accumulated `tool_use`/`tool_calls` deltas.

Observed patterns in **Cursor**, **Zed Agent**, and **Claude Code Desktop**:
- **Streaming + interleaving**: text and tool calls render as they arrive, not at message completion. Chat shows text; tool calls render as cards in an Agent Activity pane (Zed, Cursor) or inline (Claude Code terminal). [51][52]
- **Interruption**: a single Esc/Cancel button pauses the loop at the next safe boundary (post-current-tool). Zed specifically introduced a "Restore Checkpoint" that appears after Cancel, letting the user rewind the last batch of changes. [51]
- **Rewind**: Claude Code exposes `/rewind` and double-Esc; Zed shows a checkpoint list; Cursor shows per-agent-action undo. All of these are UI veneers over **per-action snapshots of modified file content**. [53][54]
- **Confirmation gates**: Cursor and Zed distinguish *read* tools (no prompt), *write* tools (preview + accept/reject), and *shell* tools (explicit confirm unless workspace is trusted). Claude Code has a permission mode switch. Workspace-trust as a single global boolean is the simplest and adequate abstraction.

**Tool registry design.** A single Rust enum or trait-object registry mapping tool name → handler. Each handler declares `JSON Schema` input, typed output, side-effect class (`read`/`write`/`shell`), and a confirmation policy. The executor consults policy before invoking; writes return a diff preview to the UI, which streams Accept/Reject back as an event.

**Trust boundary = workspace root.** File ops scope to `$WORKSPACE/**` only, via Tauri fs-scope plus in-process path normalization (reject `..`, symlink-follow check). Opening outside requires user confirmation. This matches the vision's "workspace-scoped by default". [55]

### 9. LSP client

The Monaco-side library is **`monaco-languageclient`** (TypeFox), which adapts Monaco's language features API to any LSP server via pluggable `MessageTransports`. In a browser context you can't spawn stdio processes — so the host app must proxy. [56][57]

**Tauri IPC proxy pattern** (documented working approach per TypeFox Discussions [56]):
1. Frontend calls `lsp_start({ language, workspaceRoot })` → Rust spawns the language server (e.g. `rust-analyzer`, `typescript-language-server --stdio`) and returns a numeric `session_id`.
2. Rust uses two Tokio tasks:
   - stdout → emit `lsp-msg-in-{session_id}` event per JSON-RPC message.
   - Frontend → `invoke('lsp_write', {session_id, msg})` → write to stdin.
3. Frontend wires a custom `MessageTransports` pair that reads from Tauri events and writes via `invoke`.
4. On child exit, emit `lsp-close-{session_id}`; frontend tears down the client.

**Detecting installed servers:** attempt `which <binary>` (via an explicit, allowlisted shell command in Tauri capabilities). If missing, surface a dialog with the correct install command but **do not auto-run** — the vision explicitly prohibits arbitrary shell execution. Copy-to-clipboard + link to the server's install docs is the right UX. [58]

| Server | Binary | Install command |
|--------|--------|----------------|
| rust-analyzer | `rust-analyzer` | `rustup component add rust-analyzer` |
| typescript-language-server | `typescript-language-server` | `npm i -g typescript typescript-language-server` |
| pyright | `pyright-langserver` | `npm i -g pyright` (faster than pip on Linux) |
| gopls | `gopls` | `go install golang.org/x/tools/gopls@latest` |
| clangd | `clangd` | `sudo apt install clangd` |

Many LSPs support the "stderr to host" pattern for logging — keep that connected to the Agent Activity or Output panel.

### 10. SQLite persistence

Two viable crates: **`rusqlite`** (direct libsqlite3 FFI, sync API) and **`sqlx`** (async, compile-time-checked, multi-db). For a desktop Tauri app, `sqlx` + Tauri's **`plugin-sql`** is the documented path; migrations are idempotent numbered files loaded by the plugin at init. [59][60]

Alternative: **`tauri-plugin-rusqlite2`** (community fork of `plugin-sql` that uses `rusqlite`, SQLite-only, simpler transaction story). [61]

For our use case — single embedded SQLite, no networked DB, plenty of ad-hoc queries — **`rusqlite` directly** (no plugin) is the simplest adequate choice. No async contention because the DB is local and fast; one connection behind a `Mutex` plus a thread-pool for blocking calls is enough. Manual migration table (`PRAGMA user_version`) is ~20 lines.

**Conversation schema (sketch):**
```sql
-- Workspaces (one per opened folder)
CREATE TABLE workspaces (
  id INTEGER PRIMARY KEY,
  root_path TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL
);

-- Conversations, with forking via parent_message_id
CREATE TABLE conversations (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER REFERENCES workspaces(id),
  title TEXT,
  forked_from_message_id INTEGER REFERENCES messages(id), -- null = root
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  active_model TEXT -- e.g. "claude-opus-4-7"
);

-- Messages as a DAG within a conversation
CREATE TABLE messages (
  id INTEGER PRIMARY KEY,
  conversation_id INTEGER NOT NULL REFERENCES conversations(id),
  parent_id INTEGER REFERENCES messages(id), -- enables branching
  role TEXT NOT NULL, -- user|assistant|tool
  content_json TEXT NOT NULL, -- blocks: text, tool_use, tool_result, thinking
  model TEXT, -- provider-specific model id for assistant msgs
  created_at INTEGER NOT NULL
);

CREATE INDEX idx_messages_conv_parent ON messages(conversation_id, parent_id);
```

**Branching semantics:** editing a past user message inserts a new message with the same `parent_id` (its sibling), then a new conversation row is created with `forked_from_message_id` pointing at the fork point. Tree view walks up parent_id chains.

### 11. Theming / GTK detection

XFCE stores the GTK theme name in **xfconf** under channel `xsettings`, property `/Net/ThemeName`. Reading the active theme: `xfconf-query -c xsettings -p /Net/ThemeName`. On a mixed desktop (user ran `gsettings`), check `gsettings get org.gnome.desktop.interface gtk-theme` as a fallback. [62][63]

**Dark/light detection heuristic**: the convention is "-dark" suffix on the theme name. Mint-Y-Dark, Adwaita-dark, Arc-Dark, Mint-Xia-Dark are all dark. Regex: `-dark$` case-insensitive catches ~99% of themes; default to "light" on a miss. XFCE does not yet have a portable dark-mode signal like GNOME's `color-scheme` property, so this suffix check is the standard approach.

**Implementation:** a Rust command `detect_gtk_theme()` that shells out to `xfconf-query` (via `std::process::Command` — *not* through the Tauri shell plugin, because shelling out for internal diagnostics doesn't need capability checks). Return `{ theme_name, is_dark }`. Call at startup; store as a Zustand state; the theme switch offer on first run reads this.

### 12. Packaging / CI

**GitHub Actions matrix for Linux targets** (as of April 2026):
- `ubuntu-24.04` runners are GA; also `ubuntu-24.04-arm` for arm64. [64]
- Use `ubuntu-24.04` (not `-latest`, which drifts); Tauri bundler will link against the runner's libwebkit2gtk-4.1 which matches Mint 22.

**Required Linux deps step:**
```
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libgtk-3-dev patchelf libfuse2t64
```

**Build:** `tauri-apps/tauri-action@v0` with `tagName`, `releaseName`, and `args: "--target x86_64-unknown-linux-gnu"`. It produces `.deb` and `.AppImage` if Tauri config targets those. [65][66]

**AppImage specifics**: `tauri-bundler` runs `linuxdeploy`; on Ubuntu 24.04 in GH Actions you need `libfuse2t64` and sometimes container caps (`--cap-add=sys_admin --cap-add mknod --device=/dev/fuse`). Recent Tauri issues report flakes at the linuxdeploy stage — keep a retry in CI. [23][67]

**GPG signing**: GitHub releases support GPG-signed artifacts. Import the key into GH Actions via a secret (`GPG_PRIVATE_KEY`), run `gpg --import`, then `gpg --detach-sign --armor biscuitcode_*.deb`. Publish `.deb.asc` + `.AppImage.asc` alongside.

**SHA256**: `sha256sum biscuitcode_*.deb > SHA256SUMS.txt` attached to the release.

### 13. Security posture

**Tauri v2 capability system (vs. v1 allowlist).** V2's ACL is more granular:
- Capability files in `src-tauri/capabilities/*.json` (auto-loaded).
- Each capability scopes to named windows/webviews by identifier.
- Permissions come from plugin defaults + user-authored; you can deny specific commands inside a plugin.
- Scopes are strongly typed per-plugin (e.g., `fs:scope` takes `{path: string}` entries, supports `$APPDATA`/`$HOME`/`$RESOURCE`/`$DESKTOP` variables).
- **Deny takes precedence over allow** — so a blanket allow + targeted deny is safe. [18][55][17]

**Recommended capability sketch** (will be detailed in plan):
- `fs`: `allow-read/write-text-file`, `allow-read/write-binary-file`, scope allowlist per opened workspace root + `$APPCONFIG` + `$APPDATA`.
- `shell`: only `execute` allowed with a strict command registry (initially: `which <binary>`, `ollama pull <model>`, language-server binaries). No `open` with arbitrary URL. No wildcard args.
- `http`: `fetch` limited to `https://api.anthropic.com/**`, `https://api.openai.com/**`, `http://localhost:11434/**`. CORS-unaware, but fetch-url filtering is a capability.
- No `process.exit` from frontend.
- No `clipboard-manager.read` (we write only unless user explicitly pastes in UI — and UI paste doesn't need the clipboard plugin).

**API keys** live only in keyring. The provider implementations fetch on each request (fast DBus call); never log, never pass to frontend, never include in `Display`/`Debug` impls. Crash reporters (if any) strip known fields by name.

**Telemetry**: off by default. If enabled, the server endpoint receives a UUID (generated locally, not user-bound), OS distro, Tauri version, app version, and panic type — no prompt content, no file paths, no responses. Make the on-toggle itself a keyring entry so it survives app reinstall.

### 14. Performance targets

**First-token-under-500ms on Claude streaming.** The floor is the network RTT to `api.anthropic.com` + TLS handshake + server TTFB. Realistic cold-path: 100–250 ms RTT Europe/US + ~150–400 ms API warmup. **Sub-500 ms is feasible only when the connection is warm**. Mitigations:
- Open a warm HTTP/2 connection to the API on app start (speculative HEAD or a cheap prewarm request).
- Keep-alive enabled on `reqwest`.
- Stream processing must be zero-buffered — do not wait for complete SSE events before emitting to UI; emit as soon as a `text_delta` parses.
- Measure p50/p95 to the *first `text_delta` rendered in the UI*, not to the API's TTFB alone.

**Cold-launch under 2s with Monaco bundle.** Feasible with aggressive lazy-loading: render the shell + activity bar instantly from inlined HTML, mount React, defer Monaco to a dynamic import fired when either (a) the user opens a file or (b) 500 ms after paint. On mid-range hardware, Tauri's native-webview approach gives you ~200–400 ms cold-launch to first paint before any app logic. Staying under 2s to interactive is realistic. [15][37]

Release build opts: `[profile.release] opt-level = "z"; lto = true; codegen-units = 1; strip = true; panic = "abort"`. ~20–30% size reduction, negligible runtime cost. [37]

### 15. Namespace collision verification

- **`biscuit`** crate on crates.io — a Rust JOSE (JSON Web Token/Signature/Encryption) library by lawliet89, at version 0.x. Unrelated to our product. **Do not depend on this crate; do not name internal crates `biscuit`**. [68]
- **`biscuit-auth`** crate on crates.io — Eclipse Biscuit authorization tokens, version 6.x, maintained by the Eclipse Foundation. Unrelated capabilities-based authorization library. **Do not depend; do not collide.** [69]
- **`biscuit-cli`** crate — CLI companion to `biscuit-auth`. Same category.
- **Code Biscuits** — a *family* of VS Code extensions (`CodeBiscuits.html-biscuits`, `.js-ts-biscuits`, `.css-biscuits`, `.assorted-biscuits`, `.json-biscuits`) that add inline end-of-block annotations. The publisher ID is **`CodeBiscuits`**. They are all on the VS Code Marketplace. **If we ever ship a VS Code extension, avoid that publisher name and the "Code Biscuits" brand.** [70]

Recommendation: name all internal Rust crates `biscuitcode-*` (one word: `biscuitcode-core`, `biscuitcode-agent`, `biscuitcode-providers`, `biscuitcode-lsp`, `biscuitcode-pty`), per the vision. The crate name `biscuitcode` itself (without hyphen) appears **unclaimed** on crates.io as of research time — if we want it as a workspace root, claim it defensively on day 1, even if we ship nothing under it.

---

## Best Practices

Concrete, actionable patterns with rationale.

1. **Tauri v2: write capabilities as human-readable JSON files, not generated by `migrate`.** The auto-generated `migrated.json` is over-permissive (it translates v1's coarse allowlist literally). Author 2–3 capability files by hand (`fs.json`, `shell.json`, `http.json`) with deny-by-default and specific allow scopes.
2. **Never fall back to plaintext for secrets.** Detect Secret Service → on missing, block onboarding with a clear "install gnome-keyring" prompt. The vision's security requirement is clear; a fallback undermines it.
3. **One Tauri command per coarse operation, not per fine step.** E.g., `terminal_input` takes a batch of bytes, not per-keystroke. Reduces IPC overhead dramatically (v2 raw-bytes IPC is fast but still measurable at high frequencies).
4. **Keep the `ModelProvider` trait minimal and put all streaming translation logic in each impl.** Three implementations, one event enum, one executor. Resist adding provider-specific fields to `ChatEvent`; if something only Claude has (thinking), expose it as an event variant that other providers simply never emit.
5. **Monaco: single editor instance, many models.** Matches VS Code. Don't render N Monaco instances for N tabs — memory and DOM cost is real.
6. **LSP: one child process per language server per workspace.** Do not multiplex workspaces into one rust-analyzer — analyzers cache per-root; it'll crash or misbehave.
7. **SQLite: WAL mode.** `PRAGMA journal_mode=WAL` at init. Gives concurrent reads while the agent writes messages; also faster.
8. **Git operations: use `git2` for reads, shell out to `git` for writes.** `git2`'s diff and status APIs are fast and detail-rich. But commit/push/pull UX matters and `git` handles credential helpers, signing, hooks, and LFS naturally. Shelling out keeps us compatible with the user's existing `.gitconfig`. [71]
9. **Ship the `.deb` with `Recommends: gnome-keyring, ollama` and `Suggests:` for language servers.** Documents the soft deps without hard-requiring them.
10. **Build both .deb and .AppImage in CI but label the .deb "recommended"** in README. AppImage is a fallback for non-Debian distros; it has the `libfuse2t64` UX problem on Ubuntu 24.04 that many users haven't solved. [23]

---

## Recommended Approach (per domain)

Each is the **simplest adequate** option, with rationale.

1. **Tauri** — v2.10.x (latest stable), Rust 1.85+, TypeScript 5.5+. Use `plugin-fs`, `plugin-shell`, `plugin-dialog`, `plugin-http`, `plugin-os`, `plugin-process`, `plugin-store` (for non-secret settings), `plugin-window-state`, `plugin-global-shortcut`. Skip `plugin-sql`; use `rusqlite` directly.
2. **Linux Mint 22 XFCE** — build on `ubuntu-24.04` CI runners. Link dynamic against `libwebkit2gtk-4.1-0`. Ship `.deb` as primary. Document AppImage as secondary with the `libfuse2t64` caveat.
3. **WSL2 dev flow** — project in `~/projects/biscuitcode` (not `/mnt/c`). WSLg for GUI preview. Don't try to build Windows artifacts from WSL2; do that in CI on `windows-latest` if/when needed in v1.1.
4. **Monaco** — `@monaco-editor/react` pinned locally (no CDN loader). `vite-plugin-monaco-editor` for worker bundling. Subset of languages loaded by default; rest lazy. One editor instance, one model per tab. Diff via `createDiffEditor`.
5. **xterm.js + portable-pty** — `@xterm/xterm` 5.x + `@xterm/addon-fit` + `-web-links` + `-search` + `-webgl` (canvas fallback). Rust `portable-pty` 0.8+; one PTY per terminal tab; Tokio tasks per stream direction; events keyed by session ID.
6. **Keyring** — `keyring` 3.6 with `linux-native-async-persistent` + `async-secret-service` + `crypto-rust` + `tokio`. Detect Secret Service at onboarding; block the "Add API key" flow if absent with a specific error + copy-paste install command.
7. **Model providers** — three impls behind a single `ModelProvider` trait emitting `ChatEvent`. Defaults: Anthropic `claude-opus-4-7`, OpenAI **`gpt-5.4`** (vision's `gpt-4o` is retired), Ollama **`gemma3:4b`** if ≥6GB RAM else `gemma3:1b`, escalating to `qwen2.5-coder:7b` at ≥12GB, `gemma3:12b` at ≥16GB.
8. **Agent loop** — React-style executor: `loop { stream provider → on tool_call, confirm if write/shell → execute → append tool_result → continue }`. Single global "pause" flag checked between iterations. Checkpointing = snapshot of modified-file contents keyed by `(conversation_id, message_id)`; rewind = revert to snapshot + truncate messages past the rewind point.
9. **LSP** — `monaco-languageclient` on the frontend. Rust spawns servers, proxies stdio via events keyed by session ID. Initial support: rust-analyzer, typescript-language-server, pyright, gopls, clangd. Surface missing-server prompts with copy-to-clipboard install commands; don't auto-run.
10. **SQLite** — `rusqlite` + `refinery` or hand-rolled `PRAGMA user_version` migrations. WAL mode. Blocking calls on a bounded thread pool (`tokio::task::spawn_blocking`).
11. **Theming** — `xfconf-query -c xsettings -p /Net/ThemeName` at startup; `-dark$` regex → dark mode. On first run with a light GTK theme, offer a theme switch to BiscuitCode Cream.
12. **Packaging/CI** — GH Actions on `ubuntu-24.04`. Build `.deb` + `.AppImage`. GPG sign via `GPG_PRIVATE_KEY` secret. SHA256 via `sha256sum`. Upload to a tagged release.
13. **Security** — deny-by-default capabilities. File access scoped to workspace root + app config/data dirs. Shell access pre-registered commands only. HTTP scope limited to 3 provider hosts. API keys only in keyring.
14. **Performance** — release opts (`opt-level="z"`, `lto`, `codegen-units=1`, `strip`). Monaco lazy. HTTP connection prewarm to Anthropic. Stream-as-you-parse.
15. **Naming** — internal crates `biscuitcode-*`. Claim `biscuitcode` on crates.io defensively. Avoid the `CodeBiscuits` publisher name if a VS Code extension ships.

---

## Trade-offs & Alternatives

### Editor framework

| Option | Pros | Cons | When to use |
|--------|------|------|-------------|
| **Monaco via `@monaco-editor/react`** (recommended) | VS Code parity; rich LSP; familiar to users; diff view built-in | 15 MB bundle; web workers; opinionated API | Default choice — the vision requires Monaco |
| CodeMirror 6 | ~1 MB; modular; modern API | Smaller ecosystem for LSP; fewer out-of-the-box VS Code-like features; no diff editor built in | If bundle size were the top constraint. Not here. |
| Ace | Mature, simple | Dated feel; lacks LSP integration | Ruled out |

### Layout library

| Option | Pros | Cons | When to use |
|--------|------|------|-------------|
| **react-resizable-panels** | Lightweight; persisted layouts; flex-based | Just panels, no docking | Default — matches the four-region vision perfectly |
| Dockview | Full docking with drag-and-drop, floating panels | Heavier; overkill for fixed four-region layout | If we ever want draggable/floating panels (not in v1) |
| Allotment | Similar to react-resizable-panels | Less maintained than bvaughn's | Not preferred |

### SQLite access

| Option | Pros | Cons | When to use |
|--------|------|------|-------------|
| **`rusqlite` direct** (recommended) | Minimal deps; sync API is fine for local file; no plugin abstraction | Manual migrations | Default — simplest adequate |
| `sqlx` + `tauri-plugin-sql` | Compile-time SQL checks; migrations built in; async | Extra plugin layer; sqlx-macros compile cost | If the project grows to need multiple DBs |
| `tauri-plugin-rusqlite2` | `rusqlite` with plugin-style migrations and transactions | Third-party fork; less maintained | Middle ground if `plugin-sql`'s async-only API annoys |

### Terminal

| Option | Pros | Cons | When to use |
|--------|------|------|-------------|
| **`xterm.js` + Rust `portable-pty`** (recommended) | Every major TS editor uses this; well-supported | Need to build the event proxy yourself | Default |
| `tauri-plugin-pty` (community) | Off-the-shelf | Less control; dependency on third party | If time-to-first-terminal mattered more than polish |

### Provider auth key storage

| Option | Pros | Cons | When to use |
|--------|------|------|-------------|
| **`keyring` crate + libsecret** (recommended) | Gold standard; user-visible in Seahorse | Requires Secret Service daemon | Default |
| Config-file + libsodium encryption | Works without daemon | Device-bound key, no OS integration; inferior | Ruled out — violates vision |
| Env vars | Simple | Vision forbids | Ruled out |

### LSP transport

| Option | Pros | Cons | When to use |
|--------|------|------|-------------|
| **Tauri events proxying stdio** (recommended) | No extra ports; sandboxed | Custom MessageTransports wire-up | Default |
| WebSocket bridge in a Rust sidecar | `monaco-languageclient`'s default transport | Exposes a local port; extra process | If we ever need remote LSP |

---

## Risks & Unknowns

Specific, named so the planner can address them.

### Top-priority risks

1. **WebKitGTK 4.1 rendering quirks on older Mint 22 installs.** Ubuntu 24.04 ships webkit2gtk-4.1 but minor versions vary (`webkit2gtk4.1/noble-updates`). Some CSS features (container queries, `:has()`) landed in later 4.1 patch releases. **Test the full UI on 22.0, 22.1, 22.2 base images.** File: any `.css` using modern selectors. [4]
2. **libfuse2 rename on Ubuntu 24.04.** AppImage users on stock Mint 22 may hit a "nothing happens when I double-click the AppImage" wall because `libfuse2` was renamed to `libfuse2t64`. Our .deb primary install dodges this, but AppImage users need either (a) a self-extracting wrapper that checks for `libfuse2t64` and prompts install, or (b) explicit README note. [23][24]
3. **`claude-opus-4-7` rejects non-default `temperature`/`top_p`/`top_k`.** A single line in `providers/anthropic.rs` that passes `temperature: 0.7` will HTTP 400 on Opus 4.7. The `ChatOptions` struct must either omit these fields for Opus 4.7 or the provider impl must filter by model. [46]
4. **GPT-4o retirement.** Vision defaults `OpenAIProvider` to `gpt-4o`, retired April 3, 2026. Every code path that lists OpenAI models must exclude 4o and default to `gpt-5.4-mini` (good price/perf) or `gpt-5.4`. [9]
5. **Monaco bundle size vs. 2-second cold-launch target.** If Monaco loads synchronously on app open, cold-launch will exceed 2 s on an 8 GB i5-8000 machine. Lazy loading is the mitigation; the target is achievable only with it. [37]
6. **XFCE + Wayland future.** If a user runs Mint with experimental Wayland XFCE, WebKitGTK may fall back to Xwayland. Clipboard, drag-drop, and IME behavior differ. Test at least one Wayland session before 1.0. [6]
7. **Secret Service absent in bare XFCE.** If `gnome-keyring` is uninstalled or the session didn't launch it (e.g., `startxfce4` via `xinit`), all auth flows break. Need a clean error path + install instructions. [25][26]
8. **Ollama tool-calling model availability churn.** Gemma 3 base doesn't reliably tool-call; Gemma 4 does (released 2026-04-02). Auto-pull defaults should be forward-compatible: prefer `gemma4:*` when present, fall back to `qwen2.5-coder:*` which has stable tool support. [11][12]
9. **linuxdeploy flakes in GH Actions on Ubuntu 24.04.** Reported regressions in container builds. Pin `tauri-action` version and keep a retry on the AppImage step. [67]
10. **Monaco keybindings vs. host-app global shortcuts.** Chord bindings (`Ctrl+K Ctrl+I`) in Monaco are scoped to editor focus. A global `plugin-global-shortcut` fires only when the app is unfocused by default, and may conflict with DE-level bindings. Resolve by: Monaco action for in-editor, document-level keydown for in-app non-editor, `plugin-global-shortcut` only for truly global (like "bring BiscuitCode to front"). [38]

### Unknowns the maintainer should confirm

1. **Dev environment.** Confirming WSL2 Ubuntu 24.04 dev loop is the intent, and whether CI-only Linux targeting (no local Linux machine) is acceptable. Affects packaging iteration speed.
2. **Signing scope.** Is GPG signing of .deb enough, or do we also want a Debian-repo setup (adding `apt.biscuitcode.io`)? Repo hosting has ongoing cost and CI complexity.
3. **Telemetry concrete form.** "Anonymous crashes only" — is a third-party service acceptable (Sentry) or must we host the crash endpoint? Different security+privacy postures.
4. **First-run model defaults if *no* provider is configured.** Does onboarding force at least one provider to be set, or is there a "just let me explore" dry state?
5. **How deep is `Preview` in v1?** The vision lists Markdown/HTML/Images/PDF/Notebooks. Notebook rendering alone (with KaTeX + Mermaid + pygments for code blocks) is 2–3 days. Confirm notebook render is truly v1.
6. **Arm64 Linux**. Mint 22 targets x86_64 primarily, but Raspberry Pi and some dev laptops are arm64. Is arm64 `.deb` a goal for v1?
7. **Brand-token CSS variable *names*.** The vision lists them as `--biscuit-500` etc.; confirmed these must appear verbatim in Tailwind config (locked).
8. **Icon concept choice.** Vision prefers Concept A; D is on the table. Concept call needs a 16×16 render test before commit — flagged in vision itself.

---

## Sources

1. [New Features in Linux Mint 22.1 'Xia' — Linux Mint](https://www.linuxmint.com/rel_xia_whatsnew.php)
2. [Linux Mint 22.2 release — based on Ubuntu 24.04 and Linux 6.14](https://en.ubunlog.com/Linux-Mint-22-is-now-available-based-on-Ubuntu-2-and-Linux-24./)
3. [Tauri Core Ecosystem Releases](https://v2.tauri.app/release/)
4. [Debian packaging | Tauri v2](https://v2.tauri.app/distribute/debian/)
5. [Migration to webkit2gtk-4.1 on Linux | Tauri blog](https://v2.tauri.app/blog/tauri-2-0-0-alpha-3/)
6. [Linux Mint Monthly News – January 2026](https://blog.linuxmint.com/?p=4991)
7. [Linux Mint forums — Xfce and Wayland](https://forums.linuxmint.com/viewtopic.php?t=453726)
8. [Anthropic Models overview — current model IDs](https://platform.claude.com/docs/en/about-claude/models/overview)
9. [OpenAI All models reference](https://developers.openai.com/api/docs/models/all)
10. [GPT-5.4 API docs](https://developers.openai.com/api/docs/models/gpt-5.4)
11. [Ollama library — gemma3](https://ollama.com/library/gemma3)
12. [orieg/gemma3-tools (community tool-calling Gemma variant)](https://ollama.com/orieg/gemma3-tools)
13. [`keyring` crate docs.rs](https://docs.rs/keyring/latest/keyring/)
14. [Tauri 2.0 Stable Release blog](https://v2.tauri.app/blog/tauri-20/)
15. [Tauri vs Electron 2026 benchmarks (PkgPulse)](https://www.pkgpulse.com/blog/electron-vs-tauri-2026)
16. [Inter-Process Communication | Tauri](https://v2.tauri.app/concept/inter-process-communication/)
17. [Upgrade from Tauri 1.0 | Tauri](https://v2.tauri.app/start/migrate/from-tauri-1/)
18. [Capabilities | Tauri v2 security](https://v2.tauri.app/security/capabilities/)
19. [Embedding External Binaries (sidecar) | Tauri](https://v2.tauri.app/develop/sidecar/)
20. [Stream stdout from Sidecar — tauri-apps discussion #8641](https://github.com/orgs/tauri-apps/discussions/8641)
21. [Tauri SQL plugin](https://v2.tauri.app/plugin/sql/)
22. [Tauri Shell plugin](https://v2.tauri.app/plugin/shell/)
23. [AppImage FUSE on Ubuntu 24.04 — itsFOSS](https://itsfoss.com/cant-run-appimage-ubuntu/)
24. [AppImage on Ubuntu 24.04 — OMG!Ubuntu](https://www.omgubuntu.co.uk/2023/04/appimages-libfuse2-ubuntu-23-04)
25. [ArchWiki — GNOME/Keyring](https://wiki.archlinux.org/title/GNOME/Keyring)
26. [Linux Mint Forums — gnome keyring under XFCE](https://forums.linuxmint.com/viewtopic.php?f=110&t=97592)
27. [gtk-update-icon-cache man page](https://linuxcommandlibrary.com/man/gtk-update-icon-cache)
28. [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)
29. [Getting Tauri working in WSL — penner.me](https://penner.me/getting-tauri-working-in-wsl)
30. [libwebkit2gtk-4.0 not available in Ubuntu 24 — tauri #9662](https://github.com/tauri-apps/tauri/issues/9662)
31. [Cross-compile Tauri from WSL2 — Zenn](https://zenn.dev/junkor/articles/69ad2422b8067f)
32. [`@monaco-editor/react` on npm](https://www.npmjs.com/package/@monaco-editor/react)
33. [monaco-react GitHub repo](https://github.com/suren-atoyan/monaco-react)
34. [`vite-plugin-monaco-editor`](https://github.com/vdesjs/vite-plugin-monaco-editor)
35. [Import monaco-editor using Vite — vite Discussion #1791](https://github.com/vitejs/vite/discussions/1791)
36. [Optimizing Monaco with dynamic imports (WebAssembly Studio)](https://medium.com/@ollelauribostr/dynamic-imports-speeding-up-the-initial-loading-time-of-webassembly-studio-9f50b975472a)
37. [Tauri App Size guide](https://v2.tauri.app/concept/size/)
38. [Custom Command Palette with Monaco — rikki.dev](https://rikki.dev/posts/monaco-command-palette)
39. [marc2332/tauri-terminal](https://github.com/marc2332/tauri-terminal)
40. [Terminon — Tauri + xterm.js terminal](https://github.com/Shabari-K-S/terminon)
41. [tauri-plugin-pty](https://github.com/Tnze/tauri-plugin-pty)
42. [`@xterm/addon-web-links`](https://www.npmjs.com/package/@xterm/addon-web-links)
43. [`@xterm/addon-webgl`](https://www.npmjs.com/package/@xterm/addon-webgl)
44. [Streaming Messages — Claude API docs](https://platform.claude.com/docs/en/build-with-claude/streaming)
45. [What's new in Claude Opus 4.7](https://platform.claude.com/docs/en/about-claude/models/whats-new-claude-4-7)
46. [Anthropic API — Opus 4.7 sampling param restriction](https://platform.claude.com/docs/en/about-claude/models/whats-new-claude-4-7)
47. [OpenAI — Streaming chat completions](https://developers.openai.com/api/docs/guides/streaming-responses)
48. [OpenAI function calling](https://developers.openai.com/api/docs/guides/function-calling)
49. [Ollama FAQ](https://docs.ollama.com/faq)
50. [Ollama Setup Guide 2026 — SitePoint](https://www.sitepoint.com/ollama-setup-guide-2026/)
51. [Zed Agentic mode discussion](https://github.com/zed-industries/zed/discussions/24028)
52. [Enable Cursor-like agent approval UX — Zed discussion #53169](https://github.com/zed-industries/zed/discussions/53169)
53. [How Claude Code works](https://code.claude.com/docs/en/how-claude-code-works)
54. [Cursor vs Zed agent UX — Medium](https://medium.com/@kamilcollu/ai-code-editors-zed-cursor-and-windsurf-b8a068c9eea3)
55. [Permissions | Tauri](https://v2.tauri.app/security/permissions/)
56. [`monaco-languageclient` custom connection — TypeFox Discussion #583](https://github.com/TypeFox/monaco-languageclient/discussions/583)
57. [`monaco-languageclient` README](https://github.com/TypeFox/monaco-languageclient)
58. [LSP Mode — Rust rust-analyzer install](https://emacs-lsp.github.io/lsp-mode/page/lsp-rust-analyzer/)
59. [Building a todo app in Tauri with SQLite and sqlx](https://tauritutorials.com/blog/building-a-todo-app-in-tauri-with-sqlite-and-sqlx)
60. [Tauri SQL plugin reference](https://v2.tauri.app/plugin/sql/)
61. [`tauri-plugin-rusqlite2`](https://github.com/razein97/tauri-plugin-rusqlite2)
62. [xfconf-query — XFCE theming](https://monkeyjunglejuice.github.io/blog/learn-ocaml-light-dark-theme-switcher-gtk.tutorial.html)
63. [Dark Mode switching — ArchWiki](https://wiki.archlinux.org/title/Dark_mode_switching)
64. [Run Tauri in GitHub Actions — remarkablemark](https://remarkablemark.org/blog/2026/04/08/tauri-github-action/)
65. [Tauri GitHub Actions distribute docs](https://v2.tauri.app/distribute/pipelines/github/)
66. [`tauri-action` GH Action](https://github.com/tauri-apps/tauri-action)
67. [Can't build AppImage with linuxdeploy — tauri #14796](https://github.com/tauri-apps/tauri/issues/14796)
68. [`biscuit` (JOSE/JWT) on crates.io](https://crates.io/crates/biscuit)
69. [`biscuit-auth` on crates.io](https://crates.io/crates/biscuit-auth)
70. [Code Biscuits VS Code extensions — Marketplace](https://marketplace.visualstudio.com/items?itemName=CodeBiscuits.assorted-biscuits)
71. [`git2-rs` crate](https://github.com/rust-lang/git2-rs)
