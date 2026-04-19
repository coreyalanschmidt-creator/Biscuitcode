// src/components/OnboardingModal.tsx
//
// Phase 8 deliverable: 3-screen onboarding that runs on first launch.
//
// Screens:
//   1. Welcome — logo + tagline + Next
//   2. Pick models — provider cards; must set at least one or click Skip
//   3. Open a folder — file picker or Continue without a folder
//
// Blocking: renders over WorkspaceGrid with a dark overlay. The only way
// to reach the main UI is to complete all three screens or use Skip in step 2.
//
// State stored in localStorage under 'biscuitcode-onboarded'.

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';

const STORAGE_KEY = 'biscuitcode-onboarded';

export function useOnboardingDone(): boolean {
  return localStorage.getItem(STORAGE_KEY) === '1';
}

function markOnboardingDone() {
  localStorage.setItem(STORAGE_KEY, '1');
}

// ---------------------------------------------------------------------------
// Step 1 — Welcome
// ---------------------------------------------------------------------------

interface WelcomeStepProps {
  onNext: () => void;
}

function WelcomeStep({ onNext }: WelcomeStepProps) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center text-center gap-6">
      {/* Logo */}
      <div
        className="w-20 h-20 rounded-[22%] flex items-center justify-center"
        style={{ background: 'linear-gradient(135deg, #241A13, #0E0906)' }}
      >
        <svg viewBox="0 0 512 512" width="56" height="56" role="img" aria-label="BiscuitCode">
          <polyline
            points="156,156 256,256 156,356"
            fill="none"
            stroke="#E8B04C"
            strokeWidth="48"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          <rect x="276" y="332" width="96" height="24" rx="12" fill="#E8B04C" />
        </svg>
      </div>

      <div>
        <h1 className="text-2xl font-semibold text-cocoa-50 mb-2">
          {t('onboarding.welcome.title')}
        </h1>
        <p className="text-sm text-cocoa-300">{t('onboarding.welcome.subtitle')}</p>
      </div>

      <button
        className="px-8 py-2.5 bg-biscuit-500 hover:bg-biscuit-400 text-cocoa-900 font-semibold rounded text-sm"
        onClick={onNext}
      >
        {t('onboarding.welcome.next')}
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 2 — Pick models
// ---------------------------------------------------------------------------

interface PickModelsStepProps {
  onNext: () => void;
  onSkip: () => void;
}

function PickModelsStep({ onNext, onSkip }: PickModelsStepProps) {
  const { t } = useTranslation();
  const [keyringOk, setKeyringOk] = useState<boolean | null>(null);
  const [anthropicKeySet, setAnthropicKeySet] = useState(false);
  const [showKeyInput, setShowKeyInput] = useState(false);
  const [keyValue, setKeyValue] = useState('');
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>('check_secret_service')
      .then(setKeyringOk)
      .catch(() => setKeyringOk(false));
    invoke<boolean>('anthropic_key_present')
      .then(setAnthropicKeySet)
      .catch(() => setAnthropicKeySet(false));
  }, []);

  const handleSaveKey = async () => {
    if (!keyValue.trim()) return;
    setSaving(true);
    setSaveError(null);
    try {
      await invoke('anthropic_set_key', { key: keyValue.trim() });
      setAnthropicKeySet(true);
      setShowKeyInput(false);
      setKeyValue('');
    } catch {
      setSaveError(t('settings.providers.saveFailed'));
    } finally {
      setSaving(false);
    }
  };

  // Keyring absent — blocking dialog.
  if (keyringOk === false) {
    return (
      <div className="flex flex-col gap-4">
        <h2 className="text-lg font-semibold text-cocoa-50">{t('onboarding.pickModels.title')}</h2>
        <div
          className="rounded border border-accent-error bg-cocoa-800 p-4 text-sm text-cocoa-100"
          role="alert"
          aria-live="assertive"
        >
          <p className="font-semibold text-accent-error mb-2">{t('errors.E001.msg')}</p>
          <code
            className="block bg-cocoa-900 rounded px-3 py-2 text-xs text-biscuit-300 select-all"
            style={{ fontFamily: "'JetBrains Mono', monospace" }}
          >
            sudo apt install gnome-keyring libsecret-1-0 libsecret-tools
          </code>
        </div>
        <button
          className="self-start px-4 py-2 bg-biscuit-500 hover:bg-biscuit-400 text-cocoa-900 font-semibold rounded text-sm"
          onClick={() =>
            invoke<boolean>('check_secret_service')
              .then(setKeyringOk)
              .catch(() => setKeyringOk(false))
          }
        >
          {t('common.retry')}
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <h2 className="text-lg font-semibold text-cocoa-50">{t('onboarding.pickModels.title')}</h2>
      <p className="text-sm text-cocoa-300">{t('onboarding.pickModels.subtitle')}</p>

      {/* Anthropic row */}
      <div className="rounded border border-cocoa-500 bg-cocoa-800 p-4 flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-cocoa-100">
            {t('onboarding.pickModels.anthropic')}
          </span>
          <span
            className={`text-[10px] font-semibold px-2 py-0.5 rounded ${
              anthropicKeySet ? 'bg-accent-ok text-cocoa-900' : 'bg-accent-error text-white'
            }`}
          >
            {anthropicKeySet
              ? t('settings.providers.statusActive')
              : t('settings.providers.statusNoKey')}
          </span>
        </div>

        {showKeyInput ? (
          <div className="flex flex-col gap-2">
            <input
              type="password"
              value={keyValue}
              onChange={(e) => setKeyValue(e.target.value)}
              placeholder="sk-ant-…"
              className="bg-cocoa-700 border border-cocoa-400 rounded px-3 py-1.5 text-sm text-cocoa-50 outline-none focus:border-biscuit-500"
              aria-label={t('settings.providers.anthropicKeyLabel')}
              style={{ fontFamily: "'JetBrains Mono', monospace" }}
            />
            {saveError && <p className="text-xs text-accent-error">{saveError}</p>}
            <div className="flex gap-2">
              <button
                className="px-3 py-1 bg-biscuit-500 hover:bg-biscuit-400 text-cocoa-900 font-semibold rounded text-xs"
                onClick={handleSaveKey}
                disabled={saving}
              >
                {saving ? '…' : t('common.save')}
              </button>
              <button
                className="px-3 py-1 bg-cocoa-600 hover:bg-cocoa-500 text-cocoa-200 rounded text-xs"
                onClick={() => { setShowKeyInput(false); setKeyValue(''); setSaveError(null); }}
              >
                {t('common.cancel')}
              </button>
            </div>
          </div>
        ) : (
          !anthropicKeySet && (
            <button
              className="self-start text-xs text-biscuit-400 hover:underline"
              onClick={() => setShowKeyInput(true)}
            >
              {t('onboarding.pickModels.addKey')}
            </button>
          )
        )}
      </div>

      {/* OpenAI row — placeholder */}
      <div className="rounded border border-cocoa-600 bg-cocoa-800 p-4 flex items-center justify-between opacity-50">
        <span className="text-sm font-medium text-cocoa-300">{t('onboarding.pickModels.openai')}</span>
        <span className="text-[10px] text-cocoa-400">{t('settings.providers.landsInPhase', { phase: '6a' })}</span>
      </div>

      {/* Ollama row — placeholder */}
      <div className="rounded border border-cocoa-600 bg-cocoa-800 p-4 flex items-center justify-between opacity-50">
        <span className="text-sm font-medium text-cocoa-300">{t('onboarding.pickModels.ollama')}</span>
        <span className="text-[10px] text-cocoa-400">{t('settings.providers.landsInPhase', { phase: '6a' })}</span>
      </div>

      <div className="flex justify-between mt-2">
        <button
          className="text-xs text-cocoa-400 hover:underline"
          onClick={onSkip}
        >
          Skip for now
        </button>
        <button
          className={`px-6 py-2 font-semibold rounded text-sm ${
            anthropicKeySet
              ? 'bg-biscuit-500 hover:bg-biscuit-400 text-cocoa-900'
              : 'bg-cocoa-600 text-cocoa-400 cursor-not-allowed'
          }`}
          onClick={anthropicKeySet ? onNext : undefined}
          disabled={!anthropicKeySet}
          title={anthropicKeySet ? undefined : 'Add at least one provider key or skip.'}
        >
          {t('onboarding.welcome.next')}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 3 — Open a folder
// ---------------------------------------------------------------------------

interface OpenFolderStepProps {
  onDone: () => void;
}

function OpenFolderStep({ onDone }: OpenFolderStepProps) {
  const { t } = useTranslation();

  const handleOpenFolder = async () => {
    const selected = await dialogOpen({ directory: true, multiple: false });
    if (selected) {
      await invoke('fs_open_folder', { path: selected });
    }
    onDone();
  };

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h2 className="text-lg font-semibold text-cocoa-50">{t('onboarding.openFolder.title')}</h2>
        <p className="text-sm text-cocoa-300 mt-1">{t('onboarding.openFolder.subtitle')}</p>
      </div>

      <div className="flex flex-col gap-3">
        <button
          className="w-full px-6 py-3 bg-biscuit-500 hover:bg-biscuit-400 text-cocoa-900 font-semibold rounded text-sm"
          onClick={handleOpenFolder}
        >
          {t('onboarding.openFolder.openButton')}
        </button>
        <button
          className="w-full px-6 py-3 bg-cocoa-700 hover:bg-cocoa-600 border border-cocoa-500 text-cocoa-200 rounded text-sm"
          onClick={onDone}
        >
          {t('onboarding.openFolder.skipButton')}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// OnboardingModal — main export
// ---------------------------------------------------------------------------

interface OnboardingModalProps {
  onComplete: () => void;
}

export function OnboardingModal({ onComplete }: OnboardingModalProps) {
  const [step, setStep] = useState<1 | 2 | 3>(1);

  const handleStep2Skip = () => {
    // Skip leaves all badges red but lets the user proceed.
    setStep(3);
  };

  const handleDone = () => {
    markOnboardingDone();
    onComplete();
  };

  return (
    // Full-screen overlay — blocks access to WorkspaceGrid beneath.
    <div
      className="fixed inset-0 z-[200] flex items-center justify-center"
      style={{ backgroundColor: 'rgba(8,5,4,0.85)' }}
      role="dialog"
      aria-modal="true"
      aria-label="BiscuitCode onboarding"
      data-testid="onboarding-modal"
    >
      <div
        className="w-full max-w-md bg-cocoa-700 border border-cocoa-500 rounded-xl shadow-2xl p-8"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Progress dots */}
        <div className="flex justify-center gap-2 mb-8">
          {([1, 2, 3] as const).map((n) => (
            <div
              key={n}
              className={`w-2 h-2 rounded-full ${
                n === step ? 'bg-biscuit-500' : 'bg-cocoa-500'
              }`}
            />
          ))}
        </div>

        {step === 1 && <WelcomeStep onNext={() => setStep(2)} />}
        {step === 2 && <PickModelsStep onNext={() => setStep(3)} onSkip={handleStep2Skip} />}
        {step === 3 && <OpenFolderStep onDone={handleDone} />}
      </div>
    </div>
  );
}
