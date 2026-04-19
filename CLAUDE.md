# BiscuitCode — Project Operating System

> *An AI coding environment, served warm.*

This repo builds **BiscuitCode**, a VS Code-class AI coding environment natively optimized for Linux Mint 22 XFCE. It uses the **C.Alan method** for all substantive feature work.

## A. The Four Laws (apply to every action, every stage)

1. **Think Before Acting** — State assumptions. Ask when unclear. Present alternatives. Don't silently pick.
2. **Simplicity First** — Minimum code that solves the problem. No speculative features, abstractions, or configurability. If it could be half as long, rewrite it.
3. **Stay Surgical** — Touch only what the request requires. Don't "improve" adjacent code. Match existing style. Clean up only your own orphans.
4. **Verify, Don't Declare** — Define testable success criteria upfront. Run tests. Don't mark things complete without verification.

## B. The Four-Stage Pipeline

1. **researcher** → writes `docs/research.md`
2. **planner**   → writes `docs/plan.md` from research
3. **reviewer**  → audits and updates `docs/plan.md` (fresh context)
4. **coder**     → executes ONE phase of `docs/plan.md` at a time (fresh context per phase)

## Orchestration rules

- Never code from a raw vision statement. Always pass through research → plan → review first. The raw vision lives at `docs/vision.md`.
- Never run more than one phase in a single `coder` invocation. One phase = one fresh subagent.
- If a coder reports a deviation affecting later phases, pause and re-invoke `reviewer` before continuing.
- Artifacts live at `docs/research.md` and `docs/plan.md`. Keep them committed.
- For trivial tasks (typo fixes, one-line edits), the pipeline is overkill. Use judgment.

## Commands

- `/research <topic>` — stage 1
- `/plan`              — stage 2
- `/review-plan`       — stage 3
- `/run-phase <N>`     — execute phase N with a fresh coder
- `/run-all <topic>`   — full pipeline with interactive checkpoints

## Status lifecycle

`Not Started` → `In Progress` → `Complete` | `Partial` | `Blocked` | `Needs Replanning`.
Only the coder executing a phase may change its status.

---

## Project-Specific Rules

### Locked identity
Do not re-prompt on these. They are final.

| Setting | Value |
|---|---|
| Project name | **BiscuitCode** |
| Tagline | *An AI coding environment, served warm.* |
| Executable | `biscuitcode` (lowercase) |
| Display name | `BiscuitCode` |
| Bundle ID | `io.github.Coreyalanschmidt-creator.biscuitcode` |
| Config dir | `~/.config/biscuitcode/` |
| Data dir | `~/.local/share/biscuitcode/` |
| Cache dir | `~/.cache/biscuitcode/` |
| License | MIT |
| Target platform | Linux Mint 22 XFCE, x86_64 (X11 + Wayland-XFCE) |
| Stack | Tauri 2.x + React 18 + TypeScript 5 + Vite + Tailwind 3 |
| Editor | Monaco via `@monaco-editor/react` |
| Terminal | xterm.js + Rust `portable-pty` |
| Package manager | **pnpm** |

### Cross-platform development constraint

The maintainer develops on **Windows 10**. The app targets **Linux Mint 22 XFCE**. Resolution:

- **Research / Plan / Review stages**: run from Windows Claude Code session. Pure markdown, no builds.
- **Code stages**: MUST run from **WSL2 + Ubuntu 24.04** (Mint 22 is Ubuntu 22.04-derived; 24.04 gives newer toolchain with binary-compatible output). Tauri GUI works via WSLg. Source must live in WSL's native filesystem (e.g., `~/biscuitcode/`), not `/mnt/c/`, for build speed and inotify reliability.
- **Release validation**: every phase-complete checkpoint that produces a `.deb` must be installed and smoke-tested on an actual Mint 22 XFCE machine (maintainer has a secondary machine for this) before marking `Complete`.

A coder invoked from Windows cannot satisfy Phase 1+ acceptance criteria. If the session lacks WSL2 access, the coder must stop and report — not attempt partial Windows-native builds.

### Brand tokens (non-negotiable)

```
/* Biscuit — warm gold */
--biscuit-500: #E8B04C    (PRIMARY ACCENT)
--biscuit-50:  #FDF7E6   --biscuit-100: #FAE8B3   --biscuit-200: #F5D380
--biscuit-300: #F0C065   --biscuit-400: #EBB553   --biscuit-600: #C7913A
--biscuit-700: #9E722A   --biscuit-800: #74531E   --biscuit-900: #4A3413

/* Cocoa — warm dark chrome */
--cocoa-700: #1C1610      (PRIMARY DARK BG)
--cocoa-900: #080504      (DEEPEST DARK)
--cocoa-50:  #F6F0E8   --cocoa-100: #E0D3BE   --cocoa-200: #B9A582
--cocoa-300: #8A7658   --cocoa-400: #584938   --cocoa-500: #3A2F24
--cocoa-600: #28201A   --cocoa-800: #120D08

/* Semantic */
--accent-ok:    #6FBF6E   (sage)
--accent-warn:  #E8833E   (terracotta)
--accent-error: #E06B5B   (salmon)
```

### Typography (non-negotiable)

- **UI**: Inter 400/500/600. Self-hosted woff2 in `src-tauri/fonts/`. **No `system-ui` fallbacks in visible chrome.**
- **Code/Terminal**: JetBrains Mono 400/500. Self-hosted. Ligatures on by default.
- **Sizes**: 12/13/14/16 px. Line-height 1.5.

### Namespace collisions to avoid

- Rust crates `biscuit` and `biscuit-auth` exist for authorization tokens. **Do not** use those names for internal crates. Prefix with `biscuitcode-*` (e.g., `biscuitcode-core`, `biscuitcode-agent`).
- The `CodeBiscuits` VS Code extension exists. Reserved — don't reuse that name.

### Security posture

- API keys in system keyring via libsecret (`keyring` crate). Never plaintext config. Never env vars. Never logs. Never crash reports.
- Tauri allowlist: minimum-scope. No `shell.open` with arbitrary URLs. No `fs` outside workspace + config dirs.
- Telemetry off by default. If on, anonymous crashes only — no prompt content, file contents, or responses.

### Architecture Decision Records (ADRs)

Decisions that future maintainers need to be warned away from re-litigating live in `docs/adr/`. Read these before searching the web for "how to do X in Tauri":

- **`docs/adr/0001-no-stronghold.md`** — `tauri-plugin-stronghold` is deprecated and will be removed in Tauri v3. The `keyring` crate is the only forward-compatible secrets path. A web search for "Tauri secrets" will mislead.

### Resolved post-synthesis (2026-04-18)

Two corrections landed AFTER `docs/plan.md` was first written. Both are recorded in the plan's `## Synthesis Log`:

- **Q16 RESOLVED** — Gemma 4 tags verified against `https://ollama.com/library/gemma4`. Real tags are `gemma4:e2b` (2.3B effective, 7.2GB), `gemma4:e4b` (4.5B effective, 9.6GB, also `:latest`), `gemma4:26b` (MoE 25.2B/3.8B active, 18GB), `gemma4:31b` (30.7B, 20GB). Min Ollama version: `0.20.0`. All Gemma 4 variants natively support function calling.
- **Icon Concept naming** — `packaging/icons/biscuitcode-icon-concepts.html` is the authoritative reference and contains exactly THREE concepts: A ("The Prompt"), B ("The Braces"), C ("The Biscuit"). Vision text and r1/r2 say "Concept D" — that label is wrong. Treat all "Concept D" references as meaning the biscuit-shape **Concept C**.

### Success signals

- Diffs touch only files the current phase named.
- Clarifying questions arrive before implementation, not after.
- Rewrites from over-engineering are rare.
- `plan.md` phases small enough a single coder session finishes cleanly.
- `.deb` installs cleanly on fresh Mint 22 XFCE VM; uninstalls cleanly via `apt remove biscuitcode`.

### Pre-staged artifacts (authored before Phase 0)

These exist in the repo before Phase 0 runs. Coders **read and possibly edit** them; they are NOT marked Complete in `plan.md` until the relevant phase's coder verifies them.

| File | Phase | Purpose |
|---|---|---|
| `scripts/bootstrap-wsl.sh` | 0 | Idempotent apt install with pre-flight checks |
| `scripts/bootstrap-toolchain.sh` | 0 | rustup + nvm + node + pnpm + cargo-tauri-cli |
| `docs/DEV-SETUP.md` | 0 | WSL2 install procedure |
| `LICENSE` | 1 | MIT |
| `docs/adr/0001-no-stronghold.md` | 1 | Stronghold deprecation warning |
| `.gitignore` | 1 | Tauri / Rust / Node / pnpm / paranoia |
| `.github/workflows/ci.yml` | 1 / 10 | CI skeleton; gates flip from warning → error per phase |
| `tests/fixtures/canonical-tool-prompt.md` | 6a | 3-tool-call fixture for the render-gate |
| `packaging/icons/biscuitcode.svg` | 8 | Concept A master (verbatim from ref HTML) |
| `packaging/icons/biscuitcode-icon-concepts.html` | reference | Authoritative design source |
| `docs/CONVERSATION-EXPORT-SCHEMA.md` | 8 | Versioned JSON schema |
| `docs/ERROR-CATALOGUE.md` | 9 | E001–E018 skeleton; codes claim slots in earlier phases |
| `docs/RELEASE.md` | 10 | Smoke-test pointer to Global Acceptance |
| `docs/design/PROVIDER-TRAIT.md` | 5 / 6a | ModelProvider trait + ChatEvent enum spec |
| `docs/design/AGENT-LOOP.md` | 6a / 6b | ReAct + pause + snapshot/rewind architecture |
| `docs/design/CAPABILITIES.md` | 1 / 3 / 5 / 6a / 7 | Deny-by-default capability ACL design |
