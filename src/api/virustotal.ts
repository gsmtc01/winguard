// src/api/virustotal.ts
//
// VirusTotal 관련 Tauri invoke() 래퍼.
// 컴포넌트에서 invoke() 를 직접 호출하지 않는다.
// API 키 값 자체는 이 계층에서도 취급하지 않는다 (Rust 에서만 처리).

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

// ── 타입 정의 (Rust VtScanResult 와 1:1 대응) ─────────────────

export type VtStatus = "clean" | "suspicious" | "malicious" | "error";

export interface EngineResult {
  engine_name: string;
  category: string;
  result: string | null;
}

export interface VtScanResult {
  sha256: string;
  status: VtStatus;
  total_engines: number;
  malicious: number;
  suspicious: number;
  undetected: number;
  detections: EngineResult[];
  vt_link: string;
  error_message: string | null;
  was_uploaded: boolean;
}

// ── API 키 관리 ────────────────────────────────────────────────

/** API 키를 OS 키체인에 저장 */
export const saveVtApiKey = (apiKey: string): Promise<void> =>
  invoke("save_vt_api_key", { apiKey });

/** API 키 등록 여부 확인 (키 값은 반환하지 않음) */
export const hasVtApiKey = (): Promise<boolean> =>
  invoke("has_vt_api_key");

/** API 키 삭제 */
export const deleteVtApiKey = (): Promise<void> =>
  invoke("delete_vt_api_key");

// ── 파일 선택 + 검사 ──────────────────────────────────────────

/**
 * OS 파일 선택 다이얼로그를 열고 경로를 반환 (단일).
 * 취소 시 null 반환.
 */
export const pickFile = async (): Promise<string | null> => {
  const selected = await open({
    multiple: false,
    title: "검사할 파일 선택",
  });
  return typeof selected === "string" ? selected : null;
};

/**
 * OS 파일 선택 다이얼로그를 열고 경로 목록을 반환 (다중).
 * 취소 시 빈 배열 반환.
 */
export const pickFiles = async (): Promise<string[]> => {
  const selected = await open({
    multiple: true,
    title: "검사할 파일 선택 (여러 파일 가능)",
  });
  if (!selected) return [];
  if (typeof selected === "string") return [selected];
  return selected;
};

/**
 * 파일 VT 검사.
 *
 * @param filePath     검사할 파일 절대 경로
 * @param allowUpload  해시 미등록 시 VT 업로드 허용 여부
 *                     → 업로드 동의 다이얼로그 후 true 로 설정
 */
export const scanFileVt = (
  filePath: string,
  allowUpload: boolean
): Promise<VtScanResult> =>
  invoke("scan_file_vt", { filePath, allowUpload });
