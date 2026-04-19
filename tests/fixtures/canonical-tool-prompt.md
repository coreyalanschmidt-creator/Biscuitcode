# Canonical 3-Tool Prompt Fixture

> Used by `tests/e2e/agent-tool-card-render.spec.ts` (Phase 6a deliverable + Global AC) and `tests/e2e/agent-mode-demo.spec.ts` (Phase 6a deliverable). Pre-staged so the Phase 6a coder doesn't have to invent a fixture from scratch — the test names and acceptance criteria already reference this exact path in `docs/plan.md`.

## Why this fixture exists

The plan asserts that "for a prompt triggering 3 tool calls, every `tool_card_visible_<id> - tool_call_start_<id>` measure is under 250ms." That assertion needs a deterministic 3-tool-call prompt that:

1. Reliably triggers 3 distinct tool calls (not 0, not 5) on every model that supports tool use.
2. Does not depend on the workspace's specific contents (so it works in CI on a synthetic test workspace).
3. Exercises both the read-only tools shipped in Phase 6a (`read_file`, `search_code`).

## Setup (test harness creates this synthetic workspace before invoking the prompt)

The harness creates a temp workspace with exactly these files:

```
<workspace-root>/
├── README.md          (single line: "# Demo workspace")
├── src/
│   ├── alpha.ts       (single line: "// TODO: implement alpha")
│   ├── beta.ts        (single line: "// TODO: implement beta")
│   └── gamma.ts       (single line: "// no marker here")
└── tests/
    └── alpha.test.ts  (single line: "// TODO: cover alpha")
```

Three files contain `TODO`; one does not. This means a `search_code("TODO", "**/*.ts")` returns exactly 3 hits.

## The prompt

The test sends this exact user message to the agent (with agent mode ON, read-only tools registered):

```
Find every TypeScript file in src/ and tests/ that contains the string "TODO".
For each file you find, read its contents and report the file path and the
single line that contains the TODO. Do not summarize; quote the line verbatim.
```

## Expected agent behavior (verified by the e2e test)

The agent must produce, in order:

1. **Tool call #1**: `search_code` with arguments approximately `{"query": "TODO", "glob": "{src,tests}/**/*.ts"}` (exact glob shape may vary by model — the test asserts the call is `search_code` with a `query` field equal to `"TODO"` and a `glob` containing both `src` and `tests`).
2. **Tool call #2**: `read_file` with `{"path": "<one of the matching files>"}` — model picks one of the three matches.
3. **Tool call #3**: another `read_file` for a *different* one of the three matches.

(The model may issue a 4th `read_file` for the third match; the test treats that as acceptable but only asserts the timing gate on the first three. The model may also batch reads in different ways — what matters for the gate is that ≥ 3 `ToolCallStart` events occur and each gets a card rendered within 250ms.)

The final assistant text message must contain at least 3 file paths from the synthetic workspace (`src/alpha.ts`, `src/beta.ts`, or `tests/alpha.test.ts`) and at least one line containing the substring `TODO`.

## Models the test runs against

Phase 6a CI runs this test against:
- `claude-opus-4-7` (Anthropic — required to pass)
- `gpt-5.4-mini` (OpenAI — required to pass)
- `gemma4:e4b` via Ollama (optional in CI; required for the Phase 10 release-smoke checklist on a GPU/CPU runner that can host the model)

## Why this is a test fixture and not a unit test

The 250ms tool-card-render gate is a UI-rendering performance assertion against real provider streams. It cannot be unit-tested with a mocked stream because the gate exists specifically to catch regressions where the UI layer waits for a tool call to *complete* before rendering the card (instead of rendering on the start event). A mocked stream where start and complete fire instantaneously would never expose the regression.

The test must run against a real provider with realistic streaming latency. Use the `gpt-5.4-mini` row as the canonical CI signal because (a) it has stable tool-use streaming, (b) it's cheap, and (c) it has predictable inter-event timing that makes the 250ms threshold a meaningful gate rather than a flaky one.
