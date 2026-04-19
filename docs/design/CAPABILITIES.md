# Design — Tauri v2 Capability ACL

> Architecture spec consumed by Phase 1 (skeleton: core/fs/shell/http JSON files), Phase 3 (fs scope expansion to workspace root), Phase 5 (http scope for Anthropic), Phase 6a (http scope for OpenAI + Ollama, shell scope for `ollama`), and Phase 7 (shell scope for `which` + LSP binaries; preview iframe sandbox).
>
> The fundamental rule: **deny by default; grant the minimum the current feature needs; never use wildcard scopes.** A web search will turn up Tauri v1 allowlist tutorials — those do not apply. v2 uses a per-file capability ACL system.

## Why this matters

The Tauri v2 capability system is the security perimeter between the webview (untrusted code that can run any JS the LLM emits) and the Rust backend (trusted code with full FS/network/shell access). Get this wrong and a prompt-injected response could exfiltrate `~/.ssh/id_rsa`.

Every feature phase needs to answer: *what's the minimum permission expansion this phase requires, scoped to what?*

## File layout

```
src-tauri/
├── capabilities/
│   ├── core.json     # default app capabilities (always loaded)
│   ├── fs.json       # filesystem access scopes
│   ├── shell.json    # whitelisted external commands
│   └── http.json     # HTTP fetch allowlist
└── tauri.conf.json   # references all four capability files in app.security.capabilities
```

`tauri.conf.json` excerpt:

```json
{
  "app": {
    "security": {
      "capabilities": [
        "core",
        "fs",
        "shell",
        "http"
      ]
    }
  }
}
```

## `core.json` — Phase 1

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "core",
  "description": "Core capabilities required for app boot and IPC",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:event:default",
    "core:window:allow-set-title",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-toggle-maximize",
    "core:window:allow-minimize",
    "core:window:allow-start-dragging"
  ]
}
```

What's NOT in here:
- `core:window:allow-set-position` / `allow-set-size` — not used (we let WM handle); revisit if multi-window in v1.1.
- `core:webview:*` — we don't programmatically open new webviews.

## `fs.json` — grows from Phase 1 → Phase 3

### Phase 1 (initial — config dirs only)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "fs",
  "description": "Filesystem access — config and cache dirs only at boot",
  "windows": ["main"],
  "permissions": [
    {
      "identifier": "fs:allow-read-text-file",
      "allow": [
        { "path": "$APPCONFIG/**" },
        { "path": "$APPDATA/**" },
        { "path": "$APPCACHE/**" }
      ]
    },
    {
      "identifier": "fs:allow-write-text-file",
      "allow": [
        { "path": "$APPCONFIG/**" },
        { "path": "$APPDATA/**" },
        { "path": "$APPCACHE/**" }
      ]
    }
  ]
}
```

`$APPCONFIG` resolves to `~/.config/biscuitcode/`, `$APPDATA` to `~/.local/share/biscuitcode/`, `$APPCACHE` to `~/.cache/biscuitcode/`. These are the only paths writable until a workspace is opened.

### Phase 3 (workspace open)

When the user opens a folder via `fs_open_folder()`, the Rust handler **patches the fs scope at runtime** to add the workspace root:

```rust
use tauri::scope::Scopes;

#[tauri::command]
async fn fs_open_folder(app: AppHandle) -> Result<WorkspaceId, FsError> {
    let path = ask_user_for_folder().await?;
    let canonical = path.canonicalize()?;

    // Update the fs:allow-read-text-file and fs:allow-write-text-file scopes
    // to include the new workspace root. Previous workspace's scope is REMOVED.
    let scope = app.fs_scope();
    scope.allow_directory(&canonical, true)?;
    if let Some(prev) = previous_workspace_root() {
        scope.forbid_directory(&prev, true)?;
    }

    Ok(WorkspaceId::from(canonical))
}
```

Notes:
- `recursive: true` so subdirectories are scoped.
- We also need binary-file variants (`fs:allow-read-binary-file`, `fs:allow-write-binary-file`) added to capabilities for image preview, PDF, etc.
- We never add wildcard `**` outside of these explicit roots.

### Always denied (even with workspace trust)

Hard-coded path patterns the agent never touches:
- `**/.git/**` — git internals
- `**/node_modules/**`
- `**/target/**`
- `**/.cache/**` and `**/.tmp/**`
- `**/.env*` — secrets convention
- `**/*.pem`, `**/*.key`

Enforced in `biscuitcode-agent` write tools, NOT in the capability layer (so users can manually open these in the editor; only the agent is restricted).

## `shell.json` — grows from Phase 1 (empty) → Phase 6a + 7

### Phase 1 (empty; no shell access)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "shell",
  "description": "Shell access — no commands allowed at boot",
  "windows": ["main"],
  "permissions": []
}
```

### Phase 6a (Ollama install + management)

```json
{
  "permissions": [
    {
      "identifier": "shell:allow-execute",
      "allow": [
        {
          "name": "ollama",
          "cmd": "ollama",
          "args": [
            { "validator": "^(list|show|--version)$" },
            { "validator": "^pull$" },
            { "validator": "^[a-z][a-z0-9._:-]*$" },
            { "validator": "^serve$" }
          ]
        },
        {
          "name": "ollama-install",
          "cmd": "sh",
          "args": [
            { "validator": "^-c$" },
            { "validator": "^curl -fsSL https://ollama.com/install.sh \\| sh$" }
          ]
        }
      ]
    }
  ]
}
```

The `ollama-install` command is intentionally locked to the verbatim install script line — it cannot be parameterized to run arbitrary other shell strings.

### Phase 7 (LSP server discovery)

Add to `shell:allow-execute`:

```json
{
  "name": "which",
  "cmd": "which",
  "args": [
    { "validator": "^(rust-analyzer|typescript-language-server|pyright-langserver|gopls|clangd)$" }
  ]
},
{
  "name": "rust-analyzer",
  "cmd": "rust-analyzer",
  "args": []
},
{
  "name": "typescript-language-server",
  "cmd": "typescript-language-server",
  "args": [{ "validator": "^--stdio$" }]
}
```

Each LSP child gets its own entry — no `cmd: "*"` wildcard.

### `xdg-open` (Phase 3 — Reveal in File Manager + http link clicks)

```json
{
  "name": "xdg-open",
  "cmd": "xdg-open",
  "args": [
    {
      "validator": "^(https?://|/[^\\s]+)"
    }
  ]
}
```

Only http(s) URLs and absolute filesystem paths.

### Phase 6b (`run_shell` agent tool)

`run_shell` does NOT add a new shell capability. The agent's run_shell tool calls into the same `shell:allow-execute` registry as everything else — meaning the agent can ONLY run commands that are pre-registered in the capability file. The model cannot inject new commands.

This is a deliberate constraint: the v1 agent cannot run arbitrary user commands, only the curated whitelist (which is a small set: `ollama`, `which`, `xdg-open`, the LSP servers, `git` for status reads, etc.). Generalized "shell access" is a v1.1 conversation that probably involves a separate sandbox (firejail, bubblewrap) and explicit per-workspace opt-in.

## `http.json` — grows from Phase 1 (empty) → Phase 5 + 6a

### Phase 1 (empty)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "http",
  "description": "HTTP access — no hosts allowed at boot",
  "windows": ["main"],
  "permissions": []
}
```

### Phase 5 (Anthropic)

```json
{
  "permissions": [
    {
      "identifier": "http:default",
      "allow": [
        { "url": "https://api.anthropic.com/**" }
      ]
    }
  ]
}
```

### Phase 6a (OpenAI + Ollama + GitHub for updates)

Add to the allow list:

```json
[
  { "url": "https://api.anthropic.com/**" },
  { "url": "https://api.openai.com/**" },
  { "url": "http://localhost:11434/**" },
  { "url": "https://api.github.com/repos/Coreyalanschmidt-creator/biscuitcode/releases/**" }
]
```

The GitHub URL is added in Phase 9 alongside auto-update wiring; listed here so the full v1.0 allowlist is visible in one place.

## Preview iframe sandbox (Phase 7)

The HTML preview pane embeds user content (markdown that may contain raw HTML, or full HTML files) in an iframe. The sandbox attribute is the security boundary:

```html
<iframe
  sandbox="allow-scripts"
  src="<blob: URL with the rendered HTML>"
  csp="default-src 'self'; script-src 'unsafe-inline'; style-src 'self' 'unsafe-inline'"
></iframe>
```

What's deliberately NOT allowed:
- `allow-same-origin` — without it, the iframe can't reach localStorage or our cookies.
- `allow-forms` — no form submission to arbitrary endpoints.
- `allow-top-navigation` — `window.top.location = '...'` is blocked.
- `allow-popups` — no `window.open` to surprise the user.
- `allow-modals` — no `alert`/`confirm` from preview content.

A test in `tests/e2e/preview-sandbox.spec.ts` (Phase 7 deliverable) loads a malicious sample HTML attempting each of these vectors and asserts each fails.

## Capability-upgrade handling across versions

If we ever add a permission BiscuitCode v1.0 didn't have (e.g., camera access for a future video-input feature in v2.0), the user must explicitly re-grant. Tauri v2's behavior is:

- Capability files are bundled into the app — they cannot be modified at runtime by the model.
- A new app version with new capabilities triggers re-confirmation on first launch.

Phase 9 acceptance criterion exercises this path with a synthetic version-bump test.

## What's tested

- **Capability JSON schema validation** in CI: `tauri info` (or equivalent) catches schema errors before runtime.
- **Per-phase smoke**: each phase that expands capabilities has an AC asserting the new permission is granted (`fs.read` on the workspace root succeeds) AND a paired AC asserting the old denial still holds (`fs.write` on `/etc/passwd` fails with the typed error).
- **The deliberate-overscope test** in Phase 9: a synthetic capability file with `cmd: "*"` is rejected by our build pipeline with a clear error before it ever reaches CI.

## Things explicitly NOT in scope for v1

- Per-workspace capability profiles. v1 has one profile that applies everywhere; per-workspace trust is a boolean toggle, not a fine-grained permission set.
- User-editable capability files. The bundled files are immutable; no setting to "expand http allowlist with custom URL." A v1.1 power-user feature.
- Custom MIME-type permission blocks. We don't register custom MIME types yet.
- Camera/microphone/clipboard-monitor capabilities. None are needed in v1.
