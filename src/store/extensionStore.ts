import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface BrowserExtension {
  browser: string;         // "Chrome" | "Edge" | "Firefox"
  id: string;
  name: string;
  version: string;
  description: string | null;
  permissions: string;     // 콤마 구분 문자열
  enabled: boolean;
}

/** 권한 문자열에서 위험도를 계산한다 (프론트엔드 표시용) */
export function getExtRisk(permissions: string): "high" | "medium" | "low" {
  const p = permissions.toLowerCase();
  const HIGH = ["webrequest", "webrequestblocking", "nativemessaging", "debugger", "<all_urls>"];
  const MED  = ["history", "cookies", "downloads", "management",
                 "clipboardread", "clipboardwrite", "http://*/*", "https://*/*"];
  if (HIGH.some((r) => p.includes(r))) return "high";
  if (MED.some((r) => p.includes(r)))  return "medium";
  return "low";
}

type Phase = "scanning" | "loading_model" | "analyzing";

type State =
  | { status: "idle" }
  | { status: "loading"; phase: Phase }
  | { status: "done"; extensions: BrowserExtension[]; summary: string }
  | { status: "error"; message: string };

interface ExtensionStore {
  state: State;
  analyze: () => Promise<void>;
  reset: () => void;
}

export const useExtensionStore = create<ExtensionStore>((set) => ({
  state: { status: "idle" },

  analyze: async () => {
    set({ state: { status: "loading", phase: "scanning" } });

    let extensions: BrowserExtension[] = [];
    try {
      extensions = await invoke<BrowserExtension[]>("scan_extensions");
    } catch (e) {
      set({ state: { status: "error", message: `확장 프로그램 스캔 실패: ${String(e)}` } });
      return;
    }

    if (extensions.length === 0) {
      set({
        state: {
          status: "error",
          message: "설치된 브라우저 확장 프로그램을 찾을 수 없습니다. Chrome, Edge, Firefox 중 하나 이상이 설치되어 있어야 합니다.",
        },
      });
      return;
    }

    const exists = await invoke<boolean>("llm_model_exists").catch(() => false);
    if (!exists) {
      set({
        state: {
          status: "error",
          message: "AI 모델이 설치되지 않았습니다. ⚙️ 설정에서 On-Device AI 모델을 다운로드하세요.",
        },
      });
      return;
    }

    set({ state: { status: "loading", phase: "loading_model" } });

    const unlisten = await listen<string>("llm:phase", (e) => {
      if (e.payload === "analyzing") {
        set({ state: { status: "loading", phase: "analyzing" } });
      }
    });

    try {
      const result = await invoke<{ summary: string }>("llm_analyze_extensions", { extensions });
      set({ state: { status: "done", extensions, summary: result.summary } });
    } catch (e) {
      set({ state: { status: "error", message: String(e) } });
    } finally {
      unlisten();
    }
  },

  reset: () => set({ state: { status: "idle" } }),
}));
