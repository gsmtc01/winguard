// src/store/quarantineStore.ts
//
// Fix 실행 전 레지스트리 백업(격리) 기록 조회/복원/삭제.
// AGENTS.md §2 표: store/*.ts 에서 invoke() 직접 허용.

import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface RegBackupEntry {
  hive: string; // "HKLM" | "HKCU"
  path: string;
  name: string;
  value_type: string; // "DWORD" | "SZ"
  original_value: number | string;
}

export interface QuarantineMetadata {
  id: string;
  check_id: string;
  action_label: string;
  created_at: string;
  restored: boolean;
}

export interface QuarantineRecord {
  metadata: QuarantineMetadata;
  entries: RegBackupEntry[];
}

interface QuarantineState {
  records: QuarantineRecord[];
  isLoading: boolean;
  error: string | null;
  busyId: string | null;

  loadList: () => Promise<void>;
  restore: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
}

export const useQuarantineStore = create<QuarantineState>((set, get) => ({
  records: [],
  isLoading: false,
  error: null,
  busyId: null,

  loadList: async () => {
    set({ isLoading: true, error: null });
    try {
      const records = await invoke<QuarantineRecord[]>("quarantine_list");
      set({ records, isLoading: false });
    } catch (e) {
      set({ isLoading: false, error: String(e) });
    }
  },

  restore: async (id: string) => {
    set({ busyId: id, error: null });
    try {
      await invoke("quarantine_restore", { quarantineId: id });
      await get().loadList();
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ busyId: null });
    }
  },

  remove: async (id: string) => {
    set({ busyId: id, error: null });
    try {
      await invoke("quarantine_delete", { quarantineId: id });
      set((s) => ({ records: s.records.filter((r) => r.metadata.id !== id) }));
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ busyId: null });
    }
  },
}));
