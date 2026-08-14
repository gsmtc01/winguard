// src/store/llmStore.ts
// AI 분석 상태 관리 (모델 다운로드는 modelStore 참조)

import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ScanReport } from "@/api/checks";
import { useUIStore } from "@/store/uiStore";

// ── 공통 타입 ──────────────────────────────────────────────────

export interface LlmAnalysis {
  summary: string;
}

// ── 종합 분석 탭 상태 ─────────────────────────────────────────

export type LlmState =
  | { status: "idle" }
  | { status: "no_model" }           // 모델 미설치 → 설정 안내
  | { status: "loading_model" }
  | { status: "analyzing" }
  | { status: "done"; result: LlmAnalysis }
  | { status: "error"; message: string };

// ── 로드맵 탭 상태 ─────────────────────────────────────────────

export type RoadmapState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "done"; summary: string }
  | { status: "error"; message: string };

// ── Q&A 탭 상태 ───────────────────────────────────────────────

export interface QaMessage {
  id: number;
  question: string;
  /** 답변 대기 중이면 null (질문 말풍선은 즉시 표시) */
  answer: string | null;
  error?: string;
}

export interface QaState {
  history: QaMessage[];
  loading: boolean;
  error: string | null;
}

// ── 스토어 인터페이스 ──────────────────────────────────────────

export type LlmTab = "analyze" | "roadmap" | "qa";

interface LlmStore {
  tab: LlmTab;
  setTab: (tab: LlmTab) => void;

  // 종합 분석
  state: LlmState;
  start: (report: ScanReport) => Promise<void>;
  analyze: (report: ScanReport) => Promise<void>;
  reset: () => void;

  // 보안 로드맵
  roadmap: RoadmapState;
  generateRoadmap: (report: ScanReport) => Promise<void>;
  resetRoadmap: () => void;

  // Q&A
  qa: QaState;
  askQuestion: (report: ScanReport, question: string) => Promise<void>;
  clearQa: () => void;
}

// ── 내부 유틸 ─────────────────────────────────────────────────

let unlistenPhaseRef: UnlistenFn | null = null;
let qaIdSeq = 0;

/** 모델 없으면 설정 안내 메시지, 있으면 null */
async function checkModel(): Promise<boolean> {
  return invoke<boolean>("llm_model_exists").catch(() => false);
}

// ── 스토어 ────────────────────────────────────────────────────

export const useLlmStore = create<LlmStore>((set, get) => ({
  tab: "analyze",
  setTab: (tab) => set({ tab }),

  // ── 종합 분석 ─────────────────────────────────────────────

  state: { status: "idle" },

  start: async (report) => {
    const exists = await checkModel();
    if (!exists) {
      set({ state: { status: "no_model" } });
      return;
    }
    await get().analyze(report);
  },

  analyze: async (report) => {
    set({ state: { status: "loading_model" } });

    if (unlistenPhaseRef) unlistenPhaseRef();
    unlistenPhaseRef = await listen<string>("llm:phase", (e) => {
      if (e.payload === "loading")   set({ state: { status: "loading_model" } });
      if (e.payload === "analyzing") set({ state: { status: "analyzing" } });
    });

    try {
      const result = await invoke<LlmAnalysis>("llm_analyze", { report });
      set({ state: { status: "done", result } });
    } catch (e) {
      set({ state: { status: "error", message: String(e) } });
    } finally {
      if (unlistenPhaseRef) { unlistenPhaseRef(); unlistenPhaseRef = null; }
    }
  },

  reset: () => {
    if (unlistenPhaseRef) { unlistenPhaseRef(); unlistenPhaseRef = null; }
    set({ state: { status: "idle" } });
  },

  // ── 보안 로드맵 ───────────────────────────────────────────

  roadmap: { status: "idle" },

  generateRoadmap: async (report) => {
    const exists = await checkModel();
    if (!exists) {
      // 설정 패널 자동 열기
      useUIStore.getState().openSettings();
      set({ roadmap: { status: "idle" } });
      return;
    }

    set({ roadmap: { status: "loading" } });

    const unlisten = await listen<string>("llm:phase", () => {
      set({ roadmap: { status: "loading" } });
    });

    try {
      const result = await invoke<LlmAnalysis>("llm_roadmap", { report });
      set({ roadmap: { status: "done", summary: result.summary } });
    } catch (e) {
      set({ roadmap: { status: "error", message: String(e) } });
    } finally {
      unlisten();
    }
  },

  resetRoadmap: () => set({ roadmap: { status: "idle" } }),

  // ── Q&A ──────────────────────────────────────────────────

  qa: { history: [], loading: false, error: null },

  askQuestion: async (report, question) => {
    // 이전 대화 이력(질문+완성된 답변 쌍만) → 백엔드 멀티턴 컨텍스트로 전달
    const priorTurns = get()
      .qa.history.filter((m) => m.answer != null)
      .map((m) => ({ question: m.question, answer: m.answer as string }));

    // 질문 말풍선을 즉시 표시 (답변은 pending: answer=null)
    const msgId = ++qaIdSeq;
    set((s) => ({
      qa: {
        history: [...s.qa.history, { id: msgId, question, answer: null }],
        loading: true,
        error: null,
      },
    }));

    const exists = await checkModel();
    if (!exists) {
      // 모델 미설치: 방금 넣은 pending 질문을 되돌리고 설정 안내
      useUIStore.getState().openSettings();
      set((s) => ({
        qa: {
          history: s.qa.history.filter((m) => m.id !== msgId),
          loading: false,
          error: null,
        },
      }));
      return;
    }

    try {
      const result = await invoke<LlmAnalysis>("llm_ask", { report, question, priorTurns });
      set((s) => ({
        qa: {
          history: s.qa.history.map((m) =>
            m.id === msgId ? { ...m, answer: result.summary } : m
          ),
          loading: false,
          error: null,
        },
      }));
    } catch (e) {
      // 해당 질문 말풍선에 에러를 붙이고, 전역 에러도 설정
      set((s) => ({
        qa: {
          history: s.qa.history.map((m) =>
            m.id === msgId ? { ...m, error: String(e) } : m
          ),
          loading: false,
          error: String(e),
        },
      }));
    }
  },

  clearQa: () => set({ qa: { history: [], loading: false, error: null } }),
}));
