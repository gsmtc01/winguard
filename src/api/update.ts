// src/api/update.ts
//
// 업데이트 확인 관련 Tauri invoke() 래퍼.
// 실제 조회는 Rust(update_commands.rs)가 GitHub Releases 에 대해 수행한다.

import { invoke } from "@tauri-apps/api/core";

/** Rust UpdateInfo 와 1:1 대응 */
export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  title: string;
  /** 릴리즈 노트 (마크다운) */
  notes: string;
  /** ISO8601 배포일. 없으면 빈 문자열 */
  published_at: string;
  release_url: string;
  download_url: string | null;
  download_name: string | null;
  download_size: number | null;
  prerelease: boolean;
  repo: string;
}

/** 업데이트 서버에서 최신 릴리즈 정보를 가져온다. */
export const checkForUpdate = (): Promise<UpdateInfo> => invoke<UpdateInfo>("check_for_update");

/** 현재 설정된 업데이트 저장소(owner/repo). */
export const getUpdateRepo = (): Promise<string> => invoke<string>("update_repo");

/** 마지막 확인 시각을 기억해 설정 패널을 열 때마다 서버를 두드리지 않게 한다. */
const LAST_CHECK_KEY = "winguard_update_last_check";
const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000; // 6시간

export function shouldAutoCheck(now = Date.now()): boolean {
  try {
    const raw = localStorage.getItem(LAST_CHECK_KEY);
    if (!raw) return true;
    const last = Number(raw);
    return !Number.isFinite(last) || now - last >= CHECK_INTERVAL_MS;
  } catch {
    return true;
  }
}

export function markAutoChecked(now = Date.now()): void {
  try {
    localStorage.setItem(LAST_CHECK_KEY, String(now));
  } catch {
    // storage 사용 불가 — 매번 확인해도 무해하므로 무시
  }
}
