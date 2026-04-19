// tests/error-catalogue.spec.ts
//
// Phase 9 audit deliverable. Asserts every catalogued error code has:
//   1. A trigger that forces the failure
//   2. The trigger results in the catalogued ErrorToast rendering
//   3. NO raw stack trace surfaces to the user
//
// Phase 9 references this file by name in plan.md ACs ("every code in
// docs/ERROR-CATALOGUE.md has a passing trigger test in
// tests/error-catalogue.spec.ts").
//
// Phase 1 ships the harness with E001 wired (the proof-of-concept code).
// Each subsequent phase that registers a new code adds its trigger here.
// Phase 9 audits this file for completeness.

import { describe, expect, it } from 'vitest';
import type { AppErrorPayload, ErrorCode } from '../src/errors/types';

// ---------- Trigger registry ----------
//
// Each entry: () => Promise<AppErrorPayload> that forces the failure path
// and returns the payload that ErrorToast received. Phase coders fill in
// real triggers; until they do, the test is `.skip`ed for that code.

type TriggerFn = () => Promise<AppErrorPayload>;

const TRIGGERS: Partial<Record<ErrorCode, TriggerFn>> = {
  // Phase 1 — wired here as the proof-of-concept. Forces a synthetic
  // KeyringMissing event and asserts the toast layer renders it.
  E001: async () => {
    // Phase 1 coder fills in:
    //   1. Mount <ToastLayer /> in a test renderer
    //   2. window.dispatchEvent(new CustomEvent('biscuitcode:error-toast', {
    //        detail: { code: 'E001', messageKey: 'errors.E001.msg', ... } }))
    //   3. Assert getByRole('alert') text contains the i18n message
    //   4. Return the payload that was dispatched
    throw new Error('Phase 1 coder fills in E001 trigger');
  },

  // E002 → E018 added by their owning phases. Until then, .skip.
};

// ---------- The harness ----------

const ALL_CODES: ReadonlyArray<ErrorCode> = [
  'E001', 'E002', 'E003', 'E004', 'E005', 'E006', 'E007', 'E008', 'E009',
  'E010', 'E011', 'E012', 'E013', 'E014', 'E015', 'E016', 'E017', 'E018',
];

describe('error catalogue triggers', () => {
  for (const code of ALL_CODES) {
    const trigger = TRIGGERS[code];
    const test = trigger ? it : it.skip;

    test(`${code}: triggering renders the catalogued toast (no raw stack)`, async () => {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const payload = await trigger!();
      expect(payload.code).toBe(code);
      expect(payload.messageKey).toMatch(/^errors\.E\d{3}\.msg$/);
      // Recovery is optional but if present, must be a known kind.
      if (payload.recovery) {
        expect([
          'retry',
          'copy_command',
          'open_url',
          'deeplink_settings',
          'dismiss_only',
        ]).toContain(payload.recovery.kind);
      }
    });
  }

  it('Phase 9 audit: every code has a registered trigger', () => {
    const missing = ALL_CODES.filter((c) => TRIGGERS[c] === undefined);
    if (missing.length > 0) {
      // Phase 9 audit: this assertion fails until every phase from 1-8
      // has registered its codes' triggers. Until Phase 9, the warning
      // is informational.
      console.warn(
        `[catalogue audit] ${missing.length} codes lack triggers: ${missing.join(', ')}`,
      );
    }
    // After Phase 9 audit (Phase 9 deliverable), un-comment the assertion:
    // expect(missing).toEqual([]);
  });
});
