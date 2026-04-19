# Phase 4 — Merging the PTY backend crate

> Read this when starting Phase 4 (Terminal — xterm.js + portable-pty). The `biscuitcode-pty` crate skeleton is pre-staged with the public API surface locked; this guide explains how to land it in the workspace and what's still TODO.

## Pre-staged files (Phase 4 foundation)

| Path | What |
|---|---|
| `src-tauri/biscuitcode-pty/Cargo.toml` | Workspace member crate: `portable-pty 0.8`, tokio (rt-multi-thread + sync + io-util), parking_lot, ulid, thiserror, tracing |
| `src-tauri/biscuitcode-pty/src/lib.rs` | `SessionId` (ULID newtype, `term_*` prefix) + `PtySession` + `PtyRegistry` + `detect_shell()` + `PtyError`. Tests pass for `detect_shell` and `SessionId` uniqueness; `PtyRegistry` methods return `PtyError::NotImplemented`. |

## Add to the top-level workspace

In `src-tauri/Cargo.toml`'s `[workspace]` table:

```toml
[workspace]
members = [
    ".",
    "biscuitcode-core",
    "biscuitcode-providers",
    "biscuitcode-db",
    "biscuitcode-pty",          # add
]
```

In the top-level `src-tauri/Cargo.toml`'s `[dependencies]`:

```toml
biscuitcode-pty = { path = "biscuitcode-pty" }
```

Verify with `cargo build -p biscuitcode-pty` from `src-tauri/` — should compile against the stub.

## What's still TODO for the Phase 4 coder

### 1. Implement `PtyRegistry::open` / `write_input` / `resize` / `close`

Inside `src-tauri/biscuitcode-pty/src/lib.rs`. The numbered steps in the doc-comment on `PtyRegistry::open` are the spawn order:

1. `portable_pty::native_pty_system().openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })`
2. `master.spawn_command(CommandBuilder::new(shell).cwd(cwd).env_clear().envs(...))`
3. Spawn reader Tokio task: `master.try_clone_reader().read_to_buf` → Tauri emit `terminal_data_<id>`
4. Spawn writer Tokio task: receives from input channel, writes to `master.try_clone_writer()`
5. Store the master + child + JoinHandles in `PtySession` (extend the struct — current fields lock the public surface only)

The `PtySession` struct intentionally omits the runtime fields (master, child, task handles) so the public API stays stable while the impl is filled in. Add them as private fields on the struct.

### 2. Wire Tauri commands

Per `docs/plan.md` Phase 4 deliverables, the frontend calls four commands. In `src-tauri/src/main.rs`:

```rust
#[tauri::command]
async fn terminal_open(
    state: tauri::State<'_, Arc<PtyRegistry>>,
    shell: String, cwd: PathBuf, rows: u16, cols: u16,
) -> Result<SessionId, String> {
    state.open(shell, cwd, rows, cols).map_err(|e| e.to_string())
}
// terminal_input, terminal_resize, terminal_close — same pattern
```

Register the registry as Tauri-managed state at app launch:

```rust
.manage(Arc::new(PtyRegistry::new()))
.invoke_handler(tauri::generate_handler![
    terminal_open, terminal_input, terminal_resize, terminal_close,
])
```

### 3. Frontend: replace Phase 2 `TerminalPanel.tsx` shell with real xterm.js

Per plan deliverables: tabbed `xterm.js` instances with `@xterm/addon-fit`, `@xterm/addon-web-links`, `@xterm/addon-search`, `@xterm/addon-webgl` (canvas fallback). Wire each tab to a `SessionId` returned by `terminal_open`; subscribe to `terminal_data_<session_id>` Tauri event for output; send keystrokes via `terminal_input`.

`pnpm add @xterm/xterm @xterm/addon-fit @xterm/addon-web-links @xterm/addon-search @xterm/addon-webgl`.

### 4. Custom link provider: `path/to/file:line[:col]`

Register an `xterm.js` link provider that matches the `path:line` regex and emits an `open_file_at` event consumed by the Phase 3 editor. Plan AC: clicking `src/main.rs:12` opens the file at line 12.

### 5. Wire the Phase 2 ``Ctrl+` `` placeholder to the real focus action

The shortcut layer fires a placeholder toast in Phase 2; this phase replaces the handler with one that focuses the active terminal tab.

### 6. Register error code `E003 PtyOpenFailed`

Add the variant to the Rust enum in `biscuitcode-core::errors` and the TS union in `src/errors/types.ts`. Trigger test in `tests/error-catalogue.spec.ts` should force a PTY-open failure (e.g., shell binary `/bin/does-not-exist`).

### 7. Run the Phase 4 ACs

Most importantly:
- `pgrep -f 'biscuitcode.*bash'` returns no orphans 2s after closing a tab
- Five concurrent `yes > /dev/null` terminals — total CPU under one core's worth, no crash over 60s
- Resize the panel → `tput lines && tput cols` matches the new dimensions
