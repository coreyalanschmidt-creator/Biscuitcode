# Phase 7 — Merging the LSP backend crate

> Read this when starting Phase 7 (Git Panel + LSP Client + Preview Panel). The `biscuitcode-lsp` crate skeleton is pre-staged with the public API surface, language detection, and install-command table locked; this guide explains how to land it and what's still TODO. Git and Preview have NO pre-staged Rust — they're built from scratch in this phase.

## Pre-staged files (Phase 7 LSP foundation)

| Path | What |
|---|---|
| `src-tauri/biscuitcode-lsp/Cargo.toml` | Workspace member: tokio (process + io-util + sync + time), serde_json, parking_lot, ulid, thiserror, tracing. Dev-dep: tempfile. |
| `src-tauri/biscuitcode-lsp/src/lib.rs` | `SessionId` (`lsp_*` ULID newtype) + `Language` enum (Rust, Typescript, Python, Go, Cpp) with `server_binary()` / `server_args()` / `install_command()` + `detect_languages_in()` (scans for marker files) + `LspSession` + `LspRegistry` + `LspError` (with `ServerMissing { binary, install_command }` for E013) |

`detect_languages_in` and the `Language` lookup tables are fully implemented and tested. `LspRegistry::spawn` / `write` / `shutdown` are TODO.

## Add to the top-level workspace

In `src-tauri/Cargo.toml`'s `[workspace]` table:

```toml
[workspace]
members = [
    ".",
    "biscuitcode-core",
    "biscuitcode-providers",
    "biscuitcode-db",
    "biscuitcode-pty",
    "biscuitcode-agent",
    "biscuitcode-lsp",            # add
]
```

In top-level `src-tauri/Cargo.toml`'s `[dependencies]`:

```toml
biscuitcode-lsp = { path = "biscuitcode-lsp" }
```

Verify with `cargo build -p biscuitcode-lsp` from `src-tauri/`. The two implemented tests (`detect_finds_rust_when_cargo_toml_present`, `install_command_for_clangd_is_apt`) should pass.

## What's still TODO for the Phase 7 coder

### 1. Implement `LspRegistry::spawn`

Per the inline TODO block in `src-tauri/biscuitcode-lsp/src/lib.rs`:

1. `which::which(language.server_binary())` — if `Err`, return `LspError::ServerMissing` with the `install_command` for the language. (The caller surfaces the E013 toast.)
2. `tokio::process::Command::new(...).args(language.server_args()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).current_dir(workspace_root).spawn()`
3. **Reader task:** parse `Content-Length: <N>\r\n\r\n<JSON>` framing from stdout. For each frame, emit a Tauri event `lsp_msg_in_<session_id>` with the JSON body.
4. **Writer task:** receive frames from a tokio mpsc channel, serialize each as `Content-Length: <N>\r\n\r\n<JSON>` and write to stdin.
5. **Stderr drain:** spawn a task that reads stderr line-by-line and `tracing::debug!`s each. Some LSP servers (rust-analyzer especially) emit progress info on stderr.
6. Store the child + reader/writer JoinHandles + writer-channel sender on `LspSession` (extend the struct — the public fields lock the API surface only).

`which` is not in `Cargo.toml` yet. Add: `which = "6"` (or current major).

### 2. Implement `LspRegistry::write`

Push the JSON-RPC frame onto the session's writer-channel sender. Returns `LspError::SessionNotFound` if the session doesn't exist.

### 3. Implement `LspRegistry::shutdown`

Per the doc-comment: send LSP `shutdown` request, then `exit` notification, await child exit with timeout (e.g., 2 s), then `child.kill()` + reap if it didn't exit cleanly.

### 4. Wire Tauri commands

```rust
#[tauri::command]
async fn lsp_spawn(
    state: tauri::State<'_, Arc<LspRegistry>>,
    language: Language, workspace_root: PathBuf,
) -> Result<SessionId, String> {
    state.spawn(language, workspace_root).map_err(|e| e.to_string())
}

#[tauri::command]
async fn lsp_write(
    state: tauri::State<'_, Arc<LspRegistry>>,
    id: SessionId, frame: serde_json::Value,
) -> Result<(), String> {
    state.write(&id, frame).await.map_err(|e| e.to_string())
}

// lsp_shutdown — same pattern
```

### 5. Frontend: monaco-languageclient + custom MessageTransports

`pnpm add monaco-languageclient vscode-languageclient`

In a new `src/lsp/client.ts`:

```ts
import { MonacoLanguageClient } from 'monaco-languageclient';
import { listen, invoke } from '@tauri-apps/api';

function makeTransports(sessionId: string): MessageTransports {
  return {
    reader: { /* listen to lsp_msg_in_<sessionId>, push to Monaco */ },
    writer: { /* invoke('lsp_write', { id: sessionId, frame }) */ },
  };
}
```

Diagnostics rendered as Monaco squigglies + problem count in status bar.

### 6. Missing-server dialog (no auto-install)

When `LspError::ServerMissing` fires, show a toast with the `install_command` and a copy-to-clipboard button. **NEVER auto-run** `rustup component add`, `npm i -g`, etc. Plan AC: `clangd` absent on a Mint VM triggers the toast with `sudo apt install clangd` copied; the app does not auto-run it.

### 7. Git panel (no pre-staged Rust — build from scratch)

`git2-rs` for reads, shell-out to `git` for writes. Side Panel Git pane:
- Files grouped by `staged` / `unstaged` / `untracked`
- Hunk-level stage/unstage (Monaco inline diff buttons)
- Commit message input + commit button
- Push/pull buttons that stream stdout to the Terminal panel
- Branch name in status bar → clickable branch switcher dropdown
- Gutter blame: off by default; settings toggle `editor.blame.gutter = true`. Uses `git2::BlameOptions` per visible line range; re-blames on commit/save; format: `hash[0..7] · author · relative-date` in 180px gutter

### 8. Preview Panel (no pre-staged — build from scratch)

Replace Phase 2 `PreviewPanel.tsx` shell. Per plan deliverables:
- **Markdown:** `react-markdown` + `remark-gfm` + `rehype-highlight` + `mermaid` + `rehype-katex`, live update
- **HTML:** sandboxed iframe (`sandbox="allow-scripts"`, NO forms or top-navigation), live-reload on save, devtools button
- **Images:** PNG, JPG, WebP, SVG, GIF with CSS zoom/pan; animated GIFs honor loop count
- **PDF:** `pdf.js` via `react-pdf`, single-page view with prev/next
- **Notebook (`.ipynb`):** read-only render — markdown cells as markdown, code cells as JetBrains Mono, outputs as text/mime-typed blocks. **No execution, no Run-all placeholder.**
- Auto-open rule: AI-edited `.md`, `.html`, `.svg`, image → open preview as split pane (Phase 6b emits the event; this phase consumes it).

### 9. Non-editor chat mentions

`@terminal-output` (active terminal tab's visible buffer), `@problems` (all LSP diagnostics in current workspace), `@git-diff` (output of `git diff` for staged + unstaged). Picker greys out mentions when their data source is empty.

### 10. Capabilities updates

`src-tauri/capabilities/shell.json` — add `which <binary>` + the LSP binary paths to the registry; **no wildcard args**.

### 11. Register error codes

- `E012 GitPushFailed` — `git push` exited non-zero
- `E013 LspServerMissing` — already locked in `LspError::ServerMissing`; just wire the toast
- `E014 LspProtocolError` — LSP server crashed or sent malformed JSON-RPC; restart-server button
- `E015 PreviewRenderFailed` — Markdown/HTML/PDF/image preview threw during render

### 12. Run the Phase 7 ACs

Most importantly:
- Open a Rust file → `rust-analyzer` starts → hover shows type; go-to-definition jumps; diagnostics appear
- Stage a hunk via inline diff button → status flips → commit → `git log -1` shows it
- Branch switcher updates status bar within 500 ms
- Markdown preview updates within 200 ms of typing
- Image preview displays 5 fixture formats; GIF animates
- Missing `clangd` triggers `E013` toast with copyable install command (NOT auto-run)
- HTML preview iframe cannot navigate away (`window.top.location` blocked by sandbox)
