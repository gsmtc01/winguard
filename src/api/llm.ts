// src/api/llm.ts
//
// 컴포넌트에서 호출하는 LLM Tauri invoke() 래퍼.
// 탭별 분석(llm_analyze_*)은 각 store 에서 직접 호출한다 (AGENTS.md §2).

import { invoke } from "@tauri-apps/api/core";
import type { ScanReport } from "./checks";

export interface LlmSummary {
  summary: string;
}

/** 스캔 리포트 전체를 LLM 으로 요약해 보고서 본문을 만든다. */
export const generateReport = (report: ScanReport): Promise<LlmSummary> =>
  invoke<LlmSummary>("llm_generate_report", { report });
