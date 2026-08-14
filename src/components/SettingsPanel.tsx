// src/components/SettingsPanel.tsx

import { useCallback, useEffect, useState, type ComponentType } from "react";
import { getStartupEnabled, setStartupEnabled } from "@/api/system";
import { useSettingsStore, ACCENTS } from "@/store/settingsStore";
import { useCheckStore } from "@/store/checkStore";
import type { Theme, Accent, ScheduleType } from "@/store/settingsStore";
import { saveVtApiKey, hasVtApiKey, deleteVtApiKey } from "@/api/virustotal";
import { useModelStore, fmtBytes } from "@/store/modelStore";
import { openUrl } from "@tauri-apps/plugin-opener";
import { resetAllData, resetScoreHistory, resetVtHistory } from "@/store/dataReset";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  applyUpdate,
  downloadUpdate,
  type UpdateProgress,
  checkForUpdate,
  getUpdateRepo,
  markAutoChecked,
  shouldAutoCheck,
  type UpdateInfo,
} from "@/api/update";
import { Toggle } from "./Toggle";
import { MarkdownBlock } from "./MarkdownBlock";
import { QuarantinePanel } from "./QuarantinePanel";
import { IconSun, IconMonitor, IconMoon, IconClose } from "./icons";

// ── 테마 옵션 ────────────────────────────────────────────────────

type IconCmp = ComponentType<{ size?: number; className?: string }>;
const THEME_OPTIONS: { value: Theme; label: string; Icon: IconCmp; desc: string }[] = [
  { value: "light", label: "라이트", Icon: IconSun, desc: "항상 밝은 화면을 유지합니다." },
  { value: "dark", label: "다크", Icon: IconMoon, desc: "항상 어두운 화면을 유지합니다." },
  { value: "system", label: "시스템", Icon: IconMonitor, desc: "OS 테마 설정에 맞춰 자동 전환됩니다." },
];

// 액센트 스와치 (라이트 기준 대표색)
const ACCENT_SWATCH: Record<Accent, string> = {
  Forest: "#2f7a63",
  Ocean: "#2563a5",
  Violet: "#6d4bd1",
  Amber: "#b5701f",
};

// ── 섹션 제목 ────────────────────────────────────────────────────

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h3 className="text-sm font-semibold uppercase tracking-wide text-text-2">{children}</h3>;
}

// ── 확인 후 실행 버튼 ─────────────────────────────────────────────
// 되돌릴 수 없는 삭제 액션에 사용. 1차 클릭 시 "확인/취소"로 바뀌고,
// 확인을 눌러야 실제 onConfirm 이 실행된다. 실수 삭제 방지.

function ConfirmButton({
  label,
  confirmLabel = "정말 삭제",
  onConfirm,
  disabled,
  className = "",
}: {
  label: React.ReactNode;
  confirmLabel?: string;
  onConfirm: () => void;
  disabled?: boolean;
  className?: string;
}) {
  const [armed, setArmed] = useState(false);

  // 확인 대기 상태로 진입 후 4초간 반응 없으면 자동 취소
  useEffect(() => {
    if (!armed) return;
    const t = setTimeout(() => setArmed(false), 4000);
    return () => clearTimeout(t);
  }, [armed]);

  if (armed) {
    return (
      <span className="inline-flex flex-shrink-0 items-center gap-1.5">
        <button
          onClick={() => {
            setArmed(false);
            onConfirm();
          }}
          disabled={disabled}
          className="rounded-md bg-danger px-3 py-1.5 text-xs font-semibold text-white transition hover:brightness-105 disabled:opacity-50"
        >
          {confirmLabel}
        </button>
        <button
          onClick={() => setArmed(false)}
          className="rounded-md border border-border px-2.5 py-1.5 text-xs text-text-2 transition hover:bg-hover"
        >
          취소
        </button>
      </span>
    );
  }

  return (
    <button onClick={() => setArmed(true)} disabled={disabled} className={className}>
      {label}
    </button>
  );
}

// ── 테마 + 액센트 섹션 ───────────────────────────────────────────

function AppearanceSection() {
  const { theme, setTheme, accent, setAccent } = useSettingsStore();
  return (
    <div className="space-y-3">
      <SectionTitle>모양</SectionTitle>
      <div className="space-y-2">
        {THEME_OPTIONS.map((opt) => {
          const active = theme === opt.value;
          return (
            <button
              key={opt.value}
              onClick={() => setTheme(opt.value)}
              className={`flex w-full items-center gap-3 rounded-xl border px-4 py-3 text-left transition ${
                active ? "border-accent bg-accent-soft text-accent" : "border-border text-text-2 hover:bg-hover"
              }`}
            >
              <opt.Icon size={20} />
              <div>
                <p className="text-sm font-medium text-text">{opt.label}</p>
                <p className="text-xs text-text-3">{opt.desc}</p>
              </div>
              {active && <span className="ml-auto text-accent">✓</span>}
            </button>
          );
        })}
      </div>

      {/* 액센트 색상 */}
      <div className="pt-1">
        <p className="mb-2 text-xs text-text-3">강조 색상</p>
        <div className="flex gap-3">
          {ACCENTS.map((a) => {
            const active = accent === a;
            return (
              <button
                key={a}
                onClick={() => setAccent(a)}
                title={a}
                aria-label={a}
                className="flex h-9 w-9 items-center justify-center rounded-full text-white transition"
                style={{
                  background: ACCENT_SWATCH[a],
                  boxShadow: active ? `0 0 0 2px var(--surface), 0 0 0 4px ${ACCENT_SWATCH[a]}` : undefined,
                }}
              >
                {active && <span className="text-xs font-bold">✓</span>}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

// ── VT API 키 섹션 ───────────────────────────────────────────────

function ApiKeySection() {
  const [hasKey, setHasKey] = useState(false);
  const [input, setInput] = useState("");
  const [status, setStatus] = useState<"idle" | "saving" | "deleting" | "ok" | "err">("idle");
  const [errMsg, setErrMsg] = useState("");

  useEffect(() => {
    hasVtApiKey().then(setHasKey).catch(() => setHasKey(false));
  }, []);

  const handleSave = async () => {
    if (!input.trim()) return;
    setStatus("saving");
    try {
      await saveVtApiKey(input.trim());
      setHasKey(true);
      setInput("");
      setStatus("ok");
    } catch (e) {
      setErrMsg(String(e));
      setStatus("err");
    }
  };

  const handleDelete = async () => {
    setStatus("deleting");
    try {
      await deleteVtApiKey();
      setHasKey(false);
      setStatus("idle");
    } catch (e) {
      setErrMsg(String(e));
      setStatus("err");
    }
  };

  return (
    <div className="space-y-3">
      <SectionTitle>VirusTotal API 키</SectionTitle>
      <p className="text-xs text-text-3">
        API 키는 Windows 자격 증명 관리자에 안전하게 저장됩니다. 키 값은 화면에 표시되지 않습니다.
      </p>

      {hasKey ? (
        <div className="flex items-center justify-between rounded-lg border border-ok bg-ok-bg px-4 py-3">
          <span className="text-sm font-medium text-ok">API 키 등록됨</span>
          <button onClick={() => void handleDelete()} disabled={status === "deleting"} className="text-xs text-danger hover:underline disabled:opacity-50">
            {status === "deleting" ? "삭제 중…" : "삭제"}
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          <input
            type="password"
            placeholder="VT API 키 입력"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") void handleSave(); }}
            className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text placeholder-text-3 focus:border-accent focus:outline-none"
          />
          <button
            onClick={() => void handleSave()}
            disabled={!input.trim() || status === "saving"}
            className="w-full rounded-lg bg-accent py-2 text-sm font-medium text-accent-ink transition hover:brightness-105 disabled:opacity-50"
          >
            {status === "saving" ? "저장 중…" : "저장"}
          </button>
        </div>
      )}

      {status === "ok" && <p className="text-xs text-ok">API 키가 저장되었습니다.</p>}
      {status === "err" && <p className="text-xs text-danger">{errMsg}</p>}
    </div>
  );
}

// ── AI 모델 섹션 ─────────────────────────────────────────────────

function ModelSection() {
  const { exists, info, defaultDir, destDir, modelPath, status, loadedInMemory, check, setDestDir, download, remove } =
    useModelStore();

  useEffect(() => {
    void check();
  }, [check]);

  // 파일명은 WinGuard 가 정한다(llm_model_info). 사용자는 폴더만 고른다.
  const handlePickDir = async () => {
    const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
    const chosen = await openDialog({
      title: "모델을 저장할 폴더 선택",
      directory: true,
      defaultPath: destDir || defaultDir,
    });
    if (typeof chosen === "string") setDestDir(chosen);
  };

  const isDownloading = status.phase === "downloading";
  const isChecking = status.phase === "checking";
  const sizeText = info ? fmtBytes(info.size_bytes) : "-";

  return (
    <div className="space-y-3">
      <SectionTitle>On-Device AI 모델</SectionTitle>
      <p className="text-xs text-text-3">
        AI 분석 기능에 사용되는 로컬 AI 모델입니다. 인터넷 전송 없이 기기 내에서만 실행됩니다.
      </p>

      {isChecking ? (
        <div className="flex items-center gap-2 text-xs text-text-2">
          <span className="h-3 w-3 animate-spin rounded-full border-2 border-border border-t-text-3" style={{ willChange: "transform" }} />
          모델 상태 확인 중…
        </div>
      ) : exists === true ? (
        <div className="space-y-3">
          <div className="rounded-lg border border-ok bg-ok-bg px-4 py-3">
            <div className="mb-1 flex items-center gap-2">
              <span className="font-bold text-ok">✓</span>
              <span className="text-sm font-medium text-ok">AI 모델 설치됨</span>
            </div>
            {modelPath && <p className="truncate font-mono text-xs text-text-3" title={modelPath}>{modelPath}</p>}
          </div>
          {status.phase === "done" && <p className="text-xs text-ok">다운로드가 완료되었습니다. AI 분석을 사용할 수 있습니다.</p>}
          {loadedInMemory && <p className="text-center text-xs text-warn">현재 메모리에 로드됨</p>}
          {status.phase === "checking" ? (
            <div className="flex w-full items-center justify-center gap-2 rounded-lg border border-danger py-2 text-sm font-medium text-danger">
              <span className="h-3 w-3 animate-spin rounded-full border-2 border-danger-bg border-t-danger" style={{ willChange: "transform" }} />
              처리 중…
            </div>
          ) : (
            <ConfirmButton
              label="모델 제거"
              confirmLabel={`정말 제거 (약 ${sizeText})`}
              onConfirm={() => void remove()}
              className="w-full rounded-lg border border-danger py-2 text-sm font-medium text-danger transition hover:bg-danger-bg"
            />
          )}
        </div>
      ) : exists === false ? (
        <div className="space-y-3">
          <div className="rounded-lg border border-warn bg-warn-bg px-4 py-3 text-sm text-warn">
            AI 모델이 설치되지 않았습니다. 아래에서 다운로드하세요.
            <p className="mt-0.5 text-xs text-text-3">
              약 {sizeText}
              {info ? ` · ${info.display_name}` : ""}
            </p>
          </div>

          <div className="rounded-lg border border-border bg-surface-2 p-3">
            <p className="mb-1.5 text-xs text-text-3">저장 폴더</p>
            <div className="flex items-center gap-2">
              <p className="min-w-0 flex-1 truncate font-mono text-xs text-text-2" title={destDir || defaultDir}>
                {destDir || defaultDir}
              </p>
              <button onClick={() => void handlePickDir()} className="flex-shrink-0 rounded-md border border-border px-2.5 py-1 text-xs text-text-2 transition hover:bg-hover">
                변경
              </button>
            </div>
            {info && (
              <p className="mt-2 truncate font-mono text-xs text-text-3" title={info.file_name}>
                파일명: {info.file_name}
              </p>
            )}
          </div>

          <button
            onClick={() => void download()}
            disabled={isDownloading}
            className="w-full rounded-lg bg-accent py-2 text-sm font-medium text-accent-ink transition hover:brightness-105 disabled:opacity-50"
          >
            다운로드 시작
          </button>
        </div>
      ) : null}

      {isDownloading && (
        <div className="space-y-2">
          <div className="flex justify-between text-xs text-text-3">
            <span>
              {fmtBytes(status.downloaded ?? 0)}
              {(status.total ?? 0) > 0 ? ` / ${fmtBytes(status.total ?? 0)}` : ""}
            </span>
            <span>{status.percent ?? 0}%</span>
          </div>
          <div className="h-2 w-full rounded-full bg-surface-2">
            <div className="h-2 rounded-full bg-accent transition-all duration-300" style={{ width: `${status.percent ?? 0}%` }} />
          </div>
          <p className="text-center text-xs text-accent">다운로드 중… 창을 닫아도 계속됩니다.</p>
        </div>
      )}

      {status.phase === "error" && (
        <div className="space-y-1">
          <p className="text-xs text-danger">{status.errorMsg}</p>
          <button onClick={() => void check()} className="text-xs text-text-3 underline hover:text-text-2">
            다시 확인
          </button>
        </div>
      )}
    </div>
  );
}

// ── 시작 프로그램 섹션 ────────────────────────────────────────────

function StartupSection() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  useEffect(() => {
    getStartupEnabled().then(setEnabled).catch(() => setEnabled(false));
  }, []);

  const toggle = async () => {
    if (enabled === null) return;
    setBusy(true);
    setErr("");
    try {
      await setStartupEnabled(!enabled);
      setEnabled(!enabled);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-3">
      <SectionTitle>시작 프로그램</SectionTitle>
      <div className="flex items-center justify-between rounded-lg border border-border bg-surface-2 px-4 py-3">
        <div>
          <p className="text-sm font-medium text-text">Windows 시작 시 자동 실행</p>
          <p className="mt-0.5 text-xs text-text-3">트레이에 최소화된 상태로 시작합니다.</p>
        </div>
        <Toggle checked={enabled ?? false} onChange={() => void toggle()} disabled={busy || enabled === null} aria-label="시작 시 자동 실행" />
      </div>
      {err && <p className="text-xs text-danger">{err}</p>}
    </div>
  );
}

// ── 예약 검사 섹션 ────────────────────────────────────────────────

function ScheduleSection() {
  const { scheduledScan, scanSchedule, setScheduledScan, setScanSchedule } = useSettingsStore();
  const { lastScannedAt } = useCheckStore();

  const updateSchedule = (partial: Partial<typeof scanSchedule>) => {
    setScanSchedule({ ...scanSchedule, ...partial });
  };

  const formatTime = (ms: number | null) => {
    if (!ms) return "없음";
    const d = new Date(ms);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  const renderScheduleInputs = () => {
    const { type, time, dayOfWeek, dayOfMonth, month } = scanSchedule;

    return (
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <span className="w-10 text-xs font-medium text-text-2">주기</span>
          <select
            value={type}
            onChange={(e) => updateSchedule({ type: e.target.value as ScheduleType })}
            className="flex-1 rounded border border-border bg-surface px-2 py-1 text-sm text-text focus:border-accent focus:outline-none"
          >
            <option value="daily">매일</option>
            <option value="weekly">매주</option>
            <option value="monthly">매월</option>
            <option value="yearly">매년</option>
          </select>
        </div>

        {type === "yearly" && (
          <div className="flex items-center gap-2">
            <span className="w-10 text-xs font-medium text-text-2">월</span>
            <select
              value={month ?? 1}
              onChange={(e) => updateSchedule({ month: parseInt(e.target.value, 10) })}
              className="flex-1 rounded border border-border bg-surface px-2 py-1 text-sm text-text focus:border-accent focus:outline-none"
            >
              {Array.from({ length: 12 }, (_, i) => (
                <option key={i + 1} value={i + 1}>{i + 1}월</option>
              ))}
            </select>
          </div>
        )}

        {(type === "monthly" || type === "yearly") && (
          <div className="flex items-center gap-2">
            <span className="w-10 text-xs font-medium text-text-2">일</span>
            <select
              value={dayOfMonth ?? 1}
              onChange={(e) => updateSchedule({ dayOfMonth: parseInt(e.target.value, 10) })}
              className="flex-1 rounded border border-border bg-surface px-2 py-1 text-sm text-text focus:border-accent focus:outline-none"
            >
              {Array.from({ length: 31 }, (_, i) => (
                <option key={i + 1} value={i + 1}>{i + 1}일</option>
              ))}
            </select>
          </div>
        )}

        {type === "weekly" && (
          <div className="flex items-center gap-2">
            <span className="w-10 flex-shrink-0 text-xs font-medium text-text-2">요일</span>
            <div className="flex flex-1 justify-between gap-1">
              {["일", "월", "화", "수", "목", "금", "토"].map((day, idx) => (
                <button
                  key={idx}
                  onClick={() => updateSchedule({ dayOfWeek: idx })}
                  className={`h-7 flex-1 rounded text-xs font-medium transition ${
                    (dayOfWeek ?? 0) === idx
                      ? "bg-accent text-accent-ink"
                      : "bg-surface-2 text-text-2 hover:bg-hover"
                  }`}
                >
                  {day}
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="flex items-center gap-2">
          <span className="w-10 text-xs font-medium text-text-2">시간</span>
          <input
            type="time"
            value={time}
            onChange={(e) => updateSchedule({ time: e.target.value })}
            className="flex-1 rounded border border-border bg-surface px-2 py-1 text-sm text-text focus:border-accent focus:outline-none"
          />
        </div>
      </div>
    );
  };

  return (
    <div className="space-y-3">
      <SectionTitle>예약 검사</SectionTitle>
      <div className="flex items-center justify-between rounded-lg border border-border bg-surface-2 px-4 py-3">
        <div>
          <p className="text-sm font-medium text-text">자동 검사 활성화</p>
          <p className="mt-0.5 text-xs text-text-3">예약된 시간에 자동 검사를 실행합니다.</p>
        </div>
        <Toggle checked={scheduledScan} onChange={() => setScheduledScan(!scheduledScan)} aria-label="자동 검사 활성화" />
      </div>

      {scheduledScan && (
        <div className="rounded-lg border border-border px-4 py-3">
          <div className="mb-3 flex items-center justify-between">
            <span className="text-sm font-medium text-text">검사 상세 설정</span>
            <div className="text-xs text-text-3">
              마지막: {formatTime(lastScannedAt)}
            </div>
          </div>
          {renderScheduleInputs()}
        </div>
      )}
    </div>
  );
}

// ── 데이터 관리 섹션 ──────────────────────────────────────────────

function DataSection() {
  const [cleared, setCleared] = useState<string | null>(null);

  const handle = (label: string, fn: () => void) => {
    fn();
    setCleared(label);
    setTimeout(() => setCleared(null), 2000);
  };

  const DATA_ACTIONS = [
    {
      label: "보안 점수 기록 초기화",
      desc: "대시보드 점수 추이 차트 데이터를 삭제합니다.",
      fn: () => handle("보안 점수 기록", resetScoreHistory),
    },
    {
      label: "파일 검사 기록 초기화",
      desc: "VirusTotal 파일 검사 이력을 삭제합니다.",
      fn: () => handle("파일 검사 기록", resetVtHistory),
    },
    {
      label: "모든 데이터 초기화",
      desc: "점수 기록, 파일 검사 이력, 마지막 검사 결과, AI 분석·Q&A 내용을 모두 삭제합니다. (격리 항목은 아래에서 따로 삭제)",
      fn: () => handle("모든 데이터", resetAllData),
    },
  ];

  return (
    <div className="space-y-3">
      <SectionTitle>데이터 관리</SectionTitle>
      <div className="space-y-2">
        {DATA_ACTIONS.map((action) => (
          <div key={action.label} className="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface-2 px-4 py-3">
            <div className="min-w-0">
              <p className="text-sm font-medium text-text">{action.label}</p>
              <p className="mt-0.5 text-xs text-text-3">{action.desc}</p>
            </div>
            <ConfirmButton
              label="초기화"
              confirmLabel="확인"
              onConfirm={action.fn}
              className="ml-3 flex-shrink-0 rounded-md border border-danger px-3 py-1.5 text-xs font-medium text-danger transition hover:bg-danger-bg"
            />
          </div>
        ))}
      </div>
      {cleared && <p className="text-xs text-ok">{cleared}이(가) 초기화되었습니다.</p>}
    </div>
  );
}

// ── 업데이트 섹션 ─────────────────────────────────────────────────
//
// 업데이트 서버(GitHub Releases)에서 최신 릴리즈를 조회해 버전·배포일·변경 내용을
// 보여준다. 설치 파일은 앱이 직접 받지 않고 브라우저로 열어 사용자가 내려받는다.

type UpdatePhase = "idle" | "checking" | "done" | "error";

function formatReleaseDate(iso: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : d.toLocaleDateString("ko-KR");
}

function UpdateSection() {
  const [phase, setPhase] = useState<UpdatePhase>("idle");
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [errMsg, setErrMsg] = useState("");
  const [repo, setRepo] = useState("");
  const [notesOpen, setNotesOpen] = useState(false);

  // 앱 내 업데이트 상태. 사용자가 버튼을 눌러야만 시작된다.
  const [installPhase, setInstallPhase] = useState<"idle" | "downloading" | "applying">("idle");
  const [installPct, setInstallPct] = useState(0);
  const [installErr, setInstallErr] = useState("");

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let alive = true;
    listen<UpdateProgress>("update:download-progress", (e) => {
      setInstallPct(e.payload.percent);
    })
      .then((fn) => {
        if (alive) unlisten = fn;
        else fn();
      })
      .catch(() => { /* IPC 미지원 환경 */ });
    return () => { alive = false; unlisten?.(); };
  }, []);

  const runSelfUpdate = useCallback(async (target: UpdateInfo) => {
    if (!target.download_url || !target.download_name || !target.download_sha256) return;
    setInstallErr("");
    setInstallPct(0);
    setInstallPhase("downloading");
    try {
      const path = await downloadUpdate(
        target.download_url,
        target.download_name,
        target.download_sha256,
      );
      setInstallPhase("applying");
      // 성공하면 새 버전이 실행되고 이 프로세스는 곧 종료된다.
      await applyUpdate(path);
    } catch (e) {
      setInstallErr(String(e));
      setInstallPhase("idle");
    }
  }, []);

  const runCheck = useCallback(async (auto: boolean) => {
    setPhase("checking");
    setErrMsg("");
    try {
      const result = await checkForUpdate();
      setInfo(result);
      setRepo(result.repo);
      setPhase("done");
      // 업데이트가 있으면 변경 내용을 바로 펼쳐 보여준다.
      setNotesOpen(result.update_available);
      markAutoChecked();
    } catch (e) {
      setErrMsg(String(e));
      // 자동 확인 실패는 조용히 넘어간다(오프라인일 수 있음). 수동 확인만 오류 표시.
      setPhase(auto ? "idle" : "error");
    }
  }, []);

  useEffect(() => {
    let alive = true;
    getUpdateRepo()
      .then((r) => { if (alive) setRepo(r); })
      .catch(() => { /* IPC 미지원 환경 */ });
    if (shouldAutoCheck()) void runCheck(true);
    return () => { alive = false; };
  }, [runCheck]);

  const openLink = (url: string) => { void openUrl(url); };

  return (
    <div className="space-y-3">
      <SectionTitle>업데이트</SectionTitle>

      <div className="rounded-lg border border-border bg-surface-2 px-4 py-3">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <p className="text-sm font-medium text-text">새 버전 확인</p>
            <p className="mt-0.5 truncate text-xs text-text-3" title={repo}>
              {repo ? `업데이트 서버: ${repo}` : "업데이트 서버에서 최신 버전을 확인합니다."}
            </p>
          </div>
          <button
            onClick={() => void runCheck(false)}
            disabled={phase === "checking"}
            className="flex-shrink-0 rounded-md border border-border px-3 py-1.5 text-xs font-medium text-text-2 transition hover:bg-hover disabled:opacity-50"
          >
            {phase === "checking" ? "확인 중…" : "지금 확인"}
          </button>
        </div>
      </div>

      {phase === "done" && info && (
        <div
          className={`space-y-2 rounded-lg border px-4 py-3 ${
            info.update_available ? "border-accent bg-accent-soft" : "border-ok bg-ok-bg"
          }`}
        >
          <div className="flex items-center gap-2">
            <span className={`text-sm font-semibold ${info.update_available ? "text-accent" : "text-ok"}`}>
              {info.update_available ? `새 버전 v${info.latest_version}` : "최신 버전입니다"}
            </span>
            {info.prerelease && (
              <span className="rounded-full bg-warn-bg px-1.5 py-0.5 text-xs font-medium text-warn">
                미리보기
              </span>
            )}
          </div>

          <p className="text-xs text-text-2">
            현재 v{info.current_version}
            {info.update_available ? ` → 최신 v${info.latest_version}` : ""}
            {formatReleaseDate(info.published_at) ? ` · ${formatReleaseDate(info.published_at)} 배포` : ""}
          </p>

          <div>
            <button
              onClick={() => setNotesOpen((v) => !v)}
              className="text-xs text-text-2 hover:underline"
            >
              {notesOpen ? "▲ 변경 내용 닫기" : "▼ 변경 내용 보기"}
            </button>
            {notesOpen && (
              <div className="mt-2 max-h-56 overflow-y-auto rounded-lg border border-border bg-surface px-3 py-2" data-scroll>
                <p className="mb-1 text-xs font-semibold text-text">{info.title}</p>
                <MarkdownBlock text={info.notes} compact />
              </div>
            )}
          </div>

          {info.update_available && (
            <div className="flex flex-wrap items-center gap-2 pt-0.5">
              {/* 앱 내 업데이트는 해시가 확인될 때만 제공한다.
                  검증 없이 실행 파일을 갈아끼우지 않는다. */}
              {info.download_url && info.download_sha256 && (
                <button
                  onClick={() => void runSelfUpdate(info)}
                  disabled={installPhase === "downloading" || installPhase === "applying"}
                  className="rounded-lg bg-accent px-3 py-1.5 text-xs font-semibold text-accent-ink transition hover:brightness-105 disabled:cursor-not-allowed disabled:opacity-50"
                >
                  {installPhase === "downloading"
                    ? `내려받는 중… ${installPct}%`
                    : installPhase === "applying"
                      ? "교체 중…"
                      : "지금 업데이트"}
                </button>
              )}
              {info.download_url && (
                <button
                  onClick={() => openLink(info.download_url as string)}
                  className={
                    info.download_sha256
                      ? "rounded-lg border border-border px-3 py-1.5 text-xs text-text-2 transition hover:bg-hover"
                      : "rounded-lg bg-accent px-3 py-1.5 text-xs font-semibold text-accent-ink transition hover:brightness-105"
                  }
                >
                  브라우저로 받기
                  {info.download_size ? ` (${fmtBytes(info.download_size)})` : ""}
                </button>
              )}
              <button
                onClick={() => openLink(info.release_url)}
                className="rounded-lg border border-border px-3 py-1.5 text-xs text-text-2 transition hover:bg-hover"
              >
                릴리즈 페이지 ↗
              </button>
            </div>
          )}
          {info.update_available && !info.download_url && (
            <p className="text-xs text-text-3">
              이 릴리즈에 설치 파일이 없습니다. 릴리즈 페이지에서 직접 확인하세요.
            </p>
          )}
          {info.update_available && info.download_url && !info.download_sha256 && (
            <p className="text-xs text-text-3">
              이 릴리즈에 무결성 확인용 .sha256 파일이 없어 앱 내 업데이트를 제공하지 않습니다.
              브라우저로 받아 직접 교체하세요.
            </p>
          )}
          {installErr && <p className="text-xs text-danger">{installErr}</p>}
        </div>
      )}

      {phase === "error" && (
        <div className="rounded-lg border border-danger bg-danger-bg px-4 py-3">
          <p className="text-xs text-danger">{errMsg}</p>
        </div>
      )}
    </div>
  );
}

// ── 앱 정보 (푸터) ────────────────────────────────────────────────
// 버전을 하드코딩하지 않고 Tauri 에서 런타임 조회한다. 브라우저 프리뷰 등
// IPC 미지원 환경에서는 조회 실패 → package.json 기준 기본값으로 폴백.

const FALLBACK_VERSION = "0.1.0";

function AppFooter() {
  const [version, setVersion] = useState(FALLBACK_VERSION);

  useEffect(() => {
    let alive = true;
    import("@tauri-apps/api/app")
      .then((m) => m.getVersion())
      .then((v) => {
        if (alive && typeof v === "string" && v.trim()) setVersion(v.trim());
      })
      .catch(() => {
        /* IPC 미지원 환경: 폴백 버전 유지 */
      });
    return () => {
      alive = false;
    };
  }, []);

  return (
    <div className="border-t border-border px-5 py-4 text-center">
      <p className="text-xs font-medium text-text-2">WinGuard v{version}</p>
      <p className="mt-0.5 flex items-center justify-center gap-1 text-xs text-text-3">
        <span>On-Device AI ·</span>
        <button onClick={() => { void openUrl("https://ai.google.dev/gemma/terms"); }} className="hover:underline">
          Gemma 4 E2B (Apache 2.0 License)
        </button>
      </p>
    </div>
  );
}

// ── 메인 패널 ────────────────────────────────────────────────────

export const SettingsPanel = ({ onClose }: { onClose: () => void }) => {
  return (
    <>
      {/* 오버레이 */}
      <div className="fixed inset-0 z-40 bg-black/40" onClick={onClose} aria-hidden="true" />

      {/* 슬라이드 패널 */}
      <aside className="fixed right-0 top-0 z-50 flex h-full w-80 flex-col border-l border-border bg-surface shadow-2xl">
        <div className="flex items-center justify-between border-b border-border px-5 py-4">
          <h2 className="text-base font-semibold text-text">설정</h2>
          <button onClick={onClose} aria-label="닫기" className="flex h-8 w-8 items-center justify-center rounded-lg text-text-2 transition hover:bg-hover">
            <IconClose size={18} />
          </button>
        </div>

        <div data-scroll className="flex-1 space-y-8 overflow-y-auto px-5 py-6">
          <AppearanceSection />
          <div className="border-t border-border" />
          <ModelSection />
          <div className="border-t border-border" />
          <ApiKeySection />
          <div className="border-t border-border" />
          <StartupSection />
          <div className="border-t border-border" />
          <ScheduleSection />
          <div className="border-t border-border" />
          <QuarantinePanel />
          <div className="border-t border-border" />
          <DataSection />
          <div className="border-t border-border" />
          <UpdateSection />
        </div>

        <AppFooter />
      </aside>
    </>
  );
};
