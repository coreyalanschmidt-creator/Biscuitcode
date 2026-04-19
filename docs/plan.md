# Implementation Plan: BiscuitCode (Final, Synthesized)

> Synthesis of `plan-r1.md` (with reviewer-r1 audit) and `plan-r2.md` (with reviewer-r2 audit), drawing on `research-r1.md` and `research-r2.md`. Authored by the synthesis pass of the C.Alan pipeline on 2026-04-18. **This is the source-of-truth plan.** The round-1/round-2 artifacts remain in the repo as the audit trail.

## Synthesis Log

### 2026-04-18 — Synthesis Pass

Both plans were internally coherent after their respective reviewer audits. This pass picks the strongest position on each axis and lands a single executable plan. Decisions are recorded so the audit trail is intact.

**Kept from plan-r2:**
- **Phase 6 split into 6a (read-only tools + Agent Activity UI) + 6b (write tools + inline edit + rewind).** The split isolates the highest-risk subsystems; if 6b needs replanning, the read-only agent stays shippable. Reviewer-r2 explicitly cited this as r2-stronger.
- **Anthropic prompt caching** (`cache_control: {type: "ephemeral"}` on system prompt + tool definitions). r1 missed this; ~5x cost reduction on long conversations.
- **Secret Service detection via `busctl list --user`** (read-only DBus name-check) BEFORE any keyring API call. r1's keyring-probe approach risks activating the daemon with a known credential.
- **Wayland-XFCE row dropped from smoke matrix.** Mint 22 XFCE ships 4.18 with no Wayland; r1's smoke row was unreachable.
- **`react-virtuoso` named for chat + Agent Activity virtualization** from the first panel that streams content (Phase 5).
- **Inline edit = Zed-style split-diff** via `monaco.editor.createDiffEditor`. Simpler than Cursor-style in-place decoration; uses Monaco-native primitives.
- **Error taxonomy scaffolded in Phase 1, codes added per feature phase, audited in Phase 9.** Distributed ownership beats retrospective catalogue.
- **i18n scaffolding in Phase 2** (all user-facing strings via `t('key')`, English-only bundle). ~1 hour cost; saves v1.1 find-and-replace sweep.
- **a11y "reasonable posture"** in v1: keyboard nav, ARIA labels on icon buttons, `aria-live="polite"` on streaming chat. Full WCAG AA is post-v1.
- **Reasoning-model TTFT exemption.** The p50-under-500ms gate applies only to non-reasoning models.
- **`tauri-plugin-stronghold` ADR warning.** Recorded so future maintainers searching "Tauri secrets" don't land on deprecated docs.
- **Auto-update in v1** (Tauri updater for AppImage; GitHub Releases API check-for-updates button for `.deb`).
- **Provider rollout: all three providers ship together with the read-only agent surface in 6a.** Validates the `ChatEvent` contract across three real providers before tools depend on it.

**Kept from plan-r1:**
- **Phase numbering and tighter overall scope.** Used r1's structure for non-agent phases (3, 4, 5, 7-style packaging).
- **Phase 5 keeps a single-provider end-to-end (Anthropic only)** as the minimum viable chat milestone. All-three-providers happens in 6a, NOT in Phase 5. This preserves r1's "one provider E2E first" milestone clarity while still validating the trait against three providers before tools land.
- **Workspace crate creation deferred to the phase that first uses each crate** (no speculative empties in Phase 1). Only `biscuitcode-core` is created in Phase 1.
- **Defensive `biscuitcode` crates.io claim on day 1.**
- **Phase Index dependency rigor.**

**Compromises landed (neither plan got it exactly right):**
- **Phase 9 absorbs r2's "Data/Persistence Polish"** (conversation branching UI, export/import) but does NOT absorb r2's separate "auto-update + a11y audit + error catalogue consolidation" phase. Those three items fold into Phase 9 (a11y audit, error consolidation) and Phase 10 (auto-update CI). This trims r2's 13 phases back to **12** without losing the substance.
- **Wayland-Cinnamon best-effort smoke** kept in Phase 10. Wayland-XFCE smoke dropped (r2 correction).
- **Vision's Phase 6 ("Remaining Providers")** scope is fully absorbed into 6a; Phase 7 in this plan is git+preview+LSP rather than r1's "OpenAI+Ollama" — net the same total work, different ordering.

**Updated per maintainer direction (2026-04-18, mid-synthesis):**
- **Gemma 4 is the PRIMARY Ollama default**, not "preferred when present." Gemma 3 ladders are kept ONLY as a fallback for systems whose Ollama install does not yet have Gemma 4 available. Default ladder is now Gemma 4-first across every RAM tier; `qwen2.5-coder:7b` remains the agent-mode alternative at 12 GB+ for its proven tool-calling stability; Gemma 3 ladder is a documented fallback for older Ollama versions only. (See Architecture Decisions and Phase 6a deliverables.)

**Open Questions inherited from both rounds:** consolidated below; one new question (Q16) raised by synthesis on Gemma 4 tag specificity.

### 2026-04-18 — Synthesis Self-Audit

The synthesis pass above was performed **in the same Claude Code session that launched the planner/reviewer subagents** — *not* in a fresh context window as the C.Alan method ideally prescribes. This was a deliberate trade-off after repeated session timeouts forced a pivot away from a sixth subagent invocation. Disclosed here so the maintainer can apply additional scrutiny where they think same-context anchoring may have biased a decision.

To partially compensate, this self-audit applies the same five-axis reviewer criteria to `plan.md` itself.

**Findings by axis:**

1. **Completeness — 2 issues, fixed inline:**
   - Vision §Coding Features mandates Monaco "multi-cursor" and "minimap" — both are Monaco built-ins (free) but were not explicitly named as Phase 3 acceptance criteria. **Fix:** added explicit AC items in Phase 3.
   - Phase 6a's "agent mode on" demo AC referenced `read_file + search_code` without a concrete prompt or expected tool sequence. **Fix:** specified the exact prompt text and the expected tool call sequence so the test is reproducible.

2. **Accuracy — 1 issue, flagged not fixed (Law 1):**
   - The Ollama RAM-tier table in Phase 6a names `gemma4:9b` and `gemma4:27b` as Gemma 4 mid-tier and large-tier defaults. **These exact tag names are NOT verified.** Research-r2 cites `gemma4:e2b` specifically (the small multimodal variant); larger Gemma 4 tier tags were extrapolated by analogy with the Gemma 3 family. **Fix:** added Open Question Q16 documenting the assumption; Phase 6a deliverable already says "or closest available Gemma 4 mid/large-tier" so the runtime selection logic tolerates tag-name drift, but the exact pull invocation must be verified at coder time against `https://ollama.com/library/gemma4`. Coder of Phase 6a is on notice to confirm tag names before hardcoding them.

3. **Consistency — 0 issues:** Phase Index DAG validated (0→1→2; 2→{3,4,5}; 5→6a; 6a→{6b,8}; 3→{6b,7}; {7,8}→9; 9→10) — no cycles, no orphans. Phase count `12` matches the 12 rows in the index. Crate naming `biscuitcode-*` consistent throughout.

4. **Simplicity — 0 issues:** Every addition over the union of r1∩r2 traces to either (a) explicit research support (font canary → r2 G9; auto-update → r2 G3; etc.), (b) a maintainer directive (Gemma 4 primary, 2026-04-18), or (c) a vision Hard Constraint not previously assigned to a phase (multi-cursor / minimap). No gold-plating identified.

5. **Verifiability — 1 issue, fixed inline:**
   - Phase 6a's tool-card render trace gate referenced `agent_tool_card_visible_within_250ms` as the e2e test name; same gate appears in Global AC. **Fix:** clarified that the test fixture lives at `tests/e2e/agent-tool-card-render.spec.ts` and uses the canonical 3-tool prompt defined in `tests/fixtures/canonical-tool-prompt.md` — eliminates ambiguity about which test asserts the gate.

**Synthesis assumptions explicitly disclosed (Law 1):**
- Plan-r1 and plan-r2 were read in their reviewer-corrected forms; research-r1 and research-r2 were absorbed primarily via the planner/reviewer agent return summaries rather than re-read end-to-end during this synthesis pass. A genuinely fresh-context synthesizer would have re-read everything. The maintainer may wish to spot-check by diffing this `plan.md` against specific research claims they care about.
- Where the two reviewers' "where r1/r2 stronger" notes contradicted, this synthesis broke ties using these tiebreakers in order: (1) what does the maintainer's standing direction support? (2) what does the more recently dated research support? (3) what minimizes blast radius if wrong?

**Files modified by self-audit:** Phase 3 ACs (multi-cursor, minimap); Phase 6a "agent mode on" AC (concrete prompt + tool sequence); Open Questions (Q16 added); Global AC reference to test fixture path. Synthesis Log itself (this section).

---

## Vision Summary

BiscuitCode is a Tauri 2.10.x + React 18 + TypeScript 5 desktop AI coding environment targeting Linux Mint 22 XFCE (Ubuntu 24.04 / WebKitGTK 4.1 / kernel 6.8) with VS Code parity: Monaco editor, xterm.js over `portable-pty`, LSP client for five languages, git panel, preview pane, and a four-region resizable shell. Three AI providers (Anthropic, OpenAI, Ollama) sit behind a unified `ModelProvider` trait emitting a flattened `ChatEvent` stream; a ReAct agent loop calls workspace-scoped tools with explicit confirmation gates on writes; API keys live in libsecret via the Rust `keyring` crate with **no plaintext fallback** (block onboarding instead). "Done" = a GPG-signed `biscuitcode_1.0.0_amd64.deb` that installs clean on a stock Mint 22 XFCE VM, cold-launches in under 2 s on i5-8xxx / 8 GB hardware, completes 3-screen onboarding in under 2 minutes, and survives `apt remove biscuitcode` cleanly.

---

## Assumptions

Carried from both research rounds plus planning-specific assumptions. Confidence flags: [HIGH]/[MED]/[LOW].

1. **[HIGH]** Canonical target = Mint 22.1 Xia (kernel 6.8, XFCE 4.18). Smoke matrix also covers 22.0 and 22.2 (both XFCE 4.18; 4.20 is backport-only on 22.2). Ubuntu 24.04 noble is the Debian base. (r1 §2; r2 C1)
2. **[HIGH]** Tauri pin: v2.10.x. Capability files hand-authored, never `tauri migrate`-generated. (r1 §1)
3. **[HIGH]** Linux webview is `libwebkit2gtk-4.1-0`; declared in `.deb` `Depends`. Ubuntu 24.04 does **not** ship webkit2gtk-4.0. (r1 §1)
4. **[HIGH]** `@xterm/*` scoped packages only; legacy `xterm-addon-*` are deprecated. (r1 §5)
5. **[HIGH]** `keyring` 3.6.x with features `linux-native-async-persistent + async-secret-service + crypto-rust + tokio`. **No plaintext fallback; block onboarding if Secret Service unavailable.** Detection is read-only via `busctl list --user`, never a keyring probe. (r1 §6; r2 D6)
6. **[HIGH]** `tauri-plugin-stronghold` is **deprecated and slated for removal in Tauri v3.** Do not evaluate, reference, or use it outside the explicit ADR warning. (r2 A7)
7. **[HIGH]** Provider defaults corrected from vision:
   - Anthropic: `claude-opus-4-7` (**omit** `temperature`/`top_p`/`top_k` — Opus 4.7 returns HTTP 400 on those fields).
   - OpenAI: `gpt-5.4-mini` (NOT `gpt-4o` — retired 2026-04-03).
   - Ollama: **`gemma4:*` is the primary default ladder** across every RAM tier where the user's Ollama install has Gemma 4 (released 2026-04-03; Ollama v0.20.0+ required). **Verified Gemma 4 tags as of 2026-04-18:** `gemma4:e2b` (2.3B effective, 7.2GB, 128K), `gemma4:e4b` (4.5B effective, 9.6GB, 128K — same as `:latest`), `gemma4:26b` (MoE 25.2B/3.8B active, 18GB, 256K), `gemma4:31b` (30.7B, 20GB, 256K). **All Gemma 4 variants natively support function calling** (no community fine-tunes needed, unlike Gemma 3 base). `qwen2.5-coder:7b` remains the agent-mode alternative for code-heavy workflows. **Gemma 3 ladder kept ONLY as fallback** when the user's Ollama version is < 0.20.0 or pulls return 404 for Gemma 4 tags.
8. **[HIGH]** Anthropic SSE streaming: `message_start → content_block_{start,delta,stop} → message_delta → message_stop`. `input_json_delta` deltas are partial strings; full `input` object is only safe at `content_block_stop`. (r1 §7; r2 D3)
9. **[HIGH]** **Anthropic prompt caching is on by default.** `cache_control: {type: "ephemeral"}` on the system prompt and tool definitions. ~5x cost reduction on long conversations. (r2 New Risks #1)
10. **[HIGH]** Monaco loads via `@monaco-editor/react` pinned locally (no CDN), `vite-plugin-monaco-editor` for workers, **explicit `languageWorkers: []`** at startup (no default languages bundled at boot) to keep the cold bundle lean. TS worker silenced via `setDiagnosticsOptions` when LSP connects. (r1 §4; r2 D2)
11. **[HIGH]** SQLite via `rusqlite` direct (no `plugin-sql`), WAL mode, `PRAGMA user_version` migrations. DAG message schema with `parent_id` for branching. (r1 §10)
12. **[HIGH]** Git: `git2-rs` for reads (status, diff, blame), shell-out to `git` for writes (commit, push, pull). `gix` swap is a v1.1+ target. (r1 Best Practice #8)
13. **[HIGH]** LSP: Rust spawns language servers, proxies stdio via Tauri events keyed by `session_id`; frontend wires `monaco-languageclient` with custom `MessageTransports`. **No auto-install of LSP binaries — copy-to-clipboard install command only.** (r1 §9)
14. **[HIGH]** All code-phase work runs from WSL2 + Ubuntu 24.04 with the project rooted in `~/biscuitcode/` (never `/mnt/c/`). A coder invoked from a Windows-native shell must stop and report. (CLAUDE.md; r1 §3)
15. **[MED]** **Wayland-XFCE is NOT reachable on any Mint 22 release.** XFCE 4.18 lacks Wayland; 22.2's XFCE edition stays on 4.18. Drop Wayland-XFCE smoke testing from the release matrix. Cinnamon-Wayland 22.2 is a best-effort row only. (r2 C1)
16. **[MED]** GitHub Actions runner is `ubuntu-24.04` (pinned, not `-latest`). Release builds GPG-signed via `GPG_PRIVATE_KEY` secret; SHA256 via `sha256sum`. (r1 §12)
17. **[MED]** Auto-update is **in scope for v1** but minimal: Tauri updater plugin for AppImage; GitHub Releases API check-for-updates button for `.deb` (manual download triggered, no auto-install of `.deb` because that requires sudo). No apt repo hosting in v1. (r2 G3)
18. **[MED]** Chat and Agent Activity panels use `react-virtuoso` for message virtualization from the first panel that streams content (Phase 5). (r2 D8)
19. **[MED]** Inline edit UX is **Zed-style split-diff** via `monaco.editor.createDiffEditor`. Whole-diff Accept/Reject in v1; per-hunk in v1.1. (r2 D7)
20. **[MED]** i18n scaffolding is in scope for v1 (all user-facing strings wrapped in `t('key')`; English-only bundle). Cost ≈ 1 hour in Phase 2. (r2 G1)
21. **[MED]** Accessibility is "reasonable posture" in v1: keyboard-only navigation, ARIA labels on icon buttons, `aria-live="polite"` on streaming chat, focus rings. Full WCAG AA is post-v1. (r2 G2)
22. **[MED]** Icon Concept A ("The Prompt") ships in v1. A 16x16 render legibility check happens inside Phase 9 before the icon is declared done; Concept D is deferred unless A fails the legibility test.
23. **[MED]** Telemetry is scaffolded as an off-by-default setting in v1 with no wire implementation — endpoint choice is a v1.1 decision (Open Question).
24. **[MED]** Notebook preview is read-only render in v1 (per vision); execution deferred to v2.
25. **[LOW]** Arm64 is NOT a v1 target. `.deb` ships x86_64 only.
26. **[LOW]** VS Code theme import is a UI placeholder only in v1.

---

## Architecture Decisions

Each decision cites the research section. Decisions marked **(synthesis)** depart from at least one round-1/round-2 plan; rationale is in the Synthesis Log above.

- **Tauri v2.10.x with hand-authored capability ACL files** under `src-tauri/capabilities/{core,fs,shell,http}.json`, deny-by-default scopes. Workspace-root fs scope patched at runtime via `FsScope::allow_directory`. (r1 §13)
- **Brand tokens verbatim** in Tailwind theme + Rust palette constants + CSS custom properties. No `system-ui` in visible chrome; self-hosted Inter + JetBrains Mono in `src-tauri/fonts/`.
- **Font fallback chain**: `'Inter', 'Ubuntu', sans-serif` for UI; `'JetBrains Mono', 'Ubuntu Mono', 'DejaVu Sans Mono', monospace` for code. Ubuntu fonts ship on Mint 22 by default — a *named* system fallback, not the forbidden `system-ui` keyword. (r2 G9)
- **React 18 + Zustand** for state; `react-resizable-panels` for layout; **`react-virtuoso`** for both chat and Agent Activity message lists. (r1 Trade-offs; r2 D8) **(synthesis: virtuoso adopted)**
- **Monaco single-instance, one `ITextModel` per tab.** Languages registered on-demand, not at startup. `createDiffEditor` for the inline-edit split diff. TS worker silenced when LSP is active. (r1 §4; r2 D2)
- **`ModelProvider` trait → flattened `ChatEvent` enum**: `TextDelta`, `ThinkingDelta`, `ToolCallStart`, `ToolCallDelta`, `ToolCallEnd`, `Done { stop_reason, usage }`, `Error`. Provider quirks live in each impl. (r1 §7)
- **Anthropic prompt caching default-on**: `cache_control: {type: "ephemeral"}` on system prompt and tool definitions. (r2 New Risks #1) **(synthesis: r2-source)**
- **Updated provider defaults**: Anthropic `claude-opus-4-7` (no sampling params); OpenAI `gpt-5.4-mini`; **Ollama primary = Gemma 4 ladder by RAM, using verified tag names** `gemma4:e2b` / `gemma4:e4b` / `gemma4:26b` / `gemma4:31b`. All Gemma 4 variants natively support function calling — no community fine-tunes. `qwen2.5-coder:7b` remains the agent-mode alternative at 12 GB+ for code-heavy workflows. Gemma 3 ladder retained only as a fallback when the user's Ollama version (< 0.20.0) doesn't recognize Gemma 4 tags. (r1 §7; r2 D5; **synthesis: maintainer direction 2026-04-18; tags verified 2026-04-18 against ollama.com/library/gemma4**)
- **ReAct loop with read-only tool surface in v1.0 (Phase 6a) and write tools gated behind explicit per-tool confirmation UX (Phase 6b).** Split isolates the highest-risk work. (r2 A4) **(synthesis: 6a/6b split adopted)**
- **Ordering: providers-then-tools**. Anthropic alone in Phase 5 (E2E text-only), OpenAI + Ollama join in Phase 6a *alongside* the read-only tool surface, validating the `ChatEvent` contract across three providers before write tools land in 6b. **(synthesis: hybrid of r1 staging + r2 ordering)**
- **Inline edit = Zed-style split-diff** via `createDiffEditor`. Whole-diff accept/reject in v1; per-hunk in v1.1. (r2 D7) **(synthesis: r2-source)**
- **LSP via `monaco-languageclient` + Rust stdio proxy** over Tauri events. One LSP child per (language, workspace) pair. Copy-to-clipboard install commands only. (r1 §9)
- **SQLite via `rusqlite` direct**, WAL mode, hand-rolled `PRAGMA user_version` migrations, DAG schema with `parent_id` for branching. (r1 §10)
- **Git: `git2-rs` for reads, shell-out for writes.** Inherits user's `.gitconfig`, credential helpers, signing, LFS. (r1 Best Practice #8)
- **Theming: `xfconf-query -c xsettings -p /Net/ThemeName`** with `gsettings` fallback; dark heuristic via `-dark$` regex. (r1 §11)
- **Secret Service detection via `busctl list --user`** (read-only, no daemon activation), *before* any keyring API call. (r2 D6) **(synthesis: r2-source)**
- **Auto-update: dual path in v1.** AppImage users get the Tauri updater plugin (v2.10.x); `.deb` users get a "Check for updates" button that opens the GitHub Releases page (no auto-install of `.deb`). No apt repo in v1. (r2 G3) **(synthesis: r2-source)**
- **Error taxonomy scaffolded in Phase 1** (`src/errors/types.ts` + `src/errors/ErrorToast.tsx` + Rust `thiserror` enum in `biscuitcode-core`). Each feature phase **adds its own codes** as it touches a failure surface. Phase 9 audits the catalogue rather than building it from zero. (r2 G6) **(synthesis: distributed catalogue per r2)**
- **Internal Rust crates prefixed `biscuitcode-*`** (`-core`, `-agent`, `-providers`, `-lsp`, `-pty`, `-db`). Crates created in the phase that first uses them — no speculative empties. **Defensively claim `biscuitcode` on crates.io day 1.** Avoid `biscuit`, `biscuit-auth`, `biscuit-cli`, `CodeBiscuits`. (r1 §15)
- **Stronghold plugin explicitly forbidden** — recorded as a top-of-file ADR warning so a future maintainer searching "Tauri secrets" does not land on deprecated docs. (r2 A7)
- **Wayland-XFCE drop**. Mint 22 XFCE ships 4.18 (no Wayland). Smoke matrix drops the Wayland-XFCE row; Cinnamon-Wayland 22.2 is a best-effort test only. (r2 C1) **(synthesis: r2-correction)**
- **Reasoning-model TTFT exemption**. `gpt-5.4-pro` and other reasoning-only models emit no output until reasoning finishes (3–30 s). The p50-under-500ms TTFT gate applies only to non-reasoning models; reasoning runs show a `Thinking…` state. (r2 New Risks #2) **(synthesis: r2-source)**
- **i18n scaffolding in Phase 2**: every user-facing string goes through `t('key')`; English-only bundle in v1. (r2 G1)
- **a11y posture in v1**: keyboard-only navigation, ARIA labels on icon buttons, `aria-live="polite"` on streaming chat, focus rings. Full WCAG AA is post-v1. (r2 G2)

---

## Phase Index

| # | Phase | Status | Complexity | Depends on |
|---|-------|--------|------------|------------|
| 0 | Dev Environment Bootstrap (WSL2 + toolchain) | Not Started | Low | — |
| 1 | Scaffold + Brand Tokens + Capability Skeleton + Error Infra | Not Started | Medium | 0 |
| 2 | Four-Region Layout + Shortcuts + i18n Scaffold + Installable .deb | Not Started | Medium | 1 |
| 3 | Editor + File Tree + Find/Replace | Not Started | Medium | 2 |
| 4 | Terminal (xterm.js + portable-pty) | Not Started | Medium | 2 |
| 5 | Keyring + Anthropic Provider + Chat Panel (virtualized E2E) | Not Started | Medium | 2 |
| 6a | OpenAI + Ollama Providers + Read-Only Tool Surface + Agent Activity UI | Not Started | Medium | 5 |
| 6b | Write Tools + Inline Edit (split-diff) + Rewind | Not Started | High | 3, 6a |
| 7 | Git Panel + LSP Client + Preview Panel | Not Started | High | 3 |
| 8 | Onboarding + Settings UI + Theming + Icon + Data Polish | Not Started | Medium | 5, 6a |
| 9 | a11y Audit + Error Catalogue Consolidation + Auto-Update Wiring | Not Started | Low | 7, 8 |
| 10 | Packaging + CI + GPG Signing + Release Smoke Test | Not Started | Medium | 9 |

Total: **12 phases** (0 through 10, with Phase 6 split 6a/6b). Estimated calendar: Phase 0 half day; Phases 1/2/4/5/9/10 ≈ 1 day each; Phases 3/6a/8 ≈ 2 days each; Phase 6b ≈ 2 days; Phase 7 ≈ 3 days. **Total ≈ 16 focused working days** — between r1's 15 and r2's 17.

---

## Phases

### Phase 0 — Dev Environment Bootstrap (WSL2 + toolchain)

**Goal:** Bring the Windows-host maintainer to a working WSL2 + Ubuntu 24.04 dev environment with `cargo tauri --version` succeeding, project rooted at `~/biscuitcode/`, all apt deps installed — *before* any code phase runs.

**Deliverables:**
- `scripts/bootstrap-wsl.sh` — idempotent apt install of: `pkg-config libdbus-1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf libfuse2t64 file build-essential curl gnome-keyring libsecret-1-0 libsecret-tools`. **`busctl` is NOT a separate apt package** — it ships with `systemd`; the script asserts `command -v busctl` and errors clearly if missing.
- `scripts/bootstrap-toolchain.sh` — installs rustup (stable 1.85+), `cargo-tauri-cli@2.10.1`, Node.js 20+ via nvm, `pnpm@9+`.
- `docs/DEV-SETUP.md` (short) — WSL2 install, why the project must live in `$HOME` (inotify + speed), bootstrap usage, `pnpm tauri dev` launching into WSLg.
- PR description includes output of `cargo tauri --version`, `node --version`, `pnpm --version`, `rustc --version`, `apt list --installed | grep webkit2gtk-4.1-dev`, `busctl --user list | head`.

**Acceptance criteria:**
- [ ] `bash scripts/bootstrap-wsl.sh` on fresh WSL2 Ubuntu 24.04 exits `0`.
- [ ] `cargo tauri --version` prints `tauri-cli 2.10.x`.
- [ ] `pnpm --version` prints `9.x` or higher.
- [ ] `apt list --installed 2>/dev/null | grep libwebkit2gtk-4.1-dev` returns a line.
- [ ] `command -v busctl` resolves to `/usr/bin/busctl` (or equivalent).
- [ ] `busctl --user list 2>/dev/null | grep -c org.freedesktop.secrets` returns `0` or `1`. `1` proves the session is fully set up; `0` is acceptable for a CI-style headless WSL session and triggers the documented PAM-start workaround.
- [ ] Project working directory resolves under `$HOME` (NOT `/mnt/c/`); script asserts via `realpath .`.
- [ ] `docs/DEV-SETUP.md` exists and is linked from `README.md`.

**Dependencies:** None.
**Complexity:** Low.
**Split rationale:** The vision assumes a working dev env. Research-r1 §3 documents multiple WSL2 gotchas (inotify on `/mnt/c`, webkit-4.0 vs 4.1 rename, `libfuse2t64` confusion) that are each single-sentence fixes only if the environment is correct from minute one. Making bootstrap a named phase enforces "Phase 1 can actually build" rather than discovering missing libs mid-scaffold. Deliberately Low complexity / half-day so it doesn't inflate the real phases.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 1 — Scaffold + Brand Tokens + Capability Skeleton + Error Infra

**Goal:** Create the empty BiscuitCode Tauri project, wire brand tokens into Tailwind + Rust, author capability ACL files, scaffold the error-toast + Rust error-enum infrastructure, and ship a window that paints on cocoa-700 with biscuit accent.

**Deliverables:**
- `pnpm create tauri-app` output scaffolded with React + TS + Vite + Tailwind. App name `biscuitcode`. Bundle ID `io.github.Coreyalanschmidt-creator.biscuitcode`. **License MIT** in both `package.json` and `Cargo.toml`.
- Internal workspace crate: **only `biscuitcode-core` this phase.** Sibling crates (`-agent`, `-providers`, `-db`, `-pty`, `-lsp`) are created in the phase that first uses them.
- `tauri.conf.json` with `bundle.active: true`, `bundle.identifier`, Linux section declaring `webkitVersion: "4.1"`, `deb.depends: ["libwebkit2gtk-4.1-0", "libgtk-3-0"]`, `deb.recommends: ["gnome-keyring", "ollama"]`.
- `tailwind.config.ts` with brand tokens *verbatim* (biscuit-50..900, cocoa-50..900, semantic ok/warn/error) as CSS custom properties + Tailwind theme extension.
- Self-hosted fonts: `src-tauri/fonts/Inter-{Regular,Medium,SemiBold}.woff2`, `JetBrainsMono-{Regular,Medium}.woff2`. `@font-face` rules in `src/theme/fonts.css`. Font fallback chain per Architecture Decisions — **no `system-ui`** in primary chrome.
- `src-tauri/capabilities/{core,fs,shell,http}.json` — hand-authored, deny-by-default. Core grants only `core:default`. `fs` allows `$APPCONFIG`, `$APPDATA`, `$APPCACHE` only. `shell` and `http` empty (added per feature phase).
- `src/theme/tokens.ts` exporting palette as TS constants for JS-only colour math.
- Rust `biscuitcode-core::palette` module exposing the same values.
- **Error infra (NEW, distributed catalogue per r2 G6):**
  - `src/errors/types.ts` — TypeScript discriminated union of error categories (one per top-level failure class), each with `code`, `userMessage`, `recoveryAction`, `docsLink` slots.
  - `src/errors/ErrorToast.tsx` — single component that renders any error in the union; never displays raw stack.
  - Rust `biscuitcode-core::errors` — `thiserror`-derived enum mirroring the categories; converts to a serializable `ErrorPayload` for IPC to the toast.
  - This phase ships ONE category fully wired (`E001 KeyringMissing`) as the proof-of-concept; subsequent phases add their own codes.
- **ADR `docs/adr/0001-no-stronghold.md`** — records that `tauri-plugin-stronghold` is deprecated and shall not be used; `keyring` crate is the only secrets path.
- Window chrome: default decorations off, custom titlebar showing `BiscuitCode` in Inter 14px, cocoa-700 background.
- CI workflow skeleton at `.github/workflows/ci.yml` with lint + typecheck + test + audit jobs (full content lands in Phase 10).

**Acceptance criteria:**
- [ ] `pnpm install && pnpm tauri dev` opens a WSLg window in under 2s on the dev machine.
- [ ] Document background is `#1C1610`; a single `--biscuit-500` (`#E8B04C`) accent bar renders on the sidebar placeholder.
- [ ] `curl -sS http://localhost:1420/` returns HTML with `Inter` loaded from `/fonts/`, not from any CDN. `grep -F 'fonts.googleapis' src/` returns no hits.
- [ ] `src-tauri/capabilities/fs.json` contains `"permissions"` with read scoped to `$APPCONFIG/$APPDATA/$APPCACHE` only; `grep -c '"identifier": "fs:allow-write"' src-tauri/capabilities/fs.json` returns `0`.
- [ ] `cargo tree -p biscuitcode-core` lists `biscuitcode-core` as a workspace member.
- [ ] `cargo build -p biscuitcode-core` succeeds with `-D warnings`.
- [ ] `grep -cE '^(biscuit|biscuit-auth|biscuit-cli)\s*=' src-tauri/Cargo.toml` returns `0` (no namespace-collision crate is a dependency).
- [ ] Triggering `E001 KeyringMissing` (mock IPC) renders the `ErrorToast` with the user-friendly message; no stack shown in devtools console.
- [ ] `docs/adr/0001-no-stronghold.md` exists and is referenced from `CLAUDE.md`'s Architecture Decisions section.
- [ ] CI workflow skeleton present; PR touching only `README.md` triggers it and the `lint` job exits `0`.

**Dependencies:** Phase 0.
**Complexity:** Medium.
**Split rationale:** Scaffold + brand + capabilities + error infra all need to land before any feature phase touches the corresponding surfaces. Brand tokens wrong = whole UI redo. Capabilities wrong = security holes. Error infra wrong = each subsequent phase invents its own pattern. The Stronghold ADR is in this phase because it's the first opportunity to prevent a future Tauri-secrets web search from misleading us.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 2 — Four-Region Layout + Shortcuts + i18n Scaffold + Installable .deb

**Goal:** Render the full Activity Bar / Side Panel / Editor Area / Bottom Panel / Chat Panel / Status Bar layout with `react-resizable-panels`, wire every shortcut from the vision table, scaffold i18n so all user-facing strings go through `t('key')`, and produce the first installable-to-VM `.deb`.

**Deliverables:**
- `src/layout/WorkspaceGrid.tsx` using `react-resizable-panels` with panel sizes persisted via Zustand + `localStorage` bridge (one record per persisted panel). Outer window geometry handled separately by `plugin-window-state`. Two distinct concerns — do not conflate.
- Components (empty shells with labelled placeholders): `ActivityBar`, `SidePanel`, `EditorArea`, `TerminalPanel`, `ChatPanel`, `AgentActivityPanel`, `PreviewPanel`, `StatusBar`.
- `ActivityBar` 48px, icons via `lucide-react` (Files, Search, Git, Chats, Settings). Active icon: 2px `--biscuit-500` left-edge bar.
- **Shortcut layer in `src/shortcuts/global.ts`** handling the full vision table:
  | Shortcut | Action | Phase that wires real behavior |
  |---|---|---|
  | `Ctrl+B` | toggle side panel | this phase |
  | `Ctrl+J` | toggle bottom panel | this phase |
  | `Ctrl+Alt+C` | toggle chat panel | this phase |
  | `Ctrl+P` | quick file open | Phase 3 |
  | `Ctrl+Shift+P` | command palette | this phase |
  | `` Ctrl+` `` | toggle terminal focus | Phase 4 |
  | `Ctrl+\` | split editor horizontally | Phase 3 |
  | `Ctrl+K Ctrl+I` | inline AI edit on selection | Phase 6b |
  | `Ctrl+L` | send selection to chat | Phase 5 |
  | `Ctrl+Shift+L` | new chat | Phase 5 |
  | `F1` | help | Phase 8 |
  Placeholders fire toast `"<shortcut> registered; wiring lands in Phase <n>"` so verifiability is honest. Chord support via two-stage handler.
- Command palette (`Ctrl+Shift+P`) with registered commands: `View: Toggle Side Panel`, `View: Toggle Bottom Panel`, `View: Toggle Chat Panel`. Enough to prove the registry works.
- Status bar renders `git:main`, `0 errors`, `claude-opus-4-7`, `Ln 1 C1` — all static placeholders this phase.
- **i18n scaffold:** `i18next` + `react-i18next` configured. `src/locales/en.json` containing every user-facing string in this phase. All `<button>`, `<label>`, toast text routed through `t('key')`. Lint via `i18next-parser` ensures no untranslated literals.
- `cargo tauri build --target x86_64-unknown-linux-gnu` produces `biscuitcode_0.1.0_amd64.deb`.

**Acceptance criteria:**
- [ ] Every region in the vision's ASCII layout renders with the correct default size (Activity 48px, Side 260px, Bottom 240px, Chat 380px).
- [ ] Pressing `Ctrl+B` toggles side panel visibility; after re-open the previous width is restored.
- [ ] Pressing `Ctrl+Shift+P`, typing "toggle bottom", pressing Enter toggles the bottom panel.
- [ ] **All 11 shortcuts in the table are dispatched.** Unit test `shortcuts/global.spec.ts` iterates over an explicit `KeyboardEvent` array for each shortcut and asserts either an action ran or the placeholder toast fired. None silently no-op.
- [ ] `npx i18next-parser --dry-run --fail-on-untranslated-strings` exits `0`.
- [ ] `pnpm tauri build` produces `src-tauri/target/release/bundle/deb/biscuitcode_0.1.0_amd64.deb`.
- [ ] On a Mint 22 XFCE VM: `sudo dpkg -i biscuitcode_0.1.0_amd64.deb` then `dpkg -s biscuitcode | grep -F 'Version: 0.1.0'` returns one line.
- [ ] After install, Whisker menu → Development → **BiscuitCode** exists with the placeholder icon and launches the app.
- [ ] `sudo apt remove biscuitcode` removes the binary, desktop entry, and icon; `ls /usr/share/applications/biscuitcode.desktop` returns "no such file."

**Dependencies:** Phase 1.
**Complexity:** Medium.
**Split rationale:** This is where the app first becomes a thing a user could install — the vision's Phase 1 runnable checkpoint. Bundling the shortcut layer here (rather than deferring to polish) avoids a late-stage "oh wait, Ctrl+B was never actually global" scramble. i18n scaffolding here costs ~1 hour but saves a v1.1 find-and-replace sweep across every phase's strings. The `.deb` being producible here also de-risks Phase 10 — packaging is now an incremental tightening rather than a from-scratch build.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 3 — Editor + File Tree + Find/Replace

**Goal:** Working Monaco multi-tab editor (with split-pane), live file tree with real workspace-scoped filesystem ops, and in-file + cross-file find/replace.

**Deliverables:**
- `@monaco-editor/react` pinned locally (no CDN), `vite-plugin-monaco-editor` configured with **explicit `languageWorkers: []`** at startup; languages registered on first file open by extension. Workers emitted under `monacoeditorwork/`.
- `EditorArea.tsx`: tab bar (dirty dot, middle-click close, `Ctrl+W`, `Ctrl+Shift+T` reopen-closed-tab), one Monaco instance, `ITextModel` per tab, language autodetection from extension, JetBrains Mono 14px, ligatures on by default.
- **`Ctrl+\` split editor horizontally** — wires the placeholder from Phase 2 to a real second `ITextModel` view in a new pane. Both panes scroll independently, share the model if same file.
- **`Ctrl+P` quick-open** — fuzzy file search palette pulling from the workspace file tree.
- Diff view stub (`monaco.editor.createDiffEditor`) instantiable but not wired (Phase 6b uses it).
- `SidePanel: Files` tree using a lazy `FileTreeNode` component. Initial workspace = `open-folder` dialog (via `plugin-dialog`). Context menu: New File, New Folder, Rename, Delete, Reveal in File Manager (`xdg-open`), Copy Path, Open in Terminal (emits event consumed by Phase 4).
- Rust commands in `src-tauri/src/commands/fs.rs`: `fs_list(path)`, `fs_read(path)`, `fs_write(path, bytes)`, `fs_rename(from, to)`, `fs_delete(path)`, `fs_create_dir(path)`, `fs_open_folder() -> WorkspaceId`. Each validates path-is-descendant-of-workspace or returns typed `OutsideWorkspace` error (registers as `E002` in the catalogue).
- `fs.json` capability amended: `fs:allow-read-text-file`, `fs:allow-write-text-file`, `fs:allow-read-binary-file`, `fs:allow-write-binary-file`, each scoped dynamically via `fs:scope` updated per workspace-open.
- Find in file (`Ctrl+F`) — Monaco built-in, just unhidden.
- Find in files (`Ctrl+Shift+F`) — Side Panel pane with regex/case/whole-word toggles. Backend uses `ignore` + `grep` crates over the workspace root.
- File-tree git status colouring placeholder (hook exists; real git in Phase 7).
- **Monaco lazy-load proof**: `performance.measure` instrumentation confirms Monaco bundle is fetched after initial paint (not blocking it).

**Acceptance criteria:**
- [ ] Open a TypeScript file → syntax highlight correct; JetBrains Mono renders; ligatures toggle in settings placeholder.
- [ ] **Multi-cursor (vision-mandated)**: `Alt+Click` adds a second cursor; `Ctrl+D` selects next occurrence and adds cursor; both Monaco built-ins, verified live.
- [ ] **Minimap (vision-mandated)**: rendered on the right edge of the editor by default (Monaco built-in); toggle via `editor.minimap.enabled` setting verified.
- [ ] Ctrl+W closes current tab; middle-click does the same; `Ctrl+Shift+T` reopens the most recently closed tab with cursor preserved.
- [ ] `Ctrl+\` splits the editor pane horizontally; both panes render and can show different files.
- [ ] `Ctrl+P` quick-open lists the workspace files with fuzzy match; selecting opens in the active pane.
- [ ] New File via tree creates the file on disk; rename updates disk name; delete asks confirm and removes.
- [ ] `fs_read` on a path outside the workspace root returns the typed `OutsideWorkspace` error and the toast renders error code `E002`.
- [ ] `Ctrl+Shift+F` for "TODO" across a 1k-file workspace returns results in under 2s.
- [ ] `pnpm tauri build && dpkg-deb -c biscuitcode_*.deb | grep -c monacoeditorwork` ≥ 5 (workers packaged).
- [ ] **Cold-launch to shell (no file open) under 2s on i5-8xxx.** Verified by `tests/cold-launch.sh`: `start=$(date +%s%N) ; biscuitcode & ; until wmctrl -l | grep -q BiscuitCode ; do sleep 0.05 ; done ; echo $(( ($(date +%s%N) - start) / 1000000 ))ms` reports under 2000.

**Dependencies:** Phase 2.
**Complexity:** Medium (edging into High because of Monaco worker wiring + scoped-fs runtime patching + split-pane).
**Split rationale:** Editor + file tree belong together — neither is useful alone, and the file-scope capability work needs both. Find/replace is bundled because Monaco gives Ctrl+F essentially for free, and cross-file find uses the same `fs` scope validation. Split-pane and quick-open are in this phase because they touch the editor directly. Git-status colouring is deliberately NOT here; it's in Phase 7 with the rest of git.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 4 — Terminal (xterm.js + portable-pty)

**Goal:** Multi-tab integrated terminal with `xterm.js`, real PTY-backed shells, clickable links and paths, wired to "Open in Terminal" from Phase 3 and to `` Ctrl+` `` focus from Phase 2.

**Deliverables:**
- **Create workspace crate `biscuitcode-pty` here.**
- `TerminalPanel.tsx` with tabbed `xterm.js` instances, `@xterm/addon-fit`, `@xterm/addon-web-links`, `@xterm/addon-search`, `@xterm/addon-webgl` (with canvas fallback).
- Rust `biscuitcode-pty` crate exposing commands: `terminal_open(shell, cwd, rows, cols) -> SessionId`, `terminal_input(session_id, bytes)`, `terminal_resize(session_id, rows, cols)`, `terminal_close(session_id)`.
- Two Tokio tasks per session: reader (PTY master → `terminal_data_{session_id}` event), writer (consumes queued input). Hash-map of sessions under `Arc<RwLock<HashMap<SessionId, PtySession>>>`.
- Shell detection: read `$SHELL`, else `getent passwd $UID`, else `/bin/bash`.
- Custom link provider matching `path/to/file:line[:col]` → emits `open_file_at` event consumed by editor.
- `` Ctrl+` `` focuses terminal panel (wires the Phase 2 placeholder); `+` button opens new tabs.
- Tab close drops the PTY master/slave and kills the child.
- New error code `E003 PtyOpenFailed` registered.

**Acceptance criteria:**
- [ ] Open terminal → prompt appears in under 500ms; `echo $SHELL` returns the user's shell.
- [ ] Resizing the terminal panel resizes the PTY (`tput lines && tput cols` after resize match panel dimensions).
- [ ] Click a URL in terminal output → opens in browser via `plugin-shell` (allow-listed http/https only).
- [ ] Click `src/main.rs:12` in terminal output → opens `src/main.rs` at line 12 in the editor.
- [ ] Close a terminal tab → `pgrep -f 'biscuitcode.*bash'` returns no orphans after 2s.
- [ ] Five concurrent terminals each running `yes > /dev/null` → total CPU under one core's worth on the test machine; no crash over 60s.

**Dependencies:** Phase 2.
**Complexity:** Medium.
**Split rationale:** Terminal is small enough to stand alone — vision allocates one day. Sequencing it before Phase 5 (chat) is intentional: it doesn't need providers, provides an early OS-integration win, and de-risks the Tokio stream-task pattern Phase 5 (provider streaming) and Phase 7 (LSP) will reuse.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 5 — Keyring + Anthropic Provider + Chat Panel (virtualized E2E text-only)

**Goal:** User can add an Anthropic API key in settings (stored in libsecret), open the chat panel, pick `claude-opus-4-7`, type a message, and watch streaming text render with prompt caching active — no tools, no agent loop yet.

**Deliverables:**
- **Create workspace crates `biscuitcode-providers` and `biscuitcode-db` here.**
- `biscuitcode-core::secrets` module wrapping `keyring` 3.6 with features `linux-native-async-persistent + async-secret-service + crypto-rust + tokio`. API: `async fn set/get/delete(service, key, value?)`.
- **Startup pre-flight `secret_service_available()` via `busctl --user list | grep org.freedesktop.secrets`** (read-only, NEVER a keyring probe). If absent, emits an event that blocks API-key entry and shows the install prompt in onboarding (full onboarding lands in Phase 8).
- `biscuitcode-providers::anthropic::AnthropicProvider` implementing `ModelProvider`:
  - `reqwest` with HTTP/2 keep-alive, optional prewarm on app start.
  - SSE parsing of `message_start → content_block_{start,delta,stop} → message_delta → message_stop`.
  - Delta-type handling: `text_delta` → `TextDelta`, `thinking_delta` → `ThinkingDelta`, `input_json_delta` accumulated → `ToolCallDelta`/`End` (full input only at `content_block_stop`).
  - **Sampling-param gotcha:** when model is `claude-opus-4-7`, the impl unconditionally omits `temperature`/`top_p`/`top_k` from the request body. Unit test `requests_strip_sampling_for_opus_47` asserts the request JSON lacks those keys.
  - **Prompt caching default-on:** `cache_control: {type: "ephemeral"}` applied to the system prompt and (when present) to tool definitions.
  - Models list: `claude-opus-4-7` (default), `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`, `claude-opus-4-6` (marked legacy in UI).
- `ChatPanel.tsx` with **`react-virtuoso`-virtualized** message list (markdown via `react-markdown` + `remark-gfm`), code blocks with copy button (apply/run deferred to Phase 6b), model picker reading from provider list, send button, streaming token rendering. **Wires `Ctrl+L` (send selection to chat) and `Ctrl+Shift+L` (new chat) from Phase 2.**
- `biscuitcode-db` crate using `rusqlite` with WAL mode, `PRAGMA user_version` migrations, initial schema: `workspaces`, `conversations`, `messages` per research §10. Migration file embedded as Rust const string.
- `http.json` capability: fetch allowlist `https://api.anthropic.com/**`.
- Settings stub (`SettingsProviders.tsx`): list providers, status badges (green = key valid via test request, yellow = key present but untested, red = no key / invalid), test-connection button.
- New error codes: `E004 AnthropicAuthInvalid`, `E005 AnthropicNetworkError`, `E006 AnthropicRateLimited`.
- First-token-latency measurement emitted as a telemetry-scaffold event (no wire).

**Acceptance criteria:**
- [ ] `Settings → Models → Anthropic → Add key` stores the key in libsecret; `secret-tool search service biscuitcode` returns the value from the daemon, NOT from any file under `~/.config/biscuitcode/`.
- [ ] `grep -r 'ANTHROPIC_API_KEY\|sk-ant' ~/.config/biscuitcode/` returns nothing after key entry.
- [ ] On a VM without `gnome-keyring`, the add-key flow shows error code `E001 KeyringMissing` with the exact install command (`sudo apt install gnome-keyring libsecret-1-0`); no plaintext file created.
- [ ] Typing "say hi in three words" → assistant tokens render in **under 500ms p50, under 1200ms p95**, measured by `tests/ttft-bench.ts` over 20 sequential prompts after a 1-minute prewarm. (Reasoning models exempt.)
- [ ] Sending the same message with `claude-opus-4-7` selected and `temperature: 0.7` attempted via devtools shim returns HTTP 200 (the provider filtered the field).
- [ ] **Prompt-caching verification:** sending the same long system prompt twice within 5 minutes — second response includes `cache_read_input_tokens > 0` in the `message_delta` usage block. Unit test `cache_control_applied_to_system_prompt` asserts the request body contains `"cache_control":{"type":"ephemeral"}`.
- [ ] The conversation is persisted — reopen app, prior message visible, `messages` table populated.
- [ ] `Ctrl+L` with a selection in Monaco inserts the selection as a quoted block in the chat input.
- [ ] `Ctrl+Shift+L` opens a fresh conversation.

**Dependencies:** Phase 2.
**Complexity:** Medium (high on the keyring edge cases).
**Split rationale:** Combining keyring + Anthropic + chat panel into one phase matches the vision's "one provider E2E" checkpoint. Keyring alone is too small; provider alone has no UI; chat panel alone has nothing to call. Bundling them produces a real runnable milestone ("chat with Claude works") in 2 days. Phase 6a brings the other providers + tools because adding two more providers before tools exist would stall the more valuable agent-loop work.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 6a — OpenAI + Ollama Providers + Read-Only Tool Surface + Agent Activity UI

**Goal:** Ship the remaining two providers behind the same `ModelProvider` trait, ship the read-only half of the agent tool surface (`read_file`, `search_code`), and render the Agent Activity panel with live-streaming tool-call cards. Ollama detection + install + Gemma 4 auto-pull included. End state: user can ask "find all TODO comments and summarize them" with any of three providers and watch the agent work.

**Deliverables:**
- **Create workspace crate `biscuitcode-agent` here** (read-only tools subset only; write tools land in 6b).
- `biscuitcode-providers::openai::OpenAIProvider`:
  - SSE parsing of Chat Completions deltas.
  - Per-index `tool_calls` argument accumulation until `finish_reason === "tool_calls"`, then emit `ToolCallStart + ToolCallDelta* + ToolCallEnd` into the same `ChatEvent` stream.
  - **Default `gpt-5.4-mini`.** Picker exposes `gpt-5.4`, `gpt-5.4-pro` (reasoning), `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5.3 Instant`. Legacy `gpt-5.2 Thinking` shown but tagged legacy until 2026-06-05.
  - `reasoning.effort` surfaced as an optional per-conversation setting.
  - Reasoning models exempt from the TTFT gate; UI shows `Thinking…` state.
- `biscuitcode-providers::ollama::OllamaProvider`:
  - NDJSON parsing of `/api/chat` (line-delimited JSON, one object per line).
  - `tools` passthrough in OpenAI-function-call format; extract `message.tool_calls` from the final non-done chunk. **Robust XML-tag fallback:** if tool_calls is empty but `message.content` contains a `<tool_call>...</tool_call>` block (common with Gemma 3 base / community fine-tunes), regex-extract and emit it as a `ToolCallStart/End` pair.
  - Model picker pulls from `GET /api/tags` (local models).
  - **Primary default ladder = Gemma 4 by RAM (verified tags as of 2026-04-18):**
    | RAM | Primary default | Why this tier | Agent-mode alternative | Fallback if Gemma 4 unavailable |
    |---|---|---|---|---|
    | < 8 GB | `gemma4:e2b` | 7.2GB file; smallest viable | `gemma4:e2b` | `gemma3:1b` |
    | 8–16 GB | `gemma4:e4b` | 9.6GB file; `:latest` alias | `gemma4:e4b` | `gemma3:4b` |
    | 16–32 GB | `gemma4:e4b` | leaves headroom for editor + browser | `qwen2.5-coder:7b` (preferred for code-heavy agent mode) | `gemma3:4b` + `qwen2.5-coder:7b` |
    | 32–48 GB | `gemma4:26b` | MoE: 25.2B total / **3.8B active** = highly RAM-efficient at runtime | `qwen2.5-coder:32b` | `gemma3:12b` + `qwen2.5-coder:7b` |
    | ≥ 48 GB | `gemma4:31b` | 30.7B dense; best quality if RAM allows | `qwen2.5-coder:32b` | `gemma3:27b` |

    **Tag verification:** `gemma4:e2b`, `gemma4:e4b`, `gemma4:26b`, `gemma4:31b` confirmed against `https://ollama.com/library/gemma4` on 2026-04-18. `gemma4:e4b` is also published as `gemma4:latest`. `gemma4:31b-cloud` exists for cloud-hosted use; not appropriate for our local-default story.

    **All Gemma 4 variants support native function calling** (per Google's release notes, verified by Ollama's NDJSON `/api/chat` endpoint emitting structured `message.tool_calls`). The XML-tag fallback below is therefore needed only for Gemma 3 community fine-tunes — kept in the code path as defensive parsing.

    **Selection logic at first run:** ping `GET /api/tags` to list local models. If any `gemma4:*` is present, use it as the primary default per the table above. If not, attempt `ollama pull` of the appropriate Gemma 4 tier; if that pull fails (e.g., Ollama version < 0.20.0 doesn't recognize the tag), fall back to the Gemma 3 ladder shown in the rightmost column and surface a one-time toast `Gemma 4 unavailable on your Ollama version (need >= 0.20.0); using Gemma 3 fallback. Run 'curl -fsSL https://ollama.com/install.sh | sh' to upgrade.` (catalogue code `E007 GemmaVersionFallback`).
  - `ollama_install()`: detects absence via `curl -sSfm 1 http://localhost:11434/api/version` and `which ollama`. On missing, shows confirm dialog with verbatim command `curl -fsSL https://ollama.com/install.sh | sh` and runs via `plugin-shell` *only after* user confirms.
  - `ollama_pull(model)`: progress events piped from `ollama pull` stdout to a progress bar.
  - RAM detection via `sysinfo` crate.
- `http.json` capability: add `http://localhost:11434/**` and `https://api.openai.com/**` to fetch allowlist.
- `shell.json` capability: add `ollama` to command registry, argument regex limited to `pull <model>`, `list`, `show <model>`, `serve`, `--version`.
- Per-conversation model switch: chat panel model dropdown is conversation-scoped, persisted to `conversations.active_model`.
- All three provider status badges go live (green/yellow/red).
- **Read-only tool surface in `biscuitcode-agent::tools`:**
  - `read_file(path)` — workspace-scope-validated, returns file contents up to 256KB.
  - `search_code(query, glob?, regex?)` — wraps the Phase 3 `ignore`+`grep` backend, returns matches with line numbers.
- `biscuitcode-agent::executor` — ReAct loop, READ-ONLY mode this phase:
  - Accepts a conversation; streams from selected provider.
  - On `ToolCallEnd`, decodes args, executes the read-only tool (no confirmation needed for reads), appends `ToolResult`, continues looping until `Done` with no further tool calls.
  - Pause flag checked at loop boundaries (single atomic bool).
  - **Worst-case pause latency: 5 seconds** when no tool is currently running.
  - Write/shell tools registered as `not_yet_available` errors so the model gets a clear signal that those land in 6b.
- **`AgentActivityPanel.tsx`** rendering tool calls as collapsible cards (running/ok/error status, timing, pretty-JSON args, streamed result). Uses `react-virtuoso` for virtualization (shared abstraction from Phase 5). Badge on chat message links to the card.
- **Agent mode toggle** in chat panel (default off). When off, loop stops after first assistant message; when on, auto-continues on tool calls.
- **Tool-card render trace instrumentation:** on every `ToolCallStart` event the executor emits `performance.mark('tool_call_start_<id>')`; when the Agent Activity card first paints, a MutationObserver emits `performance.mark('tool_card_visible_<id>')`. Persisted in debug log for the gate in Phase 9.
- **Chat context mentions — editor-local subset:** typing `@` in chat input opens picker for `@file` (fuzzy over workspace tree), `@folder`, `@selection` (current editor selection). Each resolves to a structured context block in the user message. Non-editor mentions land in Phase 7.
- **Drag-file-into-chat:** dropping a file from the file tree onto the chat input inserts an `@file:<path>` token.

**Acceptance criteria:**
- [ ] With an OpenAI key set, sending a message using `gpt-5.4-mini` streams text; tool call for `search_code("TODO")` returns valid JSON args and completes.
- [ ] Switching the same conversation to `claude-opus-4-7` mid-thread preserves prior messages; Claude sees the OpenAI tool result as input.
- [ ] On a VM without Ollama, clicking "Install Ollama" shows the confirm dialog with the full `curl | sh` command before executing; declining does nothing.
- [ ] **After Ollama install on a 16 GB system: the picker shows `gemma4:e4b` selected by default; `ollama list` confirms `gemma4:e4b` was pulled (9.6 GB).** On systems whose Ollama version (< 0.20.0) does not recognize Gemma 4 tags, the picker shows a Gemma 3 default with the `E007 GemmaVersionFallback` toast and the upgrade-Ollama install command.
- [ ] **RAM-tier selection verified against the table in deliverables:** 4 GB system → `gemma4:e2b` (or `gemma3:1b` fallback); 12 GB → `gemma4:e4b`; 32 GB → `gemma4:26b`; 64 GB → `gemma4:31b`. Each tier verified by spinning up a constrained-RAM VM and confirming the picker's default + the `ollama pull` command issued.
- [ ] **Native tool calling on Gemma 4:** sending an "agent mode on" prompt with a registered `read_file` tool to `gemma4:e4b` returns a structured `message.tool_calls` array in the NDJSON stream — NOT a `<tool_call>` XML block in `message.content`. (XML-tag fallback path is exercised separately by the Gemma 3 fallback test below.)
- [ ] **Concrete agent-mode demo:** with agent mode ON, sending the exact prompt `"List every file under src/ that contains the string TODO and summarize each TODO in one sentence"` to Anthropic produces (in order) (1) a `search_code` tool call with `query: "TODO"` and `glob: "src/**"`, (2) a `read_file` call for each match, (3) a final assistant text message containing one summary line per file. Repeating the same prompt against Ollama with a Gemma 4 model produces the same tool call sequence (timing may differ). Verified by `tests/e2e/agent-mode-demo.spec.ts`.
- [ ] **Cross-provider snapshot:** a single `ChatEvent` stream produced by an equivalent "hello" prompt has identical event shape across all three providers (snapshot test `tests/provider-event-shape.spec.ts`).
- [ ] **Agent pause:** pressing Pause during a long agent run stops before the next tool call AND **within 5 seconds** if no tool is currently running.
- [ ] **Read-only safety:** the model attempting to call `write_file` receives a clear error indicating "tool not available in this build (lands in 6b)" rather than a tool-not-found 500.
- [ ] Typing `@` in chat opens the mention picker; `@file` then a filename inserts the structured token; the backend sees the file content in the request payload.
- [ ] Dropping a file from the tree onto chat input inserts the same `@file:<path>` token as the picker.
- [ ] **Tool-card render latency gate**: for the canonical 3-tool prompt at `tests/fixtures/canonical-tool-prompt.md`, every `tool_card_visible_<id> - tool_call_start_<id>` measure is under `250ms` — e2e test `tests/e2e/agent-tool-card-render.spec.ts`.
- [ ] On a Gemma 3 fallback that emits `<tool_call>` XML in `message.content`, the Ollama provider extracts and emits a `ToolCallStart/End` pair correctly (regex-tested).

**Dependencies:** Phase 5 (trait, chat panel, keyring, conversation persistence).
**Complexity:** Medium.
**Split rationale:** Bundling all three providers WITH the read-only agent surface means the `ChatEvent` contract gets validated against three real providers before any write tool depends on it. Provider quirks surface here (Anthropic content-blocks, OpenAI indexed deltas, Ollama NDJSON + XML fallback) where the loop can be debugged in isolation from the riskier write/rewind work. The Agent Activity UI is here because every provider emits `ToolCallStart/End` events the panel needs to render, and the read-only tool surface gives the panel real data to display.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 6b — Write Tools + Inline Edit (split-diff) + Rewind

**Goal:** Add the write-side of the agent tool surface (`write_file`, `run_shell`, `apply_patch`) with explicit per-tool confirmation UX, ship inline AI edit (`Ctrl+K Ctrl+I`) using Monaco's split-diff editor, and ship per-action rewind so any agent action can be undone.

**Deliverables:**
- **Write tools in `biscuitcode-agent::tools`:**
  - `write_file(path, contents)` — workspace-scope-validated; **always requires confirmation** unless `workspace.trust = true`. Confirmation modal shows the full diff (created file → preview, edited file → unified diff).
  - `run_shell(command, args, cwd)` — sandboxed: no `sudo`, no shell metacharacters in unquoted args, no network calls except via the provider HTTP scope. Always requires confirmation unless `workspace.trust = true`. Modal shows the verbatim command line.
  - `apply_patch(path, patch)` — like `write_file` but takes a unified-diff patch; same confirmation rules.
- **Per-action snapshot:** before each write/shell tool, snapshot the affected file(s) to `~/.cache/biscuitcode/snapshots/{conversation_id}/{message_id}/...` and record the manifest in the messages table. Snapshots are kept for the conversation's lifetime; a Phase 8 cleanup ages out snapshots > 30 days for closed conversations.
- **Workspace trust toggle** in settings (stored in `settings.json`). When on, write/shell tools auto-approve. Per-workspace, persisted by workspace path hash.
- **Inline edit (`Ctrl+K Ctrl+I`)** — wires the Phase 2 placeholder:
  - Select code → shortcut → popover with description input → backend calls provider with edit prompt + selection + file path → diff opens in a `monaco.editor.createDiffEditor` split pane → buttons: Accept, Reject, Regenerate.
  - **Zed-style split-diff** (NOT Cursor-style in-place decoration). Whole-diff Accept/Reject in v1; per-hunk in v1.1.
  - Streaming: as the provider streams, the diff editor updates incrementally.
- **Rewind UI:** conversation header shows a rewind button per assistant message that performed write/shell tool calls. Clicking it (a) restores snapshots referenced by that message and any messages after it, (b) truncates messages past that point in the DB.
- **Apply/Run buttons on chat code blocks:** `Apply` opens the affected file and applies the patch; `Run` pushes the selected code into a new terminal tab (no auto-exec — user hits Enter).
- New error codes: `E008 WriteToolDenied`, `E009 ShellForbiddenPrefix`, `E010 SnapshotFailed`, `E011 RewindFailed`.

**Acceptance criteria:**
- [ ] Write-tool call ("create a file `hi.txt` with contents `hello`") triggers a confirmation modal showing the diff; decline prevents file creation; accept creates it.
- [ ] Rewind on the assistant message that created `hi.txt` restores its pre-create state (file removed) and removes messages after.
- [ ] `Ctrl+K Ctrl+I` on a selected function inside Monaco streams a diff into a split-diff pane; Accept applies, Reject discards, Regenerate re-streams.
- [ ] `run_shell` called with `sudo rm -rf /` is rejected before execution with error code `E009 ShellForbiddenPrefix`; the catalogued toast shows.
- [ ] `run_shell` called with `curl https://example.com` (no allow-listed host) is rejected; `curl https://api.anthropic.com/...` would also be rejected because shell-out HTTP isn't the provider scope.
- [ ] All workspace-trust-off runs prompt; with workspace-trust-on the same runs do not prompt (verified per workspace, by path hash).
- [ ] **Snapshot integrity:** after a multi-step agent run that edits 3 files, rewind restores all 3 to their pre-run state byte-identical (`sha256sum` matches).
- [ ] **Snapshot crash safety:** killing the app mid-write-tool leaves the snapshot manifest in a recoverable state (next launch can complete the rewind cleanly).

**Dependencies:** Phase 3 (file system, tabs, diff-editor stub), Phase 6a (read-only tools, executor, agent activity UI).
**Complexity:** High.
**Split rationale:** This is the highest-risk subsystem in the project — a correctness bug in rewind could delete user code. Splitting it from 6a means the read-only agent stays shippable if 6b needs replanning. Inline edit is in this phase rather than Phase 3 because it depends on the provider (Phase 5) and on the confirmation/diff UX this phase defines. Rewind is here too because its snapshots are a side-effect of the write tool's execution, not a later add-on.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 7 — Git Panel + LSP Client + Preview Panel

**Goal:** VS Code parity features: git panel with stage/unstage/commit/push/pull, working LSP client for five languages, and preview panel covering Markdown, HTML, images (5 formats), PDF, and read-only notebooks.

**Deliverables:**
- **Create workspace crate `biscuitcode-lsp` here.**
- **Git** via `git2-rs` (reads) + `std::process::Command('git')` (writes):
  - Side Panel Git pane: files grouped by `staged`/`unstaged`/`untracked`, hunk-level stage/unstage (Monaco inline diff buttons), commit message input, commit button, push/pull buttons that stream stdout to the Terminal panel.
  - Branch name in status bar, clickable → branch switcher dropdown.
  - **Gutter blame:** off by default; settings toggle `editor.blame.gutter = true` enables it. Uses `git2::BlameOptions` per visible line range; re-blames on `git commit` or file save; shows `hash[0..7] · author · relative-date` in left gutter (180px column).
  - File tree git status colours (M/U/A/D) now live.
- **LSP** via `biscuitcode-lsp` crate + `monaco-languageclient` frontend:
  - Rust spawns `rust-analyzer`, `typescript-language-server --stdio`, `pyright-langserver --stdio`, `gopls`, `clangd` based on detected project files (`Cargo.toml`, `package.json`/`tsconfig.json`, `pyproject.toml`/`requirements.txt`, `go.mod`, `CMakeLists.txt`/`compile_commands.json`).
  - Tauri events `lsp-msg-in-{session_id}` + `lsp_write` command as proxy; frontend `MessageTransports` adapter.
  - **Missing-server dialog: copy-to-clipboard install command.** No auto-run of `rustup component add`, `npm i -g`, etc.
  - Diagnostics rendered as Monaco squigglies + problem count in status bar.
- **Preview Panel** (split pane in editor area, NOT a new window):
  - Markdown: `react-markdown` + `remark-gfm` + `rehype-highlight` + `mermaid` + `rehype-katex`, live update.
  - HTML: sandboxed iframe with `sandbox="allow-scripts"` (no forms, no top-navigation), live-reload on save, devtools button.
  - **Images: PNG, JPG, WebP, SVG, GIF** with CSS zoom/pan; animated GIFs honor loop count via `<img>`.
  - PDF: `pdf.js` via `react-pdf`, single-page view with next/prev.
  - Notebook (`.ipynb`): read-only render — parse cells, render markdown cells as markdown, code cells as JetBrains Mono, outputs as text/mime-typed blocks. **No execution, no "Run all" placeholder button.**
  - Auto-open rule: AI-edited `.md`, `.html`, `.svg`, image → open preview as split pane (Phase 6b emits the event; this phase consumes it).
- `shell.json` capability: add `which <binary>` and the LSP binary paths to the registry; no wildcard args.
- **Non-editor chat mentions land here:** `@terminal-output` (active terminal tab's visible buffer), `@problems` (all LSP diagnostics in current workspace), `@git-diff` (output of `git diff` for staged + unstaged). Picker disables mentions when their data source has no content (e.g., no terminals open → `@terminal-output` greyed out).
- New error codes: `E012 GitPushFailed`, `E013 LspServerMissing`, `E014 LspProtocolError`, `E015 PreviewRenderFailed`.

**Acceptance criteria:**
- [ ] Open a Rust file → `rust-analyzer` starts → hover shows type; go-to-definition jumps correctly; diagnostics appear.
- [ ] In a repo: stage a hunk via the inline diff button; status changes from `unstaged` to `staged`; commit with a message; `git log -1` shows it.
- [ ] Branch switcher shows all local branches; switching updates the status bar within 500ms.
- [ ] Opening `README.md` and hitting preview shows rendered markdown side-by-side; typing updates the preview within 200ms.
- [ ] A `.ipynb` with 3 cells renders read-only with cell borders; no run controls visible.
- [ ] Image preview correctly displays PNG, JPG, WebP, SVG, and GIF samples (5 fixture files); GIF animates.
- [ ] Missing language server (e.g., `clangd` absent) triggers a toast with error code `E013 LspServerMissing` and a copy-to-clipboard `sudo apt install clangd` command; the app does not auto-run it.
- [ ] HTML preview iframe cannot navigate away (`window.top.location` attempts blocked by sandbox).
- [ ] Gutter blame off by default; enabling in settings shows `hash · author · relative-date` strings in the editor gutter for the active file; toggling off removes them.
- [ ] Typing `@` in chat with a terminal open and an LSP diagnostic present surfaces `@terminal-output`, `@problems`, `@git-diff` options in the picker (disabled items when no data).

**Dependencies:** Phase 3 (editor, file tree, fs scope).
**Complexity:** High.
**Split rationale:** Git + LSP + Preview are three distinct subsystems, but each alone is a half-day and they all share Phase 3's editor. Splitting them into three phases would create thrash (3× PR overhead, 3× VM smoke test). They're independent enough that a coder may parallelize internally, but the plan treats them as one coherent "VS Code parity" phase to keep the phase count honest. If a coder finds the scope too wide at execution time, they may flag `Needs Replanning` and we'll split.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 8 — Onboarding + Settings UI + Theming + Icon + Data Polish

**Goal:** Ship the 3-screen onboarding, full settings UI with raw-JSON power-user mode, three themes with live preview, the final icon set rendered from `packaging/icons/biscuitcode.svg` (Concept A), conversation export/import, branching UI, and snapshot cleanup.

**Deliverables:**
- **Onboarding flow** (`OnboardingModal.tsx`) — 3 screens:
  1. **Welcome**: BiscuitCode logo + tagline + Next.
  2. **Pick models**: provider cards (Anthropic, OpenAI, Ollama). Each: add-key UI or Install Ollama button. Must set at least one before Next.
  3. **Open a folder**: file picker; also offers "Continue without a folder" for just-exploring mode.
- **Keyring absence handling in Step 2:** if `busctl` pre-flight fails, Step 2 shows a blocking dialog with the exact `sudo apt install gnome-keyring libsecret-1-0 libsecret-tools` command and a Retry button.
- **Settings page** (`SettingsPage.tsx`) with sections: General, Editor, Models, Terminal, Appearance, Security, Conversations, About. Raw JSON editor button opens `~/.config/biscuitcode/settings.json` in the Monaco editor for power-users.
- **Three themes:** `BiscuitCode Warm` (dark, default), `BiscuitCode Cream` (light), `High Contrast`. Each defined as CSS variable overrides in `src/theme/themes.ts`. Live preview on hover in Appearance pane.
- **GTK theme detection at startup:** Rust `detect_gtk_theme()` via `xfconf-query -c xsettings -p /Net/ThemeName`, fallback `gsettings get org.gnome.desktop.interface gtk-theme`. Regex `-dark$` (case-insensitive) → dark; otherwise light. On first run with a light GTK theme, offer to switch to Cream.
- **Icon:** `packaging/icons/biscuitcode.svg` authored as **Concept A** — biscuit-gold `>_` glyph on cocoa-dark rounded-square (#1C1610, 22% corner radius). Render with `rsvg-convert` to `biscuitcode-{16,32,48,64,128,256,512}.png`. `.ico` for Windows future.
- **16x16 render verification:** CI step asserts `biscuitcode-16.png` pixel-level legibility — at least 2 distinct pixels forming a `>` shape and 3 pixels for `_`. Visual diff against a checked-in reference. **If the check fails, switch to Concept D ("The Biscuit") and re-test.**
- **Conversation branching UI:** edit a past user message → fork; tree view in conversation header showing branches with parent pointers (DB schema already supports via `parent_id`). Switching branches loads the alternate message chain.
- **Conversation export/import:** Settings → Conversations → "Export all" produces `biscuitcode-conversations-<date>.json` (full DAG); "Import" merges a previously-exported file (skipping duplicates by `(conversation_id, message_id)`).
- **Snapshot cleanup:** background task deletes `~/.cache/biscuitcode/snapshots/<conv>/...` directories whose conversations have been deleted OR whose snapshots are > 30 days old AND the conversation is closed. Setting under Conversations to disable.
- **Telemetry placeholder:** Settings → General → "Send anonymous crash reports" toggle. Default off. **No wire implementation in v1** — toggle persists to settings.json but no endpoint is called. Tooltip: "Endpoint TBD; reports not yet sent."
- **VS Code theme import:** placeholder entry under Appearance, disabled, tooltip "Coming in v1.1."
- **Font-load canary** (per r2 G9): on startup, a hidden offscreen `<span style="font-family: Inter">` is measured; if the metrics match the system fallback (Ubuntu) instead of Inter, log a warning and show a one-time toast `Inter font failed to load — using Ubuntu fallback. Re-install BiscuitCode to restore.` (catalogue code `E016 FontLoadFailed`).

**Acceptance criteria:**
- [ ] Fresh install → first launch shows onboarding; no way to reach the main UI without either setting a provider or clicking "Skip" in step 2 (skip leaves all badges red).
- [ ] Onboarding step 2 on a keyring-absent VM shows the install command (error code `E001`); Retry progresses once `gnome-keyring` is installed.
- [ ] Settings → Appearance → hover Cream → preview shows cocoa-50 background, biscuit-900 text; selecting Cream → theme persists across restart.
- [ ] With GTK theme `Mint-Xia-Light` set, first run offers to switch to Cream; the offer does not appear on later launches.
- [ ] **16x16 icon legible** — CI pixel check passes; manual visual check in the XFCE system tray shows `>_` recognizable.
- [ ] `desktop-file-validate packaging/deb/biscuitcode.desktop` exits `0`.
- [ ] `grep -rn 'system-ui' src/` returns no hits in primary chrome (mono fallbacks OK).
- [ ] **Conversation branching:** edit a past user message → fork created; tree view shows two branches with timestamps; switching loads correct messages.
- [ ] **Conversation export:** clicking "Export all" produces a file matching schema in `docs/CONVERSATION-EXPORT-SCHEMA.md`. Re-importing the same file produces zero new rows (duplicate detection works).
- [ ] **Snapshot cleanup:** running the cleanup task on a workspace with a 31-day-old closed-conversation snapshot deletes the snapshot directory; an open conversation's snapshots are untouched regardless of age.
- [ ] **Font canary:** simulating Inter load failure (delete woff2 in dev) triggers `E016 FontLoadFailed` toast on next launch.

**Dependencies:** Phase 5 (onboarding needs keyring + Anthropic), Phase 6a (Ollama install path).
**Complexity:** Medium.
**Split rationale:** Onboarding + settings + theming + icon + data polish cluster naturally as user-chrome work that needs the provider setup from Phase 5/6a and the data layer from Phase 5. Doing this before Phase 9 (a11y + error consolidation) is critical because a11y audit needs the final UI surface to audit. Conversation branching ships here (rather than 6b) because it's a DB-pure feature that needs no agent involvement — it's polish on top of Phase 5's persistence.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 9 — a11y Audit + Error Catalogue Consolidation + Auto-Update Wiring

**Goal:** Audit the cumulative error catalogue scaffolded in Phases 1–8, achieve "reasonable a11y posture" across all panels (keyboard nav, ARIA, focus rings), and wire the auto-update mechanism (Tauri updater plugin for AppImage; GitHub Releases API check-for-updates button for `.deb`).

**Deliverables:**
- **Error catalogue audit (`docs/ERROR-CATALOGUE.md`):** consolidates every `E0NN` registered across Phases 1–8 into one document. Each entry: code, category, user-facing message, recovery action, docs link. Asserts every registered code has a corresponding `ErrorToast` test that triggers the failure path and inspects the rendered toast. Adds any missing low-priority codes spotted during audit.
- **a11y audit pass:**
  - All icon buttons get `aria-label` from `t('a11y.<key>')`.
  - Streaming chat container has `role="log"` and `aria-live="polite"`.
  - Modals trap focus, restore focus on close, dismiss on Escape.
  - All interactive elements reachable via Tab in a sensible order.
  - Focus rings visible (2px `--biscuit-500` outline) on every focusable element.
  - High Contrast theme verified to meet WCAG 2.1 AA contrast for text and UI controls (axe-core run in CI).
- **Auto-update — AppImage path:**
  - `tauri-plugin-updater` configured. Update endpoint = a static JSON manifest at `https://github.com/Coreyalanschmidt-creator/biscuitcode/releases/latest/download/latest.json` (generated by Phase 10 CI).
  - On launch, check for updates (configurable interval; default 24h); if newer, prompt user with changelog excerpt; on accept, download + replace + restart.
- **Auto-update — `.deb` path:**
  - "Check for updates" button in Settings → About queries GitHub Releases API (`/repos/Coreyalanschmidt-creator/biscuitcode/releases/latest`) and compares the tag to current version.
  - If newer, show a modal with changelog and a "Download .deb" button that opens the release page in the browser. **No auto-install of `.deb`** (requires sudo).
  - Toast on first launch after install if a newer version is detected.
- New error codes: `E017 UpdateCheckFailed`, `E018 UpdateDownloadFailed`.

**Acceptance criteria:**
- [ ] `docs/ERROR-CATALOGUE.md` exists and lists at least the 18 codes registered across Phases 1–8 + this phase. Each entry has all five fields filled.
- [ ] For each catalogued error, an e2e test triggers the failure (e.g., disconnect network → `E005`, revoke key → `E004`, stop `ollama serve` → Ollama-down) and asserts the catalogued toast renders. Test file: `tests/error-catalogue.spec.ts`.
- [ ] **a11y audit:** `pnpm test:a11y` (axe-core) reports zero violations on the canonical screens (Welcome, Editor, Chat, Settings, Onboarding).
- [ ] Tab through the app from a clean launch: no element traps focus; the order is sensible (Activity Bar → Side Panel → Editor → Chat → Bottom Panel → Status Bar).
- [ ] In High Contrast theme, every text/UI-control combination passes 4.5:1 contrast (axe-core).
- [ ] **AppImage update:** running an older AppImage with a newer `latest.json` published prompts the user; accepting downloads and replaces; relaunch shows the new version.
- [ ] **`.deb` update check:** clicking "Check for updates" with a newer release tag fetches the release info and shows the modal with the changelog; clicking Download opens the release page.
- [ ] No update path attempts `sudo` or auto-install of `.deb`.

**Dependencies:** Phase 7 (final UI surfaces to audit), Phase 8 (settings UI hosts the update toggle/button).
**Complexity:** Low.
**Split rationale:** Each of these three concerns is small (~half day) but together they don't fit into Phase 8's polish phase without diluting its focus. Auto-update specifically needs Phase 10's CI to publish the `latest.json` manifest, but the app-side wiring lives here so Phase 10 can be pure CI/packaging.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

### Phase 10 — Packaging + CI + GPG Signing + Release Smoke Test

**Goal:** Build `biscuitcode_1.0.0_amd64.deb` + `BiscuitCode-1.0.0-x86_64.AppImage` in GitHub Actions on `ubuntu-24.04` runners, GPG-sign, publish SHA256, generate the auto-update manifest, and smoke-test on a fresh Mint 22 XFCE VM.

**Deliverables:**
- `tauri.conf.json` `bundle` section finalized: `targets: ["deb", "appimage"]`, `deb.depends: ["libwebkit2gtk-4.1-0", "libgtk-3-0"]`, `deb.recommends: ["gnome-keyring", "ollama"]`, `deb.suggests: ["rust-analyzer", "typescript-language-server", "pyright", "gopls", "clangd"]`, `deb.section: "devel"`, correct `maintainer`, `description`.
- `.github/workflows/release.yml` — on tag `v*`:
  - Runner `ubuntu-24.04`.
  - Linux deps install step (full list from research §12).
  - `pnpm install --frozen-lockfile`.
  - `tauri-apps/tauri-action@v0` with `args: "--target x86_64-unknown-linux-gnu"`, `tagName: v__VERSION__`, `releaseName: BiscuitCode v__VERSION__`.
  - GPG import using `GPG_PRIVATE_KEY` secret; `gpg --detach-sign --armor` both artifacts.
  - **Generate `latest.json`** for the Tauri updater: `{"version": "...", "notes": "...", "platforms": {"linux-x86_64": {"signature": "...", "url": "..."}}}`.
  - `sha256sum biscuitcode_*.deb BiscuitCode-*.AppImage > SHA256SUMS.txt`.
  - Upload `.deb`, `.AppImage`, `.deb.asc`, `.AppImage.asc`, `latest.json`, `SHA256SUMS.txt` to the release.
  - `linuxdeploy` retry wrapper for AppImage step (handles the documented flake).
- `.github/workflows/ci.yml` — on PR: lint (`cargo clippy -D warnings`, `pnpm lint`), typecheck (`tsc --noEmit`), tests (`cargo test --workspace`, `pnpm test`, `pnpm test:a11y`), security audits (`cargo audit`, `pnpm audit --prod`). This file was skeleton-scaffolded in Phase 1 and is fully populated here.
- AppImage `libfuse2t64` handling: README banner + a postinstall wrapper script that prompts install if missing.
- Release smoke-test checklist in `docs/RELEASE.md` — pointer to Global Acceptance Criteria rather than a restatement. The "Release smoke test" section reads: "Run the full Global Acceptance Criteria checklist on a fresh Mint 22 XFCE VM. If any item fails, do not tag the release." VM matrix: one X11 session each on 22.0, 22.1, 22.2. **No Wayland-XFCE row** (not reachable). Cinnamon-Wayland 22.2 is a best-effort row that does not block release.
- **Three screenshots for README** using `BiscuitCode Warm` theme: main editor with chat, Agent Activity mid-run, preview split pane.
- README: install instructions (.deb double-click via GDebi; AppImage chmod+run), screenshots, license, link to `docs/DEV-SETUP.md`.

**Acceptance criteria:**
- [ ] Pushing a `v1.0.0` tag triggers CI; within ~15 min the release page has both artifacts, both `.asc` signatures, `latest.json`, and `SHA256SUMS.txt`.
- [ ] `gpg --verify biscuitcode_1.0.0_amd64.deb.asc biscuitcode_1.0.0_amd64.deb` returns "Good signature".
- [ ] `sha256sum -c SHA256SUMS.txt` passes.
- [ ] `latest.json` validates against the Tauri updater schema (`tauri updater check` in a v1.0.0 client returns `up_to_date`; in a v0.9.0 client returns `update_available` with the v1.0.0 URL).
- [ ] On fresh Mint 22 XFCE VM (X11 — 22.0, 22.1, 22.2): Global Acceptance Criteria checklist passes 100%.
- [ ] On Mint 22.2 with Cinnamon-Wayland session (best-effort): cold-launch succeeds, clipboard copy/paste in terminal works. Failures here are logged in release notes but do not block.
- [ ] `tests/cold-launch.sh` reports under 2000ms on the i5-8xxx test machine.
- [ ] `apt remove biscuitcode` removes binary, desktop entry, icons across all 7 sizes, and the `/usr/bin/biscuitcode` symlink.
- [ ] README screenshots render without `lorem ipsum` or any `TODO` strings.
- [ ] `cargo audit` clean; `pnpm audit --prod` clean.

**Dependencies:** Phase 9 (needs auto-update wiring that consumes `latest.json`).
**Complexity:** Medium.
**Split rationale:** Packaging is the "prove it's shippable" phase. It deliberately lands last because the `.deb` has been producible since Phase 2 — this phase is about signing, CI, the AppImage `libfuse2t64` caveat, the auto-update manifest, and the release checklist rather than packaging-from-scratch.
**Status:** Not Started

#### Pre-Mortem
_To be filled by coder before implementation._

#### Execution Notes
_To be filled by coder after implementation._

---

## Global Acceptance Criteria

Span the whole project; checked at Phase 10 against the signed `v1.0.0` `.deb`.

- [ ] `sudo dpkg -i biscuitcode_1.0.0_amd64.deb` installs clean on fresh Mint 22 XFCE (22.0, 22.1, 22.2) VMs; `sudo apt remove biscuitcode` removes everything it installed.
- [ ] Cold-launch budget: `tests/cold-launch.sh` reports under 2000ms on i5-8xxx / 8GB hardware.
- [ ] No console errors in devtools or Rust logs during a normal 5-minute session: open folder, edit file, chat, run agent tool, commit via git panel. (`journalctl --user -t biscuitcode --since '5m ago' | grep -iE 'error|panic' | wc -l` returns `0`.)
- [ ] All 11 keyboard shortcuts in the vision's table work as specified (test `shortcuts/global.spec.ts` passes; manual checklist in `docs/RELEASE.md`).
- [ ] `grep -rnE 'lorem|TODO|FIXME|placeholder|XXX' src/ src-tauri/src/` returns zero user-visible hits.
- [ ] Typography audit: `grep -rn 'system-ui' src/` returns no hits in primary chrome (named-system fallbacks like `'Ubuntu', sans-serif` are OK).
- [ ] Dark theme uses Cocoa scale exclusively: `grep -rnE '#000000|#fff\b|#ffffff' src/theme/` returns zero hits.
- [ ] Every failure path has an actionable error: every code in `docs/ERROR-CATALOGUE.md` has a passing test in `tests/error-catalogue.spec.ts`.
- [ ] First-token-latency on Claude streaming (non-reasoning models): p50 under 500ms, p95 under 1200ms, measured by `tests/ttft-bench.ts` over 20 prompts on a warm connection. Reasoning models exempt (`Thinking…` state shown).
- [ ] Provider tool calls render as Agent Activity cards within 250ms of `content_block_start` — e2e test `tests/e2e/agent-tool-card-render.spec.ts` against `tests/fixtures/canonical-tool-prompt.md`.
- [ ] `cargo audit` and `pnpm audit --prod` return zero critical vulnerabilities.
- [ ] `desktop-file-validate packaging/deb/biscuitcode.desktop` exits `0`.
- [ ] All dependencies MIT/Apache-2.0/BSD compatible — `cargo-license` + `license-checker-rseidelsohn` reports clean.
- [ ] Icon legible at 16x16 in the XFCE system tray (CI pixel check + manual visual confirm).
- [ ] `pnpm test:a11y` (axe-core) reports zero violations on canonical screens.
- [ ] **Gemma 4 default verified:** on a system with Ollama ≥ 0.20.0, the picker selects the correct Gemma 4 tier per the verified RAM table (e2b / e4b / 26b / 31b); on a system with Ollama < 0.20.0, the `E007 GemmaVersionFallback` toast fires and the Gemma 3 ladder is selected with the upgrade-Ollama install command surfaced.
- [ ] **Prompt caching verified:** Anthropic responses include `cache_read_input_tokens > 0` after the second prompt of a long-system-prompt conversation.
- [ ] **Snapshot/rewind correctness:** multi-step agent run writes 3 files; rewind restores all 3 byte-identical (`sha256sum` matches).
- [ ] **AppImage updater:** v0.9.0 → v1.0.0 update flow works end-to-end on a fresh VM.
- [ ] **`.deb` update check:** "Check for updates" button on a v0.9.0 install detects v1.0.0 release and opens the release page.
- [ ] Release smoke-test checklist in `docs/RELEASE.md` passes 100% on every fresh VM in the matrix.

---

## Open Questions

Carried forward from both rounds. None block execution; all have planner-default positions the maintainer may override.

1. **Telemetry backend.** Vision allows opt-in anonymous crashes. Wire Sentry (vendor dep), self-hosted endpoint, or ship UI toggle in v1 with no wire (current default)?
2. **AppImage `libfuse2t64` UX.** README banner only, or also an AppImage wrapper script that prompts install? Current default: both.
3. **Icon Concept D spike.** Plan ships A; CI 16x16 check decides whether to fall back to D. Should we render both upfront and pick? Default: trust A, fall back to D only if check fails.
4. **Arm64 build.** v1 = x86_64 only. Defer arm64 to v1.1? Default: defer.
5. **Debian repo (`apt.biscuitcode.io`).** GitHub releases only in v1; apt repo deferred.
6. **Secret Service auto-recovery.** Block onboarding with install prompt vs. attempt `gnome-keyring-daemon --replace` ourselves? Default: be conservative — block.
7. **LSP install auto-run.** Default: confirmed no — copy-to-clipboard only.
8. **Notebook deferred-execution scope.** v1 is read-only render with no run controls. Confirm? Default: yes.
9. **Conversation DB growth cap.** Currently bounded by manual "Clear old conversations" in Phase 8. Auto-prune at >500MB or similar? Default: defer to v1.1.
10. **Split-editor `Ctrl+\` behavior.** Wired in Phase 3 as a true multi-pane split-model. Vertical split (`Ctrl+K Ctrl+\`) — v1 or v1.1? Default: defer.
11. **Chat-mention resolution.** Substring/whole-file in v1; semantic (LSP-symbol) is v1.1.
12. **Error-path catalogue size.** ~18 entries across Phases 1–9. Subclasses parametrized, not separately enumerated.
13. **Workspace-trust granularity.** Currently per-workspace boolean. Per-tool granularity (e.g., trust read+write but not shell) is v1.1.
14. **Update-check frequency.** Currently 24h. Configurable in v1.1.
15. **Reasoning-mode timeout.** Currently no UI timeout for reasoning runs (provider may take 30s+). Add a cancel button at 60s? Default: yes — Phase 6a's executor pause flag covers this; explicit timeout button is v1.1.
16. **(Synthesis-added, RESOLVED 2026-04-18)** ~~Gemma 4 exact tag names.~~ **Resolved by direct verification against `https://ollama.com/library/gemma4`:** the actual tags are `gemma4:e2b` (2.3B effective, 7.2GB), `gemma4:e4b` (4.5B effective, 9.6GB, also `:latest`), `gemma4:26b` (MoE 25.2B/3.8B active, 18GB), `gemma4:31b` (30.7B, 20GB). Naming convention is `e<N>b` for edge variants and plain integers for full-size — different from the Gemma 3 family. The synthesis pass had extrapolated `gemma4:9b` / `gemma4:27b` which do not exist. **Plan updated.** Minimum Ollama version for Gemma 4 = `0.20.0` (released 2026-04-03 same-day as the Gemma 4 model drop). Open known issue: tool-call streaming via Ollama's OpenAI-compatible API has bugs (GitHub anomalyco/opencode#20995); we use `/api/chat` directly which is unaffected.
