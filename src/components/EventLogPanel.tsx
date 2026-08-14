import { useState } from "react";
import { useEventLogStore, type EventLogEntry } from "@/store/eventlogStore";
import { MarkdownBlock } from "./MarkdownBlock";
import { IconSparkle, IconSearch } from "./icons";

// ── 이벤트 ID 설명 매핑 ───────────────────────────────────────

const EVENT_DESCRIPTIONS: Record<number, string> = {
  4625: "로그인 실패",
  4648: "명시적 자격증명 로그온",
  4697: "서비스 설치",
  4698: "예약 작업 생성",
  4720: "계정 생성",
  4732: "보안 그룹 구성원 추가",
  4740: "계정 잠금",
  7045: "새 서비스 설치",
};

// ── 위험도 배지 ───────────────────────────────────────────────

function LevelBadge({ level }: { level: string }) {
  if (level === "고위험") {
    return <span className="inline-flex items-center whitespace-nowrap rounded bg-danger-bg px-1.5 py-0.5 text-xs font-semibold text-danger">고위험</span>;
  }
  if (level === "주의") {
    return <span className="inline-flex items-center whitespace-nowrap rounded bg-warn-bg px-1.5 py-0.5 text-xs font-semibold text-warn">주의</span>;
  }
  return <span className="inline-flex items-center whitespace-nowrap rounded bg-surface-2 px-1.5 py-0.5 text-xs text-text-3">정보</span>;
}

// ── 이벤트 테이블 ─────────────────────────────────────────────

type FilterType = "all" | "고위험" | "주의";

function EventLogTable({ entries }: { entries: EventLogEntry[] }) {
  const [showAll, setShowAll] = useState(false);
  const [filter, setFilter] = useState<FilterType>("all");

  const highCount = entries.filter((e) => e.level === "고위험").length;
  const warnCount = entries.filter((e) => e.level === "주의").length;

  const stats: Record<number, number> = {};
  for (const e of entries) stats[e.event_id] = (stats[e.event_id] ?? 0) + 1;

  const filtered = filter === "all" ? entries : entries.filter((e) => e.level === filter);
  const visible = showAll ? filtered : filtered.slice(0, 20);

  const filterBtn = (f: FilterType, label: string, count: number) => (
    <button
      onClick={() => { setFilter(f); setShowAll(false); }}
      className={`rounded-full px-3 py-1 text-xs font-medium transition ${
        filter === f ? "bg-accent text-accent-ink" : "bg-surface-2 text-text-2 hover:bg-hover"
      }`}
    >
      {label} ({count})
    </button>
  );

  return (
    <div className="mt-4 space-y-4">
      {/* 요약 통계 */}
      <div className="grid grid-cols-3 gap-3">
        <div className="rounded-[13px] border border-border bg-surface p-3 text-center">
          <div className="text-xl font-bold text-text">{entries.length}</div>
          <div className="mt-0.5 text-xs text-text-3">전체 이벤트</div>
        </div>
        <div className="rounded-[13px] border border-danger bg-surface p-3 text-center">
          <div className="text-xl font-bold text-danger">{highCount}</div>
          <div className="mt-0.5 text-xs text-text-3">고위험</div>
        </div>
        <div className="rounded-[13px] border border-warn bg-surface p-3 text-center">
          <div className="text-xl font-bold text-warn">{warnCount}</div>
          <div className="mt-0.5 text-xs text-text-3">주의</div>
        </div>
      </div>

      {/* 이벤트 ID별 요약 */}
      {Object.keys(stats).length > 0 && (
        <div className="flex flex-wrap gap-2">
          {Object.entries(stats).sort(([, a], [, b]) => b - a).map(([id, count]) => (
            <span key={id} className="inline-flex items-center gap-1 rounded-full bg-surface-2 px-2 py-0.5 text-xs text-text-2">
              <span className="font-mono">{id}</span>
              <span className="text-text-3">·</span>
              <span>{EVENT_DESCRIPTIONS[Number(id)] ?? "기타"}</span>
              <span className="font-semibold text-text">{count}건</span>
            </span>
          ))}
        </div>
      )}

      {/* 필터 */}
      <div className="flex items-center gap-2">
        <button
          onClick={() => { setFilter("all"); setShowAll(false); }}
          className={`rounded-full px-3 py-1 text-xs font-medium transition ${
            filter === "all" ? "bg-accent text-accent-ink" : "bg-surface-2 text-text-2 hover:bg-hover"
          }`}
        >
          전체 ({entries.length})
        </button>
        {filterBtn("고위험", "고위험", highCount)}
        {filterBtn("주의", "주의", warnCount)}
      </div>

      {/* 이벤트 테이블 */}
      {entries.length === 0 ? (
        <div className="space-y-2 rounded-xl border border-border bg-surface-2 p-6 text-center">
          <p className="text-sm text-text-2">최근 24시간 동안 보안 관련 이벤트가 없거나 조회 권한이 없습니다.</p>
          <p className="text-xs text-text-3">보안 이벤트 로그(Security) 조회에는 관리자 권한이 필요합니다.</p>
        </div>
      ) : (
        <>
          <div className="overflow-x-auto rounded-[13px] border border-border">
            <table className="min-w-full text-xs">
              <thead>
                <tr className="bg-surface-2 text-left">
                  <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">시간</th>
                  <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">이벤트</th>
                  <th className="px-3 py-2 font-semibold text-text-2">내용</th>
                  <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">위험도</th>
                </tr>
              </thead>
              <tbody>
                {visible.map((e, i) => {
                  const isHigh = e.level === "고위험";
                  const isWarn = e.level === "주의";
                  return (
                    <tr key={i} className={isHigh ? "bg-danger-bg" : isWarn ? "bg-warn-bg" : i % 2 === 0 ? "bg-surface" : "bg-surface-2"}>
                      <td className="whitespace-nowrap px-3 py-2 font-mono text-text-2">
                        {e.time_created.slice(11)}
                        <div className="text-text-3">{e.time_created.slice(0, 10)}</div>
                      </td>
                      <td className="whitespace-nowrap px-3 py-2">
                        <div className="font-semibold text-text">{e.label}</div>
                        <div className="font-mono text-text-3">{e.event_id}</div>
                      </td>
                      <td className="max-w-[320px] px-3 py-2 text-text-2">
                        <div className="line-clamp-2" title={e.message}>
                          {e.message || <span className="italic text-text-3">내용 없음</span>}
                        </div>
                      </td>
                      <td className="whitespace-nowrap px-3 py-2">
                        <LevelBadge level={e.level} />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          {filtered.length > 20 && (
            <button onClick={() => setShowAll((v) => !v)} className="text-xs text-accent hover:underline">
              {showAll ? "▲ 줄이기" : `▼ 전체 보기 (${filtered.length - 20}개 더)`}
            </button>
          )}
        </>
      )}
    </div>
  );
}

const PHASE_MESSAGES: Record<string, string> = {
  scanning: "보안 이벤트 로그 수집 중...",
  loading_model: "AI 모델 로드 중...",
  analyzing: "AI 분석 중...",
};

// ── 메인 패널 ─────────────────────────────────────────────────

export function EventLogPanel() {
  const { state, analyze, reset } = useEventLogStore();

  return (
    <div className="w-full max-w-[1040px] space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-text">보안 이벤트 로그 분석</h2>
          <p className="mt-0.5 text-xs text-text-2">최근 24시간 Windows 보안 이벤트를 수집하고 AI가 이상 징후를 분석합니다.</p>
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
          <EventLogTable entries={state.entries} />
        </div>
      )}
    </div>
  );
}
