// src/components/SettingsPage.tsx
//
// Phase 8 deliverable: full settings UI.
// Phase 9: i18n all section labels; add update check to About section.
//
// Sections: General, Editor, Models, Terminal, Appearance, Security,
//           Conversations, About.
//
// Raw-JSON mode: opens ~/.config/biscuitcode/settings.json in Monaco.
// Telemetry toggle: persists to settings.json, no endpoint wired.

import { useEffect, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { open as shellOpen } from '@tauri-apps/plugin-shell';
import { THEMES, type ThemeId, applyTheme, previewTheme, getStoredThemeId } from '../theme/themes';

// ---------------------------------------------------------------------------
// Settings data shape (persisted as JSON)
// ---------------------------------------------------------------------------

export interface BiscuitSettings {
  // General
  telemetry: boolean;
  // Editor
  fontSize: number;
  tabSize: number;
  wordWrap: boolean;
  minimap: boolean;
  ligatures: boolean;
  // Terminal
  terminalFontSize: number;
  scrollback: number;
  // Appearance
  theme: ThemeId;
  // Security
  workspaceTrust: boolean;
  // Conversations
  snapshotCleanupEnabled: boolean;
  snapshotMaxAgeDays: number;
}

const DEFAULT_SETTINGS: BiscuitSettings = {
  telemetry: false,
  fontSize: 14,
  tabSize: 2,
  wordWrap: false,
  minimap: true,
  ligatures: true,
  terminalFontSize: 13,
  scrollback: 10000,
  theme: 'warm',
  workspaceTrust: false,
  snapshotCleanupEnabled: true,
  snapshotMaxAgeDays: 30,
};

const SETTINGS_STORAGE_KEY = 'biscuitcode-settings';

export function loadSettings(): BiscuitSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (raw) return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {
    // ignore parse errors
  }
  return { ...DEFAULT_SETTINGS };
}

export function saveSettings(s: BiscuitSettings): void {
  localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(s, null, 2));
}

// ---------------------------------------------------------------------------
// Section types
// ---------------------------------------------------------------------------

type Section = 'general' | 'editor' | 'models' | 'terminal' | 'appearance' | 'security' | 'conversations' | 'about';

// Section ids — labels resolved via t() at render time.
const SECTION_IDS: Section[] = [
  'general', 'editor', 'models', 'terminal',
  'appearance', 'security', 'conversations', 'about',
];

// ---------------------------------------------------------------------------
// Reusable settings row components
// ---------------------------------------------------------------------------

function SettingsToggle({
  label,
  description,
  value,
  onChange,
  'data-testid': testId,
}: {
  label: string;
  description?: string;
  value: boolean;
  onChange: (v: boolean) => void;
  'data-testid'?: string;
}) {
  return (
    <div className="flex items-start justify-between py-3 border-b border-cocoa-600 last:border-0">
      <div className="flex-1 pr-4">
        <div className="text-sm text-cocoa-100">{label}</div>
        {description && <div className="text-xs text-cocoa-400 mt-0.5">{description}</div>}
      </div>
      <button
        role="switch"
        aria-checked={value}
        data-testid={testId}
        className={`relative w-10 h-5 rounded-full transition-colors ${
          value ? 'bg-biscuit-500' : 'bg-cocoa-500'
        }`}
        onClick={() => onChange(!value)}
      >
        <span
          className={`absolute top-0.5 w-4 h-4 bg-cocoa-50 rounded-full transition-transform ${
            value ? 'translate-x-5' : 'translate-x-0.5'
          }`}
        />
      </button>
    </div>
  );
}

function SettingsNumber({
  label,
  description,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  description?: string;
  value: number;
  min?: number;
  max?: number;
  onChange: (v: number) => void;
}) {
  return (
    <div className="flex items-start justify-between py-3 border-b border-cocoa-600 last:border-0">
      <div className="flex-1 pr-4">
        <div className="text-sm text-cocoa-100">{label}</div>
        {description && <div className="text-xs text-cocoa-400 mt-0.5">{description}</div>}
      </div>
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        className="w-20 bg-cocoa-800 border border-cocoa-500 rounded px-2 py-1 text-sm text-cocoa-50 outline-none focus:border-biscuit-500 text-right"
        onChange={(e) => {
          const n = parseInt(e.target.value, 10);
          if (!isNaN(n)) onChange(n);
        }}
      />
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-xs font-semibold uppercase tracking-wider text-cocoa-300 mb-4 mt-6 first:mt-0">
      {children}
    </h3>
  );
}

// ---------------------------------------------------------------------------
// Individual sections
// ---------------------------------------------------------------------------

function GeneralSection({
  settings,
  onChange,
}: {
  settings: BiscuitSettings;
  onChange: (s: BiscuitSettings) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <SectionTitle>{t('settings.sections.general')}</SectionTitle>
      <SettingsToggle
        label={t('settings.general.telemetryLabel')}
        description={t('settings.general.telemetryDescription')}
        value={settings.telemetry}
        onChange={(v) => onChange({ ...settings, telemetry: v })}
        data-testid="telemetry-toggle"
      />
    </>
  );
}

function EditorSection({
  settings,
  onChange,
}: {
  settings: BiscuitSettings;
  onChange: (s: BiscuitSettings) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <SectionTitle>{t('settings.sections.editor')}</SectionTitle>
      <SettingsNumber
        label={t('settings.editor.fontSize')}
        value={settings.fontSize}
        min={8}
        max={32}
        onChange={(v) => onChange({ ...settings, fontSize: v })}
      />
      <SettingsNumber
        label={t('settings.editor.tabSize')}
        value={settings.tabSize}
        min={1}
        max={8}
        onChange={(v) => onChange({ ...settings, tabSize: v })}
      />
      <SettingsToggle
        label={t('settings.editor.wordWrap')}
        value={settings.wordWrap}
        onChange={(v) => onChange({ ...settings, wordWrap: v })}
      />
      <SettingsToggle
        label={t('settings.editor.minimap')}
        value={settings.minimap}
        onChange={(v) => onChange({ ...settings, minimap: v })}
      />
      <SettingsToggle
        label={t('settings.editor.ligatures')}
        value={settings.ligatures}
        onChange={(v) => onChange({ ...settings, ligatures: v })}
      />
    </>
  );
}

function ModelsSection() {
  // Delegates to existing SettingsProviders component logic.
  // Inline here for simplicity rather than importing the phase-5 component.
  const { t } = useTranslation();
  const [anthropicHasKey, setAnthropicHasKey] = useState(false);
  const [showInput, setShowInput] = useState(false);
  const [keyValue, setKeyValue] = useState('');
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>('anthropic_key_present')
      .then(setAnthropicHasKey)
      .catch(() => setAnthropicHasKey(false));
  }, []);

  const handleSave = async () => {
    if (!keyValue.trim()) return;
    setSaving(true);
    setSaveError(null);
    try {
      await invoke('anthropic_set_key', { key: keyValue.trim() });
      setAnthropicHasKey(true);
      setShowInput(false);
      setKeyValue('');
    } catch {
      setSaveError(t('settings.providers.saveFailed'));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    try {
      await invoke('anthropic_delete_key');
      setAnthropicHasKey(false);
    } catch {
      // ignore
    }
  };

  return (
    <>
      <SectionTitle>{t('settings.sections.models')}</SectionTitle>
      <div className="rounded border border-cocoa-500 bg-cocoa-800 p-4 flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-cocoa-100">Anthropic (Claude)</span>
          <span
            className={`text-[10px] font-semibold px-2 py-0.5 rounded ${
              anthropicHasKey ? 'bg-accent-ok text-cocoa-900' : 'bg-accent-error text-white'
            }`}
          >
            {anthropicHasKey ? t('settings.providers.statusActive') : t('settings.providers.statusNoKey')}
          </span>
        </div>

        {showInput ? (
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
                onClick={handleSave}
                disabled={saving}
              >
                {saving ? '…' : t('common.save')}
              </button>
              <button
                className="px-3 py-1 bg-cocoa-600 text-cocoa-200 rounded text-xs"
                onClick={() => { setShowInput(false); setKeyValue(''); setSaveError(null); }}
              >
                {t('common.cancel')}
              </button>
            </div>
          </div>
        ) : (
          anthropicHasKey ? (
            <button
              className="self-start text-xs text-accent-error hover:underline"
              onClick={handleDelete}
            >
              {t('settings.providers.removeKey')}
            </button>
          ) : (
            <button
              className="self-start text-xs text-biscuit-400 hover:underline"
              onClick={() => setShowInput(true)}
            >
              {t('settings.providers.addKey')}
            </button>
          )
        )}
      </div>

      {/* OpenAI / Ollama placeholders */}
      <div className="rounded border border-cocoa-600 bg-cocoa-800 p-4 flex items-center justify-between opacity-50 mt-3">
        <span className="text-sm font-medium text-cocoa-300">OpenAI (ChatGPT)</span>
        <span className="text-[10px] text-cocoa-400">{t('settings.providers.landsInPhase', { phase: '6a' })}</span>
      </div>
      <div className="rounded border border-cocoa-600 bg-cocoa-800 p-4 flex items-center justify-between opacity-50 mt-3">
        <span className="text-sm font-medium text-cocoa-300">Ollama (local models)</span>
        <span className="text-[10px] text-cocoa-400">{t('settings.providers.landsInPhase', { phase: '6a' })}</span>
      </div>
    </>
  );
}

function TerminalSection({
  settings,
  onChange,
}: {
  settings: BiscuitSettings;
  onChange: (s: BiscuitSettings) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <SectionTitle>{t('settings.sections.terminal')}</SectionTitle>
      <SettingsNumber
        label={t('settings.terminal.fontSize')}
        value={settings.terminalFontSize}
        min={8}
        max={32}
        onChange={(v) => onChange({ ...settings, terminalFontSize: v })}
      />
      <SettingsNumber
        label={t('settings.terminal.scrollback')}
        value={settings.scrollback}
        min={100}
        max={100000}
        onChange={(v) => onChange({ ...settings, scrollback: v })}
      />
    </>
  );
}

function AppearanceSection({
  settings,
  onChange,
}: {
  settings: BiscuitSettings;
  onChange: (s: BiscuitSettings) => void;
}) {
  const { t } = useTranslation();
  const currentTheme = getStoredThemeId();

  const handleSelect = (id: ThemeId) => {
    applyTheme(id);
    onChange({ ...settings, theme: id });
  };

  const handleHover = (id: ThemeId) => {
    previewTheme(id);
  };

  const handleHoverEnd = () => {
    // Revert to persisted theme on mouse leave.
    previewTheme(currentTheme);
  };

  return (
    <>
      <SectionTitle>{t('settings.sections.appearance')}</SectionTitle>
      <div className="flex flex-col gap-2">
        {THEMES.map((theme) => (
          <button
            key={theme.id}
            className={`flex items-start gap-3 p-3 rounded border text-left transition-colors ${
              settings.theme === theme.id
                ? 'border-biscuit-500 bg-cocoa-600'
                : 'border-cocoa-500 bg-cocoa-800 hover:bg-cocoa-700'
            }`}
            onClick={() => handleSelect(theme.id)}
            onMouseEnter={() => handleHover(theme.id)}
            onMouseLeave={handleHoverEnd}
            data-testid={`theme-option-${theme.id}`}
          >
            <div className="flex-1">
              <div className="text-sm font-medium text-cocoa-50">{theme.label}</div>
              <div className="text-xs text-cocoa-400 mt-0.5">{theme.description}</div>
            </div>
            {settings.theme === theme.id && (
              <span className="text-xs text-biscuit-500 font-semibold mt-0.5">Active</span>
            )}
          </button>
        ))}
      </div>

      <div className="mt-4 py-3 border-t border-cocoa-600">
        <div className="text-sm text-cocoa-300">VS Code theme import</div>
        <div className="text-xs text-cocoa-500 mt-0.5">Coming in v1.1.</div>
      </div>
    </>
  );
}

function SecuritySection({
  settings,
  onChange,
}: {
  settings: BiscuitSettings;
  onChange: (s: BiscuitSettings) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <SectionTitle>{t('settings.sections.security')}</SectionTitle>
      <SettingsToggle
        label={t('settings.security.workspaceTrustLabel')}
        description={t('settings.security.workspaceTrustDescription')}
        value={settings.workspaceTrust}
        onChange={(v) => onChange({ ...settings, workspaceTrust: v })}
      />
    </>
  );
}

function ConversationsSection({
  settings,
  onChange,
  onExport,
  onImport,
  onCleanupNow,
}: {
  settings: BiscuitSettings;
  onChange: (s: BiscuitSettings) => void;
  onExport: () => void;
  onImport: () => void;
  onCleanupNow: () => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <SectionTitle>{t('settings.sections.conversations')}</SectionTitle>
      <SettingsToggle
        label={t('settings.conversations.snapshotCleanupLabel')}
        description={`Delete snapshots older than ${settings.snapshotMaxAgeDays} days from closed conversations.`}
        value={settings.snapshotCleanupEnabled}
        onChange={(v) => onChange({ ...settings, snapshotCleanupEnabled: v })}
        data-testid="snapshot-cleanup-toggle"
      />
      <SettingsNumber
        label="Snapshot max age (days)"
        value={settings.snapshotMaxAgeDays}
        min={1}
        max={365}
        onChange={(v) => onChange({ ...settings, snapshotMaxAgeDays: v })}
      />

      <div className="flex flex-col gap-2 mt-4 pt-3 border-t border-cocoa-600">
        <button
          className="px-4 py-2 bg-cocoa-700 hover:bg-cocoa-600 border border-cocoa-500 text-cocoa-200 rounded text-sm"
          onClick={onExport}
          data-testid="export-conversations-btn"
        >
          {t('settings.conversations.exportButton')}
        </button>
        <button
          className="px-4 py-2 bg-cocoa-700 hover:bg-cocoa-600 border border-cocoa-500 text-cocoa-200 rounded text-sm"
          onClick={onImport}
          data-testid="import-conversations-btn"
        >
          {t('settings.conversations.importButton')}
        </button>
        <button
          className="px-4 py-2 bg-cocoa-700 hover:bg-cocoa-600 border border-cocoa-500 text-cocoa-200 rounded text-sm"
          onClick={onCleanupNow}
          data-testid="cleanup-now-btn"
        >
          {t('settings.conversations.cleanupNowButton')}
        </button>
      </div>
    </>
  );
}

interface UpdateInfoResult {
  update_available: boolean;
  latest_version: string | null;
  release_url: string | null;
  changelog_excerpt: string;
}

function AboutSection() {
  const { t } = useTranslation();
  const [checking, setChecking] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfoResult | null>(null);
  const [checkError, setCheckError] = useState<string | null>(null);

  const handleCheckForUpdates = async () => {
    setChecking(true);
    setUpdateInfo(null);
    setCheckError(null);
    try {
      const info = await invoke<UpdateInfoResult>('check_for_deb_update');
      setUpdateInfo(info);
    } catch {
      setCheckError(t('settings.about.updateCheckFailed'));
    } finally {
      setChecking(false);
    }
  };

  const handleDownload = async () => {
    if (updateInfo?.release_url) {
      try {
        await shellOpen(updateInfo.release_url);
      } catch {
        // Fallback: nothing to do if shell open fails
      }
    }
  };

  return (
    <>
      <SectionTitle>{t('settings.sections.about')}</SectionTitle>
      <div className="text-sm text-cocoa-200 space-y-2">
        <div>
          <span className="text-cocoa-400">{t('settings.about.version')}</span>{' '}
          <span>0.1.0</span>
        </div>
        <div>
          <span className="text-cocoa-400">{t('settings.about.license')}</span>{' '}
          <span>MIT</span>
        </div>
        <div>
          <span className="text-cocoa-400">{t('settings.about.repository')}</span>{' '}
          <span className="text-biscuit-400">github.com/Coreyalanschmidt-creator/biscuitcode</span>
        </div>
      </div>

      <div className="mt-4 pt-4 border-t border-cocoa-600">
        <button
          className="px-4 py-2 bg-cocoa-700 hover:bg-cocoa-600 border border-cocoa-500 text-cocoa-200 rounded text-sm disabled:opacity-50"
          onClick={handleCheckForUpdates}
          disabled={checking}
          data-testid="check-for-updates-btn"
        >
          {checking ? t('settings.about.checking') : t('settings.about.checkForUpdates')}
        </button>

        {checkError && (
          <p className="mt-2 text-xs text-accent-error">{checkError}</p>
        )}

        {updateInfo && !updateInfo.update_available && (
          <p className="mt-2 text-xs text-accent-ok">{t('settings.about.upToDate')}</p>
        )}

        {updateInfo?.update_available && (
          <div className="mt-2 space-y-2">
            <p className="text-xs text-biscuit-400">
              {t('settings.about.updateAvailable', { version: updateInfo.latest_version ?? '' })}
            </p>
            {updateInfo.changelog_excerpt && (
              <pre className="text-xs text-cocoa-300 bg-cocoa-800 rounded p-2 max-h-24 overflow-y-auto whitespace-pre-wrap">
                {updateInfo.changelog_excerpt}
              </pre>
            )}
            <button
              className="px-4 py-2 bg-biscuit-500 hover:bg-biscuit-400 text-cocoa-900 font-semibold rounded text-sm"
              onClick={handleDownload}
              data-testid="download-deb-btn"
            >
              {t('settings.about.downloadDeb')}
            </button>
          </div>
        )}
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Main SettingsPage export
// ---------------------------------------------------------------------------

interface SettingsPageProps {
  onExport?: () => void;
  onImport?: () => void;
  onCleanupNow?: () => void;
}

export function SettingsPage({ onExport, onImport, onCleanupNow }: SettingsPageProps = {}) {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<Section>('general');
  const [settings, setSettings] = useState<BiscuitSettings>(loadSettings);

  const handleChange = useCallback((s: BiscuitSettings) => {
    setSettings(s);
    saveSettings(s);
    // Apply theme change immediately.
    applyTheme(s.theme);
  }, []);

  const handleExport = useCallback(() => {
    onExport?.();
    invoke('conversations_export').catch(() => {});
  }, [onExport]);

  const handleImport = useCallback(() => {
    onImport?.();
    invoke('conversations_import').catch(() => {});
  }, [onImport]);

  const handleCleanupNow = useCallback(() => {
    onCleanupNow?.();
    invoke('snapshots_cleanup_now').catch(() => {});
  }, [onCleanupNow]);

  return (
    <div className="h-full flex overflow-hidden" data-testid="settings-page">
      {/* Sidebar */}
      <nav
        className="w-40 flex-shrink-0 bg-cocoa-800 border-r border-cocoa-600 py-2 overflow-y-auto"
        aria-label="Settings sections"
      >
        {SECTION_IDS.map((id) => (
          <button
            key={id}
            className={`w-full text-left px-3 py-2 text-xs transition-colors ${
              activeSection === id
                ? 'bg-cocoa-700 text-cocoa-50 border-l-2 border-biscuit-500'
                : 'text-cocoa-300 hover:bg-cocoa-700 hover:text-cocoa-100 border-l-2 border-transparent'
            }`}
            onClick={() => setActiveSection(id)}
            data-testid={`settings-section-${id}`}
          >
            {t(`settings.sections.${id}`)}
          </button>
        ))}
      </nav>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {activeSection === 'general'       && <GeneralSection settings={settings} onChange={handleChange} />}
        {activeSection === 'editor'        && <EditorSection settings={settings} onChange={handleChange} />}
        {activeSection === 'models'        && <ModelsSection />}
        {activeSection === 'terminal'      && <TerminalSection settings={settings} onChange={handleChange} />}
        {activeSection === 'appearance'    && <AppearanceSection settings={settings} onChange={handleChange} />}
        {activeSection === 'security'      && <SecuritySection settings={settings} onChange={handleChange} />}
        {activeSection === 'conversations' && (
          <ConversationsSection
            settings={settings}
            onChange={handleChange}
            onExport={handleExport}
            onImport={handleImport}
            onCleanupNow={handleCleanupNow}
          />
        )}
        {activeSection === 'about'         && <AboutSection />}
      </div>
    </div>
  );
}
