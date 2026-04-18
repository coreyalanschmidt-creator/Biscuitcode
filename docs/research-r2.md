# BiscuitCode — Research Dossier, Round 2

> Complement to `docs/research-r1.md`. Round 2 challenges r1 where evidence has shifted, surfaces alternatives r1 dismissed or skipped, digs into r1's shallower sections, and fills gaps r1 missed entirely. Synthesis of r1 + r2 is the next stage; this file is not a rewrite of r1 and does not stand alone. Authored 2026-04-18.

---

## Framing

**Role in the pipeline.** Research-r2 is the second of two research dossiers that feed the planner's synthesis step. Where r1 is first-principles and comprehensive, r2 is adversarial and corrective — its job is to make the synthesis non-trivial by giving the planner real alternatives, not an echo. A claim r1 made adequately is not restated here; I note "r1 section X covered this well" and move on.

**Relationship to r1.** I accept the bulk of r1's findings. The places I push back are enumerated in "R1 Findings Challenged or Refined". I also dissent from two of r1's *Recommended Approach* picks where fresh evidence reopens the debate: Tauri `plugin-sql` (r1 picks `rusqlite` direct) and `git2-rs` (r1 picks `git2` + shell-out). Dissent is specific and bounded — r1 is more often right than not.

**Shape.** Per the round-2 brief: reinforcement table, challenge table, alternatives, deep dives, gap coverage, explicit verify-pass/fail, new risks, updated recommendation deltas, sources.

---

## R1 Findings Reinforced by New Evidence

Short table. One row = one r1 claim that holds up under r2 sourcing.

| r1 § | Claim | Additional source |
|---|---|---|
| §7 / Risks #3 | Opus 4.7 rejects `temperature`/`top_p`/`top_k` with HTTP 400. | Bedrock users confirm 400s in the wild; Anthropic migration guide says "omit these parameters entirely; use prompting to guide behavior". [1][2] |
| §7 / Risks #4 | GPT-4o retired from ChatGPT Feb 13, 2026; "after April 3, GPT-4o was fully retired across all plans". API deprecation timeline is separate but tight. | OpenAI blog, OpenAI help center. [3][4] |
| §1 | Tauri stable is v2.10.x in April 2026 (tauri 2.10.3 on docs.rs, tauri-cli 2.10.1). | crates.io/docs.rs. [5] |
| §5 | `@xterm/*` scoped packages are the canonical path; the old `xterm-addon-*` packages are deprecated and explicitly marked "move to @xterm/addon-*". | xterm.js releases page and deprecation notices. [6] |
| §7 | Ollama NDJSON streaming format for `/api/chat` confirmed; tool-calls land as a JSON object on a line, not SSE. | Ollama official blog "Streaming responses with tool calling" + community bug reports referencing NDJSON. [7][8] |
| §1 / Risks #2 | `libfuse2` is renamed to `libfuse2t64` on Ubuntu 24.04; AppImage won't launch without it installed. | AppImage troubleshooting + Launchpad bug + OMG!Ubuntu. [9][10] |
| §7 | Claude model IDs `claude-opus-4-7`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001` (alias `claude-haiku-4-5`) are the current defaults per Anthropic docs. Opus 4.7 released 2026-04-16. | Anthropic Models overview + Opus 4.7 launch post. [1][11] |
| §7 (tool streaming) | Each `input_json_delta` carries a partial JSON substring; the complete `input` object is only guaranteed at `content_block_stop`. | Anthropic streaming docs explicitly state: "The deltas are partial JSON strings, whereas the final tool_use.input is always an object." [12] |
| §15 | `biscuit`, `biscuit-auth`, `biscuit-cli` are existing authorization-token crates and must be avoided; the `biscuitcode` crate name appears unclaimed. | Reconfirmed on crates.io; no new claim since r1 authored. [13] |

No edits needed on these rows — r1 got them right. The planner can keep its Assumption #5 (keyring 3.6.x), Assumption #3 (WebKitGTK 4.1), Assumption #9 (model defaults *after* the updates r1 already applied).

---

## R1 Findings Challenged or Refined

Eight claims where r2 evidence materially revises the picture.

### C1. Mint 22.2 kernel and XFCE version — half right, half wrong

- **r1 claim (§2, Assumption #1):** "kernel 6.8 on 22.0/22.1, kernel 6.14 on 22.2" and "XFCE 4.20 available in 22.2."
- **r2 evidence:** A *fresh install* of 22.2 uses HWE kernel 6.14. An *upgrade* from 22.1 keeps 6.8 unless the user opts into HWE via the Kernel Manager. The "22.2 XFCE edition ships XFCE 4.20" claim does not match Phoronix / Linux Mint forum reporting as of late 2025 — 22.2 XFCE still ships **XFCE 4.18** because that's what Ubuntu Noble carries; XFCE 4.20 is *available* via backports but not the default. [14][15][16]
- **Practical implication:** The planner's Mint-22.2 smoke test line "Wayland-XFCE session (22.2 only, where XFCE 4.20 is available)" in Phase 10 assumes XFCE 4.20 ships in 22.2. It does not. Wayland-XFCE on Mint 22.2 is XFCE 4.18-based, and Wayland support in XFCE 4.18 is absent to near-absent. The planner should narrow that row to "Wayland smoke-test on **Cinnamon** 22.2 (Cinnamon has experimental Wayland, XFCE still does not even in 22.2)" — OR drop the Wayland test to "best effort" and accept that XFCE+Wayland is not practically reachable for a v1 release smoke test.

### C2. `plugin-sql` vs `rusqlite` — r1's "use rusqlite directly" is defensible but not obviously the simplest

- **r1 claim (§10, Recommendation #1, #10):** Skip `plugin-sql`; use `rusqlite` directly with hand-rolled `PRAGMA user_version` migrations and a `Mutex` wrapper.
- **r2 evidence:** `tauri-plugin-sql` v2.3.2 (current) already registers migrations idempotently via `Builder::add_migrations(...)` and exposes a uniform JS API; `tauri-plugin-rusqlite2` v2.2.4 (community) exposes the same migration pattern over `rusqlite`. Both are maintained. [17][18] For a single-SQLite app with no frontend-DB access need, the direct-rusqlite path saves ~200 lines of abstraction, but the plugin path saves *us* from hand-rolling the migration registry and gives a known transaction-safe shape.
- **Counter-evidence to r1's reasoning:** r1's argument is "sync API is fine because the DB is local; spawn_blocking + Mutex". That's correct for read-heavy workloads. For our case — streaming assistant messages being persisted while the executor is also writing snapshots — there's measurable write contention on the single `Mutex`. A connection pool (either sqlx via plugin-sql, or a `r2d2`/`deadpool-sqlite` wrapper) avoids it.
- **Practical implication:** r1's recommendation is still reasonable, but it is *not* a clean "simplest adequate" win. The synthesis step should treat this as a real coin-flip and decide based on whether the planner wants frontend access to the DB (favors plugin-sql) or purely backend access (favors rusqlite). The plan-r1 chose rusqlite direct; I don't reverse that call, but I want the planner to know the simpler-plumbing case for the plugin is stronger than r1 presented.

### C3. `git2-rs` + shell-out for writes — this was defensible in r1's timeframe, but `gix` has matured enough to warrant a revisit

- **r1 claim (§Best Practice #8):** Use `git2` for reads, shell out to system `git` for writes. Inherits credential helpers, LFS, signing.
- **r2 evidence:** `gix` (gitoxide) ships production-grade status and worktree-mutation features now. However, **push is still not implemented** as of early 2026, and the CLI (`gix`) is explicitly called "forever unstable." Full merge, rebase, commit hooks, and advanced write operations remain WIP. [19][20][21]
- **Practical implication:** `gix` is not ready to replace `git2` for our v1, which needs commit, push, and pull. r1's recommendation stands *for v1*. But I want the planner to record that this is a **short-term** stance. Our commit/push write paths are isolated behind a small module; swapping shell-out for `gix` in v1.1 or v1.2 is a deliberate future win. Don't architect around shell-out as if it were permanent.
- **New nuance r1 missed:** Shelling out to `git` from Tauri's shell plugin requires specific capability allow-list entries. `git commit -m`, `git push`, `git pull` all need argument patterns allowed. If we allow `git` broadly (arg `*`) we undo the security posture; if we allow only specific subcommands per the command-registry model, we need a more precise match syntax than r1 sketched. (See Deep Dive D1.)

### C4. "r1 treats ReAct loop as settled; consider single-turn + human-confirm"

- **r1 claim (§8, Recommendation #8):** ReAct loop with pause flag between iterations, per-action rewind snapshots.
- **r2 evidence:** Claude Code's own agent-loop docs describe exactly ReAct shape — "receive prompt → evaluate → execute tools → repeat" — so picking ReAct isn't wrong. [22][23] But for *v1* of BiscuitCode, a strict single-turn-with-human-confirm loop would ship faster and give the user more control, at the cost of feeling less agentic. Zed's Inline Assistant is close to this model: one instruction → one streamed edit → accept/reject. [24][25]
- **Practical implication:** ReAct is the right long-term shape, but the plan-r1 treats Phase 6 as "full ReAct + rewind + inline edit + tool registry + activity UI" and flags it High. A simpler v1 is: (a) **Chat mode** stays non-agentic — each turn is one assistant response, no tool calls at all. (b) **Inline edit** is a fixed one-shot: select → prompt → stream-diff → accept. (c) **Agent mode** is the only place tool calls happen, and it is explicitly v1.1 or a narrow "read-only tools only" subset in v1. This is a legitimate simplification — the synthesis step should not silently adopt it, but should consider it. If the planner keeps full ReAct in Phase 6, it should at least split writes to v1.1.

### C5. `monaco-languageclient` vs. CodeMirror 6's LSP extension — r1 skipped this comparison in favor of the vision's Monaco lock-in, fair, but worth noting

- **r1 claim (§4, Trade-offs: Editor):** Monaco is mandated by vision; CodeMirror is smaller but not chosen.
- **r2 evidence:** Vision locks Monaco — fair. But the *LSP transport* choice does not have to follow. CodeMirror 6's `@codemirror/lsp-client` is a ~1 MB addition that handles many of the wire-up concerns `monaco-languageclient` makes us solve via custom MessageTransports. [26][27]
- **Practical implication:** No change — Monaco is locked, so `monaco-languageclient` follows. But I flag that the planner's Phase 8 LSP work is the single biggest "custom MessageTransports + Tauri event proxy + stderr passthrough" piece of wiring; if the scope slips, falling back to a CodeMirror-6 editor in the Problems/Output pane only (Monaco still for the main editor) is a legitimate de-risk. The vision doesn't forbid CodeMirror in secondary panels.

### C6. r1's per-provider ChatEvent enum is good, but the tool-call streaming edge cases in r1 are under-specified

- **r1 claim (§7):** A flat `ChatEvent` enum flattens three provider shapes.
- **r2 evidence (unchanged shape, new specifics):**
  - **Anthropic** supports `anthropic-beta: fine-grained-tool-streaming-2025-05-14` which streams `input_json_delta` without buffering/JSON validation — dangerous but lower-latency. [12] r1 did not mention this beta header. If we adopt it, we must handle truncation mid-parameter at `max_tokens`.
  - **OpenAI** streaming events are `response.function_call_arguments.delta` (Responses API) or `choices[0].delta.tool_calls[].function.arguments` (Chat Completions API). The two are **not** wire-compatible; Responses API uses `output_index` for correlation, Chat Completions uses `tool_calls[i].index`. [28][29] r1 only documented the Chat Completions form. If we ever target Responses API (which has the reasoning-models), we'd need a *second* decoder.
  - **Ollama** tool-call streaming: the *community* consensus is that Ollama emits `tool_calls` on the final non-`done` chunk, not streamed character-by-character. For some models the tool call lands as plain text (`qwen3.5:9b sometimes prints tool call instead of executing`). [30] r1 covered this at a high level but did not name the "model prints tool-call-shaped text" failure mode as something the executor must detect.
- **Practical implication:** The planner's Phase 5 should include a unit test for the partial-JSON-at-`max_tokens` truncation case (for the fine-grained beta path, if adopted). Phase 7's OpenAI handling should assert we're using Chat Completions API and skip Responses API parsing; if we later want reasoning models, we need a second decoder. Phase 7's Ollama handling needs a "model emitted tool-call-shaped text instead of structured `tool_calls`" fallback — detect common patterns (`<tool_call>...</tool_call>`, `{"name":..., "arguments":...}` wrapped in prose) and either treat as a failure or parse heuristically.

### C7. "Secret Service absent on XFCE" — r1's mitigation is right but the detection path is more fragile than r1 admits

- **r1 claim (§6, Risks #7):** Detect `org.freedesktop.secrets` on DBus at startup, block onboarding if absent.
- **r2 evidence:** A DBus *activation* (`dbus-launch gnome-keyring-daemon --start --components=secrets`) can spin up a new keyring daemon with an empty password even when the session didn't start one. This is what Python Keyring's CI recipe does. It *unlocks a keyring nobody asked for* with a known password. Bad for production. On the detection side: simply querying `org.freedesktop.secrets` may cause DBus to try to activate it, so "detect before call" is not a side-effect-free probe. [31][32]
- **Practical implication:** Detection should use `busctl list --user | grep org.freedesktop.secrets` (read-only, no activation) rather than directly calling the service. Or we catch the activation failure specifically and surface it, instead of trying to "test if available" first. The planner's Phase 5 acceptance criterion ("On a VM without `gnome-keyring`, add-key flow shows the exact install command") should specify that detection uses `busctl list --user` and does not inadvertently start the daemon.

### C8. `xterm.js` is solid but libghostty is a real v1.1+ alternative worth flagging

- **r1 claim (§5, Trade-offs: Terminal):** xterm.js + portable-pty. Alternative is `tauri-plugin-pty`.
- **r2 evidence:** `libghostty-vt` shipped in 2026 as a C-ABI library extracted from Ghostty's VT core. "Already backing more than a dozen terminal projects" including IDEs. Supports Kitty graphics protocol. [33][34] For v1, integrating a C-ABI library is significantly heavier than xterm.js; FFI + Rust bindings + rendering on Canvas/WebGL from Rust side — not a path we should take in v1.
- **Practical implication:** No change for v1 (xterm.js stays). Flag as a v1.1 performance-and-features play if we hit xterm.js limits on rendering large output or Kitty-graphics-protocol inline images. The planner's tradeoff table should note libghostty's existence, if only to avoid re-discovering it later.

---

## Alternatives to R1 Recommendations

For each major r1 pick, the serious "path not taken" with a verdict.

### A1. Editor: Monaco vs. CodeMirror 6

- r1 picks Monaco (vision-locked).
- Alternative: CodeMirror 6. 300 KB core vs ~5 MB gzipped Monaco, native modular LSP, mobile-friendly, no workers. [26][35]
- When it would win: if bundle size were the top constraint and VS Code parity were explicitly optional. Sourcegraph's migration from Monaco to CodeMirror is the strongest industry signal here — they reduced their bundle by multiple MB and gained control over embedding. [36]
- Why r1 is still right: vision mandates Monaco *by name* ("Monaco Editor via `@monaco-editor/react`"). Parity with VS Code extensions and settings is a stated goal. CodeMirror would cost us "VS Code parity" bragging rights and invalidate the "feels like VS Code" first-impression test. Keep Monaco; note CodeMirror only if Phase 3 cold-launch budget blows up badly.

### A2. Frontend framework: React 18 + Zustand vs. SolidJS vs. Preact signals

- r1 picks React 18 + Zustand (vision-implied).
- Alternative: SolidJS. ~6x smaller bundles, fine-grained reactivity, fewer re-render traps when rendering streaming chat. [37][38] Preact with signals is the middle ground — React-compatible API, small runtime.
- When it would win: if v1 bundle size were budget-critical. Our chat panel's per-token re-renders are a known React perf pothole.
- Why r1 is still right: vision says React 18 + TypeScript. Monaco has a first-class React wrapper (`@monaco-editor/react`); the Solid wrapper is community-maintained. `react-resizable-panels`, `react-markdown`, `react-virtuoso` all assume React. Switching costs outweigh benefit at our scale. Keep React. *But:* the chat panel and Agent Activity panel need virtualization (see Deep Dive D3) to not suffer; without it, React's re-render cost on streaming-updates is the real perf hit, not bundle size.

### A3. SQLite: rusqlite direct vs. plugin-sql vs. plugin-rusqlite2

- r1 picks rusqlite direct.
- Alternative: `plugin-sql` (official sqlx-based) OR `tauri-plugin-rusqlite2` (community rusqlite-based with plugin niceties).
- When `plugin-sql` would win: if we ever want frontend JS to hit the DB directly for prototyping. We don't.
- When `plugin-rusqlite2` would win: if we want plugin-style migrations without sqlx compile cost. Mild win.
- Why r1 is plausibly still right: dependency minimalism. One less plugin = one less attack surface + one less version-skew problem. Verdict: r1 is defensible, but the case is closer than r1 presented (see Challenge C2). Worth a one-line mention in the plan's ADR that we evaluated and picked direct.

### A4. Agent loop: full ReAct vs. single-turn inline edit only (v1), ReAct in v1.1

- r1 picks full ReAct in Phase 6 (high complexity).
- Alternative: Ship **inline edit** (select → prompt → diff) and **non-agentic chat** in v1. Put tool-calling and ReAct in v1.1. User can ask Claude anything; assistant can't run tools. Inline-edit is the "AI touches code" surface.
- When alternative would win: if Phase 6 risk actually manifests and blows the calendar. The vision explicitly calls out agent mode as a feature, so this is a real cut.
- Why r1 is probably still right: vision places "Agent mode toggle" in core features, and the Agent Activity panel is a distinct UI region in the vision's ASCII layout. Cutting ReAct would leave a visible placeholder. Keep ReAct in v1 — but narrow: **read tools only** (`read_file`, `search_code`) in v1; `write_file`, `run_shell`, `apply_patch` guarded behind explicit per-tool toggles. This reduces Phase 6 risk by shrinking the tool registry and sidestepping the confirmation-UX work for writes.

### A5. Git: `git2` + shell-out vs. pure `gix` vs. pure shell-out

- r1 picks hybrid.
- Alternative: all shell-out. Zero Rust git dependency; slower status/diff for large repos (shell-out per-file is expensive).
- Alternative: pure `gix` — not viable for v1 (no push).
- Verdict: r1 is right. Pure shell-out is legitimately slower for file-tree status colouring on big repos; `gix` can't do push. Keep the hybrid. Add a line in the plan saying `gix` is the v1.1 swap target.

### A6. Terminal: xterm.js vs. libghostty-vt

- r1 picks xterm.js (unanimous with every web-embedded terminal).
- Alternative: libghostty-vt for VT parsing + custom renderer in Canvas from Rust side.
- When it would win: if we care about Kitty graphics protocol (inline images from terminal output) — genuinely useful for data-science workflows in v1.1+. Not v1. Keep xterm.js.

### A7. Secrets: `keyring` crate vs. Tauri `plugin-stronghold`

- r1 picks `keyring` crate.
- Alternative: `plugin-stronghold` — IOTA Stronghold-based encrypted store, not Secret Service.
- **Key finding:** `plugin-stronghold` is officially **not recommended** and will be removed in Tauri v3. [39]
- Verdict: r1 is right. Stronghold is a dead end. Do not even list it as an option in the plan.

### A8. Updater: Tauri updater plugin vs. apt repo vs. AppImage self-update

- r1 did not address this (gap — see G3 below).
- Alternative 1: Tauri updater plugin — works with AppImage (downloads .tar.gz patch) and does not work with .deb (which is the *primary* artifact). [40]
- Alternative 2: host an apt repo (`apt.biscuitcode.io`) so `apt upgrade` works — ongoing cost + setup but native-feeling on Mint. Plan-r1 Open Question #5 defers this; I agree it should stay deferred.
- Alternative 3: do nothing; README says "download new .deb and reinstall". Cheapest, worst UX.
- Verdict: For v1, accept the "download new .deb" flow, add a "new version available" toast that fires when the app checks a GitHub releases JSON at startup. For AppImage users, enable the Tauri updater plugin (it works for AppImage only). This is a small addition to Phase 9 or 10 — see G3.

---

## R1 Shallow Coverage — Deep Dives

### D1. Tauri v2 capability authoring for a real-world app

r1 §13 sketched the shape. Planner needs concrete detail.

**Capability file location and schema.** Capabilities live in `src-tauri/capabilities/*.json` (or `*.toml`), auto-loaded by the builder. Each file has:

```jsonc
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-fs",            // unique per capability
  "description": "File access for workspace + config dirs",
  "windows": ["main"],                // glob-matches window labels
  "platforms": ["linux"],             // optional
  "permissions": [
    "fs:default",                     // inherits plugin defaults
    { "identifier": "fs:allow-read-text-file",
      "allow": [{ "path": "$APPCONFIG/**" },
                { "path": "$APPDATA/**" }] },
    // ...
  ]
}
```

**Path variables available** (per Tauri v2 schema): `$APPCONFIG`, `$APPDATA`, `$APPLOCALDATA`, `$APPCACHE`, `$APPLOG`, `$AUDIO`, `$CACHE`, `$CONFIG`, `$DATA`, `$LOCALDATA`, `$DESKTOP`, `$DOCUMENT`, `$DOWNLOAD`, `$EXE`, `$FONT`, `$HOME`, `$PICTURE`, `$PUBLIC`, `$RUNTIME`, `$TEMPLATE`, `$VIDEO`, `$RESOURCE`, `$APP`, `$LOG`, `$TEMP`. [41][42]

**"Workspace root + dotfolders" pattern.** The workspace root is user-chosen at runtime. Static JSON scopes can't cover this — they need to be patched live. Tauri exposes `tauri::scope::fs::FsScope::allow_directory(path, recursive)` for runtime patching. The pattern:

1. Static capability grants fs access to `$APPCONFIG`, `$APPDATA`, `$APPCACHE` only.
2. On `fs_open_folder()` command: validate the path, then add it to the fs scope at runtime with `allow_directory(workspace_root, true)`.
3. On `fs_close_workspace()`: `forbid_directory(old_root, true)` to revoke.

**Race to watch out for.** Between `allow_directory` completing and the IPC command returning to the frontend, another IPC call could fail a capability check against the old scope. Guard with an `Arc<RwLock<WorkspacePath>>` so commands read a coherent workspace state; any command that performs fs work reads the RwLock once at entry.

**Capability changes across app versions.** Capability files are bundled into the app — they're static at build time. If v1.0 → v1.1 adds a permission (e.g., `fs:allow-read-binary-file`), users who upgrade simply get the new capability. No migration. If v1.1 *removes* a permission, any previously-opened workspace path is still in runtime scope but gets denied on next call. User-facing: they see a "permission denied" error, not a crash. Mitigation: add a "permissions changed, please reopen folder" toast on version upgrade.

**Deny-by-default with precedence.** In Tauri v2, *deny rules take precedence over allow rules*. The pattern for the plan's "workspace-only" posture is:

```jsonc
"permissions": [
  { "identifier": "fs:allow-read-text-file",
    "allow": [{ "path": "**" }],  // broad allow
    "deny":  [{ "path": "/etc/**" }, { "path": "/root/**" }, { "path": "/proc/**" }, { "path": "/sys/**" }]
  }
]
```
The above is *not* what we want — we want affirmative allows only. But if a "workspace catch-all with blacklist" approach is ever tempting for ergonomics, know that deny wins.

### D2. Monaco bundle size + code splitting specifics

r1 §4 quoted "15 MB unminified, 5 MB gzipped." That's roughly right for full Monaco, but the planner needs numbers per-feature.

**Measured sizes** (from community reports, 2024–2026 consensus):

- `monaco-editor` core (no languages): ~2.5 MB uncompressed, ~700 KB gzipped.
- Per-language contribution: TypeScript worker alone is ~1.5 MB uncompressed. JSON / CSS / HTML workers: ~500 KB each. `typescript.ts` language service worker is the single biggest chunk. [43][44]
- `@monaco-editor/react` wrapper: ~15 KB gzipped — negligible.
- `monaco-languageclient` + `@typefox/monaco-editor-react`: adds ~10 MB uncompressed ("huge bump in bundle size"). [45]

**Practical code-split strategy:**

1. Defer Monaco to dynamic import after first paint OR first file-open. r1 covered this.
2. Use `vite-plugin-monaco-editor` with an *explicit* `languageWorkers: []` (no default languages). Add languages at runtime on-demand via `monaco.languages.register({ id: 'rust' })` and lazy import of the contribution.
3. **Do not** load the TypeScript worker until a `.ts`/`.tsx`/`.js`/`.jsx` file opens. That alone is 30-40% of the editor's cold bundle.
4. If `monaco-languageclient` is on the page, Monaco's default TS worker fights rust-analyzer/typescript-language-server for diagnostics. Disable the default TS worker when LSP is active: `monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({ noSemanticValidation: true, noSyntaxValidation: true })`.

**Tauri asset server interaction.** Under `tauri://localhost` (prod), Vite's output `dist/` is served via `Tauri`'s custom protocol. Web workers must have same-origin headers. `vite-plugin-monaco-editor` emits workers to `dist/monacoeditorwork/` which Tauri serves fine. No CSP issue unless the app's `tauri.conf.json` `app.security.csp` is tightened — which r1 did not mention and the planner should leave at the default (which allows worker scripts).

### D3. Claude streaming + tool-use edge cases

r1 §7 captured the happy path. Here's what it didn't flag.

**Thinking blocks.** Claude Sonnet 4.6 and Haiku 4.5 support `extended thinking` (opt-in via `thinking: { type: 'enabled', budget_tokens: N }`). Opus 4.7 does **not** expose a thinking toggle — "adaptive thinking" is always on. [1] Thinking blocks stream as `content_block_start {type: "thinking"}` → `thinking_delta` → `signature_delta` → `content_block_stop`. Your UI should render thinking separately from final answer text (some apps show it in a collapsible "thought process" section; Claude Code shows it inline in italic).

**Partial-tool-use JSON.** `input_json_delta` deltas can split JSON keys or values anywhere — not aligned to tokens. Accumulate as a string; parse only at `content_block_stop`. For a *live* tool-card UI, you can render the partial args string as raw text (un-pretty-printed) and swap to pretty-printed JSON on complete.

**Stop reason sequences.** The terminal event is `message_stop`. Before it, `message_delta` carries the final `stop_reason` and `stop_sequence`. Possible `stop_reason` values: `end_turn`, `max_tokens`, `stop_sequence`, `tool_use`, `pause_turn`. When `stop_reason === "tool_use"`, the stream contained at least one `tool_use` content block that the caller must execute and feed back. [12]

**Prompt caching with tool use.** As of early 2026, Anthropic workspace-level cache isolation landed (Feb 5, 2026). Thinking blocks get cached as part of the content, but toggling thinking mode invalidates cache for message history. Cache cost: 90% off cached-read input tokens; cache-write is 125% of input. For BiscuitCode's long-conversation use case, setting `cache_control: {type: "ephemeral"}` on the system prompt and on older user messages is a ~5x cost savings over a long conversation. r1 did not mention caching at all. [46][47]

**Fine-grained tool streaming (beta).** `anthropic-beta: fine-grained-tool-streaming-2025-05-14` reduces latency by skipping JSON validation during streaming. Risk: stream ending mid-value at `max_tokens`. Only adopt if latency is a measured problem and the UI can handle truncation.

### D4. OpenAI tool-use streaming quirks

r1 §7 covered Chat Completions. The planner should know:

**Chat Completions API vs. Responses API.** These are two different endpoints with two different wire shapes.

- **Chat Completions (`/v1/chat/completions`):** `choices[0].delta.tool_calls[]` with `index` field for correlation across chunks. Arguments are a string that concatenates across deltas.
- **Responses API (`/v1/responses`, newer, covers reasoning models):** events named `response.function_call_arguments.delta` with `output_index` field. [28][29]

**Reasoning models.** GPT-5.4-pro and the -pro line expose `reasoning.effort`: `{none, low, medium, high, xhigh}` (xhigh new in 2026-04 alongside Opus 4.7). Reasoning traces are **not exposed in the API response** (unlike Claude's thinking blocks) — you get final output only. Reasoning happens before the first streamed token, which is why TTFT for reasoning models can be 3-30s vs sub-second for non-reasoning models. Planner's Phase 5 TTFT budget ("p50 under 500ms on Claude") does not apply to reasoning models — the UI needs a "Thinking..." state for reasoning runs.

**Tool args concatenation pattern (Chat Completions):**
```js
const acc = {};  // index -> { id, name, args }
for (const chunk of stream) {
  for (const tc of chunk.choices[0].delta.tool_calls ?? []) {
    if (!acc[tc.index]) acc[tc.index] = { id: tc.id, name: tc.function?.name, args: '' };
    if (tc.function?.arguments) acc[tc.index].args += tc.function.arguments;
  }
  if (chunk.choices[0].finish_reason === 'tool_calls') {
    // emit ToolCallStart/Delta/End events for each acc[i]
  }
}
```

### D5. Ollama tool-use model-by-model

r1 §7 gave a table. Here's the nuance.

**Native tool-call emission (structured `message.tool_calls`):**
- `qwen2.5-coder:7b` — reliable. Ollama docs confirm. [48]
- `qwen2.5-coder:14b`, `:32b` — reliable.
- `llama3.1:8b` — reliable but slower to emit.
- `mistral-nemo:12b` — reliable.
- `phi-4` — reliable.
- `gemma3:*` (base) — **does not reliably emit structured tool calls**; Google's release notes claim support but Ollama's integration is flaky. Community variants (`orieg/gemma3-tools`, `PetrosStav/gemma3-tools`, fine-tuned on Hermes function-calling dataset) emit `<tool_call>...</tool_call>` XML tags that the app must parse. [49][50]
- `gemma4:*` — native tool calling (per the April 2026 release). [51]
- `gemma4:e2b` (2B params, multimodal) — inherits native tool calling from the larger model family.

**Failure modes to handle:**
1. Model emits `tool_call` as *text content* inside `message.content` instead of `message.tool_calls` — common on gemma3 base. Executor must regex-scan content for `<tool_call>...</tool_call>` or `{"name":...,"arguments":...}` blobs.
2. Model emits a tool call with **invalid JSON** in arguments — common on smaller models. Executor should catch JSON-parse errors and surface them as a recoverable error (loop back to model with a "your last tool call had invalid JSON, please retry" user message).
3. Model emits a tool call for a tool **not in the provided `tools` list** (hallucinated tool). Reject with a specific error to the model.
4. Tool call streaming timing: tool_calls typically arrive on the **last non-done chunk**, not streamed character by character. The executor cannot show a "streaming" tool card for Ollama — it renders all at once.

**Recommendation:** Default to `qwen2.5-coder:7b` for tool-use-heavy workflows instead of `gemma3:4b` on a system with 12GB+ RAM. `gemma3:4b` is a fine *chat* default but the wrong default for agent mode.

### D6. Rust `keyring` 3.6.x fallback behavior in detail

r1 §6 said "Secret Service absent → block." r2 deepens this.

**What happens under the hood.** The `keyring` 3.6 crate with features `linux-native-async-persistent + async-secret-service + crypto-rust + tokio` uses:
1. First attempt: `dbus-secret-service` to talk to `org.freedesktop.secrets` over the session DBus.
2. Fallback: Linux `keyutils` (in-memory kernel session keyring — lost at logout).
3. In-memory only: `KeyringImpl::Memory` — test/headless fallback, but note this is *only* if both above fail *and* you explicitly configure memory-only.

**Error messages (specific strings):**
- "The name org.freedesktop.secrets was not provided by any .service files" — Secret Service binary not installed.
- "Failed to unlock collection" — Secret Service running but user's keyring is locked (password prompt required).
- "Connection refused" — DBus session bus not available (e.g., `startxfce4` launched via `xinit` without PAM).

**Pre-call detection (no side effects).** Do NOT call into `keyring` to probe — any call will attempt the DBus chain. Instead:

```rust
// Rust: read-only check that doesn't activate the service.
fn secret_service_available() -> bool {
    std::process::Command::new("busctl")
        .args(["list", "--user", "--no-pager"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("org.freedesktop.secrets"))
        .unwrap_or(false)
}
```

This avoids the Python-Keyring-CI antipattern of "start gnome-keyring with a known password" (which silently weakens security). If detection returns false, onboarding blocks with an install prompt and a link to the error-recovery doc.

### D7. Inline edit (Ctrl+K Ctrl+I) UX — primary-source patterns

r1 §8 bundled this with "agent loop" without detail. What the three prior-art products actually do:

**Cursor Composer (inline edit):**
- Select code → Ctrl+K → small popover input at cursor → stream the diff into the editor in-place, line-by-line.
- Decorations: red strikethrough for removed lines, green highlight for added lines, inline accept/reject per-hunk buttons in the gutter.
- No modal dialog. UI commits when all hunks accepted, or reverts on Esc.

**Zed Inline Assistant:**
- Ctrl+Enter on selection → side-panel input (not popover) → streams edit into the same file but in a *pending* buffer shown as a split diff inline.
- Explicit split-diff view (the editor doubles vertically, showing before/after side by side in the same pane for the selection). [52][53]
- Accept via Cmd+Enter (Mac) or Ctrl+Enter.

**Continue.dev VerticalDiffManager:**
- Inline diff decorations directly in VS Code editor.
- Streaming LLM output rendered incrementally as the model produces each line.
- Accept/reject per-file; no per-hunk granularity in v1, added later. [54]

**What to pick for BiscuitCode v1:**
- Cursor-style in-place streaming with per-hunk accept/reject is the most polished UX but is the hardest to build — requires precise Monaco decoration management and a hunk-level state machine.
- Zed-style split-diff is simpler because Monaco has `createDiffEditor` built-in. Stream into a right-side pending buffer, user accepts whole diff at once (per-hunk is a v1.1 feature).
- **Recommendation for plan:** ship Zed-style split-diff in v1. Simpler, uses Monaco-native primitives, still feels professional. Cursor-style in-place is a v1.1 polish.

### D8. Agent Activity UI — streaming tool-call cards smoothness

r1 did not specify rendering framework for the tool-call cards. The plan's Phase 6 acceptance criterion is `tool_card_visible_<id> - tool_call_start_<id> < 250ms`, which is load-bearing.

**Problem shape.** Tool cards grow live (streaming args, streaming result), collapse/expand, and a conversation can accrue 50+ cards. Naive React re-renders the whole list on every token.

**Options:**
1. **Plain React with per-card memoization** (`React.memo` + `useReducer` per card). Works up to ~20-30 cards. Above that, re-render cost shows up as jank.
2. **`react-virtuoso`** — explicitly designed for chat-style streaming lists. Has a `VirtuosoMessageList` component built for human/AI conversations. Dynamic item heights handled automatically. [55][56]
3. **`react-window`** — fixed-height rows only. Poor fit (tool cards have variable, growing heights).
4. **Plain div + `requestAnimationFrame` batching** — roll your own. Worth it only if a profiled scenario requires it.

**Recommendation:** Use `react-virtuoso`'s `VirtuosoMessageList` for the Agent Activity panel and also for the Chat Panel. It handles streaming-updates, scroll-position preservation, and dynamic height. Same library → two panels → shared mental model. Adding it to the plan is a Phase 6 deliverable; lightweight enough (~90 KB gzipped) to not dent bundle size.

---

## Gaps R1 Missed

Items the brief called out, covered to the depth the planner needs.

### G1. i18n / l10n strategy for v1

**r1 status:** Silent.

**What v1 needs:** English-only UI is fine for v1; the question is whether to *architect* for i18n. Two options:

1. **Hardcode strings.** Simplest. Adds technical debt if we ever go multi-language.
2. **Wrap every user-facing string in `t('key')` from day 1**, with a single English bundle. Zero runtime cost, buys the ability to add locales later without sweeping every file.

**Recommendation:** Option 2 with `react-i18next`. [57] One JSON bundle (`src/locales/en.json`), a single `useTranslation()` hook, all chrome text wrapped. Extra cost: ~1 hour in Phase 2 to set up, 5 min per component after. Saves weeks of find-and-replace work if we ever localize. Zero bundle size cost (English-only). No impact on Tauri or build. Settings page has a placeholder "Language" dropdown with only English enabled in v1.

**Impact on plan:** Add to Phase 2 as a small deliverable.

### G2. Accessibility (a11y) posture

**r1 status:** Mentions High Contrast theme; no broader a11y story.

**What v1 needs:**

1. **Keyboard navigation parity.** Every interactive panel must be reachable via Tab. `Ctrl+F6` or `F6` to cycle focus between four regions (vision shortcut table doesn't include this — worth adding). Chat input, code editor, file tree, terminal all must support full keyboard operation.
2. **ARIA labels on every icon-only button.** Activity Bar icons, chat panel send, inline-edit popover buttons, tool card expand/collapse.
3. **Screen reader announcements** for streaming text. `aria-live="polite"` on the chat messages container so VoiceOver/Orca can read streamed tokens. For tool call results, `aria-live="assertive"` on completions.
4. **High contrast theme** is listed in vision; r1 covers. No change.
5. **Keyboard-only workflow test** must pass as part of Phase 10 release smoke-test.
6. **Focus indicators** — 2px outline on every focused element. Tailwind's `focus-visible:` utilities + a `--focus-ring: var(--biscuit-500)` token.

**Limitations of scope:**
- Full WCAG 2.1 AA compliance is out of scope for v1; the goal is "reasonable posture, no obvious fails."
- Monaco has its own a11y model; don't fight it.

**Impact on plan:** Add a11y smoke-test to Phase 9 (error-path + a11y catalogue) and to Global Acceptance Criteria.

### G3. Auto-update strategy

**r1 status:** Silent. Planner Open Question #5 mentions Debian repo as "defer."

**Options:**

1. **Tauri updater plugin.** Works with AppImage; does **not** work with .deb (Tauri docs explicitly state). [40]
2. **Apt repo hosted at `apt.biscuitcode.io`.** Natural on Mint; `apt upgrade` just works. Ongoing hosting + GPG sign per-release. Setup cost: 1-2 days. Runtime cost: ~$5/month storage.
3. **GitHub Releases JSON check at startup.** App fetches `https://api.github.com/repos/*/releases/latest`, shows a toast if newer. Zero infra. Manual download+install by user.
4. **Nothing.** README only.

**Recommendation for v1:** Ship (3) for .deb users and (1) for AppImage users. (2) is a v1.1 target if adoption warrants.

- AppImage: Tauri updater plugin configured to pull from GitHub Releases signed `.tar.gz`.
- .deb: "Check for updates" button in About, backed by a GitHub Releases API call. On newer version, show download link + instructions. Do not auto-download or auto-install the .deb (requires sudo; out of app scope).

**Impact on plan:** Add to Phase 9 (Settings) or Phase 10 (Packaging). Estimated half-day.

### G4. First-run crash / bad-install recovery

**r1 status:** Silent.

**What can go wrong:** Onboarding partially completes (e.g., key saved but folder not selected), then app crashes. On next launch: the "set-up-at-least-one-provider" state machine may skip onboarding and land on a broken main UI.

**Design:**
1. **Idempotent onboarding.** Each of the 3 steps checks its own completion state (`provider_configured`, `workspace_configured`) on entry. If any step incomplete, show onboarding from that step.
2. **State source.** Onboarding progress stored in `~/.config/biscuitcode/settings.json` under `onboarding: { version: 1, completed_steps: ["welcome", "provider"] }`. If settings.json is corrupt, treat as "no onboarding done" and reshow.
3. **Reset onboarding.** Settings → About → "Re-run onboarding" button for recovery.
4. **Corrupt DB recovery.** SQLite corruption = rename the file to `conversations.db.corrupt.<timestamp>`, recreate fresh. Surface to user: "Previous conversation history was corrupted; starting fresh. Old file preserved at [path]."

**Impact on plan:** Add to Phase 9 deliverables. 0.5 day.

### G5. Telemetry implementation (even off-by-default)

**r1 status:** Covered at posture level (§13), not at schema/transport level. Planner Open Question #1 defers.

**Minimum viable schema (opt-in, anonymous, no content):**
```jsonc
{
  "event": "app_start" | "crash" | "error" | "feature_use",
  "app_version": "1.0.0",
  "os": "linux",
  "os_dist": "linuxmint",
  "os_version": "22.1",
  "session_id": "uuid-v4 per session",
  "install_id": "uuid-v4 per install, generated on first run",
  "timestamp_utc": "2026-04-18T12:34:56Z",
  "payload": { /* event-specific, NEVER prompt or file content */ }
}
```

**Transport:** HTTPS POST to a single endpoint. For v1, self-host a minimal endpoint or defer the wire and just store to a local file (user can inspect what *would* be sent).

**Opt-in UX:** Settings → Privacy → "Send anonymous crash reports" toggle, off by default. On flip: show the exact schema in a dialog, confirm. Store opt-in status in keyring (not settings.json) so reinstall preserves it.

**Data retention:** If disabled, spool events to `~/.cache/biscuitcode/telemetry/spool.jsonl` (capped at 1 MB, rotating) so the user can inspect what the app *would* send. Never upload.

**Impact on plan:** Shippable stub in Phase 9. Full wire is v1.1.

### G6. Error taxonomy

**r1 status:** "Every failure path has a specific, actionable error" is a principle; no hierarchy enumerated.

**Error class hierarchy (concrete, for plan's Phase 9 catalogue):**

| Code | Class | Cause | User-facing action |
|---|---|---|---|
| `E001` | `NetworkUnreachable` | DNS or TCP fail to api.anthropic.com et al. | "Check your internet connection." |
| `E002` | `AuthInvalid` | 401/403 from provider | "API key rejected. Re-enter in Settings → Models." |
| `E003` | `ProviderDown` | 500-599 from provider | "Provider is having issues. Try again in a few minutes." |
| `E004` | `ProviderRateLimit` | 429 from provider | "Rate limit hit. Retrying in N seconds." (auto-retry w/ backoff) |
| `E005` | `KeyringMissing` | DBus org.freedesktop.secrets unavailable | "Install gnome-keyring: `sudo apt install gnome-keyring`" |
| `E006` | `KeyringLocked` | Secret service running, collection locked | "Unlock your keyring and retry." |
| `E007` | `FsPermissionDenied` | Linux permission on workspace file | "Permission denied on [path]. Check file permissions." |
| `E008` | `FsOutsideWorkspace` | Tool tried to access outside workspace root | "Path outside workspace rejected: [path]." |
| `E009` | `LspMissing` | Language server binary not found | Copy-to-clipboard install command per language. |
| `E010` | `LspCrashed` | Language server exited | "Language server crashed. [Restart] button." |
| `E011` | `OllamaDown` | localhost:11434 unreachable | "Ollama not running. Start with `ollama serve`." |
| `E012` | `OllamaModelMissing` | Tag not locally pulled | "Model [x] not pulled. [Pull] button." |
| `E013` | `OllamaPullFailed` | `ollama pull` failed | Surface stderr line + retry. |
| `E014` | `ShellForbidden` | Shell command not in allowlist | "Command [x] not allowed." (silent in trust-mode) |
| `E015` | `ToolArgsInvalid` | Tool call produced invalid JSON args | Re-prompt model with "arguments invalid, retry." |
| `E016` | `GitAuthRequired` | push/pull needs credentials | "Git requires credentials. [Open credential helper]." |
| `E017` | `DbCorrupt` | SQLite integrity check fails | Auto-recover; preserve old file. |
| `E018` | `CapabilityDenied` | Tauri ACL rejected a command | "Internal: capability missing [x]. File a bug." |

Each becomes a typed error in Rust (`#[derive(thiserror::Error)]` or similar), plus a toast component in `src/errors/ErrorToast.tsx` keyed by code. plan-r1 already calls for a catalogue in Phase 9; this fleshes it out.

### G7. Backup / export of conversation DB

**r1 status:** Silent.

**Need:** User-facing "Export all conversations" and "Import conversations" in Settings → Data. Formats:
- **JSON export:** one file per workspace, JSON array of `{conversation, messages[]}`. Simple, user-readable.
- **SQLite dump:** `~/.local/share/biscuitcode/conversations.db` is directly copyable; Settings → Data → "Open data folder" button reveals in file manager.
- **Per-conversation export:** right-click a conversation in Chats → Export as Markdown.

**Impact on plan:** Phase 9, 0.5 day.

### G8. Multi-window support ("File → New Window")

**r1 status:** Silent.

**Vision status:** Not mentioned. But this is table stakes for a VS Code-class IDE.

**Options:**
1. **Single window only for v1.** Simplest. User opens a second folder by closing and reopening. Most desktop apps do this (e.g., Slack, Figma). Acceptable.
2. **Multi-window.** `WebviewWindow::new(&app, "workspace-2", ...)` in Rust. Each window has its own workspace, conversation, panel state. Shared settings and keyring. Non-trivial state management. [58]
3. **Multi-tab (VS Code-like).** One window, tabbed workspaces. Also non-trivial.

**Recommendation for v1:** Ship single-window. Document "to open a second folder, close and reopen." Multi-window is a v1.1 feature (explicit in plan's scope). Avoids state-sync pitfalls during the first release.

**Impact on plan:** No change, but record in ADRs that single-window is a deliberate v1 choice.

### G9. Font loading fallback

**r1 status:** Covered typography choice, not failure mode.

**Need:** Self-hosted Inter and JetBrains Mono woff2 files. If `src-tauri/fonts/Inter-Regular.woff2` fails to load (file not bundled, corrupt download, etc.), what happens?

**Reality on Linux Mint 22 XFCE default fonts:**
- `Ubuntu` font is default system sans.
- `Ubuntu Mono` is default mono.
- No Inter, no JetBrains Mono.

**Fallback strategy:**
1. Primary chrome: `@font-face { font-family: 'Inter'; src: url('/fonts/Inter-Regular.woff2') format('woff2'); }` with CSS: `font-family: 'Inter', -apple-system, 'Ubuntu', sans-serif;` — the vision bans `system-ui` in primary chrome, but allows *named* system fonts as fallback. `'Ubuntu'` is present on Mint 22 by default; `-apple-system` is for future macOS.
2. Monospace: `font-family: 'JetBrains Mono', 'Ubuntu Mono', 'DejaVu Sans Mono', monospace;`.
3. **Boot-time font-load check:** in a one-time startup effect, read `document.fonts.ready` and verify `document.fonts.check('14px Inter')` returns true. If false, emit a telemetry event (`font_load_failed`) and log a visible "Font not loaded; using fallback" debug line. Production users will likely never see this, but it's a canary.
4. **Packaging:** `.deb` postinst does NOT try to install system fonts (our woff2s are bundled in `/opt/biscuitcode/fonts/`). No OS-level font install.

**Impact on plan:** Small addition to Phase 1 (font-face rules have proper fallbacks) and Phase 9 (font-load canary).

---

## Verified R1 Claims (Primary-Source Pass/Fail)

| r1 claim | Verified? | Source |
|---|---|---|
| Tauri stable is v2.10.x in April 2026 | **PASS** (2.10.3 on docs.rs) | [5] |
| GPT-4o retired April 3, 2026 | **PASS** (ChatGPT retired Feb 13; full retirement across plans April 3) | [3][4] |
| Claude model IDs `claude-opus-4-7`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001` current | **PASS**; Opus 4.7 launched 2026-04-16 | [1][11] |
| Opus 4.7 rejects non-default temperature/top_p/top_k → 400 | **PASS** (Bedrock reports, Cursor forums, migration guide) | [1][2] |
| Gemma 3 supports tool calling natively in Ollama | **FAIL** — Google's model claims support, but Ollama integration is flaky; base Gemma 3 does not reliably emit structured `tool_calls`. Community variants work via XML tag emission. | [49][50] |
| Gemma 4 has native tool calling | **PASS** (April 2026 release, Ollama library) | [51] |
| Mint 22.2 ships kernel 6.14 by default | **PARTIAL** — fresh install yes (HWE); upgrades from 22.1 do not auto-update kernel | [14][15] |
| Mint 22.2 ships XFCE 4.20 | **FAIL** — 22.2 XFCE edition still on XFCE 4.18 (same as Ubuntu Noble base). 4.20 is available via backports, not default. | [14][15][16] |
| `libfuse2t64` on Ubuntu 24.04 needed for AppImage | **PASS** (confirmed by AppImage docs, OMG!Ubuntu, itsFOSS) | [9][10] |
| `@xterm/xterm` 5.x is the scoped-package path; old `xterm-addon-*` deprecated | **PASS** | [6] |

**Net:** One outright fail (XFCE 4.20), one partial (kernel 6.14 on upgrade path), one flagged (Gemma 3 tool calling is more nuanced than r1 conveys). The rest of the specific claims verified.

---

## New Risks and Unknowns

Items surfaced during r2 that did not appear in r1's Risks section.

1. **Anthropic prompt-caching cost implications.** A long-running conversation with no cache_control hints will cost 5-10x more than one with a well-placed ephemeral cache breakpoint on the system prompt. r1 did not cover caching. The plan should include `cache_control: {type: "ephemeral"}` on system prompts and anchor tool definitions at the Phase 5 provider impl. [46][47]
2. **Reasoning-model TTFT blows the "under 500ms first token" target by 10-30x.** GPT-5.4-pro and other reasoning-mode models emit zero content until reasoning completes. Phase 5 TTFT criterion must exclude reasoning models explicitly, and the UI needs a "Thinking..." spinner for reasoning runs.
3. **Capability file skew across app versions.** Users upgrading from v1.0 → v1.1 whose workspace was open at upgrade time may encounter capability-denied errors on previously-working commands if v1.1 changes capabilities. Need an "upgrade notice: reopen folder" prompt on version mismatch.
4. **XFCE Wayland session unavailability on Mint 22.2.** The planner's Wayland smoke-test row is not achievable on XFCE 22.2. Either drop the test or scope it to Cinnamon 22.2 (not XFCE at all). Implications for Phase 10.
5. **`tauri-plugin-stronghold` is being deprecated** in Tauri v3. [39] If any forum answer or tutorial suggests Stronghold for secrets, ignore it. `keyring` crate is the only forward-compatible path.
6. **Ollama model-output tool-call XML parsing is fragile.** Some Gemma 3 variants emit `<tool_call>...</tool_call>` wrapped in prose; one bad parse = stuck loop. Robust error recovery + a "your tool call was malformed" feedback message needed.
7. **`react-virtuoso` licensing.** MIT. Safe. No concern. (But verify in plan's license-checker step — part of r1's existing gate.)
8. **Sentry for Tauri has no official SDK.** Community plugin `sentry-tauri` works but is not officially supported by either Tauri or Sentry. [59][60] If we go Sentry for telemetry, bake in plan's expectation that the plugin could break on Tauri v2.11 / v3 upgrade.
9. **Monaco TS worker conflicts with LSP.** If rust-analyzer and typescript-language-server are active via `monaco-languageclient`, Monaco's default TS worker fights for diagnostics authority. Silence the built-in via `setDiagnosticsOptions({ noSemanticValidation: true, noSyntaxValidation: true })` when LSP is connected.
10. **First-token latency target on Ollama.** r1 §14 budget of "under 500ms first token" is Claude-specific. Ollama on gemma3:4b runs at tens of tokens/sec on i5-8xxx — first token in ~1-2 seconds on warm model, 5-15 seconds on cold-loaded model. Phase 7 acceptance should say "under 3s warm" for Ollama, with a cold-load-time disclaimer.

---

## Updated Recommendations (Delta from R1)

Bounded to changes the plan should consider adopting.

1. **Revise Phase 6 scope:** ship ReAct loop with **read-only tools in v1** (`read_file`, `search_code`). Defer `write_file`, `run_shell`, `apply_patch` to v1.1 OR implement them with explicit per-tool confirmation toggles off-by-default. This de-risks the highest-complexity phase. Rewind becomes simpler too (only snapshots for writes, which are now v1.1). Synthesis step can decide whether this is a cut or stays as r1 had it.
2. **Replace Phase 10 Wayland-XFCE smoke-test row** with "Cinnamon-Wayland smoke-test on Mint 22.2" OR drop entirely with a note that XFCE 4.18 does not have Wayland. Vision says "X11 and Wayland-XFCE" but that's not reachable on the actual target distro.
3. **Add Anthropic prompt caching** to Phase 5 deliverables: `cache_control: {type: "ephemeral"}` on system prompts + tool definitions. Measurable cost saving.
4. **Narrow Ollama default from `gemma3:4b` to `qwen2.5-coder:7b`** when tool use is needed (agent mode on) and RAM ≥ 12 GB. Keep `gemma3:4b` as the chat-only default.
5. **Add i18n scaffolding** in Phase 2 (react-i18next, one English bundle, all strings wrapped). 1-hour addition, prevents v1.1 find-and-replace pain.
6. **Add a11y checklist** to Phase 9 and Global Acceptance: keyboard-only navigation, ARIA labels on icon buttons, `aria-live="polite"` on chat messages.
7. **Add auto-update strategy** to Phase 9 or 10: Tauri updater plugin for AppImage + GitHub-Releases-API "check for updates" for .deb. Skip apt repo for v1.
8. **Add concrete error taxonomy** (E001–E018 above) to Phase 9 catalogue. r1 had the principle; r2 provides the hierarchy.
9. **Add conversation export/import** to Phase 9 Settings. 0.5 day. User-owned-data principle.
10. **Font-load canary** in Phase 9 Settings > About. Non-blocking.
11. **Use `react-virtuoso`** for Agent Activity and Chat Panel message lists. Add to Phase 5 and Phase 6 deliverables. Handles streaming with stable perf.
12. **Adopt Chat Completions API (not Responses API)** for OpenAI in Phase 7. If reasoning models ever land, they need a separate decoder. Document this as a limitation.
13. **Secret Service detection via `busctl list --user`** (read-only) rather than calling `keyring::get` as a probe. Avoids accidental daemon activation. Small change to Phase 5.
14. **Capability upgrade handling:** on app version bump, if capabilities changed, show a "reopen workspace" toast. Phase 10.

---

## Sources

1. [What's new in Claude Opus 4.7 — Claude API Docs](https://platform.claude.com/docs/en/about-claude/models/whats-new-claude-4-7)
2. [Claude Code Opus 4.7 top_p deprecated error fix — Apiyi](https://help.apiyi.com/en/claude-code-opus-4-7-top-p-deprecated-error-fix-en.html)
3. [Retiring GPT-4o and older models — OpenAI blog](https://openai.com/index/retiring-gpt-4o-and-older-models/)
4. [Retiring GPT-4o and other ChatGPT models — OpenAI Help Center](https://help.openai.com/en/articles/20001051-retiring-gpt-4o-and-other-chatgpt-models)
5. [tauri 2.10.3 — docs.rs](https://docs.rs/crate/tauri/latest)
6. [Releases — xtermjs/xterm.js GitHub](https://github.com/xtermjs/xterm.js/releases)
7. [Streaming responses with tool calling — Ollama Blog](https://ollama.com/blog/streaming-tool)
8. [Ollama tool-calling + streaming issue #12557 — GitHub](https://github.com/ollama/ollama/issues/12557)
9. [AppImage FUSE troubleshooting — AppImage docs](https://docs.appimage.org/user-guide/troubleshooting/fuse.html)
10. [Can't Run AppImage on Ubuntu 24.04? Here's How to Fix it — itsFOSS](https://itsfoss.com/cant-run-appimage-ubuntu/)
11. [Introducing Claude Opus 4.7 in Amazon Bedrock — AWS blog](https://aws.amazon.com/blogs/aws/introducing-anthropics-claude-opus-4-7-model-in-amazon-bedrock/)
12. [Streaming Messages — Claude API Docs](https://platform.claude.com/docs/en/build-with-claude/streaming)
13. [biscuit-auth on crates.io](https://crates.io/crates/biscuit-auth)
14. [Linux Mint 22.2 Zara release notes — 9to5Linux](https://9to5linux.com/linux-mint-22-2-zara-is-now-available-for-download-heres-whats-new)
15. [Linux Mint 22.2 Beta Wayland — Phoronix](https://www.phoronix.com/news/Linux-Mint-22.2-Beta)
16. [Xfce and Wayland — Linux Mint Forums](https://forums.linuxmint.com/viewtopic.php?t=453726)
17. [tauri-plugin-sql 2.3.2 — docs.rs](https://docs.rs/crate/tauri-plugin-sql/latest)
18. [tauri-plugin-rusqlite2 — GitHub](https://github.com/razein97/tauri-plugin-rusqlite2)
19. [gitoxide — GitHub](https://github.com/GitoxideLabs/gitoxide)
20. [gitoxide crate-status.md — GitHub](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md)
21. [gix towards 1.0 — issue #470](https://github.com/GitoxideLabs/gitoxide/issues/470)
22. [How the agent loop works — Claude Code Docs](https://code.claude.com/docs/en/agent-sdk/agent-loop)
23. [Claude Code Architecture Explained — DEV Community](https://dev.to/brooks_wilson_36fbefbbae4/claude-code-architecture-explained-agent-loop-tool-system-and-permission-model-rust-rewrite-41b2)
24. [Inline Assistant — Zed Docs](https://zed.dev/docs/ai/inline-assistant)
25. [Introducing Zed AI — Zed Blog](https://zed.dev/blog/zed-ai)
26. [CodeMirror vs Monaco Editor comparison — PARA Garden](https://agenthicks.com/research/codemirror-vs-monaco-editor-comparison)
27. [Understand Monaco vs CodeMirror comparison — StudyRaid](https://app.studyraid.com/en/read/15534/540311/monaco-vs-codemirror-comparison)
28. [OpenAI Responses API streaming events](https://platform.openai.com/docs/api-reference/responses-streaming)
29. [OpenAI function calling guide](https://developers.openai.com/api/docs/guides/function-calling)
30. [Ollama qwen3.5:9b tool call issue #14745](https://github.com/ollama/ollama/issues/14745)
31. [secret-service — docs.rs](https://docs.rs/secret-service/latest/secret_service/)
32. [ArchWiki — GNOME/Keyring](https://wiki.archlinux.org/title/GNOME/Keyring)
33. [Libghostty Is Coming — Mitchell Hashimoto](https://mitchellh.com/writing/libghostty-is-coming)
34. [Ghostling — C API demo for libghostty](https://github.com/ghostty-org/ghostling)
35. [@monaco-editor/react — Bundlephobia](https://bundlephobia.com/package/@monaco-editor/react)
36. [Migrating from Monaco Editor to CodeMirror — Sourcegraph](https://sourcegraph.com/blog/migrating-monaco-codemirror)
37. [SolidJS vs React 2026 — BoundDev](https://www.boundev.com/blog/solidjs-vs-react-2026-performance-guide)
38. [State of Solid.js in 2026 — listiak.dev](https://listiak.dev/blog/the-state-of-solid-js-in-2026-signals-performance-and-growing-influence)
39. [Stronghold plugin — Tauri v2 docs (deprecation note)](https://v2.tauri.app/plugin/stronghold/)
40. [Updater plugin — Tauri v2](https://v2.tauri.app/plugin/updater/)
41. [Capabilities — Tauri v2 security](https://v2.tauri.app/security/capabilities/)
42. [Tauri config schema draft-07](https://schema.tauri.app/config/2)
43. [Monaco large bundle size — issue #678 monaco-languageclient](https://github.com/TypeFox/monaco-languageclient/issues/678)
44. [monaco-editor-webpack-plugin excluding features issue #97](https://github.com/microsoft/monaco-editor-webpack-plugin/issues/97)
45. [monaco-vscode-api bundle size issue #383](https://github.com/CodinGame/monaco-vscode-api/issues/383)
46. [Prompt caching — Claude API Docs](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
47. [Building with extended thinking — Claude API Docs](https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking)
48. [qwen2.5-coder — Ollama library](https://ollama.com/library/qwen2.5-coder)
49. [gemma3-ollama-tools — GitHub (why base Gemma 3 doesn't tool-call in Ollama)](https://github.com/IllFil/gemma3-ollama-tools)
50. [orieg/gemma3-tools — Ollama library](https://ollama.com/orieg/gemma3-tools)
51. [Building Privacy-First AI Agents with Gemma 4 and Ollama — Dev|Journal](https://earezki.com/ai-news/2026-04-13-how-to-implement-tool-calling-with-gemma-4-and-python/)
52. [Split Diffs are Here — Zed Blog](https://zed.dev/blog/split-diffs)
53. [Zed preview release 0.189.0](https://zed.dev/releases/preview/0.189.0)
54. [Diff Management — Continue DeepWiki](https://deepwiki.com/continuedev/continue/6.8-diff-management)
55. [React Virtuoso homepage](https://virtuoso.dev/)
56. [react-virtuoso — GitHub](https://github.com/petyosi/react-virtuoso)
57. [react-i18next documentation](https://www.i18next.com/)
58. [Tauri v2 multi-window discussion #10997](https://github.com/tauri-apps/tauri/discussions/10997)
59. [sentry-tauri — GitHub](https://github.com/timfish/sentry-tauri)
60. [Does Sentry have a Tauri v2 SDK? — Sentry Help Center](https://sentry.zendesk.com/hc/en-us/articles/28526715924251-Does-Sentry-have-a-Tauri-v2-SDK)

---

*End of research-r2.md. Complementary to research-r1.md; read together, not separately.*
