import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { SeverityBadge } from "./SeverityBadge";
import { FixButton } from "./FixButton";
import { MarkdownBlock } from "./MarkdownBlock";
import { IconSparkle, IconClose } from "./icons";
import { severityConfig } from "@/lib/severity";
import type { CheckResult } from "@/store/checkStore";
import type { LlmAnalysis } from "@/store/llmStore";

// ── AI 설명 패널 ───────────────────────────────────────────────

type ExplainState =
  | { status: "idle" }
  | { status: "loading"; phase: "loading_model" | "analyzing" }
  | { status: "done"; summary: string }
  | { status: "error"; message: string };

function AiExplainPanel({ check }: { check: CheckResult }) {
  const [explainState, setExplainState] = useState<ExplainState>({ status: "idle" });

  const handleExplain = async () => {
    if (explainState.status === "loading") return;

    const exists = await invoke<boolean>("llm_model_exists").catch(() => false);
    if (!exists) {
      setExplainState({
        status: "error",
        message: "AI 모델이 설치되지 않았습니다. 설정에서 On-Device AI 모델을 다운로드하세요.",
      });
      return;
    }

    setExplainState({ status: "loading", phase: "loading_model" });

    const unlisten = await listen<string>("llm:phase", (e) => {
      if (e.payload === "analyzing") setExplainState({ status: "loading", phase: "analyzing" });
    });

    try {
      const result = await invoke<LlmAnalysis>("llm_explain_check", { check });
      setExplainState({ status: "done", summary: result.summary });
    } catch (e) {
      setExplainState({ status: "error", message: String(e) });
    } finally {
      unlisten();
    }
  };

  const phaseMsg =
    explainState.status === "loading" && explainState.phase === "loading_model"
      ? "AI 모델 로드 중..."
      : "항목 분석 중...";

  return (
    <div className="mt-2">
      {explainState.status === "idle" && (
        <button
          onClick={() => void handleExplain()}
          className="flex items-center gap-1 text-xs text-accent transition hover:brightness-110"
        >
          <IconSparkle size={13} />
          AI 상세 설명
        </button>
      )}

      {explainState.status === "loading" && (
        <div className="mt-1 flex items-center gap-2">
          <div className="h-3 w-3 flex-shrink-0 animate-spin rounded-full border-2 border-accent-soft border-t-accent" style={{ willChange: "transform" }} />
          <p className="text-xs text-accent">{phaseMsg}</p>
        </div>
      )}

      {explainState.status === "done" && (
        <div className="mt-2 rounded-lg border border-accent bg-accent-soft p-3">
          <div className="mb-2 flex items-center justify-between">
            <div className="flex items-center gap-1">
              <IconSparkle size={12} className="text-accent" />
              <span className="text-xs font-semibold text-accent">AI 설명</span>
              <span className="ml-1 rounded-full bg-surface px-1.5 py-0.5 text-xs text-accent">On-Device</span>
            </div>
            <button onClick={() => setExplainState({ status: "idle" })} className="text-xs text-accent transition hover:brightness-110">
              닫기
            </button>
          </div>
          <div className="max-h-64 space-y-1.5 overflow-y-auto pr-0.5">
            <MarkdownBlock text={explainState.summary} compact />
          </div>
          <div className="mt-2 flex items-center justify-between border-t border-accent pt-2">
            <span className="text-xs text-text-3">On-Device AI</span>
            <button onClick={() => void handleExplain()} className="text-xs text-accent transition hover:brightness-110">
              재분석
            </button>
          </div>
        </div>
      )}

      {explainState.status === "error" && (
        <div className="mt-2 flex items-start justify-between gap-2 rounded-lg border border-danger bg-danger-bg p-2.5">
          <p className="text-xs text-danger">{explainState.message}</p>
          <button onClick={() => setExplainState({ status: "idle" })} className="flex-shrink-0 text-danger transition hover:brightness-110">
            <IconClose size={13} />
          </button>
        </div>
      )}
    </div>
  );
}

// ── CheckCard ─────────────────────────────────────────────────

const AI_EXPLAIN_SEVERITIES = new Set(["warning", "danger", "critical"]);

export const CheckCard = ({ check }: { check: CheckResult }) => {
  const [showEvidence, setShowEvidence] = useState(false);
  const cfg = severityConfig[check.severity];
  const hasEvidence = Object.keys(check.evidence).length > 0;
  const canExplain = AI_EXPLAIN_SEVERITIES.has(check.severity);

  return (
    <div className={`space-y-3 rounded-[14px] border bg-surface p-5 ${cfg.border}`}>
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="mb-1 flex items-center gap-2">
            <SeverityBadge severity={check.severity} />
            {cfg.deduction && (
              <span className="font-mono text-xs font-semibold text-text-3">{cfg.deduction}</span>
            )}
            <h3 className="text-sm font-semibold text-text">{check.title}</h3>
          </div>
          <p className="text-sm text-text-2">{check.message}</p>
        </div>
        <FixButton uri={check.action_uri} />
      </div>

      {hasEvidence && (
        <button onClick={() => setShowEvidence((v) => !v)} className="text-xs text-text-2 transition hover:text-text">
          {showEvidence ? "▲ 근거 숨기기" : "▼ 근거 보기"}
        </button>
      )}

      {showEvidence && hasEvidence && (
        <div className="space-y-1 rounded-lg bg-surface-2 px-3 py-2">
          {Object.entries(check.evidence).map(([k, v]) => (
            <div key={k} className="flex gap-2 break-all font-mono text-xs text-text-2">
              <span className="shrink-0 text-text-3">{k}:</span>
              <span>{v}</span>
            </div>
          ))}
        </div>
      )}

      {canExplain && <AiExplainPanel check={check} />}
    </div>
  );
};
