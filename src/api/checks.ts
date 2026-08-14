import { invoke } from "@tauri-apps/api/core";
import type { Severity } from "@/lib/severity";

export type { Severity };

export interface CheckResult {
  id: string;
  title: string;
  severity: Severity;
  message: string;
  action_uri: string | null;
  evidence: Record<string, string>;
  checked_at: number;
}

export interface ScanReport {
  score: number;
  checks: CheckResult[];
  scanned_at: number;
}

export const runScan = (): Promise<ScanReport> => invoke("run_scan");

/** 시스템 트레이 툴팁에 최신 보안 점수를 반영한다. 트레이 미지원 환경에서는 무시. */
export const updateTrayTooltip = (score: number): Promise<void> =>
  invoke<void>("update_tray_tooltip", { score }).catch(() => undefined);

