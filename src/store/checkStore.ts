import { create } from "zustand";
import { persist } from "zustand/middleware";
import { runScan, updateTrayTooltip } from "@/api/checks";
import type { ScanReport, CheckResult } from "@/api/checks";
import { saveHistory } from "@/lib/history";

export type { CheckResult, ScanReport };

interface CheckStore {
  report: ScanReport | null;
  lastScannedAt: number | null;
  isLoading: boolean;
  lastError: string | null;
  runScan: () => Promise<void>;
  /** 저장된 마지막 검사 결과를 지운다 (설정 > 데이터 관리). */
  clearReport: () => void;
}

export const useCheckStore = create<CheckStore>()(
  persist(
    (set) => ({
      report: null,
      lastScannedAt: null,
      isLoading: false,
      lastError: null,
  runScan: async () => {
    set({ isLoading: true, lastError: null });
    try {
      const report = await runScan();
      saveHistory(report);
      set({ report, lastScannedAt: Date.now(), isLoading: false });
      // 시스템 트레이 툴팁에 점수 반영
      await updateTrayTooltip(report.score);
    } catch (e) {
      set({ isLoading: false, lastError: String(e) });
    }
  },

  clearReport: () =>
    set({ report: null, lastScannedAt: null, isLoading: false, lastError: null }),
}), {
  name: "winguard-checkstore",
  // 검사 결과만 저장한다. isLoading/lastError 까지 저장하면 검사 도중 앱이 죽었을 때
  // 다음 실행에서 "검사 중" 상태로 굳어 자동 검사가 시작되지 않는다.
  partialize: (s) => ({ report: s.report, lastScannedAt: s.lastScannedAt }),
}));
