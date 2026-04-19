// src/state/panelsStore.ts
//
// Zustand store for panel visibility + sizes. Persisted via localStorage
// so panel layout survives restarts.
//
// Phase 2 deliverable. Outer-window geometry (position, maximized state)
// is handled SEPARATELY by `tauri-plugin-window-state` — those are two
// concerns, do NOT conflate.

import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

/** Default panel widths/heights — match the vision's UI Layout section. */
const DEFAULT_SIDE_WIDTH = 260;
const DEFAULT_BOTTOM_HEIGHT = 240;
const DEFAULT_CHAT_WIDTH = 380;

interface PanelsState {
  // Visibility
  sideVisible: boolean;
  bottomVisible: boolean;
  chatVisible: boolean;

  // Sizes (px). `react-resizable-panels` works in percentages internally;
  // we convert at the layout boundary.
  sideWidthPx: number;
  bottomHeightPx: number;
  chatWidthPx: number;

  // Active activity-bar item — dictates what the side panel shows.
  activeActivity: 'files' | 'search' | 'git' | 'chats' | 'settings';

  // Actions
  toggleSide: () => void;
  toggleBottom: () => void;
  toggleChat: () => void;
  /** Show the bottom panel without toggling (used by Ctrl+` shortcut). */
  setBottomVisible: (v: boolean) => void;
  setActiveActivity: (a: PanelsState['activeActivity']) => void;
  setSideWidth: (px: number) => void;
  setBottomHeight: (px: number) => void;
  setChatWidth: (px: number) => void;
}

export const usePanelsStore = create<PanelsState>()(
  persist(
    (set) => ({
      sideVisible: true,
      bottomVisible: true,
      chatVisible: true,

      sideWidthPx: DEFAULT_SIDE_WIDTH,
      bottomHeightPx: DEFAULT_BOTTOM_HEIGHT,
      chatWidthPx: DEFAULT_CHAT_WIDTH,

      activeActivity: 'files',

      toggleSide:   () => set((s) => ({ sideVisible:   !s.sideVisible })),
      toggleBottom: () => set((s) => ({ bottomVisible: !s.bottomVisible })),
      toggleChat:   () => set((s) => ({ chatVisible:   !s.chatVisible })),
      setBottomVisible: (v) => set({ bottomVisible: v }),

      setActiveActivity: (a) => set({ activeActivity: a, sideVisible: true }),

      setSideWidth:    (px) => set({ sideWidthPx: clamp(px, 180, 600) }),
      setBottomHeight: (px) => set({ bottomHeightPx: clamp(px, 120, 600) }),
      setChatWidth:    (px) => set({ chatWidthPx: clamp(px, 280, 720) }),
    }),
    {
      name: 'biscuitcode-panels-v1',
      storage: createJSONStorage(() => localStorage),
      version: 1,
    },
  ),
);

function clamp(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, n));
}
