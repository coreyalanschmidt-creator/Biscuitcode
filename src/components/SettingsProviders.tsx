// src/components/SettingsProviders.tsx
//
// Phase 5 deliverable: provider status badges + API key entry.
//
// Shows Anthropic provider with status badge:
//   green  = key present (validated via keyring lookup)
//   yellow = key present but test-connection not run yet (Phase 5: same as green)
//   red    = no key / keyring unavailable
//
// OpenAI and Ollama status badges are placeholders until Phase 6a.

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';

type Badge = 'green' | 'yellow' | 'red';

interface ProviderRowProps {
  name: string;
  badge: Badge;
  onAddKey?: () => void;
  onDeleteKey?: () => void;
  hasKey: boolean;
  phase?: string;
}

function ProviderRow({ name, badge, onAddKey, onDeleteKey, hasKey, phase }: ProviderRowProps) {
  const { t } = useTranslation();
  const badgeColor =
    badge === 'green'
      ? 'bg-accent-ok text-cocoa-900'
      : badge === 'yellow'
      ? 'bg-biscuit-500 text-cocoa-900'
      : 'bg-accent-error text-white';
  const badgeLabel =
    badge === 'green' ? t('settings.providers.statusActive') :
    badge === 'yellow' ? t('settings.providers.statusUntested') :
    t('settings.providers.statusNoKey');

  return (
    <div className="flex items-center gap-3 py-3 border-b border-cocoa-500 last:border-0">
      <div className="flex-1">
        <div className="text-sm font-medium text-cocoa-100">{name}</div>
        {phase && (
          <div className="text-xs text-cocoa-400 mt-0.5">{phase}</div>
        )}
      </div>
      <span className={`text-[10px] font-semibold px-2 py-0.5 rounded ${badgeColor}`}>
        {badgeLabel}
      </span>
      {hasKey ? (
        <button
          className="text-xs text-accent-error hover:underline"
          onClick={onDeleteKey}
          aria-label={t('settings.providers.deleteKey', { name })}
        >
          {t('settings.providers.removeKey')}
        </button>
      ) : (
        <button
          className="text-xs text-biscuit-400 hover:underline"
          onClick={onAddKey}
          aria-label={t('settings.providers.addKeyLabel', { name })}
          disabled={!onAddKey}
        >
          {t('settings.providers.addKey')}
        </button>
      )}
    </div>
  );
}

export function SettingsProviders() {
  const { t } = useTranslation();
  const [anthropicHasKey, setAnthropicHasKey] = useState(false);
  const [pendingKey, setPendingKey] = useState('');
  const [showInput, setShowInput] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [secretServiceAvail, setSecretServiceAvail] = useState<boolean | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const avail = await invoke<boolean>('check_secret_service');
        setSecretServiceAvail(avail);
        if (avail) {
          const present = await invoke<boolean>('anthropic_key_present');
          setAnthropicHasKey(present);
        }
      } catch {
        setSecretServiceAvail(false);
      }
    })();
  }, []);

  const handleAddKey = async () => {
    if (!pendingKey.trim()) return;
    setSaving(true);
    setError(null);
    try {
      await invoke('anthropic_set_key', { key: pendingKey.trim() });
      setAnthropicHasKey(true);
      setShowInput(false);
      setPendingKey('');
    } catch (e) {
      const msg = typeof e === 'string' ? e : t('settings.providers.saveFailed');
      setError(msg === 'E001' ? t('errors.E001.msg') : msg);
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteKey = async () => {
    try {
      await invoke('anthropic_delete_key');
      setAnthropicHasKey(false);
    } catch {
      // Silently ignore delete failure in Phase 5.
    }
  };

  const anthropicBadge: Badge =
    secretServiceAvail === false
      ? 'red'
      : anthropicHasKey
      ? 'green'
      : 'red';

  return (
    <div className="p-4">
      <h2 className="text-base font-semibold text-cocoa-100 mb-4">
        {t('settings.providers.title')}
      </h2>

      {secretServiceAvail === false && (
        <div className="mb-4 rounded border border-accent-error/40 bg-cocoa-600 px-3 py-2 text-xs text-accent-error">
          {t('errors.E001.msg')}
          <br />
          <code className="mt-1 block text-cocoa-200">
            sudo apt install gnome-keyring libsecret-1-0 libsecret-tools
          </code>
        </div>
      )}

      <div className="rounded border border-cocoa-500 px-4 bg-cocoa-600">
        <ProviderRow
          name="Anthropic (Claude)"
          badge={anthropicBadge}
          hasKey={anthropicHasKey}
          onAddKey={secretServiceAvail ? () => setShowInput(true) : undefined}
          onDeleteKey={handleDeleteKey}
        />
        <ProviderRow
          name="OpenAI"
          badge="red"
          hasKey={false}
          phase={t('settings.providers.landsInPhase', { phase: '6a' })}
        />
        <ProviderRow
          name="Ollama (local)"
          badge="red"
          hasKey={false}
          phase={t('settings.providers.landsInPhase', { phase: '6a' })}
        />
      </div>

      {showInput && (
        <div className="mt-4">
          <label className="block text-xs text-cocoa-300 mb-1">
            {t('settings.providers.anthropicKeyLabel')}
          </label>
          <div className="flex gap-2">
            <input
              type="password"
              value={pendingKey}
              onChange={e => setPendingKey(e.target.value)}
              onKeyDown={e => { if (e.key === 'Enter') handleAddKey(); }}
              placeholder="sk-ant-…"
              className="flex-1 rounded bg-cocoa-700 border border-cocoa-400 text-sm text-cocoa-100 px-2 py-1 focus:outline-none focus:border-biscuit-500"
              autoFocus
              aria-label={t('settings.providers.anthropicKeyLabel')}
            />
            <button
              onClick={handleAddKey}
              disabled={saving || !pendingKey.trim()}
              className="px-3 py-1 rounded bg-biscuit-500 text-cocoa-900 text-xs font-semibold disabled:opacity-40"
            >
              {saving ? t('common.save') + '…' : t('common.save')}
            </button>
            <button
              onClick={() => { setShowInput(false); setPendingKey(''); setError(null); }}
              className="px-3 py-1 rounded border border-cocoa-400 text-xs text-cocoa-300"
            >
              {t('common.cancel')}
            </button>
          </div>
          {error && (
            <p className="mt-1 text-xs text-accent-error">{error}</p>
          )}
        </div>
      )}
    </div>
  );
}
