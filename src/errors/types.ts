// src/errors/types.ts
//
// TypeScript discriminated-union mirror of biscuitcode-core::errors.
// Source of truth for the catalogue is `docs/ERROR-CATALOGUE.md`.
//
// Every error rendered to the user comes through this type. Never display
// a raw stack — render an ErrorToast with one of these payloads.
//
// Phase 1 ships ONE category fully wired (E001 KeyringMissing) as the
// proof-of-concept; subsequent phases add their own variants.

/** Discriminator for the union. Mirrors Rust's enum variant names. */
export type ErrorCode =
  | 'E001' // KeyringMissing                    (Phase 1 — wired)
  | 'E002' // OutsideWorkspace                  (Phase 3)
  | 'E003' // PtyOpenFailed                     (Phase 4)
  | 'E004' // AnthropicAuthInvalid              (Phase 5)
  | 'E005' // AnthropicNetworkError             (Phase 5)
  | 'E006' // AnthropicRateLimited              (Phase 5)
  | 'E007' // GemmaVersionFallback              (Phase 6a)
  | 'E008' // WriteToolDenied                   (Phase 6b)
  | 'E009' // ShellForbiddenPrefix              (Phase 6b)
  | 'E010' // SnapshotFailed                    (Phase 6b)
  | 'E011' // RewindFailed                      (Phase 6b)
  | 'E012' // GitPushFailed                     (Phase 7)
  | 'E013' // LspServerMissing                  (Phase 7)
  | 'E014' // LspProtocolError                  (Phase 7)
  | 'E015' // PreviewRenderFailed               (Phase 7)
  | 'E016' // FontLoadFailed                    (Phase 8)
  | 'E017' // UpdateCheckFailed                 (Phase 9)
  | 'E018'; // UpdateDownloadFailed             (Phase 9)

/** What every error payload carries to the toast. */
export interface BaseErrorPayload {
  code: ErrorCode;
  /** i18n key — render with `t(messageKey, { ...interpolations })`. */
  messageKey: string;
  /** Interpolation values for the i18n template (file paths, error reasons, etc.). */
  interpolations?: Record<string, string | number>;
  /** What the user can DO about it. UI renders as a button or copy-to-clipboard. */
  recovery?: ErrorRecovery;
  /** Optional anchor link in our own docs. */
  docsAnchor?: string;
}

/** A specific recovery action the toast offers the user. */
export type ErrorRecovery =
  | { kind: 'retry'; label?: string }
  | { kind: 'copy_command'; command: string; label?: string }
  | { kind: 'open_url'; url: string; label?: string }
  | { kind: 'deeplink_settings'; section: string; label?: string }
  | { kind: 'dismiss_only' };

// ---------- Per-code payload shapes ----------
// Each variant narrows the BaseErrorPayload by tying `code` to a literal.
// Add a new variant when claiming a code; remove only with extreme caution
// (codes are NEVER reused after they ship).

export interface E001_KeyringMissing extends BaseErrorPayload {
  code: 'E001';
  messageKey: 'errors.E001.msg';
  recovery: {
    kind: 'copy_command';
    command: 'sudo apt install gnome-keyring libsecret-1-0 libsecret-tools';
    label: 'Copy install command';
  };
}

export interface E002_OutsideWorkspace extends BaseErrorPayload {
  code: 'E002';
  messageKey: 'errors.E002.msg';
  interpolations: { path: string };
  recovery: { kind: 'dismiss_only' };
}

export interface E007_GemmaVersionFallback extends BaseErrorPayload {
  code: 'E007';
  messageKey: 'errors.E007.msg';
  recovery: {
    kind: 'copy_command';
    command: 'curl -fsSL https://ollama.com/install.sh | sh';
    label: 'Copy upgrade command';
  };
}

// Phase 4 — E003 registered.
export interface E003_PtyOpenFailed extends BaseErrorPayload {
  code: 'E003';
  messageKey: 'errors.E003.msg';
  interpolations: { reason: string };
  recovery: { kind: 'dismiss_only' };
}

// (Variants for E004–E006, E008–E018 are added by their owning phases.
//  Use the same pattern: extend BaseErrorPayload with a literal `code`
//  and the specific i18n key + recovery type.)

/** The discriminated union the toast accepts. */
export type AppErrorPayload =
  | E001_KeyringMissing
  | E002_OutsideWorkspace
  | E003_PtyOpenFailed
  | E007_GemmaVersionFallback;
// Add more here as phases register their codes.

/** Type guard: is this a known catalogued error? */
export function isAppError(x: unknown): x is AppErrorPayload {
  if (typeof x !== 'object' || x === null) return false;
  const obj = x as Record<string, unknown>;
  return typeof obj.code === 'string' && typeof obj.messageKey === 'string';
}
