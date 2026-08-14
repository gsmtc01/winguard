import { describe, it, expect, beforeEach, vi } from "vitest";
import { resetAllData, resetScoreHistory, resetVtHistory, onDataReset } from "./dataReset";
import { useCheckStore } from "./checkStore";
import { useLlmStore } from "./llmStore";
import { loadHistory } from "@/lib/history";
import { loadVtHistory } from "@/lib/vtHistory";
import type { ScanReport } from "@/api/checks";

const report: ScanReport = {
  score: 72,
  scanned_at: 1_700_000_000,
  checks: [
    {
      id: "uac",
      title: "UAC",
      severity: "warning",
      message: "낮음",
      action_uri: null,
      evidence: {},
      checked_at: 1_700_000_000,
    },
  ],
};

function seedEverything() {
  localStorage.setItem("winguard_score_history", JSON.stringify([{ score: 70, scanned_at: 1, danger: 0, warning: 1 }]));
  localStorage.setItem(
    "winguard_vt_history",
    JSON.stringify([{ sha256: "a", file_name: "f", status: "clean", malicious: 0, total_engines: 70, vt_link: "", scanned_at: 1 }])
  );
  useCheckStore.setState({ report, lastScannedAt: 123 });
  useLlmStore.setState({
    state: { status: "done", result: { summary: "분석 결과" } },
    roadmap: { status: "done", summary: "로드맵" },
    qa: { history: [{ id: 1, question: "질문", answer: "답변" }], loading: false, error: null },
  });
}

describe("데이터 초기화", () => {
  beforeEach(() => {
    localStorage.clear();
    useCheckStore.setState({ report: null, lastScannedAt: null, isLoading: false, lastError: null });
    useLlmStore.getState().reset();
    useLlmStore.getState().resetRoadmap();
    useLlmStore.getState().clearQa();
  });

  it("점수 기록 초기화는 점수 이력만 지운다", () => {
    seedEverything();
    resetScoreHistory();

    expect(loadHistory()).toEqual([]);
    // 파일 검사 이력과 마지막 검사 결과는 남는다
    expect(loadVtHistory()).toHaveLength(1);
    expect(useCheckStore.getState().report).not.toBeNull();
  });

  it("파일 검사 기록 초기화는 VT 이력만 지운다", () => {
    seedEverything();
    resetVtHistory();

    expect(loadVtHistory()).toEqual([]);
    expect(loadHistory()).toHaveLength(1);
  });

  it("모든 데이터 초기화 후 잔존 데이터가 없다", () => {
    seedEverything();
    resetAllData();

    expect(loadHistory()).toEqual([]);
    expect(loadVtHistory()).toEqual([]);

    // 마지막 검사 결과 — localStorage(winguard-checkstore)에도 남으면 안 된다
    const check = useCheckStore.getState();
    expect(check.report).toBeNull();
    expect(check.lastScannedAt).toBeNull();
    expect(localStorage.getItem("winguard-checkstore") ?? "").not.toContain('"uac"');

    // AI 분석 · 로드맵 · Q&A 대화 내용
    const llm = useLlmStore.getState();
    expect(llm.state.status).toBe("idle");
    expect(llm.roadmap.status).toBe("idle");
    expect(llm.qa.history).toEqual([]);
  });

  it("초기화하면 구독자에게 범위를 알린다", () => {
    const spy = vi.fn();
    const unsubscribe = onDataReset(spy);

    resetScoreHistory();
    resetVtHistory();
    resetAllData();

    expect(spy.mock.calls.map((c) => c[0])).toEqual(["score", "vt", "all"]);

    unsubscribe();
    resetScoreHistory();
    expect(spy).toHaveBeenCalledTimes(3);
  });
});

describe("checkStore persist", () => {
  it("isLoading/lastError 는 저장하지 않는다", () => {
    localStorage.clear();
    useCheckStore.setState({ report, lastScannedAt: 1, isLoading: true, lastError: "실패" });

    const raw = localStorage.getItem("winguard-checkstore") ?? "";
    expect(raw).not.toContain("isLoading");
    expect(raw).not.toContain("lastError");
  });
});
