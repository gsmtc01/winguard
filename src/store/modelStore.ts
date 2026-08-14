// src/store/modelStore.ts
// AI 모델 다운로드·설치 상태 (SettingsPanel 전용)

import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface DownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

export type ModelPhase =
  | "idle"
  | "checking"
  | "downloading"
  | "done"
  | "error";

export interface ModelStatus {
  phase: ModelPhase;
  downloaded?: number;
  total?: number;
  percent?: number;
  errorMsg?: string;
}

/** 백엔드(llm_model_info)가 정해주는 모델 메타데이터. 프런트에서 하드코딩하지 않는다. */
export interface ModelInfo {
  file_name: string;
  display_name: string;
  size_bytes: number;
  default_dir: string;
  url: string;
}

interface ModelStore {
  /** 현재 모델 설치 여부 (null = 아직 확인 안 함) */
  exists: boolean | null;
  /** 파일명·용량 등 모델 메타데이터 (null = 아직 조회 전) */
  info: ModelInfo | null;
  /** 기본 저장 폴더 */
  defaultDir: string;
  /** 사용자가 고른 저장 폴더 (파일명은 고를 수 없다) */
  destDir: string;
  /** 설치된 모델 파일 경로 */
  modelPath: string;
  /** 다운로드/설치 상태 */
  status: ModelStatus;
  /** 현재 메모리에 모델이 로드되어 있는지 (근사값 — 분석 후 true) */
  loadedInMemory: boolean;

  check: () => Promise<void>;
  setDestDir: (dir: string) => void;
  download: () => Promise<void>;
  unload: () => Promise<void>;
  remove: () => Promise<void>;
}

export function fmtBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

let unlistenProgress: UnlistenFn | null = null;

export const useModelStore = create<ModelStore>((set, get) => ({
  exists: null,
  info: null,
  defaultDir: "",
  destDir: "",
  modelPath: "",
  status: { phase: "idle" },
  loadedInMemory: false,

  check: async () => {
    set({ status: { phase: "checking" } });
    try {
      const exists = await invoke<boolean>("llm_model_exists");
      const info = await invoke<ModelInfo>("llm_model_info");
      const modelPath = await invoke<string>("llm_model_path");
      const loadedInMemory = await invoke<boolean>("llm_is_model_loaded");
      set({
        exists,
        info,
        defaultDir: info.default_dir,
        modelPath,
        loadedInMemory,
        destDir: get().destDir || info.default_dir,
        status: { phase: "idle" },
      });
    } catch (e) {
      set({ status: { phase: "error", errorMsg: String(e) } });
    }
  },

  setDestDir: (dir: string) => set({ destDir: dir }),

  unload: async () => {
    set({ status: { phase: "checking" } });
    try {
      await invoke("llm_unload_model");
      const loadedInMemory = await invoke<boolean>("llm_is_model_loaded");
      set({ loadedInMemory, status: { phase: "idle" } });
    } catch (e) {
      set({ status: { phase: "error", errorMsg: String(e) } });
    }
  },

  remove: async () => {
    set({ status: { phase: "checking" } });
    try {
      await invoke("llm_delete_model");
      set({
        exists: false,
        modelPath: "",
        loadedInMemory: false,
        status: { phase: "idle" },
      });
    } catch (e) {
      set({ status: { phase: "error", errorMsg: String(e) } });
    }
  },

  download: async () => {
    const { destDir, defaultDir } = get();
    const finalDir = destDir || defaultDir;

    if (unlistenProgress) unlistenProgress();
    unlistenProgress = await listen<DownloadProgress>("llm:download-progress", (e) => {
      set({
        status: {
          phase: "downloading",
          downloaded: e.payload.downloaded,
          total: e.payload.total,
          percent: e.payload.percent,
        },
      });
    });

    set({ status: { phase: "downloading", downloaded: 0, total: 0, percent: 0 } });

    try {
      await invoke("llm_download_model", { destDir: finalDir });
      const modelPath = await invoke<string>("llm_model_path");
      set({ exists: true, modelPath, status: { phase: "done" } });
    } catch (e) {
      set({ status: { phase: "error", errorMsg: String(e) } });
    } finally {
      if (unlistenProgress) { unlistenProgress(); unlistenProgress = null; }
    }
  },
}));
