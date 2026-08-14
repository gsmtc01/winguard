// src/store/deviceStore.ts
//
// 카메라·마이크 접근 제어 상태 관리

import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface AppAccess {
  name: string;
  key: string;
  path: string;
  allowed: boolean;
  is_uwp: boolean;
}

export interface DeviceStatus {
  camera_allowed: boolean;
  mic_allowed: boolean;
  camera_apps: AppAccess[];
  mic_apps: AppAccess[];
}

type LoadPhase =
  | "idle"
  | "loading"      // get_device_status 호출 중
  | "analyzing"    // llm_analyze_device 호출 중
  | "done"
  | "error";

interface DeviceState {
  phase: LoadPhase;
  status: DeviceStatus | null;
  analysis: string | null;
  error: string | null;

  // 전체 상태 로드
  load: () => Promise<void>;

  // 전역 토글
  toggleCamera: (allow: boolean) => Promise<void>;
  toggleMic: (allow: boolean) => Promise<void>;

  // 앱별 토글
  toggleAppCamera: (appKey: string, allow: boolean) => Promise<void>;
  toggleAppMic: (appKey: string, allow: boolean) => Promise<void>;

  // AI 분석
  analyze: () => Promise<void>;

  reset: () => void;
}

export const useDeviceStore = create<DeviceState>((set, get) => ({
  phase: "idle",
  status: null,
  analysis: null,
  error: null,

  load: async () => {
    set({ phase: "loading", error: null });
    try {
      const status = await invoke<DeviceStatus>("get_device_status");
      set({ status, phase: "done" });
    } catch (e) {
      set({ phase: "error", error: String(e) });
    }
  },

  toggleCamera: async (allow: boolean) => {
    try {
      await invoke("set_camera_access", { allow });
      // 로컬 상태 즉시 반영
      set((s) => ({
        status: s.status
          ? { ...s.status, camera_allowed: allow }
          : s.status,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  toggleMic: async (allow: boolean) => {
    try {
      await invoke("set_mic_access", { allow });
      set((s) => ({
        status: s.status
          ? { ...s.status, mic_allowed: allow }
          : s.status,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  toggleAppCamera: async (appKey: string, allow: boolean) => {
    try {
      await invoke("set_app_camera_access", { appKey, allow });
      set((s) => {
        if (!s.status) return {};
        return {
          status: {
            ...s.status,
            camera_apps: s.status.camera_apps.map((a) =>
              a.key === appKey ? { ...a, allowed: allow } : a
            ),
          },
        };
      });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  toggleAppMic: async (appKey: string, allow: boolean) => {
    try {
      await invoke("set_app_mic_access", { appKey, allow });
      set((s) => {
        if (!s.status) return {};
        return {
          status: {
            ...s.status,
            mic_apps: s.status.mic_apps.map((a) =>
              a.key === appKey ? { ...a, allowed: allow } : a
            ),
          },
        };
      });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  analyze: async () => {
    const { status } = get();
    if (!status) return;

    // 모델 설치 확인
    const modelExists = await invoke<boolean>("llm_model_exists").catch(() => false);
    if (!modelExists) {
      set({
        phase: "error",
        error: "AI 모델이 설치되지 않았습니다. ⚙️ 설정에서 On-Device AI 모델을 다운로드하세요.",
      });
      return;
    }

    set({ phase: "analyzing", analysis: null, error: null });
    try {
      const res = await invoke<{ summary: string }>("llm_analyze_device", {
        status,
      });
      set({ analysis: res.summary, phase: "done" });
    } catch (e) {
      set({ phase: "error", error: String(e) });
    }
  },

  reset: () =>
    set({ phase: "idle", status: null, analysis: null, error: null }),
}));
