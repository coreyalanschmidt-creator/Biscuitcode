// tests/unit/agentStore.spec.ts
//
// Phase 6a-i acceptance criteria: minimum 5 tests covering the
// plan-required action names: addCard, updateCardArgs, completeCard,
// errorCard, clearCards.

import { beforeEach, describe, expect, it } from 'vitest';
import { useAgentStore } from '../../src/state/agentStore';

beforeEach(() => {
  useAgentStore.setState({ cards: [], agentMode: false, conversationId: null });
});

describe('agentStore — Phase 6a-i required API', () => {
  it('addCard creates a card with status:running, empty argsJson, and null result', () => {
    useAgentStore.getState().addCard('t1', 'read_file');
    const card = useAgentStore.getState().cards[0];
    expect(card).toBeDefined();
    expect(card.id).toBe('t1');
    expect(card.name).toBe('read_file');
    expect(card.status).toBe('running');
    expect(card.argsJson).toBe('');
    expect(card.result).toBeNull();
    expect(card.endedAt).toBeNull();
    expect(card.startedAt).toBeGreaterThan(0);
  });

  it('updateCardArgs appends delta to argsJson for the matching card', () => {
    useAgentStore.getState().addCard('t2', 'search_code');
    useAgentStore.getState().updateCardArgs('t2', '{"pat');
    useAgentStore.getState().updateCardArgs('t2', 'tern":"foo"}');
    const card = useAgentStore.getState().cards[0];
    expect(card.argsJson).toBe('{"pattern":"foo"}');
  });

  it('completeCard sets status:ok and result on the matching card', () => {
    useAgentStore.getState().addCard('t3', 'read_file');
    useAgentStore.getState().completeCard('t3', 'file contents here');
    const card = useAgentStore.getState().cards[0];
    expect(card.status).toBe('ok');
    expect(card.result).toBe('file contents here');
    expect(card.endedAt).not.toBeNull();
  });

  it('errorCard sets status:error and result to the error string on the matching card', () => {
    useAgentStore.getState().addCard('t4', 'read_file');
    useAgentStore.getState().errorCard('t4', 'file not found');
    const card = useAgentStore.getState().cards[0];
    expect(card.status).toBe('error');
    expect(card.result).toBe('file not found');
    expect(card.endedAt).not.toBeNull();
  });

  it('clearCards removes all cards from the store', () => {
    useAgentStore.getState().addCard('t5a', 'read_file');
    useAgentStore.getState().addCard('t5b', 'search_code');
    expect(useAgentStore.getState().cards).toHaveLength(2);
    useAgentStore.getState().clearCards();
    expect(useAgentStore.getState().cards).toHaveLength(0);
  });

  it('addCard does not affect cards with different ids', () => {
    useAgentStore.getState().addCard('t6a', 'read_file');
    useAgentStore.getState().addCard('t6b', 'search_code');
    useAgentStore.getState().completeCard('t6a', 'result-a');
    const [cardA, cardB] = useAgentStore.getState().cards;
    expect(cardA.status).toBe('ok');
    expect(cardB.status).toBe('running');
  });
});
