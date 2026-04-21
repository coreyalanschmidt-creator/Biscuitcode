// tests/a11y.spec.tsx
//
// Axe-core accessibility gate.
//
// Covers:
//   1. ChatPanel — no axe violations at moderate+ severity.
//   2. OnboardingModal — no axe violations + Escape is swallowed (focus trap).
//   3. ErrorToast E001 (copy_command recovery) — no violations.
//   4. ErrorToast E009 (dismiss_only recovery) — no violations.
//   5. ErrorToast E016 (font-canary, dismiss_only recovery) — no violations.
//
// Severity gate: critical, serious, moderate (minor/cosmetic suppressed).
//
// vitest-axe 0.1.0 ships the matcher but has a known compatibility gap with
// vitest 3's expect internals (the jest-style matcherHint API diverged). We
// use axe-core directly and assert with a plain vitest expect so the gate
// stays simple and dependency-free. The severity filter is explicit below.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, cleanup } from '@testing-library/react';
import axe from 'axe-core';
import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';

// ---------------------------------------------------------------------------
// Inline helper — filters axe results to moderate+ severity only.
// Returns the list of violations that count as failures.
// ---------------------------------------------------------------------------

const IMPACT_LEVELS: axe.ImpactValue[] = ['moderate', 'serious', 'critical'];

function moderatePlusViolations(results: axe.AxeResults): axe.Result[] {
  return results.violations.filter(
    (v) => v.impact !== null && IMPACT_LEVELS.includes(v.impact as axe.ImpactValue),
  );
}

// Run axe on a container element, return filtered violations.
async function runAxe(container: HTMLElement): Promise<axe.Result[]> {
  const results = await axe.run(container);
  return moderatePlusViolations(results);
}

// Assertion helper: prints violation ids and help text on failure.
function assertNoViolations(violations: axe.Result[]): void {
  if (violations.length === 0) return;
  const summary = violations
    .map((v) => `  [${v.impact}] ${v.id}: ${v.help}`)
    .join('\n');
  throw new Error(`axe found ${violations.length} violation(s) at moderate+ severity:\n${summary}`);
}

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock('@tauri-apps/api/core', () => ({
  // Return appropriate defaults per command name so components don't crash.
  invoke: vi.fn((cmd: string) => {
    if (cmd === 'anthropic_list_models') return Promise.resolve([]);
    if (cmd === 'anthropic_key_present') return Promise.resolve(false);
    if (cmd === 'check_secret_service') return Promise.resolve(false);
    return Promise.resolve(false);
  }),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => undefined),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn().mockResolvedValue(null),
}));

// react-virtuoso uses ResizeObserver + scroll layout absent in jsdom.
// Replace with a flat renderer so axe can inspect all rendered items.
vi.mock('react-virtuoso', () => ({
  Virtuoso: ({
    data,
    itemContent,
  }: {
    data: unknown[];
    itemContent: (index: number, item: unknown) => React.ReactNode;
  }) => (
    <div>
      {(data ?? []).map((item, index) => (
        <div key={index}>{itemContent(index, item)}</div>
      ))}
    </div>
  ),
}));

// ---------------------------------------------------------------------------
// i18n bootstrap
// ---------------------------------------------------------------------------

await i18next.use(initReactI18next).init({
  lng: 'en',
  resources: {
    en: {
      translation: {
        common: {
          appName: 'BiscuitCode',
          tagline: 'An AI coding environment, served warm.',
          ok: 'OK',
          cancel: 'Cancel',
          retry: 'Retry',
          dismiss: 'Dismiss',
          close: 'Close',
          save: 'Save',
          open: 'Open',
          copyCommand: 'Copy command',
          openSettings: 'Open Settings',
        },
        panels: {
          chatPanel: 'Chat Panel',
          chats: 'Chats',
          agentActivity: 'Agent Activity',
        },
        chat: {
          you: 'You',
          assistant: 'Assistant',
          modelPickerLabel: 'Select model',
          newChat: 'New chat',
          emptyHint: 'Type a message.',
          inputLabel: 'Chat message',
          inputPlaceholder: 'Message…',
          shortcutHint: 'shortcuts',
          sendButton: 'Send',
          sending: 'Sending…',
          noKeyBanner: 'No key set.',
          errorNoKey: 'No key.',
          errorStream: 'Stream error.',
          errorSend: 'Send error.',
          agentMode: 'Agent',
          agentModeLabel: 'Agent mode',
          agentModeTitle: 'Agent mode tooltip',
          mentionPickerLabel: 'File mention picker',
          mentionNoResults: 'No matching files',
          apply: 'Apply',
          run: 'Run',
          applyCode: 'Apply code to editor',
          runCode: 'Run code in terminal',
          rewind: 'Rewind',
          rewindLabel: 'Rewind conversation',
          rewindError: 'Rewind failed.',
        },
        agent: {
          emptyHint: 'No tool calls yet.',
          running: 'running…',
          runningLabel: 'Agent running…',
          pauseLabel: 'Pause agent',
          doneLabel: 'Agent done',
          args: 'Arguments',
          result: 'Result',
          status: { running: 'Running', ok: 'Done', error: 'Error' },
        },
        onboarding: {
          welcome: {
            title: 'Welcome to BiscuitCode',
            subtitle: "Let's get you coding.",
            next: 'Next',
          },
          pickModels: {
            title: 'Pick your AI models',
            subtitle: 'Add at least one provider.',
            anthropic: 'Anthropic (Claude)',
            openai: 'OpenAI',
            ollama: 'Ollama',
            addKey: 'Add API key',
            installOllama: 'Install Ollama',
          },
          openFolder: {
            title: 'Open a folder',
            subtitle: 'Open or skip.',
            openButton: 'Open folder…',
            skipButton: 'Continue without a folder',
          },
        },
        settings: {
          providers: {
            title: 'Models & Providers',
            statusActive: 'Active',
            statusUntested: 'Untested',
            statusNoKey: 'No key',
            addKey: 'Add key',
            addKeyLabel: 'Add API key for {{name}}',
            removeKey: 'Remove key',
            deleteKey: 'Remove API key for {{name}}',
            saveFailed: 'Failed to save key.',
            anthropicKeyLabel: 'Anthropic API key',
            landsInPhase: 'Lands in Phase {{phase}}',
          },
        },
        errors: {
          E001: {
            msg: 'BiscuitCode needs a system keyring to store API keys safely.',
          },
          E009: {
            msg: 'BiscuitCode blocked the shell command `{{command}}` for safety.',
          },
          E016: {
            msg: "BiscuitCode's bundled fonts didn't load.",
          },
        },
        mentions: {
          noTerminals: 'no terminals open',
          noProblems: 'no diagnostics',
        },
      },
    },
  },
  interpolation: { escapeValue: false },
});

// ---------------------------------------------------------------------------
// Imports (after mocks and i18n so module resolution picks up mocks)
// ---------------------------------------------------------------------------

import { ChatPanel } from '../src/components/ChatPanel';
import { OnboardingModal } from '../src/components/OnboardingModal';
import { ErrorToast } from '../src/errors/ErrorToast';
import type {
  E001_KeyringMissing,
  E009_ShellForbiddenPrefix,
  E016_FontLoadFailed,
} from '../src/errors/types';

// ---------------------------------------------------------------------------
// Teardown
// ---------------------------------------------------------------------------

afterEach(() => {
  cleanup();
});

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe('a11y — ChatPanel', () => {
  it('has no moderate+ axe violations', async () => {
    const { container } = render(<ChatPanel />);
    const violations = await runAxe(container);
    assertNoViolations(violations);
  });
});

describe('a11y — OnboardingModal', () => {
  it('has no moderate+ axe violations', async () => {
    const { container } = render(
      <OnboardingModal onComplete={vi.fn()} />,
    );
    const violations = await runAxe(container);
    assertNoViolations(violations);
  });

  it('swallows Escape — modal stays mounted and onComplete is not called', () => {
    const onComplete = vi.fn();
    const { getByTestId } = render(<OnboardingModal onComplete={onComplete} />);

    // Modal must be present.
    expect(getByTestId('onboarding-modal')).toBeTruthy();

    // Dispatch Escape on document — the focus-trap handler calls e.preventDefault().
    const escapeEvent = new KeyboardEvent('keydown', {
      key: 'Escape',
      bubbles: true,
      cancelable: true,
    });
    document.dispatchEvent(escapeEvent);

    // Modal must still be mounted: Escape must not complete/dismiss it.
    expect(getByTestId('onboarding-modal')).toBeTruthy();
    expect(onComplete).not.toHaveBeenCalled();
  });
});

describe('a11y — ErrorToast', () => {
  const dismiss = vi.fn();

  const e001: E001_KeyringMissing = {
    code: 'E001',
    messageKey: 'errors.E001.msg',
    recovery: {
      kind: 'copy_command',
      command: 'sudo apt install gnome-keyring libsecret-1-0 libsecret-tools',
      label: 'Copy install command',
    },
  };

  const e009: E009_ShellForbiddenPrefix = {
    code: 'E009',
    messageKey: 'errors.E009.msg',
    interpolations: { command: 'rm -rf /' },
    recovery: { kind: 'dismiss_only' },
  };

  const e016: E016_FontLoadFailed = {
    code: 'E016',
    messageKey: 'errors.E016.msg',
    recovery: { kind: 'dismiss_only' },
  };

  it('E001 (copy_command recovery) has no moderate+ violations', async () => {
    const { container } = render(
      <ErrorToast error={e001} onDismiss={dismiss} />,
    );
    const violations = await runAxe(container);
    assertNoViolations(violations);
  });

  it('E009 (dismiss_only recovery) has no moderate+ violations', async () => {
    const { container } = render(
      <ErrorToast error={e009} onDismiss={dismiss} />,
    );
    const violations = await runAxe(container);
    assertNoViolations(violations);
  });

  it('E016 (font-canary, dismiss_only) has no moderate+ violations', async () => {
    const { container } = render(
      <ErrorToast error={e016} onDismiss={dismiss} />,
    );
    const violations = await runAxe(container);
    assertNoViolations(violations);
  });
});
