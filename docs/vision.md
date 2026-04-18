# Super Prompt v3: Build **BiscuitCode** — A Multi-Model AI Coding Environment for Linux Mint XFCE

> Preserved verbatim from `biscuitcode-super-prompt-v3.md`. This is the raw vision — the C.Alan pipeline (`docs/research.md`, `docs/plan.md`) derives from it.

---

## Mission

Build a production-quality desktop application called **BiscuitCode** — a VS Code-class AI coding environment natively optimized for **Linux Mint 22 XFCE** (Xia, kernel 6.8, GTK 3/4). It must feel as polished as VS Code and Claude Code Desktop, support Claude, ChatGPT, and local Gemma models out of the box, and install with a single double-click on a `.deb` file. Nothing about this should feel like a toy or a prototype. Quality bar: if a stranger installed this cold, they should not be able to tell it wasn't made by a funded team.

## Locked Identity

These values are final. Do not prompt me to reconsider them.

| Setting | Value |
|---|---|
| Project name | **BiscuitCode** |
| Tagline | *An AI coding environment, served warm.* |
| GitHub repo | `biscuitcode` |
| Executable | `biscuitcode` (lowercase) |
| Display name | `BiscuitCode` |
| Bundle ID | `io.github.Coreyalanschmidt-creator.biscuitcode` |
| Config dir | `~/.config/biscuitcode/` |
| Data dir | `~/.local/share/biscuitcode/` (conversation DB, logs, cached models) |
| Cache dir | `~/.cache/biscuitcode/` |
| License | MIT |

## Brand Tokens (use verbatim in Tailwind config + Rust palette constants)

The palette leans warm — golden biscuit gold on rich cocoa dark. Developer-friendly, Gruvbox-adjacent, distinct from any official Linux Mint branding. Do not change without asking.

```
/* Biscuit (primary brand — warm gold) */
--biscuit-50:  #FDF7E6
--biscuit-100: #FAE8B3
--biscuit-200: #F5D380
--biscuit-300: #F0C065
--biscuit-400: #EBB553
--biscuit-500: #E8B04C   <- PRIMARY ACCENT (Biscuit Gold)
--biscuit-600: #C7913A
--biscuit-700: #9E722A
--biscuit-800: #74531E
--biscuit-900: #4A3413

/* Cocoa (warm dark neutrals for backgrounds / UI chrome) */
--cocoa-50:    #F6F0E8
--cocoa-100:   #E0D3BE
--cocoa-200:   #B9A582
--cocoa-300:   #8A7658
--cocoa-400:   #584938
--cocoa-500:   #3A2F24
--cocoa-600:   #28201A
--cocoa-700:   #1C1610   <- PRIMARY DARK BG
--cocoa-800:   #120D08
--cocoa-900:   #080504   <- DEEPEST DARK

/* Semantic */
--accent-ok:    #6FBF6E   (sage green — complements warm palette, distinct from brand)
--accent-warn:  #E8833E   (terracotta orange — distinct from biscuit-gold)
--accent-error: #E06B5B   (salmon — reads clearly on warm dark)
```

## Typography

- UI: **Inter** (weights 400, 500, 600). Self-hosted woff2 in `src-tauri/fonts/`. No `system-ui` fallbacks in primary UI chrome.
- Code / Terminal: **JetBrains Mono** (weights 400, 500). Self-hosted. Ligatures on by default, toggle in settings.
- UI sizes: 12px (secondary), 13px (default), 14px (primary), 16px (headings). Line-height 1.5.

## Hard Constraints (Non-Negotiable)

1. **Target platform:** Linux Mint 22 XFCE, x86_64. Must work on both X11 and Wayland-XFCE sessions. Must detect the user's GTK theme even if overridden by our palette.
2. **Stack:** Tauri 2.x (Rust backend) + React 18 + TypeScript 5 + Vite + Tailwind CSS 3. **Do not use Electron.**
3. **Editor:** Monaco Editor via `@monaco-editor/react`. Full syntax highlighting, multi-cursor, find/replace, minimap, diff view.
4. **Terminal:** xterm.js in the frontend, pty backend via Rust `portable-pty` crate.
5. **Install:** `.deb` installs to `/opt/biscuitcode`, registers `/usr/share/applications/biscuitcode.desktop`, installs icons to `/usr/share/icons/hicolor/{16,32,48,64,128,256,512}x{same}/apps/biscuitcode.png`, symlinks `/usr/bin/biscuitcode`. Also produce an AppImage.
6. **First run:** welcome screen with the BiscuitCode logo, 3-step onboarding (pick models → enter API keys OR install Ollama → open a folder), coding within 2 minutes.
7. **Secrets:** API keys stored in system keyring via libsecret (`keyring` Rust crate). Never in plaintext config. Never in env vars.
8. **All dependencies:** MIT / Apache-2.0 / BSD compatible.

**Watch out for namespace collisions:**
- The Rust `biscuit` / `biscuit-auth` crates exist (authorization tokens — totally unrelated category). Do **not** use those crate names for any of our Rust crates. Name internal crates `biscuitcode-*` (e.g., `biscuitcode-core`, `biscuitcode-agent`).
- The `CodeBiscuits` / "Code Biscuits" VS Code annotation extension exists. If we ever ship a VS Code extension later, do not use that name.

## Icon

Use the design direction from the reference file `biscuitcode-icon-concepts.html`. My preferred concept is **Concept A: "The Prompt"** — a biscuit-gold `>_` terminal glyph centered on a cocoa-dark rounded-square (`#1C1610` background, 22% corner radius). **Concept D: "The Biscuit"** — a literal biscuit-shape (round, with digestive-biscuit pricks around the edge) containing the `>_` glyph — is an on-brand alternative I'll consider if you (Claude Code) find it renders more distinctively in a launcher grid. Do not pick Concept B or C without flagging it for me.

Deliver as:
- `packaging/icons/biscuitcode.svg` (master)
- `packaging/icons/biscuitcode-{16,32,48,64,128,256,512}.png` (rasterized via `rsvg-convert` or `inkscape --export-type=png`)
- `packaging/icons/biscuitcode.ico` (Windows, future use)

Icon must remain legible at 16x16 in a system tray. Test this before declaring the icon done.

## Architecture

```
biscuitcode/
|-- src-tauri/                 # Rust backend
|   |-- src/
|   |   |-- main.rs
|   |   |-- commands/
|   |   |   |-- fs.rs          # sandboxed file ops (workspace-scoped)
|   |   |   |-- terminal.rs    # pty management
|   |   |   |-- git.rs         # git2 / libgit2 wrapper
|   |   |   |-- keyring.rs     # secret storage
|   |   |   `-- models.rs      # model proxy (hides keys, avoids CORS)
|   |   |-- agent/
|   |   |   |-- tools.rs       # read_file, write_file, run_shell, search_code, apply_patch
|   |   |   |-- executor.rs    # ReAct-style agent loop
|   |   |   `-- providers/
|   |   |       |-- mod.rs     # ModelProvider trait
|   |   |       |-- anthropic.rs
|   |   |       |-- openai.rs
|   |   |       `-- ollama.rs
|   |   `-- db.rs              # SQLite for conversation history
|   `-- tauri.conf.json
|-- src/                       # React frontend
|   |-- components/
|   |   |-- ActivityBar.tsx
|   |   |-- SidePanel.tsx
|   |   |-- EditorArea.tsx
|   |   |-- TerminalPanel.tsx
|   |   |-- ChatPanel.tsx
|   |   |-- AgentActivityPanel.tsx
|   |   |-- PreviewPanel.tsx
|   |   `-- StatusBar.tsx
|   |-- layout/
|   |   `-- WorkspaceGrid.tsx  # react-resizable-panels
|   |-- state/                 # Zustand stores
|   |-- providers/             # frontend model provider UI
|   `-- theme/                 # token bridge, GTK detection
`-- packaging/
    |-- deb/                   # control, postinst, postrm
    |-- appimage/
    `-- icons/
```

## UI Layout (Match This Exactly)

Four resizable, hideable regions. Background `--cocoa-700`, 1px dividers `--cocoa-500`, accent `--biscuit-500`.

```
+--+------------+------------------------------+------------------+
| A|            |                              |                  |
| c|            |      Editor Area             |   Chat Panel     |
| t|  Side      |   (Monaco, tabs on top)      |                  |
| i|  Panel     |                              |  +------------+  |
| v|            |                              |  | Messages   |  |
| i|  - Files   |                              |  |            |  |
| t|  - Search  +------------------------------+  |            |  |
| y|  - Git     |                              |  +------------+  |
|  |  - Chats   |   Terminal / Agent Activity  |  [model v] [sub] |
| B|            |   (tabbed)                   |                  |
| a|            |                              |                  |
| r|            |                              |                  |
+--+------------+------------------------------+------------------+
| git:main * 0 warn * LSP: ts-server * claude-opus-4-7 * Ln 42 C7 |
+-----------------------------------------------------------------+
```

**Activity Bar (left, 48px):** Files, Search, Git, Chats, Settings. Active icon: 2px `--biscuit-500` bar on the left edge.

**Side Panel (collapsible, default 260px):** contextual.

**Editor Area (flexes):** Monaco tabs, dirty dot, middle-click close, Ctrl+W close, Ctrl+\\ split horizontally. Preview opens as a split pane, never a new window.

**Bottom Panel (collapsible, default 240px, tabbed):** Terminal, Agent Activity, Problems, Output.
  - **Agent Activity** shows a live stream of tool calls as collapsible cards: tool name, arguments (pretty JSON), streamed result, timing, status icon (running/ok/error). This is the "watch the AI work" view.

**Chat Panel (right, collapsible, default 380px):** message list with markdown rendering, code blocks with copy/apply/run buttons, `@file` `@folder` `@selection` `@terminal-output` `@problems` `@git-diff` mentions. Model selector + agent-mode toggle + send button pinned to bottom.

**Status Bar (22px):** git branch, problem count, active LSP, current model (click to switch), cursor position.

**Keyboard shortcuts:**

| Shortcut | Action |
|---|---|
| `Ctrl+B` | toggle side panel |
| `Ctrl+J` | toggle bottom panel |
| `Ctrl+Alt+C` | toggle chat panel |
| `Ctrl+P` | quick file open |
| `Ctrl+Shift+P` | command palette |
| `Ctrl+\`` | toggle terminal focus |
| `Ctrl+K Ctrl+I` | inline AI edit on selection |
| `Ctrl+L` | send current selection to chat |
| `Ctrl+Shift+L` | new chat |
| `F1` | help |

## Model Integration

```rust
#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSpec>,
        opts: ChatOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent>> + Send>>>;
    fn supports_tools(&self) -> bool;
    fn supports_vision(&self) -> bool;
}
```

**Required initial providers:**

1. **AnthropicProvider** — `https://api.anthropic.com/v1/messages`, streaming SSE, tool use. Default `claude-opus-4-7`. Expose: `claude-opus-4-7`, `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`.
2. **OpenAIProvider** — `https://api.openai.com/v1/chat/completions`, streaming, tool use. Default `gpt-4o`. Expose common models including reasoning-mode ones behind a toggle.
3. **OllamaProvider** — `http://localhost:11434/api/chat`, streaming, tool use where model supports it. On first launch, detect Ollama; if missing, offer a one-click install running `curl -fsSL https://ollama.com/install.sh | sh` with a confirmation dialog showing the exact command. Auto-pull `gemma2:9b` as the default (detect RAM; suggest `gemma2:2b` instead if system has <8GB). Also surface: `llama3.1:8b`, `qwen2.5-coder:7b`, and any locally pulled models.

**Provider UX:**
- Settings → Models page, status badge per provider (green/yellow/red).
- Add-model wizard: key entry with test-connection button; Ollama model pulling with progress bar piped from `ollama pull`.
- Per-conversation model switch in chat panel; persisted per-conversation.

**Streaming + tool use:**
- Backend emits `ChatEvent` enum: `TextDelta`, `ToolCallStart`, `ToolCallDelta`, `ToolCallEnd`, `ToolResult`, `Done`, `Error`.
- Chat panel renders text deltas live. Tool calls render simultaneously as cards in Agent Activity; a badge in the chat message links to the activity card.

## Coding Features (VS Code parity)

- File tree with git status colors (M, U, A, D), lazy-loaded, drag-and-drop, right-click context menu (new file, new folder, rename, delete, reveal in file manager, copy path, open in terminal).
- Multi-tab editor with dirty dot, middle-click close, `Ctrl+Shift+T` reopen closed.
- Find/Replace in-file (`Ctrl+F`) and across files (`Ctrl+Shift+F`) with regex, case, whole-word toggles.
- Integrated terminal: full pty, shell detected from `$SHELL`, multiple tabs, clickable URLs and file paths.
- Git: staged/unstaged diff, hunk-level stage/unstage, commit with message, push/pull with output to terminal, branch switcher in status bar, gutter blame (settings-toggled).
- LSP client via `monaco-languageclient`. Auto-detect and launch `rust-analyzer`, `typescript-language-server`, `pyright`, `gopls`, `clangd`. Install prompts for missing ones.
- Command palette `Ctrl+Shift+P`, fuzzy search over registered commands.
- Settings: GUI page + raw JSON editor. Stored at `~/.config/biscuitcode/settings.json`.

## AI Features (Match/Exceed Claude Code Desktop)

1. **Context-aware chat:** auto-attach current file (toggle-off), `@` mentions for structured context, drag files from tree into chat.
2. **Inline edit (`Ctrl+K Ctrl+I`):** select code → shortcut → describe change in popover → AI streams a diff inline with accept/reject/regenerate. No modal dialogs.
3. **Agent mode toggle:** when on, AI calls tools autonomously. Each tool call renders live in Agent Activity. User can pause, interrupt, rewind. Writes and shell commands ask for confirmation unless "trust workspace" is enabled.
4. **Conversation persistence:** SQLite at `~/.local/share/biscuitcode/conversations.db`. Each conversation has a workspace binding and an auto-generated title.
5. **Branching:** edit a past user message → fork. Tree view in conversation header.
6. **Preview auto-trigger:** AI edits to `.md`, `.html`, `.svg`, or image files auto-open a preview tab in a split pane.

## Preview Panel

- **Markdown:** GFM, highlighted code blocks, mermaid diagrams, KaTeX math, live update as user types.
- **HTML:** sandboxed iframe, live-reload on save, devtools button.
- **Images:** PNG/JPG/WebP/SVG/GIF with zoom and pan.
- **PDF:** pdf.js viewer.
- **Notebooks:** read-only render in v1, execute deferred to v2.

## Security

- File ops workspace-scoped by default. Opening outside prompts for confirmation.
- Shell commands require confirmation unless workspace is trusted.
- Tauri allowlist minimum-scope. No `shell.open` with arbitrary URLs. No `fs` access outside workspace + config dirs.
- API keys in keyring only. Never logged. Never in crash reports.
- Optional telemetry, off by default. If on, anonymous crashes only — no prompt content, no file contents, no responses.

## Packaging & Distribution

Produce in GitHub Actions CI:

- `biscuitcode_<version>_amd64.deb` — installs to `/opt/biscuitcode`, `.desktop` in `/usr/share/applications`, icons per freedesktop spec, `postinst` runs `update-desktop-database` and `gtk-update-icon-cache`.
- `BiscuitCode-<version>-x86_64.AppImage` — portable.
- SHA256 checksums for both.
- GPG-signed GitHub release.

Test the `.deb` on a clean Linux Mint 22 XFCE VM before every release. Must appear in the Whisker menu under Development with the correct icon, launch cleanly, uninstall cleanly via `apt remove biscuitcode`.

## Theming

- Default **BiscuitCode Warm** theme uses the Biscuit + Cocoa palette above.
- Read GTK theme on launch; if user's GTK is light, offer to switch to **BiscuitCode Cream** (light theme) on first run.
- Ship three themes in v1: **BiscuitCode Warm** (default dark), **BiscuitCode Cream** (light), **High Contrast**. Settings shows live preview.
- VS Code theme import stubbed in v1.1 — UI placeholder only for v1.

## Development Phases

Build in this order. Each phase ends with a runnable checkpoint. Do not skip.

**Phase 1 — Shell (day 1-2).** Scaffold Tauri + React + TS + Tailwind with full brand tokens wired. Four-region layout with react-resizable-panels. All toggle shortcuts working. Placeholders inside regions. Ship installable `.deb`.

**Phase 2 — Editor + File Tree (day 3-5).** Monaco, multi-tab, file tree with real ops, find/replace in file.

**Phase 3 — Terminal (day 6).** xterm.js + portable-pty. Multiple tabs. Shell detection.

**Phase 4 — One Provider E2E (day 7-9).** `ModelProvider` trait + AnthropicProvider. Chat panel. Streaming text. Keyring. Deliver a version where the user can chat with Claude. No tools yet.

**Phase 5 — Agent Loop + Tools (day 10-13).** Tool registry, ReAct executor, live tool-call streaming to Agent Activity. Inline edit. Agent mode toggle.

**Phase 6 — Remaining Providers (day 14-15).** OpenAIProvider, OllamaProvider. Ollama detection + install. Gemma auto-pull with RAM-aware default.

**Phase 7 — Git + LSP + Preview (day 16-19).** Git panel, LSP client, preview panel.

**Phase 8 — Polish + Package (day 20-21).** Onboarding, settings UI, final theming, `.deb` + AppImage CI, icon set, screenshots, README.

## Quality Bar

Before marking any phase complete, verify:

1. Installs clean on a fresh Mint 22 XFCE VM via `sudo dpkg -i biscuitcode_*.deb`. Uninstalls cleanly via `apt remove biscuitcode`.
2. Launches in under 2 seconds on mid-range hardware (i5-8xxx, 8GB RAM).
3. No console errors in devtools or Rust logs during normal use.
4. All keyboard shortcuts match spec.
5. No placeholder text, no lorem ipsum, no user-visible TODO comments.
6. Screenshots look like a professional product's screenshots. If you'd be embarrassed to put it on the README, it's not done.
7. Typography consistent: Inter for UI, JetBrains Mono for code. No system-ui fallbacks in visible chrome.
8. Dark theme actually dark — no `#000`, no `#fff`. Use the Cocoa scale.
9. Every failure path (no network, bad key, Ollama not running, permission denied) has a specific, actionable error message. No raw stack traces shown to users.
10. AI feels fast: first token under 500ms on Claude, smooth streaming, tool calls render on start, not on completion.

## "Done" Definition

User downloads `biscuitcode_1.0.0_amd64.deb` → double-click → GDebi installs → Whisker menu shows BiscuitCode under Development → launches to onboarding → 3 screens → coding in under 2 minutes. Ctrl+L to ask Claude about selected code. Agent mode on to refactor a module, watching it happen in Agent Activity. Accept the diff. Commit and push from the status bar. Feels like a product, not a project.
