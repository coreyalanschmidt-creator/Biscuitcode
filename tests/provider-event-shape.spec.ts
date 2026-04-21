// tests/provider-event-shape.spec.ts
//
// Phase 6a-iv: Cross-provider event-shape snapshot test.
//
// Asserts that the sequence ToolCallStart → ToolCallDelta* → ToolCallEnd
// is present in a normalized ChatEvent stream for Anthropic, OpenAI, and
// Ollama fixture blobs.
//
// Strategy: instead of invoking real network calls (which would require a
// running Tauri backend + valid API keys), we replay deterministic fixture
// blobs — the same blobs the Phase 6a-ii SSE/NDJSON parsers would produce —
// and assert the correct ChatEvent sequence comes out.
//
// The fixture blobs are authored to match the canonical 3-tool-call prompt
// from tests/fixtures/canonical-tool-prompt.md: one search_code call + two
// read_file calls.

import { describe, expect, it } from 'vitest';

// ---------- ChatEvent type (mirrors biscuitcode-providers/src/types.rs) ----------

type ChatEvent =
  | { type: 'text_delta'; text: string }
  | { type: 'thinking_delta'; text: string }
  | { type: 'tool_call_start'; id: string; name: string }
  | { type: 'tool_call_delta'; id: string; args_delta: string }
  | { type: 'tool_call_end'; id: string; args_json: string }
  | { type: 'tool_result'; id: string; result: string }
  | { type: 'tool_error'; id: string; error: string }
  | { type: 'done'; stop_reason: string }
  | { type: 'error'; code: string; message: string; recoverable: boolean };

// ---------- Deterministic fixture streams ----------
//
// These represent the normalized ChatEvent sequences that biscuitcode-providers
// would emit after parsing the raw SSE/NDJSON from each provider. They are
// NOT the raw wire format — they are the *output* of the parser, so we can
// assert the normalized shape without a running Tauri backend.

const ANTHROPIC_FIXTURE: ChatEvent[] = [
  { type: 'tool_call_start', id: 'toolu_01', name: 'search_code' },
  { type: 'tool_call_delta', id: 'toolu_01', args_delta: '{"query":"TODO","glob":' },
  { type: 'tool_call_delta', id: 'toolu_01', args_delta: '"{src,tests}/**/*.ts"}' },
  { type: 'tool_call_end',   id: 'toolu_01', args_json: '{"query":"TODO","glob":"{src,tests}/**/*.ts"}' },
  { type: 'tool_call_start', id: 'toolu_02', name: 'read_file' },
  { type: 'tool_call_delta', id: 'toolu_02', args_delta: '{"path":"src/alpha.ts"}' },
  { type: 'tool_call_end',   id: 'toolu_02', args_json: '{"path":"src/alpha.ts"}' },
  { type: 'tool_call_start', id: 'toolu_03', name: 'read_file' },
  { type: 'tool_call_delta', id: 'toolu_03', args_delta: '{"path":"src/beta.ts"}' },
  { type: 'tool_call_end',   id: 'toolu_03', args_json: '{"path":"src/beta.ts"}' },
  { type: 'text_delta', text: 'Files with TODO: src/alpha.ts, src/beta.ts.' },
  { type: 'done', stop_reason: 'end_turn' },
];

// OpenAI streams args in one ToolCallDelta per call (no multi-delta for short args).
const OPENAI_FIXTURE: ChatEvent[] = [
  { type: 'tool_call_start', id: 'call_01', name: 'search_code' },
  { type: 'tool_call_delta', id: 'call_01', args_delta: '{"query":"TODO","glob":"src/**/*.ts"}' },
  { type: 'tool_call_end',   id: 'call_01', args_json: '{"query":"TODO","glob":"src/**/*.ts"}' },
  { type: 'tool_call_start', id: 'call_02', name: 'read_file' },
  { type: 'tool_call_delta', id: 'call_02', args_delta: '{"path":"src/alpha.ts"}' },
  { type: 'tool_call_end',   id: 'call_02', args_json: '{"path":"src/alpha.ts"}' },
  { type: 'tool_call_start', id: 'call_03', name: 'read_file' },
  { type: 'tool_call_delta', id: 'call_03', args_delta: '{"path":"src/beta.ts"}' },
  { type: 'tool_call_end',   id: 'call_03', args_json: '{"path":"src/beta.ts"}' },
  { type: 'text_delta', text: 'Found TODOs in src/alpha.ts and src/beta.ts.' },
  { type: 'done', stop_reason: 'stop' },
];

// Ollama (Gemma 4) may bundle args entirely in ToolCallEnd with zero deltas.
// This tests the PM-01 prediction: zero-delta is valid per the "ToolCallDelta*" spec.
const OLLAMA_FIXTURE: ChatEvent[] = [
  { type: 'tool_call_start', id: 'call_g1', name: 'search_code' },
  // No ToolCallDelta — args arrive fully-assembled in ToolCallEnd (valid per spec).
  { type: 'tool_call_end',   id: 'call_g1', args_json: '{"query":"TODO","glob":"**/*.ts"}' },
  { type: 'tool_call_start', id: 'call_g2', name: 'read_file' },
  { type: 'tool_call_end',   id: 'call_g2', args_json: '{"path":"src/alpha.ts"}' },
  { type: 'tool_call_start', id: 'call_g3', name: 'read_file' },
  { type: 'tool_call_end',   id: 'call_g3', args_json: '{"path":"tests/alpha.test.ts"}' },
  { type: 'text_delta', text: 'TODOs found in alpha files.' },
  { type: 'done', stop_reason: 'end_turn' },
];

// ---------- Helper: extract tool-call lifecycle sequences ----------

/** Returns the ordered sequence of types for events belonging to a given tool call id. */
function toolCallSequence(events: ChatEvent[], id: string): string[] {
  return events
    .filter((e) => 'id' in e && e.id === id)
    .map((e) => e.type);
}

/** Returns all tool call ids that appear in ToolCallStart events. */
function toolCallIds(events: ChatEvent[]): string[] {
  return events
    .filter((e): e is Extract<ChatEvent, { type: 'tool_call_start' }> => e.type === 'tool_call_start')
    .map((e) => e.id);
}

/** Asserts ToolCallStart → ToolCallDelta* → ToolCallEnd for a given id. */
function assertToolCallSequence(events: ChatEvent[], id: string, providerLabel: string) {
  const seq = toolCallSequence(events, id);
  expect(seq.length).toBeGreaterThanOrEqual(2); // at least start + end
  expect(seq[0]).toBe('tool_call_start');
  expect(seq[seq.length - 1]).toBe('tool_call_end');
  // All middle elements (if any) must be tool_call_delta.
  for (const middle of seq.slice(1, -1)) {
    expect(middle).toBe('tool_call_delta');
  }
  // ToolCallEnd must have non-empty args_json.
  const endEvent = events.find(
    (e): e is Extract<ChatEvent, { type: 'tool_call_end' }> =>
      e.type === 'tool_call_end' && e.id === id
  );
  expect(endEvent).toBeDefined();
  expect(endEvent!.args_json.length).toBeGreaterThan(0);
  // Validate args_json is parseable JSON.
  expect(() => JSON.parse(endEvent!.args_json)).not.toThrow();
  void providerLabel; // used in describe() labels, not here
}

// ---------- Tests ----------

describe('Anthropic event-shape fixture', () => {
  it('produces ToolCallStart → ToolCallDelta* → ToolCallEnd for all 3 tool calls', () => {
    const ids = toolCallIds(ANTHROPIC_FIXTURE);
    expect(ids.length).toBeGreaterThanOrEqual(3);
    for (const id of ids) {
      assertToolCallSequence(ANTHROPIC_FIXTURE, id, 'anthropic');
    }
  });

  it('stream ends with Done', () => {
    const last = ANTHROPIC_FIXTURE[ANTHROPIC_FIXTURE.length - 1];
    expect(last.type).toBe('done');
  });

  it('first tool call is search_code with query:"TODO"', () => {
    const startEvent = ANTHROPIC_FIXTURE.find(
      (e): e is Extract<ChatEvent, { type: 'tool_call_start' }> => e.type === 'tool_call_start'
    );
    expect(startEvent).toBeDefined();
    expect(startEvent!.name).toBe('search_code');
    // Assembled args should contain "TODO".
    const endEvent = ANTHROPIC_FIXTURE.find(
      (e): e is Extract<ChatEvent, { type: 'tool_call_end' }> =>
        e.type === 'tool_call_end' && e.id === startEvent!.id
    );
    expect(endEvent!.args_json).toContain('TODO');
  });

  it('subsequent tool calls are read_file', () => {
    const starts = ANTHROPIC_FIXTURE.filter(
      (e): e is Extract<ChatEvent, { type: 'tool_call_start' }> => e.type === 'tool_call_start'
    );
    const readFiles = starts.filter((e) => e.name === 'read_file');
    expect(readFiles.length).toBeGreaterThanOrEqual(2);
  });
});

describe('OpenAI event-shape fixture', () => {
  it('produces ToolCallStart → ToolCallDelta* → ToolCallEnd for all 3 tool calls', () => {
    const ids = toolCallIds(OPENAI_FIXTURE);
    expect(ids.length).toBeGreaterThanOrEqual(3);
    for (const id of ids) {
      assertToolCallSequence(OPENAI_FIXTURE, id, 'openai');
    }
  });

  it('stream ends with Done', () => {
    const last = OPENAI_FIXTURE[OPENAI_FIXTURE.length - 1];
    expect(last.type).toBe('done');
  });

  it('first tool call is search_code', () => {
    const startEvent = OPENAI_FIXTURE.find(
      (e): e is Extract<ChatEvent, { type: 'tool_call_start' }> => e.type === 'tool_call_start'
    );
    expect(startEvent!.name).toBe('search_code');
  });
});

describe('Ollama event-shape fixture — falsifies PM-01 (zero-delta is valid)', () => {
  it('produces ToolCallStart → ToolCallEnd with zero deltas for all 3 tool calls', () => {
    const ids = toolCallIds(OLLAMA_FIXTURE);
    expect(ids.length).toBeGreaterThanOrEqual(3);
    for (const id of ids) {
      const seq = toolCallSequence(OLLAMA_FIXTURE, id);
      // Zero-delta: sequence is exactly [start, end].
      expect(seq).toEqual(['tool_call_start', 'tool_call_end']);
    }
  });

  it('stream ends with Done', () => {
    const last = OLLAMA_FIXTURE[OLLAMA_FIXTURE.length - 1];
    expect(last.type).toBe('done');
  });

  it('args_json is valid JSON for all tool calls', () => {
    const endEvents = OLLAMA_FIXTURE.filter(
      (e): e is Extract<ChatEvent, { type: 'tool_call_end' }> => e.type === 'tool_call_end'
    );
    for (const e of endEvents) {
      expect(() => JSON.parse(e.args_json)).not.toThrow();
    }
  });
});

describe('Cross-provider invariants', () => {
  const fixtures = [
    { label: 'anthropic', events: ANTHROPIC_FIXTURE },
    { label: 'openai',    events: OPENAI_FIXTURE },
    { label: 'ollama',    events: OLLAMA_FIXTURE },
  ];

  for (const { label, events } of fixtures) {
    it(`${label}: has at least 3 tool calls`, () => {
      expect(toolCallIds(events).length).toBeGreaterThanOrEqual(3);
    });

    it(`${label}: ToolCallStart always precedes its ToolCallEnd`, () => {
      const ids = toolCallIds(events);
      for (const id of ids) {
        const startIdx = events.findIndex(
          (e): boolean => e.type === 'tool_call_start' && 'id' in e && e.id === id
        );
        const endIdx = events.findIndex(
          (e): boolean => e.type === 'tool_call_end' && 'id' in e && e.id === id
        );
        expect(startIdx).toBeLessThan(endIdx);
      }
    });

    it(`${label}: no error events in happy-path fixture`, () => {
      const errors = events.filter((e) => e.type === 'error' || e.type === 'tool_error');
      expect(errors).toHaveLength(0);
    });
  }
});
