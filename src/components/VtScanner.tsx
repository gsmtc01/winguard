// src/components/VtScanner.tsx
//
// 파일 VT 검사 UI — 다중 파일 스캔 + 검사 이력

import { useEffect, useRef, useState } from "react";
import {
  pickFiles,
  scanFileVt,
  hasVtApiKey,
  type VtScanResult,
  type VtStatus,
} from "@/api/virustotal";
import { saveVtHistory, loadVtHistory, removeVtHistoryEntry, type VtHistoryEntry } from "@/lib/vtHistory";
import { onDataReset } from "@/store/dataReset";
import { IconShield } from "./icons";
import { openUrl } from "@tauri-apps/plugin-opener";

// ── 파일 아이템 타입 ───────────────────────────────────────────

type FilePhase = "pending" | "scanning" | "done" | "error";

interface FileItem {
  path: string;
  name: string;
  phase: FilePhase;
  result?: VtScanResult;
  error?: string;
}

// ── 색상 매핑 (토큰) ───────────────────────────────────────────

const statusStyle: Record<VtStatus, { label: string; badge: string; card: string; border: string; dot: string }> = {
  clean: { label: "안전", badge: "bg-ok-bg text-ok", card: "bg-ok-bg", border: "border-ok", dot: "bg-ok" },
  suspicious: { label: "의심", badge: "bg-warn-bg text-warn", card: "bg-warn-bg", border: "border-warn", dot: "bg-warn" },
  malicious: { label: "악성", badge: "bg-danger-bg text-danger", card: "bg-danger-bg", border: "border-danger", dot: "bg-danger" },
  error: { label: "오류", badge: "bg-surface-2 text-text-2", card: "bg-surface-2", border: "border-border", dot: "bg-text-3" },
};

const historyStatusStyle: Record<string, string> = {
  clean: "bg-ok-bg text-ok",
  suspicious: "bg-warn-bg text-warn",
  malicious: "bg-danger-bg text-danger",
  error: "bg-surface-2 text-text-2",
};

const historyStatusLabel: Record<string, string> = {
  clean: "안전", suspicious: "의심", malicious: "악성", error: "오류",
};

// ── 헬퍼 ──────────────────────────────────────────────────────

function getFileName(path: string): string {
  return path.split(/[/\\]/).pop() ?? path;
}

// ── 결과 카드 ─────────────────────────────────────────────────

function ResultCard({ item }: { item: FileItem }) {
  const [showDetections, setShowDetections] = useState(false);

  if (!item.result) return null;
  const { result } = item;
  const s = statusStyle[result.status];

  return (
    <div className={`space-y-3 rounded-[14px] border p-4 ${s.card} ${s.border}`}>
      {/* 파일명 + 상태 */}
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className={`h-2.5 w-2.5 flex-shrink-0 rounded-full ${s.dot}`} />
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold text-text">{item.name}</p>
            <span className={`rounded-full px-2 py-0.5 text-xs font-semibold ${s.badge}`}>{s.label}</span>
            {result.was_uploaded && (
              <span className="ml-1 rounded-full bg-surface-2 px-1.5 py-0.5 text-xs text-text-3">새로 업로드</span>
            )}
          </div>
        </div>
        <button onClick={() => { void openUrl(result.vt_link); }} className="flex-shrink-0 text-xs text-info underline hover:brightness-110">
          VT ↗
        </button>
      </div>

      {/* 통계 */}
      <div className="grid grid-cols-3 gap-2">
        {[
          { label: "악성", value: result.malicious, color: "text-danger" },
          { label: "의심", value: result.suspicious, color: "text-warn" },
          { label: "안전", value: result.undetected, color: "text-ok" },
        ].map(({ label, value, color }) => (
          <div key={label} className="rounded-lg border border-border bg-surface px-2 py-1.5 text-center">
            <div className={`text-base font-bold tabular-nums ${color}`}>{value}</div>
            <div className="text-xs text-text-3">{label}</div>
          </div>
        ))}
      </div>
      <p className="text-center text-xs text-text-3">총 {result.total_engines}개 엔진</p>

      {/* 탐지 엔진 토글 */}
      {result.detections.length > 0 && (
        <div>
          <button onClick={() => setShowDetections((v) => !v)} className="text-xs text-text-2 hover:underline">
            {showDetections ? "▲ 탐지 목록 닫기" : `▼ 탐지 엔진 보기 (${result.detections.length}개)`}
          </button>
          {showDetections && (
            <div className="mt-2 overflow-hidden rounded-lg border border-border">
              <div className="max-h-40 overflow-y-auto">
                {result.detections.map((d, i) => (
                  <div key={d.engine_name} className={`flex justify-between px-3 py-1.5 text-xs ${i % 2 === 0 ? "bg-surface" : "bg-surface-2"}`}>
                    <span className="font-medium text-text">{d.engine_name}</span>
                    <span className="text-danger">{d.result ?? d.category}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      <p className="break-all font-mono text-xs text-text-3">{result.sha256}</p>
    </div>
  );
}

// ── 검사 이력 패널 ────────────────────────────────────────────

function HistoryPanel() {
  const [history, setHistory] = useState<VtHistoryEntry[]>(() => loadVtHistory());
  const [open, setOpen] = useState(false);

  // 설정 > 데이터 관리에서 이력을 지우면 이 목록도 다시 읽는다.
  // (마운트 시 한 번만 읽으므로 이벤트 없이는 지운 뒤에도 계속 보인다)
  useEffect(() => onDataReset(() => setHistory(loadVtHistory())), []);

  if (history.length === 0) return null;

  const refresh = () => setHistory(loadVtHistory());

  return (
    <div className="overflow-hidden rounded-[14px] border border-border">
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between bg-surface-2 px-4 py-3 text-sm font-medium text-text-2 transition hover:bg-hover"
      >
        <span>최근 검사 이력 ({history.length}개)</span>
        <span className="text-text-3">{open ? "▲" : "▼"}</span>
      </button>
      {open && (
        <div className="divide-y divide-border">
          {history.map((h) => (
            <div key={h.sha256} className="flex items-center gap-3 bg-surface px-4 py-2.5 transition hover:bg-hover">
              <span className={`flex-shrink-0 rounded px-1.5 py-0.5 text-xs font-semibold ${historyStatusStyle[h.status] ?? historyStatusStyle.error}`}>
                {historyStatusLabel[h.status] ?? "오류"}
              </span>
              <div className="min-w-0 flex-1">
                <p className="truncate text-xs font-medium text-text">{h.file_name}</p>
                <p className="truncate font-mono text-xs text-text-3">{h.sha256.slice(0, 20)}…</p>
              </div>
              <div className="flex flex-shrink-0 items-center gap-2">
                {h.malicious > 0 && <span className="text-xs font-semibold text-danger">{h.malicious}건 탐지</span>}
                <button onClick={() => { void openUrl(h.vt_link); }} className="text-xs text-info hover:underline">VT↗</button>
                <button
                  onClick={() => { removeVtHistoryEntry(h.sha256); refresh(); }}
                  className="text-xs text-text-3 hover:text-text-2"
                  title="이력 삭제"
                >
                  ✕
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── 메인 컴포넌트 ──────────────────────────────────────────────

type ScanPhase = "idle" | "running" | "complete" | "key_error";

export const VtScanner = () => {
  const [scanPhase, setScanPhase] = useState<ScanPhase>("idle");
  const [keyErrorMsg, setKeyErrorMsg] = useState<string>("");
  const [items, setItems] = useState<FileItem[]>([]);
  const [consentFileName, setConsentFileName] = useState<string | null>(null);
  const consentResolveRef = useRef<((approved: boolean) => void) | null>(null);

  const askConsent = (fileName: string): Promise<boolean> => {
    setConsentFileName(fileName);
    return new Promise((resolve) => {
      consentResolveRef.current = resolve;
    });
  };

  const handleConsentApprove = () => {
    setConsentFileName(null);
    consentResolveRef.current?.(true);
    consentResolveRef.current = null;
  };

  const handleConsentSkip = () => {
    setConsentFileName(null);
    consentResolveRef.current?.(false);
    consentResolveRef.current = null;
  };

  const updateItem = (idx: number, patch: Partial<FileItem>) => {
    setItems((prev) => prev.map((it, i) => (i === idx ? { ...it, ...patch } : it)));
  };

  const handleScan = async () => {
    const hasKey = await hasVtApiKey().catch(() => false);
    if (!hasKey) {
      setKeyErrorMsg("VT API 키가 등록되지 않았습니다. 우상단 설정에서 VirusTotal API 키를 입력하세요.");
      setScanPhase("key_error");
      return;
    }

    const paths = await pickFiles();
    if (paths.length === 0) return;

    const initial: FileItem[] = paths.map((p) => ({ path: p, name: getFileName(p), phase: "pending" }));
    setItems(initial);
    setScanPhase("running");

    const updated = [...initial];

    for (let i = 0; i < updated.length; i++) {
      const item = updated[i];
      updateItem(i, { phase: "scanning" });

      try {
        let result = await scanFileVt(item.path, false);

        if (result.status === "error" && result.error_message?.includes("등록되어 있지 않습니다")) {
          const approved = await askConsent(item.name);
          if (approved) {
            result = await scanFileVt(item.path, true);
          } else {
            const errItem: FileItem = { ...item, phase: "error", error: "업로드 취소됨" };
            updated[i] = errItem;
            updateItem(i, { phase: "error", error: "업로드 취소됨" });
            continue;
          }
        }

        const doneItem: FileItem = { ...item, phase: "done", result };
        updated[i] = doneItem;
        updateItem(i, { phase: "done", result });

        saveVtHistory(result, item.name);
      } catch (e) {
        const errItem: FileItem = { ...item, phase: "error", error: String(e) };
        updated[i] = errItem;
        updateItem(i, { phase: "error", error: String(e) });
      }
    }

    setScanPhase("complete");
  };

  const reset = () => {
    setScanPhase("idle");
    setItems([]);
    setKeyErrorMsg("");
    setConsentFileName(null);
    consentResolveRef.current = null;
  };

  // ── 렌더링 ───────────────────────────────────────────────────

  return (
    <div className="w-full max-w-[900px] space-y-4">
      {/* 헤더 */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-text">파일 바이러스 검사</h2>
          <p className="mt-0.5 text-xs text-text-2">
            VirusTotal의 70여 개 엔진으로 파일을 검사합니다. 여러 파일을 한 번에 선택할 수 있습니다.
          </p>
        </div>
        {(scanPhase === "complete" || scanPhase === "key_error") && (
          <button onClick={reset} className="flex-shrink-0 rounded border border-border px-2 py-1 text-xs text-text-2 transition hover:bg-hover">
            초기화
          </button>
        )}
      </div>

      {/* 업로드 동의 모달 */}
      {consentFileName && (
        <div className="space-y-4 rounded-[14px] border border-warn bg-warn-bg p-5">
          <div>
            <h3 className="mb-1 text-sm font-semibold text-warn">파일 업로드 동의 필요</h3>
            <p className="text-xs leading-relaxed text-text-2">
              <strong className="text-text">{consentFileName}</strong> 파일은 VirusTotal에 등록되어 있지 않습니다. 검사를 계속하려면 파일을 서버에 업로드해야 합니다.
            </p>
          </div>
          <ul className="list-inside list-disc space-y-1 text-xs text-text-2">
            <li>업로드된 파일은 VirusTotal 커뮤니티에 공개될 수 있습니다.</li>
            <li>개인 정보가 포함된 문서는 업로드하지 마세요.</li>
            <li>파일 크기 32 MB 초과 시 업로드가 불가합니다.</li>
          </ul>
          <div className="flex gap-2">
            <button onClick={handleConsentApprove} className="flex-1 rounded-lg bg-warn py-2 text-sm font-medium text-white transition hover:brightness-105">
              동의하고 업로드
            </button>
            <button onClick={handleConsentSkip} className="flex-1 rounded-lg border border-border py-2 text-sm text-text-2 transition hover:bg-hover">
              이 파일 건너뛰기
            </button>
          </div>
        </div>
      )}

      {/* IDLE — 드롭/선택 존 */}
      {scanPhase === "idle" && (
        <div className="flex flex-col items-center rounded-[18px] border-2 border-dashed border-border bg-surface p-11 text-center">
          <span className="flex h-[60px] w-[60px] items-center justify-center rounded-[16px] bg-accent-soft text-accent">
            <IconShield size={30} />
          </span>
          <div className="mt-4 text-[17px] font-extrabold text-text">검사할 파일을 선택하세요</div>
          <div className="mt-1.5 text-[13px] text-text-2">VirusTotal의 다양한 엔진으로 파일을 검사할 수 있습니다.</div>
          <button
            onClick={() => void handleScan()}
            className="mt-[18px] rounded-[10px] bg-accent px-[22px] py-[11px] text-[13.5px] font-bold text-accent-ink transition hover:brightness-105"
          >
            파일 선택
          </button>
        </div>
      )}

      {/* API 키 오류 */}
      {scanPhase === "key_error" && (
        <div className="space-y-3 rounded-[14px] border border-danger bg-danger-bg p-4">
          <p className="text-sm text-danger">{keyErrorMsg}</p>
          <button onClick={reset} className="text-xs text-danger underline hover:brightness-110">
            닫기
          </button>
        </div>
      )}

      {/* 진행 중 + 완료 — 파일 목록 */}
      {(scanPhase === "running" || scanPhase === "complete") && (
        <div className="space-y-3">
          {scanPhase === "running" && (
            <div className="flex items-center gap-2 text-sm text-accent">
              <span className="h-4 w-4 flex-shrink-0 animate-spin rounded-full border-2 border-accent border-t-transparent" style={{ willChange: "transform" }} />
              <span>{items.filter((it) => it.phase === "done" || it.phase === "error").length} / {items.length}개 완료</span>
            </div>
          )}
          {scanPhase === "complete" && (
            <div className="flex items-center justify-between">
              <p className="text-sm text-text-2">
                총 {items.length}개 파일 검사 완료 —{" "}
                <span className="font-semibold text-danger">악성 {items.filter((it) => it.result?.status === "malicious").length}개</span> ·{" "}
                <span className="font-semibold text-ok">안전 {items.filter((it) => it.result?.status === "clean").length}개</span>
              </p>
              <button onClick={() => void handleScan()} className="text-xs text-accent hover:underline">
                + 추가 파일 검사
              </button>
            </div>
          )}

          {items.map((item, i) => (
            <div key={i} className="space-y-2">
              {item.phase !== "done" && (
                <div className="flex items-center gap-3 rounded-lg border border-border bg-surface px-4 py-2.5">
                  {item.phase === "pending" && <span className="h-4 w-4 flex-shrink-0 rounded-full border-2 border-border" />}
                  {item.phase === "scanning" && <span className="h-4 w-4 flex-shrink-0 animate-spin rounded-full border-2 border-accent border-t-transparent" style={{ willChange: "transform" }} />}
                  {item.phase === "error" && <span className="flex-shrink-0 text-danger">✗</span>}
                  <span className="flex-1 truncate text-sm text-text-2">{item.name}</span>
                  <span className="flex-shrink-0 text-xs text-text-3">
                    {item.phase === "pending" ? "대기 중" : item.phase === "scanning" ? "검사 중..." : item.error ?? "오류"}
                  </span>
                </div>
              )}
              {item.phase === "done" && <ResultCard item={item} />}
            </div>
          ))}

          {scanPhase === "complete" && (
            <button onClick={() => void handleScan()} className="w-full rounded-xl bg-accent py-3 text-sm font-semibold text-accent-ink transition hover:brightness-105">
              새 파일 검사
            </button>
          )}
        </div>
      )}

      {/* 검사 이력 */}
      <HistoryPanel />
    </div>
  );
};
