// src/lib/history.ts
// 보안 점수 이력을 localStorage에 저장/조회한다.

import type { ScanReport } from "@/api/checks";

export interface HistoryEntry {
  score: number;
  scanned_at: number; // Unix seconds
  danger: number;
  warning: number;
}

const HISTORY_KEY = "winguard_score_history";
const MAX_ENTRIES = 30;

/** 스캔 결과를 이력에 저장한다. 중복 스캔 시간은 덮어쓴다. */
export function saveHistory(report: ScanReport): void {
  const entry: HistoryEntry = {
    score:      report.score,
    scanned_at: report.scanned_at,
    danger:     report.checks.filter(
      (c) => c.severity === "danger" || c.severity === "critical"
    ).length,
    warning: report.checks.filter((c) => c.severity === "warning").length,
  };

  const history = loadHistory().filter((h) => h.scanned_at !== entry.scanned_at);
  history.push(entry);
  history.sort((a, b) => a.scanned_at - b.scanned_at);
  const trimmed = history.slice(-MAX_ENTRIES);
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(trimmed));
  } catch {
    // storage full 등 무시
  }
}

/** 저장된 이력을 오래된 순으로 반환한다. */
export function loadHistory(): HistoryEntry[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as HistoryEntry[];
  } catch {
    return [];
  }
}

/** 이력을 모두 삭제한다. */
export function clearHistory(): void {
  localStorage.removeItem(HISTORY_KEY);
}
