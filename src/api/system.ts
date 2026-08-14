// src/api/system.ts
//
// 시작 프로그램 / 레지스트리 / 설정 창 열기 관련 Tauri invoke() 래퍼.
// 컴포넌트에서 invoke() 를 직접 호출하지 않는다 (AGENTS.md §2).

import { invoke } from "@tauri-apps/api/core";

/** Windows 시작 시 자동 실행이 켜져 있는지 확인한다. */
export const getStartupEnabled = (): Promise<boolean> => invoke<boolean>("get_startup_enabled");

/** Windows 시작 시 자동 실행을 켜거나 끈다. */
export const setStartupEnabled = (enable: boolean): Promise<void> =>
  invoke<void>("set_startup_enabled", { enable });

export interface RegDwordQuery {
  hive: string;
  path: string;
  name: string;
}

/** 레지스트리 DWORD 값을 읽는다. 키가 없으면 reject 된다. */
export const readRegDword = (query: RegDwordQuery): Promise<number> =>
  invoke<number>("read_reg_dword", query);

/** `cmd:` 접두사로 매핑된 설정 창 열기 커맨드를 실행한다. */
export const runSettingsCommand = (command: string): Promise<void> => invoke<void>(command);
