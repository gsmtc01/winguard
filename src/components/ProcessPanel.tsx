import { useState } from "react";
import { useProcessStore, type ProcessInfo } from "@/store/processStore";
import { MarkdownBlock } from "./MarkdownBlock";
import { IconSparkle, IconSearch } from "./icons";

// ── 서명 배지 ─────────────────────────────────────────────────

function SignedBadge({ signed }: { signed: boolean | null }) {
  if (signed === true) {
    return (
      <span className="inline-flex items-center gap-1 rounded bg-ok-bg px-1.5 py-0.5 text-xs font-semibold text-ok">
        서명됨
      </span>
    );
  }
  if (signed === false) {
    return (
      <span className="inline-flex items-center gap-1 rounded bg-danger-bg px-1.5 py-0.5 text-xs font-semibold text-danger">
        미서명
      </span>
    );
  }
  return <span className="rounded bg-surface-2 px-1.5 py-0.5 text-xs text-text-3">?</span>;
}

// ── 프로세스 테이블 ────────────────────────────────────────────

function ProcessTable({ processes }: { processes: ProcessInfo[] }) {
  const [showAll, setShowAll] = useState(false);

  const sorted = [...processes].sort((a, b) => {
    const aRisk = a.signed === false ? 1 : 0;
    const bRisk = b.signed === false ? 1 : 0;
    if (aRisk !== bRisk) return bRisk - aRisk;
    return b.cpu - a.cpu;
  });

  const visible = showAll ? sorted : sorted.slice(0, 20);

  return (
    <div className="mt-4">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-text">실행 중인 프로세스 ({processes.length}개)</h3>
        <span className="text-xs text-text-3">미서명 {processes.filter((p) => p.signed === false).length}개</span>
      </div>
      <div className="overflow-x-auto rounded-[13px] border border-border">
        <table className="min-w-full text-xs">
          <thead>
            <tr className="bg-surface-2 text-left">
              <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">프로세스</th>
              <th className="px-3 py-2 font-semibold text-text-2">회사</th>
              <th className="whitespace-nowrap px-3 py-2 text-right font-semibold text-text-2">CPU</th>
              <th className="whitespace-nowrap px-3 py-2 text-right font-semibold text-text-2">메모리</th>
              <th className="px-3 py-2 font-semibold text-text-2">서명</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((p, i) => {
              const isUnsigned = p.signed === false;
              return (
                <tr key={`${p.pid}-${i}`} className={isUnsigned ? "bg-danger-bg" : i % 2 === 0 ? "bg-surface" : "bg-surface-2"}>
                  <td className="whitespace-nowrap px-3 py-1.5 font-medium text-text">
                    <span title={p.path ?? undefined}>{p.name}</span>
                    <span className="ml-1.5 text-text-3">({p.pid})</span>
                  </td>
                  <td className="max-w-[180px] truncate px-3 py-1.5 text-text-2">
                    {p.company ?? <span className="italic text-text-3">알 수 없음</span>}
                  </td>
                  <td className={`px-3 py-1.5 text-right font-mono tabular-nums ${p.cpu > 10 ? "font-semibold text-danger" : "text-text-2"}`}>
                    {p.cpu.toFixed(1)}%
                  </td>
                  <td className="px-3 py-1.5 text-right font-mono tabular-nums text-text-2">{p.memory_mb.toFixed(0)}MB</td>
                  <td className="px-3 py-1.5">
                    <SignedBadge signed={p.signed} />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      {sorted.length > 20 && (
        <button onClick={() => setShowAll((v) => !v)} className="mt-2 text-xs text-accent hover:underline">
          {showAll ? "▲ 줄이기" : `▼ 전체 보기 (${sorted.length - 20}개 더)`}
        </button>
      )}
    </div>
  );
}

const PHASE_MESSAGES: Record<string, string> = {
  scanning: "프로세스 스캔 중...",
  loading_model: "AI 모델 로드 중...",
  analyzing: "AI 분석 중...",
};

// ── 메인 패널 ─────────────────────────────────────────────────

export function ProcessPanel() {
  const { state, analyze, reset } = useProcessStore();

  return (
    <div className="w-full max-w-[1040px] space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-text">실행 프로세스 이상 탐지</h2>
          <p className="mt-0.5 text-xs text-text-2">실행 중인 프로세스를 스캔하고 AI가 의심 프로세스를 분석합니다.</p>
        </div>
        {state.status === "done" && (
          <button onClick={reset} className="rounded border border-border px-2 py-1 text-xs text-text-2 transition hover:bg-hover">
            초기화
          </button>
        )}
      </div>

      {state.status === "idle" && (
        <button
          onClick={() => void analyze()}
          className="flex w-full items-center justify-center gap-2 rounded-xl bg-accent py-3 text-sm font-semibold text-accent-ink transition hover:brightness-105"
        >
          <IconSearch size={16} /> 스캔 및 AI 분석
        </button>
      )}

      {state.status === "loading" && (
        <div className="flex flex-col items-center gap-3 py-10">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-accent-soft border-t-accent" style={{ willChange: "transform" }} />
          <p className="text-sm font-medium text-accent">{PHASE_MESSAGES[state.phase] ?? "처리 중..."}</p>
        </div>
      )}

      {state.status === "error" && (
        <div className="space-y-3 rounded-xl border border-danger bg-danger-bg p-4">
          <p className="text-sm text-danger">{state.message}</p>
          <button onClick={reset} className="text-xs text-danger underline hover:brightness-110">
            다시 시도
          </button>
        </div>
      )}

      {state.status === "done" && (
        <div className="space-y-4">
          <div className="rounded-[14px] border border-accent bg-accent-soft p-4">
            <div className="mb-3 flex items-center gap-1.5">
              <IconSparkle size={14} className="text-accent" />
              <span className="text-sm font-semibold text-accent">AI 분석 결과</span>
              <span className="rounded-full bg-surface px-1.5 py-0.5 text-xs text-accent">On-Device</span>
            </div>
            <div className="max-h-80 overflow-y-auto">
              <MarkdownBlock text={state.summary} compact />
            </div>
            <div className="mt-3 border-t border-accent pt-2">
              <span className="text-xs text-text-3">On-Device AI</span>
            </div>
          </div>
          <ProcessTable processes={state.processes} />
        </div>
      )}
    </div>
  );
}
