# Morning Brief — BiscuitCode autonomous overnight session

> Authored: 2026-04-18, autonomous Claude Code session.
> Purpose: catch the maintainer up on what was done, what's next, and any decisions that need their input before code phases begin.

## TL;DR

Bootstrapped the C.Alan pipeline in this repo and ran a **2-round research + 2-round planning + 2-round review + synthesis** flow as you requested. Final source-of-truth is **`docs/plan.md`** (12 phases, ~16 working days). Pre-staged Phase 0 and Phase 1 file-only deliverables so when you're ready, `bash scripts/bootstrap-wsl.sh` from inside WSL2 starts the chain without you having to write the scripts yourself.

**Commit count this session:** 11+ commits on `main` (no remote, nothing pushed). Initial 9 covered the C.Alan pipeline through synthesis + the first round of pre-staged Phase 0/1 deliverables; subsequent commits resolved Q16 (Gemma 4 tag verification), corrected the icon Concept C/D naming, and pre-staged additional Phase 1/6a/8/10 file-only artifacts (LICENSE, real Concept A SVG, reference HTML, CI workflow skeleton, canonical tool prompt fixture, conversation export schema).

**Stop point:** before any source code that needs to compile. The plan and CLAUDE.md both forbid Windows-native builds — code phases must run from WSL2 + Ubuntu 24.04.

## What's in the repo now

```
BiscuitCode/
├── CLAUDE.md                                # Project operating system + Four Laws
├── README.md                                # Starter readme
├── LICENSE                                  # MIT (Phase 1 deliverable)
├── .gitignore                               # Tauri / Rust / Node / pnpm / OS / paranoia
├── .gitattributes                           # (was here from initial commit)
├── .claude/
│   ├── agents/{researcher,planner,reviewer,coder}.md
│   └── commands/{research,plan,review-plan,run-phase,run-all}.md
├── .github/workflows/
│   └── ci.yml                               # Phase 1 skeleton; populated through Phase 10
├── docs/
│   ├── vision.md                            # Super prompt v3, locked
│   ├── research-r1.md                       # Round 1 research (~600 lines)
│   ├── research-r2.md                       # Round 2 research (~650 lines, challenger)
│   ├── plan-r1.md                           # Round 1 plan + reviewer-r1 audit
│   ├── plan-r2.md                           # Round 2 plan + reviewer-r2 audit
│   ├── plan.md                              # ★ FINAL synthesized plan (source of truth)
│   ├── DEV-SETUP.md                         # WSL2 + toolchain setup procedure
│   ├── CONVERSATION-EXPORT-SCHEMA.md        # Phase 8 deliverable
│   ├── MORNING-BRIEF.md                     # This file
│   └── adr/
│       └── 0001-no-stronghold.md            # Why we don't use tauri-plugin-stronghold
├── scripts/
│   ├── bootstrap-wsl.sh                     # Phase 0 — apt installs (system libs)
│   └── bootstrap-toolchain.sh               # Phase 0 — rustup, nvm, node, pnpm, cargo-tauri
├── packaging/icons/
│   ├── biscuitcode.svg                      # Concept A master (verbatim from reference HTML)
│   └── biscuitcode-icon-concepts.html       # Authoritative design reference (A/B/C)
└── tests/fixtures/
    └── canonical-tool-prompt.md             # Phase 6a tool-card-render gate fixture
```

## What happened, in order

| # | Commit | What |
|---|---|---|
| 1 | `298d8c6` | Bootstrap: CLAUDE.md, all 4 agents, all 5 commands, docs/vision.md |
| 2 | `cde4597` | Research round 1 dossier (~600 lines, 15 domains covered) |
| 3 | `6a99e05` | Plan round 1 (11 phases, ~15 days) |
| 4 | `bb94b98` | Reviewer round 1 audit applied (19 issues fixed inline) |
| 5 | `2dd4c57` | Research round 2 (challenger / gaps, ~650 lines) |
| 6 | `f19f5fc` | Plan round 2 (13 phases with 6a/6b split, ~17 days) |
| 7 | `bf22bc7` | Reviewer round 2 audit applied (13 issues fixed) |
| 8 | `562a7ad` | **Synthesized final plan.md** (12 phases, ~16 days) + in-context self-audit |
| 9 | `3ebee78` | Pre-staged Phase 0 + Phase 1 file-only deliverables |

## Key decisions baked into the final plan

These came from the synthesis pass that picked between r1 and r2 positions:

### Adopted from r2 (round-2 stronger)
- **Phase 6 split into 6a (read-only tools + 3 providers + Agent Activity UI) + 6b (write tools + inline edit + rewind).** Isolates the highest-risk subsystems; if 6b needs replanning, the read-only agent stays shippable.
- **Anthropic prompt caching default-on** (`cache_control: ephemeral` on system prompt + tool defs) — ~5x cost reduction on long conversations. r1 missed this.
- **Secret Service detection via `busctl list --user`** (read-only, BEFORE any keyring API call). r1's keyring-probe approach risked activating the daemon with a known credential.
- **Wayland-XFCE smoke row dropped.** Mint 22 XFCE ships 4.18 with no Wayland; r1's smoke test was unreachable.
- **`react-virtuoso`** for chat + Agent Activity virtualization — named explicitly to avoid streaming-list jank.
- **Inline edit = Zed-style split-diff** via `monaco.editor.createDiffEditor`. Simpler than Cursor-style in-place decoration.
- **Distributed error catalogue:** scaffold infrastructure in Phase 1, codes added per feature phase, audited in Phase 9.
- **i18n scaffolding in v1** (~1 hour cost, all strings via `t('key')`); **a11y "reasonable posture"** in v1 (keyboard nav, ARIA labels, focus rings); full WCAG AA is post-v1.
- **Reasoning-model TTFT exemption.** The p50-under-500ms gate applies only to non-reasoning models.
- **Stronghold ADR warning** — `docs/adr/0001-no-stronghold.md` records why future maintainers searching "Tauri secrets" should NOT land on Stronghold.
- **Auto-update in v1**: Tauri updater for AppImage; GitHub Releases API "Check for updates" button for `.deb` (no auto-install of `.deb` — requires sudo).

### Adopted from r1 (round-1 stronger)
- **Tighter phase count.** Final = 12 (between r1's 11 and r2's 13).
- **Anthropic-only E2E in Phase 5** as the minimum viable chat milestone (then OpenAI + Ollama join in 6a alongside the read-only agent surface — a hybrid of r1's "one provider first" staging and r2's "validate ChatEvent across three providers before tools").
- **Workspace crates created in the phase that first uses them** — only `biscuitcode-core` is created in Phase 1; no speculative empties.
- **Defensive `biscuitcode` claim on crates.io day 1.**

### Updated per maintainer direction (you said "make sure we account for Gemma 4")
- **Gemma 4 is now the PRIMARY Ollama default**, not "preferred when available." Gemma 3 ladders are kept ONLY as a fallback for systems whose Ollama install does not yet have Gemma 4. New error code `E007 GemmaVersionFallback` fires when the Gemma 4 pull fails and the app falls back to Gemma 3 — surfaces a one-time toast suggesting an Ollama update.
- **RAM-tier table** updated to use `gemma4:e2b` (small multimodal, verified), `gemma4:9b` (mid-tier — extrapolated), `gemma4:27b` (large-tier — extrapolated). The exact mid/large-tier tag names are flagged as **Open Question Q16** for the Phase 6a coder to verify against `https://ollama.com/library/gemma4` before hardcoding the pull command. The runtime selection logic ("closest available Gemma 4 tier") tolerates drift.

## Honest disclosures (Law 1)

You explicitly asked me to follow the same guidelines whether I synthesize or use a subagent. I did the synthesis **in the same session** as the planner/reviewer agents — *not* in a fresh context window — because of repeated session timeouts that made spawning a sixth long-running opus subagent unreliable. To partially compensate, I ran a five-axis self-audit on `plan.md` and documented findings inside the plan itself (see `docs/plan.md` → `## Synthesis Log` → "Synthesis Self-Audit" entry). Issues found and fixed:

1. **Completeness:** Vision-mandated Monaco multi-cursor + minimap weren't named as Phase 3 acceptance criteria. **Fixed** — added explicit ACs.
2. **Verifiability:** Phase 6a's tool-card-render gate referenced an e2e test by name without specifying its file path. **Fixed** — `tests/e2e/agent-tool-card-render.spec.ts` named, canonical fixture path `tests/fixtures/canonical-tool-prompt.md`.
3. **Verifiability:** Phase 6a's "agent mode on" demo AC was generic ("a prompt that requires read_file + search_code"). **Fixed** — concrete prompt text + expected tool sequence specified.
4. **Accuracy (flagged not fixed, Law 1):** Gemma 4 mid/large-tier tag names are extrapolated. Recorded as Open Question Q16; the Phase 6a coder must verify and update at execution time.

Things a fresh-context synthesizer would have caught that I might not have:
- Same-context anchoring bias (I knew which agent said what — a fresh context wouldn't).
- I worked from agent return summaries rather than re-reading research-r2 end-to-end during synthesis. If a specific research finding matters to you, spot-check `plan.md` against `research-r2.md` directly.

## What's next — your move when you wake up

### Immediate (next 15 minutes)

1. **Skim `docs/plan.md`** — especially the `## Synthesis Log` and the Phase Index. Sanity check it against your intent.
2. **Spot-check `docs/research-r2.md`** if you want to verify any synthesis decision against the source research.
3. **Push to GitHub** if you want a remote backup before WSL2 setup:
   ```bash
   cd /c/Users/super/Documents/GitHub/BiscuitCode
   git remote add origin https://github.com/Coreyalanschmidt-creator/biscuitcode.git
   git push -u origin main
   ```

### To start Phase 0 (next 1–2 hours)

1. **Install WSL2 + Ubuntu 24.04** on your Windows machine. From an admin PowerShell:
   ```powershell
   wsl --install -d Ubuntu-24.04
   ```
   Reboot when prompted, set up your Linux user.

2. **Move (or re-clone) the repo into WSL's filesystem.** Do NOT develop from `/mnt/c/`. From inside WSL:
   ```bash
   cd ~
   git clone <your-repo-url> biscuitcode
   # OR: cp -r /mnt/c/Users/super/Documents/GitHub/BiscuitCode ~/biscuitcode
   cd ~/biscuitcode
   realpath .   # must be /home/<you>/biscuitcode, NOT /mnt/c/...
   ```

3. **Install Claude Code inside WSL** if not already there. From inside WSL, follow the Claude Code Linux install instructions.

4. **Run the bootstrap scripts:**
   ```bash
   bash scripts/bootstrap-wsl.sh         # apt installs, ~5 min
   bash scripts/bootstrap-toolchain.sh   # rustup + node + pnpm + cargo-tauri, ~10 min
   ```
   Both scripts are idempotent and have pre-flight sanity checks.

5. **Verify Phase 0 acceptance criteria** (open `docs/plan.md` and look at Phase 0's AC list). The scripts cover most automatically, but a few require eye-checks (e.g., `realpath .`).

6. **Run `/run-phase 0`** in your Claude Code WSL session to formally close out Phase 0. The coder will read `docs/plan.md`, write a pre-mortem for Phase 0, verify each AC, and update the plan with `Complete`.

### To start Phase 1 (after Phase 0 closes)

7. **`/run-phase 1`** — coder scaffolds the Tauri app, wires brand tokens, authors capability files, builds the error infra, and ships a window that paints on cocoa-700 with the biscuit accent. Acceptance criteria include a working `pnpm tauri dev` window.

### Open Questions you may want to answer before Phase 6a

These are in `docs/plan.md → ## Open Questions`. Q16 (Gemma 4 tag names) was **resolved post-briefing** by direct verification against `ollama.com/library/gemma4` — see commit `d68b1e1`. Others (Q1 telemetry backend, Q3 icon Concept C spike, Q4 arm64) only matter for v1.0 finalization (Phases 8–10).

**Icon naming correction (post-brief):** the vision text refers to the biscuit-shape alternative as "Concept D" but the authoritative reference (`packaging/icons/biscuitcode-icon-concepts.html`, now in the repo) labels it **Concept C** — there is no Concept D. r1/r2 say "Concept D" — those references all mean the same biscuit-shape Concept C. plan.md has been updated.

## Risks I want to flag

- **The 2-round flow consumed serious agent runtime.** Each opus subagent ran 5–11 minutes; the harness occasionally went unresponsive between turns (you noticed). This is a Claude Code harness limit, not a methodology issue — but it means future autonomous sessions on this project should default to sonnet for non-foundational subagent calls.
- **Gemma 4 tag specificity (Q16).** If `gemma4:9b` isn't a real Ollama tag, the auto-pull will 404. Phase 6a coder MUST verify against `https://ollama.com/library/gemma4` before merging.
- **WSL2 GUI quirks.** WSLg works but isn't perfect for testing XFCE-specific tray rendering. The vision's "16x16 system tray legibility" check (Phase 9) and several visual-polish items in Phase 10 must happen on your real Mint 22 XFCE machine, not WSL.
- **The plan has 18 error codes scattered across 9 phases.** The Phase 9 error-catalogue audit will be tedious if any phase forgot to register a code. The CI gate (`tests/error-catalogue.spec.ts` enforces "every catalogued code has a passing trigger test") catches drift.
- **Phase 6b is the riskiest phase in the plan.** Write tools + rewind + inline edit are tightly coupled; a correctness bug in rewind could delete user code. Plan's split (6a then 6b) means the read-only agent is shippable even if 6b needs replanning.

## My state right now

- All work committed to `main` in `C:/Users/super/Documents/GitHub/BiscuitCode/`.
- No remote configured — nothing pushed to GitHub yet (intentional; you decide).
- I'm in a worktree of the Typing app (`C:\Users\super\Documents\GitHub\Typing app\.claude\worktrees\focused-borg-b456c8`) which is unrelated to BiscuitCode. Don't expect to find BiscuitCode work here.
- Project memory will be saved next so future Claude Code sessions in any project know about BiscuitCode.

Good luck. The plan is bulletproof on paper — the risks are all in execution, where Phase 6 (6a/6b) is the part to take slowly.
