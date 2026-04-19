# BiscuitCode

> *An AI coding environment, served warm.*

A VS Code-class AI coding environment for **Linux Mint 22 XFCE**, built on **Tauri 2.x + Rust + React 18 + TypeScript**, with first-class support for Claude, ChatGPT, and local Gemma 4 / Ollama models.

## Screenshots

<!-- Screenshots below use the BiscuitCode Warm (dark) theme. -->
<!-- Replace placeholder paths with real screenshots before tagging v1.0.0. -->

**Main editor with chat panel**
![Main editor with chat panel](docs/screenshots/main-editor-chat.png)

**Agent Activity mid-run — tool cards streaming**
![Agent Activity mid-run](docs/screenshots/agent-activity.png)

**Markdown preview split pane**
![Markdown preview split pane](docs/screenshots/preview-split.png)

> Screenshots are taken on Mint 22.1 XFCE, 1920x1080, BiscuitCode Warm theme.

## Install

### Recommended: `.deb` package

```bash
# Download from the releases page:
# https://github.com/Coreyalanschmidt-creator/biscuitcode/releases/latest

# Verify GPG signature (recommended):
gpg --verify BiscuitCode_<version>_amd64.deb.asc BiscuitCode_<version>_amd64.deb

# Install:
sudo dpkg -i BiscuitCode_<version>_amd64.deb
sudo apt -f install        # resolves any missing dependencies

# Launch from Whisker menu → Development → BiscuitCode.
```

To uninstall:
```bash
sudo apt remove biscuit-code
```

Your settings and conversations are preserved under `~/.config/biscuitcode/` and `~/.local/share/biscuitcode/`.

### Alternative: AppImage (portable)

> **Note: `libfuse2t64` required on Ubuntu 24.04 / Mint 22.**
> Without it the AppImage will fail with "FUSE not found". Install it first:
> ```bash
> sudo apt install libfuse2t64
> ```

```bash
chmod +x BiscuitCode-<version>-x86_64.AppImage
./BiscuitCode-<version>-x86_64.AppImage
```

The AppImage supports in-app auto-update (Settings → About → Check for updates).

For full install instructions, troubleshooting, and the first-run onboarding walkthrough see **[`docs/INSTALL.md`](docs/INSTALL.md)**.

## Features

- **Monaco editor** — full VS Code editor engine: multi-cursor, minimap, IntelliSense, split panes.
- **AI chat** — stream responses from Claude (Anthropic), ChatGPT (OpenAI), or Gemma 4 running locally via Ollama.
- **Agent mode** — ReAct loop with read/write/search/shell tools. Pause, rewind, and snapshot restoration.
- **Inline edit** (`Ctrl+K Ctrl+I`) — select code, describe a change, accept the diff in a split-diff editor.
- **Integrated terminal** — xterm.js + PTY. Click `src/main.rs:12` in terminal output to jump to that line.
- **Git panel** — stage hunks, commit, view history.
- **Find in files** — workspace-wide search.
- **Markdown preview** — live side-by-side render.
- **API keys stored in system keyring** — never in plaintext, never in env vars.

## Development

This project uses the **C.Alan method** — a four-stage pipeline (Research → Plan → Review → Code) with four behavioral laws (Think · Simplify · Stay Surgical · Verify). See `CLAUDE.md` for the project operating system and `docs/plan.md` for the phased implementation plan.

### Setup

You must develop on **WSL2 + Ubuntu 24.04** (Windows-native builds are not supported; the target is Linux Mint 22 XFCE).

1. See **[`docs/DEV-SETUP.md`](docs/DEV-SETUP.md)** for the full bootstrap procedure.
2. Quick path:
   ```bash
   bash scripts/bootstrap-wsl.sh         # install system deps
   bash scripts/bootstrap-toolchain.sh   # install rust, node, pnpm, cargo-tauri
   pnpm install
   pnpm tauri dev
   ```

### Project layout

```
biscuitcode/
├── CLAUDE.md                 # Project operating system + Four Laws
├── README.md                 # This file
├── docs/
│   ├── plan.md               # Phased implementation plan (source of truth)
│   ├── DEV-SETUP.md          # WSL2 + toolchain install
│   ├── INSTALL.md            # End-user install guide
│   ├── RELEASE.md            # Release smoke-test checklist
│   ├── ERROR-CATALOGUE.md    # Error codes E001–E018
│   └── adr/                  # Architecture Decision Records
├── scripts/
│   ├── bootstrap-wsl.sh      # Idempotent apt install
│   ├── bootstrap-toolchain.sh
│   ├── check-i18n.js         # i18n key coverage check
│   └── render-icons.sh       # Render PNG icon sizes from master SVG
├── src-tauri/                # Rust backend
├── src/                      # React + TypeScript frontend
├── tests/                    # Unit, e2e, a11y, cold-launch tests
└── packaging/                # Icons, desktop entry, AppImage assets
```

## License

MIT. See [`LICENSE`](LICENSE).
