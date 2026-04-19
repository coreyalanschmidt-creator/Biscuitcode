// tests/e2e/agent-tool-card-render.spec.ts
//
// Phase 6a deliverable. Asserts the 250ms tool-card-render gate from
// Global Acceptance Criteria.
//
// The fixture and contract are in `tests/fixtures/canonical-tool-prompt.md`.
// Briefly: send a prompt that triggers exactly 3 tool calls, and assert
// `tool_card_visible_<id> - tool_call_start_<id> < 250ms` for every id.
//
// This skeleton is a placeholder until Phase 6a wires `react-virtuoso`
// chat panel + Agent Activity instrumentation. The Phase 6a coder
// replaces the body once those exist.

import { describe, expect, it } from 'vitest';

// Placeholder skipped until Phase 6a lands.
describe.skip('agent tool card render gate (Phase 6a)', () => {
  it('every tool_call_start -> tool_card_visible measure is under 250ms', async () => {
    // 1. Mount the app in an e2e harness (Playwright? @testing-library/react?
    //    — Phase 6a coder picks per the chosen test stack).
    // 2. Programmatically dispatch the canonical 3-tool prompt against
    //    the OpenAI provider (uses real network or recorded transcript).
    // 3. Wait for the conversation to complete.
    // 4. Read window.performance.getEntriesByType('measure') for entries
    //    named 'tool_card_render' (set by the Phase 6a executor + the
    //    AgentActivityPanel MutationObserver).
    // 5. Assert every entry.duration < 250.

    expect(true).toBe(true);
  });
});
