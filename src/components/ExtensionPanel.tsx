import { useState } from "react";
import { useExtensionStore, getExtRisk, type BrowserExtension } from "@/store/extensionStore";
import { MarkdownBlock } from "./MarkdownBlock";
import { IconSparkle, IconSearch } from "./icons";

// ── 위험도 배지 ───────────────────────────────────────────────

function RiskBadge({ risk }: { risk: "high" | "medium" | "low" }) {
  if (risk === "high") {
    return <span className="inline-flex items-center rounded bg-danger-bg px-1.5 py-0.5 text-xs font-semibold text-danger">고위험</span>;
  }
  if (risk === "medium") {
    return <span className="inline-flex items-center rounded bg-warn-bg px-1.5 py-0.5 text-xs font-semibold text-warn">주의</span>;
  }
  return <span className="inline-flex items-center rounded bg-surface-2 px-1.5 py-0.5 text-xs text-text-3">낮음</span>;
}

// ── 브라우저 배지 ─────────────────────────────────────────────

function BrowserBadge({ browser }: { browser: string }) {
  const styles: Record<string, string> = {
    Chrome: "bg-info-bg text-info",
    Edge: "bg-ok-bg text-ok",
    Firefox: "bg-warn-bg text-warn",
  };
  const cls = styles[browser] ?? "bg-surface-2 text-text-3";
  return <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium ${cls}`}>{browser}</span>;
}

// ── 확장 테이블 ───────────────────────────────────────────────

type FilterType = "all" | "high" | "medium";

function ExtensionTable({ extensions }: { extensions: BrowserExtension[] }) {
  const [showAll, setShowAll] = useState(false);
  const [filter, setFilter] = useState<FilterType>("all");

  const withRisk = extensions.map((e) => ({ ...e, risk: getExtRisk(e.permissions) }));

  const highCount = withRisk.filter((e) => e.risk === "high").length;
  const medCount = withRisk.filter((e) => e.risk === "medium").length;

  const sorted = [...withRisk].sort((a, b) => {
    const order = { high: 0, medium: 1, low: 2 };
    if (order[a.risk] !== order[b.risk]) return order[a.risk] - order[b.risk];
    return a.name.localeCompare(b.name);
  });

  const filtered = filter === "all" ? sorted : sorted.filter((e) => e.risk === filter);
  const visible = showAll ? filtered : filtered.slice(0, 20);

  const filterBtn = (f: FilterType, label: string, count?: number) => (
    <button
      onClick={() => { setFilter(f); setShowAll(false); }}
      className={`rounded-full px-3 py-1 text-xs font-medium transition ${
        filter === f ? "bg-accent text-accent-ink" : "bg-surface-2 text-text-2 hover:bg-hover"
      }`}
    >
      {label}{count !== undefined ? ` (${count})` : ""}
    </button>
  );

  return (
    <div className="mt-4">
      {/* 요약 통계 */}
      <div className="mb-4 grid grid-cols-3 gap-3">
        <div className="rounded-[13px] border border-border bg-surface p-3 text-center">
          <div className="text-xl font-bold text-text">{extensions.length}</div>
          <div className="mt-0.5 text-xs text-text-3">전체 확장</div>
        </div>
        <div className="rounded-[13px] border border-danger bg-surface p-3 text-center">
          <div className="text-xl font-bold text-danger">{highCount}</div>
          <div className="mt-0.5 text-xs text-text-3">고위험</div>
        </div>
        <div className="rounded-[13px] border border-warn bg-surface p-3 text-center">
          <div className="text-xl font-bold text-warn">{medCount}</div>
          <div className="mt-0.5 text-xs text-text-3">주의</div>
        </div>
      </div>

      {/* 필터 */}
      <div className="mb-3 flex items-center gap-2">
        {filterBtn("all", "전체", extensions.length)}
        {filterBtn("high", "고위험", highCount)}
        {filterBtn("medium", "주의", medCount)}
      </div>

      {/* 테이블 */}
      <div className="overflow-x-auto rounded-[13px] border border-border">
        <table className="min-w-full text-xs">
          <thead>
            <tr className="bg-surface-2 text-left">
              <th className="px-3 py-2 font-semibold text-text-2">확장 프로그램</th>
              <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">브라우저</th>
              <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">버전</th>
              <th className="px-3 py-2 font-semibold text-text-2">권한</th>
              <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">위험도</th>
            </tr>
          </thead>
          <tbody>
            {visible.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-3 py-6 text-center text-text-3">
                  해당 항목 없음
                </td>
              </tr>
            ) : (
              visible.map((e, i) => {
                const isHigh = e.risk === "high";
                const isMed = e.risk === "medium";
                return (
                  <tr key={`${e.browser}-${e.id}-${i}`} className={isHigh ? "bg-danger-bg" : isMed ? "bg-warn-bg" : i % 2 === 0 ? "bg-surface" : "bg-surface-2"}>
                    <td className="max-w-[200px] px-3 py-2 font-medium text-text">
                      <div className="truncate" title={e.name}>{e.name}</div>
                      {!e.enabled && <span className="text-xs text-text-3">(비활성)</span>}
                    </td>
                    <td className="whitespace-nowrap px-3 py-2">
                      <BrowserBadge browser={e.browser} />
                    </td>
                    <td className="whitespace-nowrap px-3 py-2 font-mono text-text-3">{e.version || "—"}</td>
                    <td className="max-w-[260px] px-3 py-2 text-text-2">
                      {e.permissions ? (
                        <span className="block truncate" title={e.permissions}>{e.permissions}</span>
                      ) : (
                        <span className="italic text-text-3">없음</span>
                      )}
                    </td>
                    <td className="whitespace-nowrap px-3 py-2">
                      <RiskBadge risk={e.risk} />
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {filtered.length > 20 && (
        <button onClick={() => setShowAll((v) => !v)} className="mt-2 text-xs text-accent hover:underline">
          {showAll ? "▲ 줄이기" : `▼ 전체 보기 (${filtered.length - 20}개 더)`}
        </button>
      )}
    </div>
  );
}

const PHASE_MESSAGES: Record<string, string> = {
  scanning: "브라우저 확장 프로그램 스캔 중...",
  loading_model: "AI 모델 로드 중...",
  analyzing: "AI 분석 중...",
};

// ── 메인 패널 ─────────────────────────────────────────────────

export function ExtensionPanel() {
  const { state, analyze, reset } = useExtensionStore();

  return (
    <div className="w-full max-w-[1040px] space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-text">브라우저 확장 프로그램 분석</h2>
          <p className="mt-0.5 text-xs text-text-2">Chrome · Edge · Firefox 확장을 스캔하고 AI가 위험 권한을 분석합니다.</p>
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
          <ExtensionTable extensions={state.extensions} />
        </div>
      )}
    </div>
  );
}
