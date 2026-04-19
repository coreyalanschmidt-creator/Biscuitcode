// src/components/ActivityBar.tsx
//
// Vertical 48px-wide bar of icon buttons on the left edge. Each button
// switches the SidePanel's content. Active item gets a 2px biscuit-500
// bar on its left edge.
//
// Phase 2 deliverable. Icons via lucide-react.

import { useTranslation } from 'react-i18next';
import {
  Files,
  Search,
  GitBranch,
  MessageSquare,
  Settings as SettingsIcon,
  type LucideIcon,
} from 'lucide-react';
import { usePanelsStore } from '../state/panelsStore';

type ActivityKey = 'files' | 'search' | 'git' | 'chats' | 'settings';

interface ActivityItem {
  key: ActivityKey;
  Icon: LucideIcon;
  labelKey: string;
}

const ITEMS: readonly ActivityItem[] = [
  { key: 'files',    Icon: Files,         labelKey: 'panels.files' },
  { key: 'search',   Icon: Search,        labelKey: 'panels.search' },
  { key: 'git',      Icon: GitBranch,     labelKey: 'panels.git' },
  { key: 'chats',    Icon: MessageSquare, labelKey: 'panels.chats' },
  { key: 'settings', Icon: SettingsIcon,  labelKey: 'panels.settings' },
];

export function ActivityBar() {
  const { t } = useTranslation();
  const { activeActivity, setActiveActivity } = usePanelsStore();

  return (
    <nav
      aria-label={t('panels.activityBar')}
      className="w-12 flex-shrink-0 flex flex-col bg-cocoa-800 border-r border-cocoa-500"
    >
      {ITEMS.map(({ key, Icon, labelKey }) => {
        const active = activeActivity === key;
        return (
          <button
            key={key}
            type="button"
            onClick={() => setActiveActivity(key)}
            aria-label={t(labelKey)}
            aria-current={active ? 'page' : undefined}
            title={t(labelKey)}
            className={`
              relative flex items-center justify-center h-12 w-12
              text-cocoa-200 hover:text-cocoa-50 transition-colors
              focus:outline-none focus:ring-2 focus:ring-biscuit-500 focus:ring-inset
              ${active ? 'text-cocoa-50' : ''}
            `}
          >
            {active && (
              <span
                aria-hidden="true"
                className="absolute left-0 top-2 bottom-2 w-0.5 bg-biscuit-500"
              />
            )}
            <Icon size={20} strokeWidth={1.75} />
          </button>
        );
      })}
    </nav>
  );
}
