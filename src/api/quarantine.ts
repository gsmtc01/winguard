// src/api/quarantine.ts
//
// 컴포넌트에서 호출하는 Quarantine(자동 백업) Tauri invoke() 래퍼.
// 목록 조회 / 복원 / 삭제는 store/quarantineStore 에서 직접 호출한다 (AGENTS.md §2).

import { invoke } from "@tauri-apps/api/core";
import type { RegBackupEntry } from "@/store/quarantineStore";

export interface RegBackupRequest {
  entries: RegBackupEntry[];
  checkId: string;
  actionLabel: string;
}

/** 수정 실행 전 레지스트리 원본 값을 백업한다. 실패하면 reject 되며,
 *  호출부는 백업 실패 시 수정 자체를 중단해야 한다 (AGENTS.md §4-11). */
export const saveRegBackup = (req: RegBackupRequest): Promise<void> =>
  invoke<void>("quarantine_save", req);
