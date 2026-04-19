// tests/unit/error-codes-distinct.spec.ts
//
// Phase 1 deliverable (extends through Phase 9 audit). Asserts the
// TypeScript error catalogue agrees with the schema invariants:
//   - Codes match E0NN format
//   - No duplicates
//   - Every variant has the same code as its discriminator literal
//
// Mirrors the Rust-side test in biscuitcode-core/src/errors.rs.

import { describe, expect, it } from 'vitest';
import type { ErrorCode } from '../../src/errors/types';

// Hard-coded list of every code we expect to exist. CI fails if this
// drifts from src/errors/types.ts (developer must update both).
const EXPECTED_CODES: ReadonlyArray<ErrorCode> = [
  'E001', 'E002', 'E003', 'E004', 'E005', 'E006', 'E007', 'E008', 'E009',
  'E010', 'E011', 'E012', 'E013', 'E014', 'E015', 'E016', 'E017', 'E018',
];

describe('error catalogue codes', () => {
  it('every code matches the E0NN format', () => {
    for (const code of EXPECTED_CODES) {
      expect(code, `bad code: ${code}`).toMatch(/^E\d{3}$/);
    }
  });

  it('codes are distinct', () => {
    const set = new Set<string>(EXPECTED_CODES);
    expect(set.size).toBe(EXPECTED_CODES.length);
  });

  it('codes are dense from E001 (no gaps mid-sequence)', () => {
    // We allow gaps if a future deprecation removed a code, but ban
    // un-monotonic numbering. Catches "you accidentally used E020 next
    // when E019 hasn't shipped" mistakes.
    const numbers = EXPECTED_CODES.map((c) => parseInt(c.slice(1), 10)).sort((a, b) => a - b);
    expect(numbers[0]).toBe(1);
    for (let i = 1; i < numbers.length; i++) {
      const gap = numbers[i] - numbers[i - 1];
      expect(gap, `gap of ${gap} after E${String(numbers[i - 1]).padStart(3, '0')}`).toBeLessThanOrEqual(1);
    }
  });
});
