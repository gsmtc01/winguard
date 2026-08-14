// src/components/LlmPanel.tsx
// AI 보안 분석 콘텐츠 — AiSlideOver 내부에 호스팅된다.

import { useRef, useEffect } from "react";
import { useLlmStore, type LlmTab } from "@/store/llmStore";
import { useUIStore } from "@/store/uiStore";
import { useModelStore, fmtBytes } from "@/store/modelStore";
import { MarkdownBlock } from "./MarkdownBlock";
import { IconSparkle } from "./icons";
import type { ScanReport } from "@/store/checkStore";

interface Props {
  report: ScanReport;
}

// ── 모델 미설치 안내 배너 ─────────────────────────────────────

function NoModelBanner() {
  const { openSettings } = useUIStore();
  // 크기를 문구에 박아두면 모델을 교체할 때 어긋난다(실제 2.89 GB 를 1.5 GB 로
  // 안내하고 있었다). 백엔드가 알려주는 실제 크기를 쓰고, 아직 조회 전이면
  // 어림값을 보여준다.
  const info = useModelStore((s) => s.info);
  const sizeText = info ? fmtBytes(info.size_bytes) : "약 3 GB";
  return (
    <div className="flex items-center justify-between gap-4 rounded-xl border border-warn bg-warn-bg p-4">
      <div>
        <p className="text-sm font-semibold text-warn">AI 모델 미설치</p>
        <p className="mt-0.5 text-xs text-text-2">
          AI 분석을 사용하려면 On-Device AI 모델을 먼저 다운로드해야 합니다. ({sizeText})
        </p>
      </div>
      <button
        onClick={openSettings}
        className="flex-shrink-0 rounded-lg bg-warn px-3 py-1.5 text-xs font-medium text-white transition hover:brightness-105"
      >
        설정에서 다운로드
      </button>
    </div>
  );
}

// ── 로딩 스피너 ───────────────────────────────────────────────

function LoadingSpinner({ message }: { message: string }) {
  return (
    <div className="flex items-center gap-3">
      <div
        className="h-4 w-4 flex-shrink-0 animate-spin rounded-full border-2 border-accent-soft border-t-accent"
        style={{ willChange: "transform" }}
      />
      <p className="text-sm text-text-2">{message}</p>
    </div>
  );
}

// ── 탭 버튼 ───────────────────────────────────────────────────

function TabBar({ active, disabled, onChange }: { active: LlmTab; disabled?: boolean; onChange: (t: LlmTab) => void }) {
  const tabs: { id: LlmTab; label: string }[] = [
    { id: "analyze", label: "종합 분석" },
    { id: "roadmap", label: "보안 로드맵" },
    { id: "qa", label: "Q&A" },
  ];
  return (
    <div className="flex gap-1 rounded-lg bg-surface-2 p-0.5">
      {tabs.map((t) => (
        <button
          key={t.id}
          disabled={disabled}
          onClick={() => onChange(t.id)}
          className={`flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition ${
            active === t.id
              ? "bg-surface text-accent shadow-sm"
              : "text-text-2 hover:text-text disabled:opacity-50 disabled:cursor-not-allowed"
          }`}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}

// ── 탭 1: 종합 분석 ───────────────────────────────────────────

function AnalyzeTab({ report }: { report: ScanReport }) {
  const { state, start, analyze, reset } = useLlmStore();

  const isLoading = state.status === "loading_model" || state.status === "analyzing";
  const loadingMsg = state.status === "loading_model" ? "AI 모델을 메모리에 로드하는 중..." : "AI 분석 중...";

  if (state.status === "no_model") return <NoModelBanner />;
  if (isLoading) return <LoadingSpinner message={loadingMsg} />;

  if (state.status === "done") {
    return (
      <div className="space-y-3">
        <div className="max-h-[440px] space-y-2 overflow-y-auto pr-0.5">
          <MarkdownBlock text={state.result.summary} />
        </div>
        <div className="flex items-center justify-between border-t border-border pt-2">
          <p className="text-xs text-text-3">On-Device AI</p>
          <div className="flex gap-2">
            <button onClick={() => void analyze(report)} className="text-xs text-accent hover:brightness-110">
              재분석
            </button>
            <button onClick={reset} className="text-xs text-text-3 hover:text-text-2">
              닫기
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (state.status === "error") {
    return (
      <div className="space-y-3">
        <p className="text-sm text-danger">{state.message}</p>
        <div className="flex gap-2">
          <button
            onClick={() => void start(report)}
            className="rounded-lg border border-danger px-3 py-1.5 text-xs text-danger transition hover:bg-danger-bg"
          >
            다시 시도
          </button>
          <button
            onClick={reset}
            className="rounded-lg border border-border px-3 py-1.5 text-xs text-text-2 transition hover:bg-hover"
          >
            닫기
          </button>
        </div>
      </div>
    );
  }

  // idle
  return (
    <div className="flex items-center justify-between">
      <p className="text-sm text-text-2">현재 보안 결과를 AI로 심층 분석합니다.</p>
      <button
        onClick={() => void start(report)}
        className="ml-4 flex-shrink-0 rounded-xl bg-accent px-4 py-2 text-sm font-medium text-accent-ink transition hover:brightness-105"
      >
        분석 시작
      </button>
    </div>
  );
}

// ── 탭 2: 보안 로드맵 ─────────────────────────────────────────

function RoadmapTab({ report }: { report: ScanReport }) {
  const { roadmap, generateRoadmap, resetRoadmap } = useLlmStore();

  if (roadmap.status === "loading") return <LoadingSpinner message="AI 분석 중..." />;
  if (roadmap.status === "done") {
    return (
      <div className="space-y-3">
        <div className="max-h-[440px] space-y-2 overflow-y-auto pr-0.5">
          <MarkdownBlock text={roadmap.summary} />
        </div>
        <div className="flex items-center justify-between border-t border-border pt-2">
          <p className="text-xs text-text-3">On-Device AI</p>
          <div className="flex gap-2">
            <button onClick={() => void generateRoadmap(report)} className="text-xs text-accent hover:brightness-110">
              재생성
            </button>
            <button onClick={resetRoadmap} className="text-xs text-text-3 hover:text-text-2">
              닫기
            </button>
          </div>
        </div>
      </div>
    );
  }
  if (roadmap.status === "error") {
    return (
      <div className="space-y-3">
        <p className="text-sm text-danger">{roadmap.message}</p>
        <button onClick={resetRoadmap} className="text-xs text-text-3 hover:text-text-2">
          닫기
        </button>
      </div>
    );
  }

  // idle
  return (
    <div className="space-y-3">
      <p className="text-sm leading-relaxed text-text-2">
        현재 보안 상태를 기반으로{" "}
        <strong className="font-medium text-text">즉시 조치 / 단기 개선 / 장기 강화</strong> 3단계 로드맵을
        생성합니다.
      </p>
      <button
        onClick={() => void generateRoadmap(report)}
        className="rounded-xl bg-accent px-4 py-2 text-sm font-medium text-accent-ink transition hover:brightness-105"
      >
        로드맵 생성
      </button>
    </div>
  );
}

// ── 탭 3: Q&A ─────────────────────────────────────────────────

const SUGGESTED_QUESTIONS = [
  "지금 가장 위험한 취약점이 무엇인가요?",
  "BitLocker를 활성화하는 방법을 단계별로 알려주세요.",
  "방화벽 설정을 강화하려면 어떻게 해야 하나요?",
  "RDP가 켜져 있으면 어떤 위험이 있나요?",
];

function QaTab({ report }: { report: ScanReport }) {
  const { qa, askQuestion, clearQa } = useLlmStore();
  const inputRef = useRef<HTMLInputElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [qa.history, qa.loading]);

  const submit = async (q?: string) => {
    const rawVal = q ?? inputRef.current?.value ?? "";
    const question = rawVal.trim();
    if (!question || qa.loading) return;
    
    if (inputRef.current) inputRef.current.value = "";
    await askQuestion(report, question);
  };

  return (
    <div className="space-y-3">
      {/* 대화 히스토리 */}
      {qa.history.length > 0 && (
        <div className="max-h-[360px] space-y-4 overflow-y-auto pr-0.5">
          {qa.history.map((msg) => (
            <div key={msg.id} className="space-y-2">
              {/* 질문 버블 — 전송 즉시 표시 */}
              <div className="flex justify-end">
                <div className="max-w-[85%] whitespace-pre-wrap rounded-2xl rounded-br-sm bg-accent px-3.5 py-2.5 text-sm leading-relaxed text-accent-ink">
                  {msg.question}
                </div>
              </div>
              {/* 답변 버블 — pending(로딩) / 에러 / 완료 */}
              <div className="flex justify-start">
                {msg.answer == null && msg.error == null ? (
                  <div className="rounded-2xl rounded-bl-sm border border-border bg-surface-2 px-3.5 py-2.5">
                    <LoadingSpinner message="AI가 답변을 작성하는 중..." />
                  </div>
                ) : msg.error != null ? (
                  <div className="max-w-[92%] rounded-2xl rounded-bl-sm border border-danger bg-danger-bg px-3.5 py-2.5 text-xs text-danger">
                    답변 생성 실패: {msg.error}
                  </div>
                ) : (
                  <div className="max-w-[92%] space-y-1.5 rounded-2xl rounded-bl-sm border border-border bg-surface-2 px-3.5 py-2.5">
                    <div className="mb-1 flex items-center gap-1">
                      <IconSparkle size={12} className="text-accent" />
                      <span className="text-xs font-medium text-accent">AI 답변</span>
                    </div>
                    <MarkdownBlock text={msg.answer ?? ""} compact />
                  </div>
                )}
              </div>
            </div>
          ))}
          <div ref={bottomRef} />
        </div>
      )}

      {/* 추천 질문 */}
      {qa.history.length === 0 && !qa.loading && (
        <div className="space-y-2">
          <p className="text-xs text-text-2">현재 보안 상태에 대해 무엇이든 물어보세요.</p>
          <div className="grid grid-cols-1 gap-1.5">
            {SUGGESTED_QUESTIONS.map((q) => (
              <button
                key={q}
                onClick={() => void submit(q)}
                className="rounded-lg border border-border px-3 py-2 text-left text-xs text-text-2 transition hover:border-accent hover:bg-hover"
              >
                {q}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* 입력창 */}
      <form
        className="flex gap-2 pt-1"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <input
          ref={inputRef}
          type="text"
          defaultValue=""
          placeholder="보안에 관해 질문하세요..."
          disabled={qa.loading}
          className="flex-1 rounded-xl border border-border bg-surface px-3 py-2 text-sm text-text placeholder-text-3 transition focus:border-accent focus:outline-none disabled:opacity-50"
        />
        <button
          type="submit"
          disabled={qa.loading}
          className="rounded-xl bg-accent px-3 py-2 text-sm font-medium text-accent-ink transition hover:brightness-105 disabled:cursor-not-allowed disabled:opacity-40"
        >
          전송
        </button>
      </form>

      {/* 대화 초기화 */}
      {qa.history.length > 0 && (
        <div className="flex items-center justify-between">
          <p className="text-xs text-text-3">On-Device AI</p>
          <button onClick={clearQa} className="text-xs text-text-3 hover:text-text-2">
            대화 초기화
          </button>
        </div>
      )}
    </div>
  );
}

// ── 메인 패널 ─────────────────────────────────────────────────

export const LlmPanel = ({ report }: Props) => {
  const { tab, setTab, state, roadmap, qa } = useLlmStore();

  const isBusy = state.status === "loading_model" || state.status === "analyzing" || roadmap.status === "loading" || qa.loading;

  return (
    <div className="space-y-4">
      <TabBar active={tab} disabled={isBusy} onChange={setTab} />
      {tab === "analyze" && <AnalyzeTab report={report} />}
      {tab === "roadmap" && <RoadmapTab report={report} />}
      {tab === "qa" && <QaTab report={report} />}
    </div>
  );
};
