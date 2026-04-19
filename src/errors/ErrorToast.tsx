// src/errors/ErrorToast.tsx
//
// The single component that renders any catalogued error. Every failure
// path in BiscuitCode flows through this component — never through a raw
// `console.error` shown to users, never via a stack trace.
//
// Phase 1 deliverable. Subsequent phases REGISTER new codes in
// src/errors/types.ts but do NOT need to modify this component.
//
// Design contract: see docs/ERROR-CATALOGUE.md and docs/design/AGENT-LOOP.md.

import { useTranslation } from 'react-i18next';
import { type AppErrorPayload, type ErrorRecovery } from './types';

interface ErrorToastProps {
  error: AppErrorPayload;
  /** Called when the user dismisses the toast (X button or auto-timeout). */
  onDismiss: () => void;
}

export function ErrorToast({ error, onDismiss }: ErrorToastProps) {
  const { t } = useTranslation();

  return (
    <div
      role="alert"
      aria-live="polite"
      className="
        flex items-start gap-3 max-w-md p-3 pr-2
        bg-cocoa-600 border border-accent-error/40 rounded-md
        text-cocoa-50 text-sm shadow-lg
      "
      data-error-code={error.code}
    >
      <ErrorIcon />
      <div className="flex-1 min-w-0">
        <div className="font-mono text-xs text-cocoa-200 mb-1">
          {error.code}
        </div>
        <div className="leading-snug">
          {t(error.messageKey, error.interpolations ?? {})}
        </div>
        {error.recovery && (
          <div className="mt-2">
            <RecoveryAction recovery={error.recovery} />
          </div>
        )}
      </div>
      <button
        type="button"
        onClick={onDismiss}
        aria-label={t('common.dismiss')}
        className="text-cocoa-200 hover:text-cocoa-50 transition-colors"
      >
        <DismissIcon />
      </button>
    </div>
  );
}

function RecoveryAction({ recovery }: { recovery: ErrorRecovery }) {
  const { t } = useTranslation();

  switch (recovery.kind) {
    case 'retry':
      return (
        <button
          type="button"
          className="px-2 py-1 text-xs bg-biscuit-500 text-cocoa-900 rounded hover:bg-biscuit-400 transition-colors"
        >
          {recovery.label ?? t('common.retry')}
        </button>
      );

    case 'copy_command':
      return (
        <CopyCommandButton command={recovery.command} label={recovery.label} />
      );

    case 'open_url':
      return (
        <a
          href={recovery.url}
          target="_blank"
          rel="noopener noreferrer"
          className="text-biscuit-400 hover:text-biscuit-300 underline text-xs"
        >
          {recovery.label ?? recovery.url}
        </a>
      );

    case 'deeplink_settings':
      return (
        <button
          type="button"
          className="text-biscuit-400 hover:text-biscuit-300 underline text-xs"
        >
          {recovery.label ?? t('common.openSettings')}
        </button>
      );

    case 'dismiss_only':
      return null;
  }
}

function CopyCommandButton({
  command,
  label,
}: {
  command: string;
  label?: string;
}) {
  const { t } = useTranslation();

  const handleCopy = async () => {
    await navigator.clipboard.writeText(command);
    // TODO: brief "Copied!" feedback inline; deferred to Phase 1 polish.
  };

  return (
    <div className="flex flex-col gap-1">
      <code className="block px-2 py-1 bg-cocoa-800 rounded font-mono text-xs text-biscuit-300 break-all">
        {command}
      </code>
      <button
        type="button"
        onClick={handleCopy}
        className="self-start px-2 py-0.5 text-xs bg-cocoa-500 text-cocoa-50 rounded hover:bg-cocoa-400 transition-colors"
      >
        {label ?? t('common.copyCommand')}
      </button>
    </div>
  );
}

function ErrorIcon() {
  // Inline SVG to avoid pulling in a full icon library for one glyph.
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="text-accent-error flex-shrink-0 mt-0.5"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="10" />
      <line x1="12" y1="8" x2="12" y2="12" />
      <line x1="12" y1="16" x2="12.01" y2="16" />
    </svg>
  );
}

function DismissIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  );
}
