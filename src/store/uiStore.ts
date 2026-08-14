// src/store/uiStore.ts
// 전역 UI 상태 — 설정 패널 / AI 슬라이드오버 열기·닫기, 사이드바 접힘

import { create } from "zustand";

interface UIStore {
  settingsOpen: boolean;
  openSettings: () => void;
  closeSettings: () => void;

  aiOpen: boolean;
  openAi: () => void;
  closeAi: () => void;

  sidebarExpanded: boolean;
  toggleSidebar: () => void;
}

export const useUIStore = create<UIStore>((set) => ({
  settingsOpen: false,
  openSettings: () => set({ settingsOpen: true }),
  closeSettings: () => set({ settingsOpen: false }),

  aiOpen: false,
  openAi: () => set({ aiOpen: true }),
  closeAi: () => set({ aiOpen: false }),

  sidebarExpanded: true,
  toggleSidebar: () => set((s) => ({ sidebarExpanded: !s.sidebarExpanded })),
}));
