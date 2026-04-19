# BiscuitCode

> *An AI coding environment, served warm.*

A VS Code-class AI coding environment for **Linux Mint 22 XFCE**, built on **Tauri 2.x + Rust + React 18 + TypeScript**, with first-class support for Claude, ChatGPT, and local Gemma 4 / Ollama models.

**Status:** Pre-v1 (planning complete, code phases 0–10 ready to execute). See `docs/plan.md` for the phased implementation plan.

## Build artifacts (when v1 ships)

- `biscuitcode_1.0.0_amd64.deb` — primary distribution for Debian/Ubuntu/Mint
- `BiscuitCode-1.0.0-x86_64.AppImage` — portable
- GPG signatures + SHA256SUMS

## Development

This project uses the **C.Alan method** — a four-stage pipeline (Research → Plan → Review → Code) with four behavioral laws (Think · Simplify · Stay Surgical · Verify). See `CLAUDE.md` for the project operating system and `docs/plan.md` for the phased implementation plan.

### Setup

You must develop on **WSL2 + Ubuntu 24.04** (the project's plan and CLAUDE.md require this — Windows-native builds are explicitly forbidden because the target is Linux Mint 22 XFCE / Ubuntu 24.04 noble).

1. See **[`docs/DEV-SETUP.md`](docs/DEV-SETUP.md)** for the full bootstrap procedure.
2. Quick path:
   ```bash
   bash scripts/bootstrap-wsl.sh         # install system deps
   bash scripts/bootstrap-toolchain.sh   # install rust, node, pnpm, cargo-tauri
   pnpm install
   pnpm tauri dev
   ```

### Project layout (planned)

```
biscuitcode/
├── CLAUDE.md                 # Project operating system + Four Laws
├── README.md                 # This file
├── .claude/
│   ├── agents/               # researcher, planner, reviewer, coder
│   └── commands/             # /research, /plan, /review-plan, /run-phase, /run-all
├── docs/
│   ├── vision.md             # Original super prompt (locked)
│   ├── research-r1.md        # Round 1 research dossier
│   ├── research-r2.md        # Round 2 research (challenger / gaps)
│   ├── plan-r1.md            # Round 1 phased plan + reviewer audit
│   ├── plan-r2.md            # Round 2 phased plan + reviewer audit
│   ├── plan.md               # ★ Final synthesized plan (source of truth)
│   ├── DEV-SETUP.md          # WSL2 + toolchain install
│   ├── adr/
│   │   └── 0001-no-stronghold.md   # Why we don't use tauri-plugin-stronghold
│   └── ERROR-CATALOGUE.md    # (built incrementally Phases 1–9)
├── scripts/
│   ├── bootstrap-wsl.sh
│   └── bootstrap-toolchain.sh
├── src-tauri/                # Rust backend (created in Phase 1)
├── src/                      # React frontend (created in Phase 1)
└── packaging/                # .deb + AppImage assets (Phase 10)
```

## License

MIT. See `LICENSE` (added in Phase 1).
