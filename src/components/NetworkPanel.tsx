import { useState } from "react";
import { useNetworkStore, type NetworkConn } from "@/store/networkStore";
import { MarkdownBlock } from "./MarkdownBlock";
import { IconSparkle, IconSearch } from "./icons";

// ── 상태 배지 ─────────────────────────────────────────────────

function StateBadge({ state }: { state: string }) {
  const isEstablished = state === "Established";
  const isListen = state === "Listen";
  return (
    <span
      className={`rounded px-1.5 py-0.5 text-xs font-medium ${
        isEstablished ? "bg-info-bg text-info" : isListen ? "bg-warn-bg text-warn" : "bg-surface-2 text-text-3"
      }`}
    >
      {state}
    </span>
  );
}

// ── 외부 IP 여부 ──────────────────────────────────────────────

function isExternal(addr: string): boolean {
  return (
    addr !== "" &&
    addr !== "0.0.0.0" &&
    addr !== "::" &&
    !addr.startsWith("127.") &&
    !addr.startsWith("::1") &&
    !addr.startsWith("10.") &&
    !addr.startsWith("192.168.") &&
    !/^172\.(1[6-9]|2\d|3[01])\./.test(addr)
  );
}

// ── 연결 테이블 ────────────────────────────────────────────────

function ConnectionTable({ connections }: { connections: NetworkConn[] }) {
  const [filter, setFilter] = useState<"all" | "established" | "listen">("all");
  const [showAll, setShowAll] = useState(false);

  const filtered = connections.filter((c) => {
    if (filter === "established") return c.state === "Established";
    if (filter === "listen") return c.state === "Listen";
    return true;
  });

  const visible = showAll ? filtered : filtered.slice(0, 25);

  const established = connections.filter((c) => c.state === "Established").length;
  const listening = connections.filter((c) => c.state === "Listen").length;
  const external = connections.filter((c) => isExternal(c.remote_address)).length;

  return (
    <div className="mt-4">
      {/* 요약 카드 */}
      <div className="mb-3 grid grid-cols-3 gap-3">
        {[
          { label: "외부 연결", value: external, color: "text-info" },
          { label: "연결 중", value: established, color: "text-ok" },
          { label: "리스닝 포트", value: listening, color: "text-warn" },
        ].map(({ label, value, color }) => (
          <div key={label} className="rounded-[13px] border border-border bg-surface px-3 py-2 text-center">
            <div className={`text-lg font-bold tabular-nums ${color}`}>{value}</div>
            <div className="text-xs text-text-3">{label}</div>
          </div>
        ))}
      </div>

      {/* 필터 */}
      <div className="mb-2 flex gap-2">
        {(["all", "established", "listen"] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`rounded-full border px-2.5 py-1 text-xs transition ${
              filter === f ? "border-accent bg-accent-soft text-accent" : "border-border text-text-2 hover:bg-hover"
            }`}
          >
            {f === "all" ? "전체" : f === "established" ? "연결 중" : "리스닝"}
          </button>
        ))}
      </div>

      {/* 테이블 */}
      <div className="overflow-x-auto rounded-[13px] border border-border">
        <table className="min-w-full text-xs">
          <thead>
            <tr className="bg-surface-2 text-left">
              <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">프로세스</th>
              <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">로컬 포트</th>
              <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">원격 주소</th>
              <th className="whitespace-nowrap px-3 py-2 font-semibold text-text-2">원격 포트</th>
              <th className="px-3 py-2 font-semibold text-text-2">상태</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((c, i) => {
              const ext = isExternal(c.remote_address);
              return (
                <tr key={i} className={ext ? "bg-info-bg" : i % 2 === 0 ? "bg-surface" : "bg-surface-2"}>
                  <td className="whitespace-nowrap px-3 py-1.5 font-medium text-text">
                    {c.process_name ?? <span className="italic text-text-3">알 수 없음</span>}
                    <span className="ml-1 text-text-3">({c.pid})</span>
                  </td>
                  <td className="px-3 py-1.5 font-mono tabular-nums text-text-2">{c.local_port}</td>
                  <td className={`px-3 py-1.5 font-mono text-text-2 ${ext ? "font-semibold text-info" : ""}`}>
                    {c.remote_address || "—"}
                  </td>
                  <td className="px-3 py-1.5 font-mono tabular-nums text-text-2">{c.remote_port || "—"}</td>
                  <td className="px-3 py-1.5">
                    <StateBadge state={c.state} />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      {filtered.length > 25 && (
        <button onClick={() => setShowAll((v) => !v)} className="mt-2 text-xs text-accent hover:underline">
          {showAll ? "▲ 줄이기" : `▼ 전체 보기 (${filtered.length - 25}개 더)`}
        </button>
      )}
    </div>
  );
}

const PHASE_MESSAGES: Record<string, string> = {
  scanning: "네트워크 연결 스캔 중...",
  loading_model: "AI 모델 로드 중...",
  analyzing: "AI 분석 중...",
};

// ── 메인 패널 ─────────────────────────────────────────────────

export function NetworkPanel() {
  const { state, analyze, reset } = useNetworkStore();

  return (
    <div className="w-full max-w-[1040px] space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-text">네트워크 연결 분석</h2>
          <p className="mt-0.5 text-xs text-text-2">현재 TCP 연결을 스캔하고 AI가 의심 연결과 노출 포트를 분석합니다.</p>
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
          <ConnectionTable connections={state.connections} />
        </div>
      )}
    </div>
  );
}
