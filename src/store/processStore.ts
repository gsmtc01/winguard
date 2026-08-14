import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface ProcessInfo {
  pid: number;
  name: string;
  path: string | null;
  company: string | null;
  /** null = 확인 불가, true = 유효, false = 없음/무효 */
  signed: boolean | null;
  cpu: number;
  memory_mb: number;
}

type Phase = "scanning" | "loading_model" | "analyzing";

type State =
  | { status: "idle" }
  | { status: "loading"; phase: Phase }
  | { status: "done"; processes: ProcessInfo[]; summary: string }
  | { status: "error"; message: string };

interface ProcessStore {
  state: State;
  analyze: () => Promise<void>;
  reset: () => void;
}

export const useProcessStore = create<ProcessStore>((set) => ({
  state: { status: "idle" },

  analyze: async () => {
    set({ state: { status: "loading", phase: "scanning" } });

    let processes: ProcessInfo[] = [];
    try {
      processes = await invoke<ProcessInfo[]>("scan_processes");
    } catch (e) {
      set({ state: { status: "error", message: `프로세스 스캔 실패: ${String(e)}` } });
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
      const result = await invoke<{ summary: string }>("llm_analyze_processes", { processes });
      set({ state: { status: "done", processes, summary: result.summary } });
    } catch (e) {
      set({ state: { status: "error", message: String(e) } });
    } finally {
      unlisten();
    }
  },

  reset: () => set({ state: { status: "idle" } }),
}));
