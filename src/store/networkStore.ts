import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface NetworkConn {
  local_address: string;
  local_port: number;
  remote_address: string;
  remote_port: number;
  state: string;
  pid: number;
  process_name: string | null;
}

type Phase = "scanning" | "loading_model" | "analyzing";

type State =
  | { status: "idle" }
  | { status: "loading"; phase: Phase }
  | { status: "done"; connections: NetworkConn[]; summary: string }
  | { status: "error"; message: string };

interface NetworkStore {
  state: State;
  analyze: () => Promise<void>;
  reset: () => void;
}

export const useNetworkStore = create<NetworkStore>((set) => ({
  state: { status: "idle" },

  analyze: async () => {
    set({ state: { status: "loading", phase: "scanning" } });

    let connections: NetworkConn[] = [];
    try {
      connections = await invoke<NetworkConn[]>("scan_network");
    } catch (e) {
      set({ state: { status: "error", message: `네트워크 스캔 실패: ${String(e)}` } });
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
      const result = await invoke<{ summary: string }>("llm_analyze_network", { connections });
      set({ state: { status: "done", connections, summary: result.summary } });
    } catch (e) {
      set({ state: { status: "error", message: String(e) } });
    } finally {
      unlisten();
    }
  },

  reset: () => set({ state: { status: "idle" } }),
}));
