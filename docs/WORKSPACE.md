# Cargo Workspace Structure (Source of Truth)

> Consolidated reference for the WSL2 coder. Each PHASE-N-MERGE.md says "add `biscuitcode-X` to the workspace" — this file shows the WHOLE workspace at v1.0 in one place. When ANY phase coder adds a crate, also update this file.

## Top-level `src-tauri/Cargo.toml`

After the Phase 1 coder runs `pnpm create tauri-app` and Phases 4/5/6/7 coders merge in their crates, the top-level `src-tauri/Cargo.toml` should look like this:

```toml
[package]
name = "biscuitcode"
version = "0.1.0"
description = "BiscuitCode — an AI coding environment, served warm."
authors = ["Corey Alan Schmidt"]
edition = "2021"
license = "MIT"
publish = false

[workspace]
members = [
    ".",
    "biscuitcode-core",          # Phase 1
    "biscuitcode-providers",     # Phase 5
    "biscuitcode-db",            # Phase 5
    "biscuitcode-pty",           # Phase 4
    "biscuitcode-agent",         # Phase 6a (read-only) + 6b (write)
    "biscuitcode-lsp",           # Phase 7
]

[lib]
name = "biscuitcode_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
# Tauri 2.10.x — pinned major.minor. Do not bump without re-running
# Phase 0 + Phase 1 ACs.
tauri = { version = "2.10", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
tauri-plugin-os = "2"
tauri-plugin-window-state = "2"
tauri-plugin-process = "2"
# Auto-update for AppImage path (Phase 9; .deb path uses GitHub Releases API).
tauri-plugin-updater = "2"

# Internal crates — listed in dependency order.
biscuitcode-core      = { path = "biscuitcode-core" }
biscuitcode-providers = { path = "biscuitcode-providers" }
biscuitcode-db        = { path = "biscuitcode-db" }
biscuitcode-pty       = { path = "biscuitcode-pty" }
biscuitcode-agent     = { path = "biscuitcode-agent" }
biscuitcode-lsp       = { path = "biscuitcode-lsp" }

# Async + serde
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Errors
thiserror = "1.0"
anyhow = "1.0"

[features]
# Default = nothing extra (lean .deb).
default = []
# Enable tray icon support — gated since libayatana-appindicator3-1 is
# only a Recommends, not a Depends, on Mint 22.
tray-icon = ["tauri/tray-icon"]
```

## Directory layout

```
src-tauri/
├── Cargo.toml                         # workspace root (above)
├── tauri.conf.json                    # patched per PHASE-1-MERGE.md
├── build.rs                           # pre-staged
├── src/
│   ├── main.rs                        # Phase 1 coder authors against scaffold
│   └── lib.rs                         # Phase 1 coder authors against scaffold
├── capabilities/
│   ├── core.json                      # pre-staged
│   ├── fs.json                        # pre-staged
│   ├── shell.json                     # pre-staged (empty; Phase 6a/7 add)
│   └── http.json                      # pre-staged (empty; Phase 5/6a add)
├── fonts/                             # Phase 1 coder downloads woff2s
├── icons/                             # Phase 8 coder rasterizes from packaging/
│
├── biscuitcode-core/                  # Phase 1 ✅ pre-staged
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── palette.rs                 ✅ complete
│       ├── errors.rs                  ✅ all 18 codes
│       └── secrets.rs                 ✅ skeleton (keyring TODO)
│
├── biscuitcode-providers/             # Phase 5/6a ✅ skeletons
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── trait.rs                   ✅ ModelProvider trait
│       ├── types.rs                   ✅ ChatEvent, Message, ToolSpec…
│       ├── anthropic/mod.rs           ⚠️  chat_stream TODO
│       ├── openai/mod.rs              ⚠️  chat_stream TODO
│       └── ollama/mod.rs              ⚠️  chat_stream TODO + Gemma 4 helpers
│
├── biscuitcode-db/                    # Phase 5 ✅ skeleton
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs                   ✅ ULIDs, row structs
│       ├── migrations.rs              ✅ runner + tests
│       └── migrations/
│           └── 0001_initial.sql       ✅ STRICT tables, indexes
│
├── biscuitcode-pty/                   # Phase 4 ✅ skeleton
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs                     ⚠️  PtyRegistry impls TODO
│
├── biscuitcode-agent/                 # Phase 6a/6b ✅ partial
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── tools/
│       │   ├── mod.rs                 ✅ Tool trait, ToolRegistry
│       │   ├── read_file.rs           ✅ COMPLETE
│       │   └── search_code.rs         ⚠️  walk + match impl TODO
│       └── executor/
│           └── mod.rs                 ⚠️  read-only dispatch complete;
│                                          write/shell + snapshot TODO (6b)
│
└── biscuitcode-lsp/                   # Phase 7 ✅ skeleton
    ├── Cargo.toml
    └── src/
        └── lib.rs                     ⚠️  spawn/write/shutdown TODO
```

Frontend layout (under repo root):

```
src/
├── main.tsx                           ✅ pre-staged (createRoot + i18n init)
├── App.tsx                            ✅ pre-staged (4-region shell)
├── index.css                          ✅ pre-staged (Tailwind + CSS vars)
├── theme/
│   ├── tokens.ts                      ✅ brand palette TS
│   └── fonts.css                      ✅ @font-face stubs
├── errors/
│   ├── types.ts                       ✅ all 18 codes typed
│   └── ErrorToast.tsx                 ✅ catalogued-only renderer
├── locales/
│   └── en.json                        ✅ all keys for Phase 1+2
├── shortcuts/
│   └── global.ts                      ✅ 11-shortcut table + chord support
├── state/
│   └── panelsStore.ts                 ✅ Zustand + localStorage
├── layout/
│   └── WorkspaceGrid.tsx              ✅ 4-region resizable
└── components/
    ├── ActivityBar.tsx                ✅ files/search/git/chats/settings
    ├── SidePanel.tsx                  ✅ shell (Phase 3/5/7/8 fill in)
    ├── EditorArea.tsx                 ✅ shell (Phase 3 fills in Monaco)
    ├── TerminalPanel.tsx              ✅ shell (Phase 4)
    ├── ChatPanel.tsx                  ✅ shell (Phase 5)
    ├── AgentActivityPanel.tsx         ✅ shell (Phase 6a)
    ├── PreviewPanel.tsx               ✅ shell (Phase 7)
    ├── StatusBar.tsx                  ✅ placeholders (Phase 5/7 fill in)
    ├── ToastLayer.tsx                 ✅ info + error toast layer
    └── CommandPalette.tsx             ✅ Ctrl+Shift+P modal
```

## Where each phase's deliverables live

| Phase | Crate(s) modified | Frontend touched |
|---|---|---|
| 0 | none (env only) | none |
| 1 | `biscuitcode-core` | tokens, fonts, error infra, layout shell |
| 2 | none new | `WorkspaceGrid` + 8 components + shortcuts + toast + palette |
| 3 | none new | `EditorArea` (Monaco), `SidePanel:Files`, find-in-files |
| 4 | `biscuitcode-pty` (impl) | `TerminalPanel` (xterm.js wiring) |
| 5 | `biscuitcode-providers` (Anthropic impl), `biscuitcode-db` (impl) | `ChatPanel` (rewrite with virtuoso), `SettingsProviders` |
| 6a | `biscuitcode-providers` (OpenAI + Ollama impls), `biscuitcode-agent` (read tools + executor) | `AgentActivityPanel`, `@` mention picker |
| 6b | `biscuitcode-agent` (write tools + snapshot + rewind) | inline-edit split-diff, confirmation modal, rewind UI |
| 7 | `biscuitcode-lsp` (impl) | `SidePanel:Git`, `PreviewPanel`, gutter blame |
| 8 | none new | `OnboardingModal`, `SettingsPage`, theme CSS, conversation branching UI, snapshot cleanup |
| 9 | none new | a11y audit, `docs/ERROR-CATALOGUE.md` audit, auto-update wiring |
| 10 | none new | `.github/workflows/release.yml`, packaging, screenshots |

## Verification

After all crates are added:

```bash
cd src-tauri && cargo build --workspace
cd src-tauri && cargo test --workspace
```

Both should pass against the pre-staged skeletons (read-only-tool tests, palette assertions, migration runner, Gemma 4 RAM tier helpers, etc.) before the per-phase coder fills in TODO blocks.
