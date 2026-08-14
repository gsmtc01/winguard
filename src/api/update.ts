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
  /** 릴리즈에 `<자산명>.sha256` 이 함께 있으면 그 해시. 없으면 null. */
  download_sha256: string | null;
  prerelease: boolean;
  repo: string;
}

/** 업데이트 서버에서 최신 릴리즈 정보를 가져온다. */
export const checkForUpdate = (): Promise<UpdateInfo> => invoke<UpdateInfo>("check_for_update");

/** 현재 설정된 업데이트 저장소(owner/repo). */
export const getUpdateRepo = (): Promise<string> => invoke<string>("update_repo");

/** 다운로드 진행률 이벤트(`update:download-progress`)의 페이로드. */
export interface UpdateProgress {
  downloaded: number;
  total: number;
  percent: number;
}

/**
 * 새 실행 파일을 내려받아 임시 폴더에 저장하고 그 경로를 돌려준다.
 * 아무것도 교체하지 않는다. 교체는 applyUpdate 가 한다.
 *
 * expectedSha256 을 주면 일치할 때만 성공한다. 불일치하면 받은 파일을 지우고
 * 오류를 낸다. 릴리즈에 함께 올린 .sha256 자산의 값을 넘긴다.
 */
export const downloadUpdate = (
  url: string,
  fileName: string,
  expectedSha256?: string,
): Promise<string> =>
  invoke<string>("update_download", { url, fileName, expectedSha256: expectedSha256 ?? null });

/**
 * 받아둔 파일로 현재 실행 파일을 교체하고 앱을 재시작한다.
 * 성공하면 이 프로세스는 곧 종료되므로 이후 코드는 실행되지 않는다고 봐야 한다.
 */
export const applyUpdate = (downloadedPath: string): Promise<void> =>
  invoke<void>("update_apply", { downloadedPath });

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
