import { useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { generateReport } from "@/api/llm";
import { useCheckStore } from "@/store/checkStore";
import { writeTextFile } from "@/api/files";
import { CheckCard } from "./CheckCard";
import { FixButton } from "./FixButton";
import { IconSearch, IconShieldAlert, IconLock, IconInfo } from "./icons";
import { severityConfig } from "@/lib/severity";
import { summarizeByArea } from "@/lib/areas";
import { reportToCsv, buildFilename } from "@/lib/export";
import { loadHistory, type HistoryEntry } from "@/lib/history";
import { onDataReset } from "@/store/dataReset";
import type { CheckResult } from "@/store/checkStore";

// ── 보안 점수 추이 차트 ───────────────────────────────────────

function ScoreHistoryChart({ entries }: { entries: HistoryEntry[] }) {
  if (entries.length < 2) return null;

  const last = entries.slice(-15);
  const W = 480;
  const H = 90;
  const PL = 28;
  const PR = 10;
  const PT = 10;
  const PB = 18;
  const innerW = W - PL - PR;
  const innerH = H - PT - PB;

  const xStep = innerW / Math.max(last.length - 1, 1);
  const toX = (i: number) => PL + i * xStep;
  const toY = (score: number) => PT + ((100 - score) / 100) * innerH;

  const points = last.map((e, i) => ({
    x: toX(i),
    y: toY(e.score),
    score: e.score,
    date: new Date(e.scanned_at * 1000),
  }));

  const scoreColor = (s: number) => (s >= 80 ? "var(--ok)" : s >= 60 ? "var(--warn)" : "var(--danger)");
  const latest = last[last.length - 1];
  const lineColor = scoreColor(latest.score);

  const pathD = points.map((p, i) => `${i === 0 ? "M" : "L"} ${p.x.toFixed(1)} ${p.y.toFixed(1)}`).join(" ");
  const fillD =
    pathD +
    ` L ${points[points.length - 1].x.toFixed(1)} ${(H - PB).toFixed(1)}` +
    ` L ${points[0].x.toFixed(1)} ${(H - PB).toFixed(1)} Z`;

  const y80 = toY(80).toFixed(1);
  const y60 = toY(60).toFixed(1);

  const fmtDate = (d: Date) =>
    `${String(d.getMonth() + 1).padStart(2, "0")}/${String(d.getDate()).padStart(2, "0")}`;

  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <p className="text-xs font-medium text-text-2">최근 보안 점수 추이</p>
        <p className="text-xs text-text-3">{last.length}회 기록</p>
      </div>
      <svg viewBox={`0 0 ${W} ${H}`} className="w-full" style={{ height: 90 }} aria-label="보안 점수 추이 차트">
        <defs>
          <linearGradient id="chartGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={lineColor} stopOpacity="0.18" />
            <stop offset="100%" stopColor={lineColor} stopOpacity="0.01" />
          </linearGradient>
        </defs>

        <text x={PL - 4} y={Number(y80) + 3.5} fontSize="8" fill="var(--text-3)" textAnchor="end">80</text>
        <text x={PL - 4} y={Number(y60) + 3.5} fontSize="8" fill="var(--text-3)" textAnchor="end">60</text>

        <line x1={PL} y1={y80} x2={W - PR} y2={y80} stroke="var(--border)" strokeWidth="0.8" strokeDasharray="4,3" />
        <line x1={PL} y1={y60} x2={W - PR} y2={y60} stroke="var(--border)" strokeWidth="0.8" strokeDasharray="4,3" />

        <path d={fillD} fill="url(#chartGrad)" />
        <path d={pathD} fill="none" stroke={lineColor} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />

        {points.map((p, i) => {
          const isLast = i === points.length - 1;
          return (
            <g key={i}>
              <circle cx={p.x} cy={p.y} r={isLast ? 4.5 : 3} fill={scoreColor(p.score)} stroke="var(--surface)" strokeWidth="1.2">
                <title>{p.score}점 ({fmtDate(p.date)})</title>
              </circle>
              {isLast && (
                <text x={p.x} y={p.y - 8} fontSize="9" fontWeight="700" fill={scoreColor(p.score)} textAnchor="middle">
                  {p.score}
                </text>
              )}
            </g>
          );
        })}

        <text x={points[0].x} y={H - 2} fontSize="8" fill="var(--text-3)" textAnchor="middle">{fmtDate(points[0].date)}</text>
        <text x={points[points.length - 1].x} y={H - 2} fontSize="8" fill="var(--text-3)" textAnchor="middle">
          {fmtDate(points[points.length - 1].date)}
        </text>
      </svg>
    </div>
  );
}

// ── 점수 게이지 ───────────────────────────────────────────────

const GAUGE_C = 2 * Math.PI * 138; // 둘레 ≈ 867.08

function ScoreGauge({ score }: { score: number | null }) {
  const pct = score == null ? 0 : Math.max(0, Math.min(100, score)) / 100;
  const dash = (GAUGE_C * pct).toFixed(1);

  const tone =
    score == null
      ? { label: "점수 대기", color: "var(--text-3)" }
      : score >= 80
      ? { label: "안전합니다", color: "var(--ok)" }
      : score >= 60
      ? { label: "주의가 필요해요", color: "var(--warn)" }
      : { label: "위험합니다", color: "var(--danger)" };

  return (
    <div className="relative h-[300px] w-[300px]">
      <svg width="300" height="300" viewBox="0 0 320 320">
        <circle cx="160" cy="160" r="138" fill="none" stroke="var(--score-track)" strokeWidth="16" />
        <circle
          cx="160"
          cy="160"
          r="138"
          fill="none"
          stroke={tone.color}
          strokeWidth="16"
          strokeLinecap="round"
          strokeDasharray={`${dash} ${GAUGE_C.toFixed(1)}`}
          transform="rotate(-90 160 160)"
          style={{ transition: "stroke-dasharray .6s ease" }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <div className="text-[13px] font-bold tracking-[.02em]" style={{ color: tone.color }}>
          {tone.label}
        </div>
        <div className="mt-1.5 text-[98px] font-extrabold leading-[.95] tracking-[-.04em] text-text">
          {score == null ? "—" : score}
        </div>
        <div className="mt-1.5 text-[12.5px] font-semibold text-text-3">100점 만점</div>
      </div>
    </div>
  );
}

// ── 조치 아이템 ───────────────────────────────────────────────

function ActionItem({ check }: { check: CheckResult }) {
  const cfg = severityConfig[check.severity];
  const Icon = check.severity === "warning" ? IconLock : check.severity === "info" ? IconInfo : IconShieldAlert;
  return (
    <div className="flex items-center gap-[15px] rounded-[14px] border border-border bg-surface px-[17px] py-[15px]">
      <span className={`flex h-[42px] w-[42px] flex-none items-center justify-center rounded-[11px] ${cfg.bg} ${cfg.text}`}>
        <Icon size={20} />
      </span>
      <div className="min-w-0 flex-1">
        <div className="text-[14.5px] font-bold text-text">{check.title}</div>
        <div className="mt-0.5 truncate text-[12.5px] text-text-2">{check.message}</div>
      </div>
      {cfg.deduction && (
        <span className={`font-mono text-[11.5px] font-bold ${cfg.text}`}>{cfg.deduction.replace("점", "")}</span>
      )}
      <FixButton uri={check.action_uri} />
    </div>
  );
}

// ── 영역별 상태 ───────────────────────────────────────────────
//
// 심각도(정상/경고/위험/심각) 분류는 게이지·조치 목록·전체 점검 항목에서
// 이미 보여주므로 여기서 반복하지 않는다. 대신 "어느 분야가 약한가"를
// 보안 영역 단위로 묶어(@/lib/areas) 다른 축의 정보를 준다.

export function AreaStatusGrid({ checks }: { checks: CheckResult[] }) {
  const summaries = summarizeByArea(checks);
  if (summaries.length === 0) return null;

  return (
    <div className="w-full">
      <div className="mb-3 flex items-baseline gap-2">
        <span className="text-[13.5px] font-extrabold text-text">영역별 상태</span>
        <span className="text-xs text-text-3">보안 분야별로 묶어 본 점검 결과</span>
      </div>
      <div className="grid grid-cols-3 gap-[14px]">
        {summaries.map(({ area, worst, total, issues }) => {
          const cfg = severityConfig[worst];
          const clean = issues === 0;
          const statusText = clean
            ? worst === "info"
              ? "참고 항목 있음"
              : "이상 없음"
            : `${cfg.label} ${issues}건`;
          return (
            <div
              key={area.id}
              className={`rounded-[13px] border bg-surface px-4 py-[15px] ${
                clean ? "border-border" : cfg.border
              }`}
            >
              <div className="text-[13px] font-bold text-text">{area.label}</div>
              <div className="mt-0.5 truncate text-[11.5px] text-text-3" title={area.desc}>
                {area.desc}
              </div>
              <div className="mt-2.5 flex items-center gap-[7px]">
                <span className={`h-2 w-2 flex-none rounded-full ${cfg.dot}`} />
                <span className={`text-[13px] font-bold ${clean ? "text-text-2" : cfg.text}`}>
                  {statusText}
                </span>
                <span className="ml-auto text-[11.5px] tabular-nums text-text-3">
                  {total - issues}/{total} 정상
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ── 메인 대시보드 ────────────────────────────────────────────

export const Dashboard = () => {
  const { report, isLoading, lastError, runScan } = useCheckStore();

  const [isExporting, setIsExporting] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [reportState, setReportState] = useState<"idle" | "generating" | "ready">("idle");
  const [reportSummary, setReportSummary] = useState<string | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>(() => loadHistory());
  const [showAllActions, setShowAllActions] = useState(false);
  const exportRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!exportOpen) return;
    const handler = (e: MouseEvent) => {
      if (exportRef.current && !exportRef.current.contains(e.target as Node)) setExportOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [exportOpen]);

  useEffect(() => {
    if (report) setHistory(loadHistory());
  }, [report]);

  // 설정 > 데이터 관리에서 기록을 지우면 차트도 즉시 비운다.
  // (localStorage 만 지우면 이 state 에 남은 값 때문에 차트가 그대로 보인다)
  useEffect(() => onDataReset(() => setHistory(loadHistory())), []);

  // 첫 진입 시 1회만 자동 스캔. 실패해도 재시도 루프에 빠지지 않도록
  // ref 로 가드한다(수동 "지금 다시 검사"로 재시도 가능).
  const autoScannedRef = useRef(false);
  useEffect(() => {
    if (autoScannedRef.current || report || isLoading) return;
    autoScannedRef.current = true;
    void runScan();
  }, [report, isLoading, runScan]);

  const handleExportCsv = async () => {
    if (!report || isExporting) return;
    setIsExporting(true);
    try {
      const path = await save({
        title: "점검 결과 저장",
        filters: [{ name: "CSV 파일", extensions: ["csv"] }],
        defaultPath: buildFilename(report),
      });
      if (path) await writeTextFile(path, reportToCsv(report));
    } catch {
      // 취소 무시
    } finally {
      setIsExporting(false);
    }
  };

  const reportDateStr = report ? new Date(report.scanned_at * 1000).toLocaleString("ko-KR") : "";
  const reportDateStamp = report
    ? (() => {
        const d = new Date(report.scanned_at * 1000);
        return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}`;
      })()
    : "";

  const handleGenerateReport = async () => {
    if (!report || reportState === "generating") return;
    setReportState("generating");
    try {
      const result = await generateReport(report);
      setReportSummary(result.summary);
      setReportState("ready");
    } catch {
      setReportState("idle");
    }
  };

  const handleSaveMarkdown = async () => {
    if (!report || !reportSummary) return;
    try {
      const path = await save({
        title: "보안 보고서 저장 (Markdown)",
        filters: [{ name: "Markdown 파일", extensions: ["md"] }],
        defaultPath: `WinGuard_보안보고서_${reportDateStamp}.md`,
      });
      if (path) {
        const header = `> 점검 일시: ${reportDateStr}  \n> 보안 점수: ${report.score}/100\n\n---\n\n`;
        await writeTextFile(path, header + reportSummary);
      }
    } catch {
      // 취소 무시
    }
  };

  // 조치가 필요한 항목 (심각 > 위험 > 경고 순)
  const priority: Record<string, number> = { critical: 0, danger: 1, warning: 2 };
  const allActionItems = report
    ? report.checks
        .filter((c) => c.severity in priority)
        .sort((a, b) => priority[a.severity] - priority[b.severity])
    : [];
  const actionItems = showAllActions ? allActionItems : allActionItems.slice(0, 3);
  const hiddenActionCount = allActionItems.length - actionItems.length;
  const issueCount = allActionItems.length;

  return (
    <div className="flex w-full max-w-[900px] flex-col items-center gap-6">
      {/* 상단 액션 바: 검사 시각 + 내보내기 */}
      <div className="flex w-full items-center gap-3">
        {report && (
          <span className="font-mono text-xs text-text-3">
            {new Date(report.scanned_at * 1000).toLocaleString("ko-KR")} 검사
          </span>
        )}
        {reportState === "generating" && (
          <div className="flex items-center gap-1.5 text-xs text-accent">
            <span className="h-3 w-3 animate-spin rounded-full border-2 border-accent-soft border-t-accent" style={{ willChange: "transform" }} />
            AI 보고서 생성 중...
          </div>
        )}
        {report && (
          <div className="relative ml-auto" ref={exportRef}>
            <button
              onClick={() => setExportOpen((v) => !v)}
              className="rounded-xl border border-border px-3 py-2 text-sm font-medium text-text-2 transition hover:bg-hover"
            >
              내보내기 {exportOpen ? "▲" : "▼"}
            </button>
            {exportOpen && (
              <div className="absolute right-0 top-full z-20 mt-1.5 w-56 overflow-hidden rounded-xl border border-border bg-surface shadow-lg">
                <div className="flex items-center justify-between px-4 py-3 transition hover:bg-hover">
                  <div>
                    <p className="text-sm font-medium text-text">CSV</p>
                    <p className="text-xs text-text-3">점검 항목 전체</p>
                  </div>
                  <button
                    onClick={() => { void handleExportCsv(); setExportOpen(false); }}
                    disabled={isExporting}
                    className="rounded-lg bg-surface-2 px-2.5 py-1 text-xs text-text-2 transition hover:bg-hover disabled:opacity-50"
                  >
                    {isExporting ? "저장 중…" : "저장"}
                  </button>
                </div>
                <div className="border-t border-border" />
                <div className="px-4 py-3 transition hover:bg-hover">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-text">AI 보고서</p>
                      <p className="text-xs text-text-3">Markdown (.md)</p>
                    </div>
                    {reportState === "idle" && (
                      <button
                        onClick={() => void handleGenerateReport()}
                        className="rounded-lg bg-accent-soft px-2.5 py-1 text-xs text-accent transition hover:brightness-105"
                      >
                        생성
                      </button>
                    )}
                    {reportState === "generating" && (
                      <span className="flex items-center gap-1.5 text-xs text-accent">
                        <span className="h-3 w-3 animate-spin rounded-full border-2 border-accent border-t-transparent" style={{ willChange: "transform" }} />
                        생성 중…
                      </span>
                    )}
                    {reportState === "ready" && (
                      <button
                        onClick={() => void handleSaveMarkdown()}
                        className="rounded-lg bg-accent-soft px-2.5 py-1 text-xs text-accent transition hover:brightness-105"
                      >
                        저장
                      </button>
                    )}
                  </div>
                  {reportState === "ready" && (
                    <div className="mt-1.5 flex items-center gap-2">
                      <span className="text-xs text-ok">생성 완료</span>
                      <button
                        onClick={() => { setReportState("idle"); setReportSummary(null); }}
                        className="text-xs text-text-3 underline hover:text-text-2"
                      >
                        재생성
                      </button>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* 게이지 + 헤드라인/조치 */}
      <div className="flex w-full items-center gap-14">
        <div className="flex flex-none flex-col items-center">
          <ScoreGauge score={report ? report.score : null} />
          <button
            onClick={() => void runScan()}
            disabled={isLoading}
            className="mt-6 flex items-center gap-2 rounded-[11px] bg-accent px-[30px] py-[13px] text-sm font-bold text-accent-ink transition hover:brightness-105 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <IconSearch size={16} />
            {isLoading ? "검사 중" : "지금 다시 검사"}
          </button>
        </div>

        <div className="min-w-0 flex-1">
          {report ? (
            <>
              <h2 className="m-0 text-[28px] font-extrabold leading-[1.28] tracking-[-.02em] text-text" style={{ textWrap: "balance" } as React.CSSProperties}>
                {issueCount === 0 ? "PC가 안전하게 보호되고 있어요" : `확인이 필요한 항목이 ${issueCount}가지 있어요`}
              </h2>
              <p className="mt-3.5 max-w-[480px] text-[15px] leading-[1.6] text-text-2">
                {issueCount === 0
                  ? "현재 위험 항목이 없습니다. 정기 검사로 상태를 계속 유지하세요."
                  : "아래 항목을 조치하면 보안 점수를 더 높일 수 있어요. 어려운 설정은 버튼으로 바로 이동합니다."}
              </p>
              {actionItems.length > 0 && (
                <div className="mt-6 flex max-w-[540px] flex-col gap-[11px]">
                  {actionItems.map((c) => (
                    <ActionItem key={c.id} check={c} />
                  ))}
                  {allActionItems.length > 3 && (
                    <button
                      onClick={() => setShowAllActions((v) => !v)}
                      className="self-start text-xs text-accent hover:underline"
                    >
                      {showAllActions ? "▲ 줄이기" : `▼ 전체 보기 (${hiddenActionCount}개 더)`}
                    </button>
                  )}
                </div>
              )}
            </>
          ) : (
            <h2 className="m-0 text-[28px] font-extrabold leading-[1.28] tracking-[-.02em] text-text">
              {isLoading ? "보안 상태를 점검하는 중입니다…" : "보안 검사를 시작하세요"}
            </h2>
          )}
        </div>
      </div>

      {/* 영역별 상태 */}
      {report && <AreaStatusGrid checks={report.checks} />}

      {/* 점수 추이 차트 */}
      {history.length >= 2 && (
        <div className="w-full rounded-[15px] border border-border bg-surface p-5">
          <ScoreHistoryChart entries={history} />
        </div>
      )}

      {lastError && (
        <div className="w-full rounded-xl border border-danger bg-danger-bg p-4 text-sm text-danger">
          오류: {lastError}
        </div>
      )}

      {/* 전체 점검 항목 */}
      {report && (
        <div className="flex w-full flex-col gap-3">
          <div className="flex items-center gap-2">
            <span className="text-[13.5px] font-extrabold text-text">전체 점검 항목</span>
            <span className="font-mono text-xs text-text-3">{report.checks.length}개</span>
          </div>
          {report.checks.map((check) => (
            <CheckCard key={check.id} check={check} />
          ))}
        </div>
      )}

      {isLoading && !report && (
        <div className="flex flex-col items-center gap-3 py-16">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-accent-soft border-t-accent" style={{ willChange: "transform" }} />
          <p className="text-sm text-text-3">Windows 보안 상태를 점검하는 중입니다...</p>
        </div>
      )}

      {!report && !isLoading && (
        <div className="py-16 text-center text-sm text-text-3">
          "지금 다시 검사" 버튼을 눌러 Windows 보안 상태를 점검하세요.
        </div>
      )}
    </div>
  );
};
