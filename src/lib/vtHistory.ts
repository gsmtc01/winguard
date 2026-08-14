// src/lib/vtHistory.ts
// VT 파일 검사 이력을 localStorage에 저장/조회한다.

import type { VtScanResult } from "@/api/virustotal";

export interface VtHistoryEntry {
  sha256: string;
  file_name: string;
  status: string;
  malicious: number;
  total_engines: number;
  vt_link: string;
  scanned_at: number; // Date.now() (ms)
}

const VT_HISTORY_KEY = "winguard_vt_history";
const MAX_ENTRIES = 20;

/** 검사 결과를 이력에 저장한다. 같은 SHA256은 갱신한다. */
export function saveVtHistory(result: VtScanResult, fileName: string): void {
  if (!result.sha256 || result.status === "error") return;
  const entry: VtHistoryEntry = {
    sha256:        result.sha256,
    file_name:     fileName,
    status:        result.status,
    malicious:     result.malicious,
    total_engines: result.total_engines,
    vt_link:       result.vt_link,
    scanned_at:    Date.now(),
  };
  const history = loadVtHistory().filter((h) => h.sha256 !== entry.sha256);
  history.unshift(entry); // 최신 순
  const trimmed = history.slice(0, MAX_ENTRIES);
  try {
    localStorage.setItem(VT_HISTORY_KEY, JSON.stringify(trimmed));
  } catch {
    // storage full 등 무시
  }
}

/** 저장된 이력을 최신 순으로 반환한다. */
export function loadVtHistory(): VtHistoryEntry[] {
  try {
    const raw = localStorage.getItem(VT_HISTORY_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as VtHistoryEntry[];
  } catch {
    return [];
  }
}

/** 이력을 모두 삭제한다. */
export function clearVtHistory(): void {
  localStorage.removeItem(VT_HISTORY_KEY);
}

/** 이력에서 특정 SHA256 항목을 제거한다. */
export function removeVtHistoryEntry(sha256: string): void {
  const history = loadVtHistory().filter((h) => h.sha256 !== sha256);
  try {
    localStorage.setItem(VT_HISTORY_KEY, JSON.stringify(history));
  } catch {
    // ignore
  }
}
