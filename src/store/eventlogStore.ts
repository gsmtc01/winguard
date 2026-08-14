import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface EventLogEntry {
  event_id: number;
  time_created: string;
  level: string;   // "고위험" | "주의" | "정보"
  label: string;   // 한국어 설명
  source: string;
  message: string;
}

type Phase = "scanning" | "loading_model" | "analyzing";

type State =
  | { status: "idle" }
  | { status: "loading"; phase: Phase }
  | { status: "done"; entries: EventLogEntry[]; summary: string }
  | { status: "error"; message: string };

interface EventLogStore {
  state: State;
  analyze: () => Promise<void>;
  reset: () => void;
}

export const useEventLogStore = create<EventLogStore>((set) => ({
  state: { status: "idle" },

  analyze: async () => {
    set({ state: { status: "loading", phase: "scanning" } });

    let entries: EventLogEntry[] = [];
    try {
      entries = await invoke<EventLogEntry[]>("scan_eventlog");
    } catch (e) {
      set({ state: { status: "error", message: `이벤트 로그 스캔 실패: ${String(e)}` } });
      return;
    }

    const exists = await invoke<boolean>("llm_model_exists").catch(() => false);
    if (!exists) {
      set({
        state: {
          status: "error",
          message:
            "AI 모델이 설치되지 않았습니다. ⚙️ 설정에서 On-Device AI 모델을 다운로드하세요.",
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
      const result = await invoke<{ summary: string }>("llm_analyze_eventlog", { entries });
      set({ state: { status: "done", entries, summary: result.summary } });
    } catch (e) {
      set({ state: { status: "error", message: String(e) } });
    } finally {
      unlisten();
    }
  },

  reset: () => set({ state: { status: "idle" } }),
}));
