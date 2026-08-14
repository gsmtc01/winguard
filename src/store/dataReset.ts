// src/store/dataReset.ts
//
// 설정 > 데이터 관리의 "초기화" 동작을 한곳에 모은다.
//
// localStorage 키만 지우면 데이터가 남아 있는 것처럼 보인다. 지워야 할 곳이 세 군데다.
//   ① localStorage  — 점수 이력, VT 검사 이력, zustand persist 스냅샷
//   ② 메모리 스토어 — 마지막 검사 리포트, AI 분석/로드맵/Q&A, 각 스캔 패널 결과
//   ③ 컴포넌트 로컬 state — 마운트 시 한 번만 읽어 둔 이력 배열
// ①②는 여기서 직접 처리하고, ③은 이벤트를 쏴서 다시 읽게 한다.

import { clearHistory } from "@/lib/history";
import { clearVtHistory } from "@/lib/vtHistory";
import { useCheckStore } from "./checkStore";
import { useDeviceStore } from "./deviceStore";
import { useEventLogStore } from "./eventlogStore";
import { useExtensionStore } from "./extensionStore";
import { useLlmStore } from "./llmStore";
import { useNetworkStore } from "./networkStore";
import { useProcessStore } from "./processStore";

/** 어떤 범위가 초기화됐는지 알리는 이벤트 이름. */
export const DATA_RESET_EVENT = "winguard:data-reset";

export type ResetScope = "score" | "vt" | "all";

function notify(scope: ResetScope) {
  window.dispatchEvent(new CustomEvent<ResetScope>(DATA_RESET_EVENT, { detail: scope }));
}

/** 보안 점수 추이 기록 삭제. 마지막 검사 결과 자체는 남긴다. */
export function resetScoreHistory(): void {
  clearHistory();
  notify("score");
}

/** VirusTotal 파일 검사 이력 삭제. */
export function resetVtHistory(): void {
  clearVtHistory();
  notify("vt");
}

/**
 * 앱이 보관 중인 검사·분석 데이터를 모두 삭제한다.
 *
 * 격리(Quarantine) 백업은 포함하지 않는다 — 수정한 설정을 되돌리는 수단이라
 * 함께 지우면 복원이 불가능해진다. 격리 항목은 전용 섹션에서 개별 삭제한다.
 */
export function resetAllData(): void {
  clearHistory();
  clearVtHistory();

  // 저장된 마지막 검사 리포트 (localStorage: winguard-checkstore)
  useCheckStore.getState().clearReport();

  // AI 분석 결과 · 로드맵 · Q&A 대화 내용
  const llm = useLlmStore.getState();
  llm.reset();
  llm.resetRoadmap();
  llm.clearQa();

  // 각 분석 패널에 남아 있는 스캔 결과
  useProcessStore.getState().reset();
  useNetworkStore.getState().reset();
  useExtensionStore.getState().reset();
  useEventLogStore.getState().reset();
  useDeviceStore.getState().reset();

  notify("all");
}

/**
 * 초기화가 일어나면 콜백을 실행한다.
 * 마운트 시 한 번만 localStorage 를 읽는 컴포넌트가 목록을 다시 읽을 때 쓴다.
 */
export function onDataReset(handler: (scope: ResetScope) => void): () => void {
  const listener = (e: Event) => handler((e as CustomEvent<ResetScope>).detail);
  window.addEventListener(DATA_RESET_EVENT, listener);
  return () => window.removeEventListener(DATA_RESET_EVENT, listener);
}
