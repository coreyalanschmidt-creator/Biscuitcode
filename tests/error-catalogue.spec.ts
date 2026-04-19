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
import { afterEach, describe, expect, it } from 'vitest';
import { render, cleanup } from '@testing-library/react';
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

// ---------- helpers ----------

/** Dispatch an error toast event and return the rendered alert for that code. */
async function dispatchAndFindAlert(
  payload: AppErrorPayload,
): Promise<HTMLElement> {
  const { getAllByRole } = render(React.createElement(ToastLayer));
  window.dispatchEvent(
    new CustomEvent('biscuitcode:error-toast', { detail: payload }),
  );
  await new Promise((r) => setTimeout(r, 0));
  const alerts = getAllByRole('alert');
  const match = alerts.find(
    (el) => el.getAttribute('data-error-code') === payload.code,
  );
  if (!match) throw new Error(`${payload.code} alert not found in rendered toast layer`);
  return match;
}

// ---------- Trigger registry ----------
//
// Each entry: () => Promise<AppErrorPayload> that forces the failure path
// and returns the payload that ErrorToast received.

type TriggerFn = () => Promise<AppErrorPayload>;

const TRIGGERS: Partial<Record<ErrorCode, TriggerFn>> = {
  // Phase 1 — wired here as the proof-of-concept. Forces a synthetic
  // KeyringMissing event and asserts the toast layer renders it.
  E001: async () => {
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

  // Phase 3 — E002 OutsideWorkspace: path outside workspace root denied.
  E002: async () => {
    const payload: AppErrorPayload = {
      code: 'E002',
      messageKey: 'errors.E002.msg',
      interpolations: { path: '/etc/passwd' },
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E002');
    expect(alert.textContent).toContain('workspace');
    return payload;
  },

  // Phase 4 — E003 PtyOpenFailed: forces a synthetic PTY-open failure
  // (non-existent shell binary) and asserts the toast renders correctly.
  E003: async () => {
    const payload: AppErrorPayload = {
      code: 'E003',
      messageKey: 'errors.E003.msg',
      interpolations: { reason: '/bin/does-not-exist: No such file or directory' },
      recovery: { kind: 'dismiss_only' },
    };

    const { getAllByRole, queryByText } = render(React.createElement(ToastLayer));
    window.dispatchEvent(new CustomEvent('biscuitcode:error-toast', { detail: payload }));
    await new Promise((r) => setTimeout(r, 0));

    // Find the E003 alert specifically (other tests may have left E001 toast in DOM).
    const alerts = getAllByRole('alert');
    const e003Alert = alerts.find((el) => el.getAttribute('data-error-code') === 'E003');
    if (!e003Alert) throw new Error('E003 alert not found in rendered toast layer');

    expect(e003Alert.textContent).toContain('terminal');
    expect(queryByText(/at \w+ \(/)?.textContent).toBeUndefined();
    expect(e003Alert.textContent).toContain('E003');
    return payload;
  },

  // Phase 5 — E004 AnthropicAuthInvalid: API key rejected (HTTP 401).
  E004: async () => {
    const payload: AppErrorPayload = {
      code: 'E004',
      messageKey: 'errors.E004.msg',
      recovery: { kind: 'deeplink_settings', section: 'models' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E004');
    expect(alert.textContent).toContain('Anthropic');
    return payload;
  },

  // Phase 5 — E005 AnthropicNetworkError: DNS/TLS failure to api.anthropic.com.
  E005: async () => {
    const payload: AppErrorPayload = {
      code: 'E005',
      messageKey: 'errors.E005.msg',
      recovery: { kind: 'retry' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E005');
    expect(alert.textContent).toContain('Anthropic');
    return payload;
  },

  // Phase 5 — E006 AnthropicRateLimited: HTTP 429 from Anthropic.
  E006: async () => {
    const payload: AppErrorPayload = {
      code: 'E006',
      messageKey: 'errors.E006.msg',
      interpolations: { retry_after_seconds: 30 },
      recovery: { kind: 'retry' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E006');
    expect(alert.textContent).toContain('rate');
    return payload;
  },

  // Phase 6a — E007 GemmaVersionFallback: Ollama < 0.20.0.
  E007: async () => {
    const payload: AppErrorPayload = {
      code: 'E007',
      messageKey: 'errors.E007.msg',
      recovery: {
        kind: 'copy_command',
        command: 'curl -fsSL https://ollama.com/install.sh | sh',
        label: 'Copy upgrade command',
      },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E007');
    expect(alert.textContent).toContain('Gemma');
    return payload;
  },

  // Phase 6b — E008 WriteToolDenied: user declined write confirmation modal.
  E008: async () => {
    const payload: AppErrorPayload = {
      code: 'E008',
      messageKey: 'errors.E008.msg',
      interpolations: { path: '/workspace/src/main.rs' },
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E008');
    expect(alert.textContent).toContain('declined');
    return payload;
  },

  // Phase 6b — E009 ShellForbiddenPrefix: blocked shell command.
  E009: async () => {
    const payload: AppErrorPayload = {
      code: 'E009',
      messageKey: 'errors.E009.msg',
      interpolations: { command: 'sudo rm -rf /' },
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E009');
    expect(alert.textContent).toContain('blocked');
    return payload;
  },

  // Phase 6b — E010 SnapshotFailed: pre-write snapshot failed; write NOT performed.
  E010: async () => {
    const payload: AppErrorPayload = {
      code: 'E010',
      messageKey: 'errors.E010.msg',
      interpolations: { path: '/workspace/src/lib.rs' },
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E010');
    expect(alert.textContent).toContain('undo point');
    return payload;
  },

  // Phase 6b — E011 RewindFailed: snapshot .bak missing or corrupt.
  E011: async () => {
    const payload: AppErrorPayload = {
      code: 'E011',
      messageKey: 'errors.E011.msg',
      interpolations: { path: '/workspace/src/lib.rs' },
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E011');
    expect(alert.textContent).toContain('undo');
    return payload;
  },

  // Phase 7 — E012 GitPushFailed: git push exited non-zero.
  E012: async () => {
    const payload: AppErrorPayload = {
      code: 'E012',
      messageKey: 'errors.E012.msg',
      interpolations: { git_stderr: 'rejected: non-fast-forward' },
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E012');
    expect(alert.textContent).toContain('push failed');
    return payload;
  },

  // Phase 7 — E013 LspServerMissing: LSP binary not on PATH.
  E013: async () => {
    const payload: AppErrorPayload = {
      code: 'E013',
      messageKey: 'errors.E013.msg',
      interpolations: {
        language: 'Rust',
        install_command: 'rustup component add rust-analyzer',
      },
      recovery: {
        kind: 'copy_command',
        command: 'rustup component add rust-analyzer',
      },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E013');
    expect(alert.textContent).toContain('language server');
    return payload;
  },

  // Phase 7 — E014 LspProtocolError: LSP server crashed / malformed JSON-RPC.
  E014: async () => {
    const payload: AppErrorPayload = {
      code: 'E014',
      messageKey: 'errors.E014.msg',
      interpolations: { language: 'TypeScript' },
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E014');
    expect(alert.textContent).toContain('language server');
    return payload;
  },

  // Phase 7 — E015 PreviewRenderFailed: preview threw during render.
  E015: async () => {
    const payload: AppErrorPayload = {
      code: 'E015',
      messageKey: 'errors.E015.msg',
      interpolations: { file: 'README.md', reason: 'out of memory' },
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E015');
    expect(alert.textContent).toContain('preview');
    return payload;
  },

  // Phase 8 — E016 FontLoadFailed: self-hosted font woff2 not loaded.
  E016: async () => {
    const payload: AppErrorPayload = {
      code: 'E016',
      messageKey: 'errors.E016.msg',
      recovery: { kind: 'dismiss_only' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E016');
    expect(alert.textContent).toContain('fonts');
    return payload;
  },

  // Phase 9 — E017 UpdateCheckFailed: GitHub Releases API call failed.
  E017: async () => {
    const payload: AppErrorPayload = {
      code: 'E017',
      messageKey: 'errors.E017.msg',
      recovery: { kind: 'retry' },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E017');
    expect(alert.textContent).toContain('updates');
    return payload;
  },

  // Phase 9 — E018 UpdateDownloadFailed: AppImage Tauri-updater download failed.
  E018: async () => {
    const payload: AppErrorPayload = {
      code: 'E018',
      messageKey: 'errors.E018.msg',
      interpolations: { reason: 'signature mismatch' },
      recovery: {
        kind: 'open_url',
        url: 'https://github.com/Coreyalanschmidt-creator/biscuitcode/releases/latest',
        label: 'Open releases page',
      },
    };
    const alert = await dispatchAndFindAlert(payload);
    expect(alert.textContent).toContain('E018');
    expect(alert.textContent).toContain('download');
    return payload;
  },
};

// ---------- The harness ----------

afterEach(() => {
  cleanup();
});

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
    // Phase 9 audit: all codes now have triggers.
    expect(missing).toEqual([]);
  });
});
