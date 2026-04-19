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

import React from 'react';
import { describe, expect, it } from 'vitest';
import { render } from '@testing-library/react';
import type { AppErrorPayload, ErrorCode } from '../src/errors/types';
import { ToastLayer } from '../src/components/ToastLayer';

// i18n must be initialised before rendering i18n-aware components.
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from '../src/locales/en.json';

if (!i18n.isInitialized) {
  void i18n.use(initReactI18next).init({
    resources: { en: { translation: en } },
    lng: 'en',
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
  });
}

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
    // Mount the ToastLayer and dispatch a synthetic E001 event.
    // Asserts: (a) role="alert" renders, (b) user-friendly message shown,
    // (c) no raw stack trace in the rendered output.
    const payload: AppErrorPayload = {
      code: 'E001',
      messageKey: 'errors.E001.msg',
      recovery: {
        kind: 'copy_command',
        command: 'sudo apt install gnome-keyring libsecret-1-0 libsecret-tools',
        label: 'Copy install command',
      },
    };

    const { getByRole, queryByText } = render(React.createElement(ToastLayer));

    window.dispatchEvent(
      new CustomEvent('biscuitcode:error-toast', { detail: payload }),
    );

    // Wait for the toast to render (state update is synchronous here but
    // give React one microtask to flush).
    await new Promise((r) => setTimeout(r, 0));

    const alert = getByRole('alert');
    // Verify user-friendly message rendered (from en.json errors.E001.msg).
    expect(alert.textContent).toContain('gnome-keyring');
    // Verify no raw stack trace visible.
    expect(queryByText(/at \w+ \(/)?.textContent).toBeUndefined();
    // Verify the error code badge is shown.
    expect(alert.textContent).toContain('E001');

    return payload;
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
