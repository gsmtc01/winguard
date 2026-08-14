// src/components/DevicePanel.tsx
//
// 카메라·마이크 접근 제어 패널 (장치 보호 탭)

import { useEffect, useState } from "react";
import { useDeviceStore, AppAccess } from "@/store/deviceStore";
import { Toggle } from "./Toggle";
import { IconCamera, IconMic, IconSparkle } from "./icons";

// ── 위험도 분류 (프론트엔드) ─────────────────────────────────────

const RISKY_NAMES = new Set([
  "python.exe", "pythonw.exe", "node.exe", "powershell.exe",
  "wscript.exe", "cscript.exe", "mshta.exe", "cmd.exe",
  "regsvr32.exe", "rundll32.exe",
]);
const RISKY_PATH_TOKENS = ["downloads", "temp", "tmp", "appdata\\local\\temp"];
const SAFE_PATH_TOKENS = ["system32", "program files", "windowsapps", "microsoft"];

function classifyRisk(app: AppAccess): "high" | "medium" | "low" {
  if (app.is_uwp) return "low";
  const lower = app.path.toLowerCase();
  const name = app.name.toLowerCase();
  if (RISKY_NAMES.has(name) || RISKY_PATH_TOKENS.some((t) => lower.includes(t))) return "high";
  if (SAFE_PATH_TOKENS.some((t) => lower.includes(t))) return "low";
  return "medium";
}

// ── 작은 컴포넌트들 ──────────────────────────────────────────────

function RiskBadge({ risk }: { risk: "high" | "medium" | "low" }) {
  const cls = { high: "bg-danger-bg text-danger", medium: "bg-warn-bg text-warn", low: "bg-ok-bg text-ok" }[risk];
  const label = { high: "고위험", medium: "주의", low: "정상" }[risk];
  return <span className={`rounded-full px-2 py-0.5 text-xs font-semibold ${cls}`}>{label}</span>;
}

/** 전역 장치 카드 (카메라 또는 마이크) */
function DeviceCard({
  Icon,
  title,
  allowed,
  onToggle,
  appCount,
  highRiskCount,
}: {
  Icon: typeof IconCamera;
  title: string;
  allowed: boolean;
  onToggle: (v: boolean) => void;
  appCount: number;
  highRiskCount: number;
}) {
  return (
    <div
      className={`flex items-center gap-5 rounded-[16px] border p-5 transition-colors ${
        allowed ? "border-accent bg-accent-soft" : "border-border bg-surface-2"
      }`}
    >
      <span
        className={`flex h-[46px] w-[46px] flex-none items-center justify-center rounded-[12px] ${
          allowed ? "bg-surface text-accent" : "bg-surface text-text-3"
        }`}
      >
        <Icon size={23} />
      </span>
      <div className="min-w-0 flex-1">
        <div className="mb-1 flex items-center gap-3">
          <span className="text-base font-bold text-text">{title}</span>
          <span className={`rounded-full px-2 py-0.5 text-xs font-semibold ${allowed ? "bg-surface text-accent" : "bg-surface-2 text-text-2"}`}>
            {allowed ? "허용" : "차단"}
          </span>
        </div>
        <p className="text-xs text-text-2">
          앱 {appCount}개 등록됨
          {highRiskCount > 0 && <span className="ml-2 font-semibold text-danger">고위험 {highRiskCount}개 허용 중</span>}
        </p>
      </div>
      <Toggle checked={allowed} onChange={() => onToggle(!allowed)} aria-label={`${title} 허용`} />
    </div>
  );
}

/** 앱별 접근 권한 테이블 */
function AppTable({ apps, onToggle }: { apps: AppAccess[]; onToggle: (key: string, allow: boolean) => void }) {
  const [filter, setFilter] = useState<"all" | "high" | "allowed" | "denied">("all");

  const filtered = apps.filter((a) => {
    if (filter === "high") return classifyRisk(a) === "high";
    if (filter === "allowed") return a.allowed;
    if (filter === "denied") return !a.allowed;
    return true;
  });

  const highCount = apps.filter((a) => classifyRisk(a) === "high").length;
  const allowedCount = apps.filter((a) => a.allowed).length;
  const deniedCount = apps.filter((a) => !a.allowed).length;

  const btnCls = (active: boolean) =>
    `rounded-full px-3 py-1 text-xs font-medium transition ${active ? "bg-accent text-accent-ink" : "bg-surface-2 text-text-2 hover:bg-hover"}`;

  if (apps.length === 0) {
    return <p className="py-4 text-center text-sm text-text-2">등록된 앱이 없습니다.</p>;
  }

  return (
    <div>
      <div className="mb-3 flex flex-wrap gap-2">
        <button className={btnCls(filter === "all")} onClick={() => setFilter("all")}>전체 {apps.length}</button>
        <button className={btnCls(filter === "high")} onClick={() => setFilter("high")}>고위험 {highCount}</button>
        <button className={btnCls(filter === "allowed")} onClick={() => setFilter("allowed")}>허용 {allowedCount}</button>
        <button className={btnCls(filter === "denied")} onClick={() => setFilter("denied")}>차단 {deniedCount}</button>
      </div>

      <div className="overflow-x-auto rounded-[13px] border border-border">
        <table className="w-full text-sm">
          <thead className="bg-surface-2 text-xs text-text-2">
            <tr>
              <th className="px-3 py-2 text-left font-semibold">앱</th>
              <th className="w-16 px-3 py-2 text-left font-semibold">종류</th>
              <th className="w-20 px-3 py-2 text-left font-semibold">위험도</th>
              <th className="w-20 px-3 py-2 text-right font-semibold">접근</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((app) => {
              const risk = classifyRisk(app);
              const rowCls = risk === "high" && app.allowed ? "bg-danger-bg" : "";
              return (
                <tr key={app.key} className={`border-t border-border ${rowCls}`}>
                  <td className="px-3 py-2">
                    <div className="max-w-xs truncate font-medium text-text">{app.name}</div>
                    {!app.is_uwp && app.path && (
                      <div className="max-w-xs truncate text-xs text-text-3" title={app.path}>{app.path}</div>
                    )}
                  </td>
                  <td className="px-3 py-2">
                    <span className="text-xs text-text-2">{app.is_uwp ? "UWP" : "Win32"}</span>
                  </td>
                  <td className="px-3 py-2">
                    <RiskBadge risk={risk} />
                  </td>
                  <td className="px-3 py-2 text-right">
                    <div className="inline-flex">
                      <Toggle checked={app.allowed} onChange={() => onToggle(app.key, !app.allowed)} aria-label={`${app.name} 접근`} />
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ── AI 분석 결과 렌더러 ──────────────────────────────────────────

function AnalysisCard({ markdown }: { markdown: string }) {
  const lines = markdown.split("\n");
  return (
    <div className="space-y-1 text-sm leading-relaxed text-text-2">
      {lines.map((line, i) => {
        if (line.startsWith("## ")) {
          return <h3 key={i} className="mb-1 mt-4 text-base font-bold text-text">{line.slice(3)}</h3>;
        }
        if (line.startsWith("### ")) {
          return <h4 key={i} className="mb-0.5 mt-2 font-semibold text-text">{line.slice(4)}</h4>;
        }
        if (line.startsWith("- ") || line.startsWith("• ")) {
          const content = line.slice(2);
          const html = content.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
          return (
            <div key={i} className="flex gap-2">
              <span className="mt-0.5 shrink-0 text-accent">•</span>
              <span dangerouslySetInnerHTML={{ __html: html }} />
            </div>
          );
        }
        if (/^\d+\.\s/.test(line)) {
          const m = /^(\d+)\.\s+(.+)/.exec(line);
          if (m) {
            const html = m[2].replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
            return (
              <div key={i} className="flex gap-2">
                <span className="w-5 shrink-0 text-right font-semibold text-accent">{m[1]}.</span>
                <span dangerouslySetInnerHTML={{ __html: html }} />
              </div>
            );
          }
        }
        if (!line.trim()) return <div key={i} className="h-1" />;
        const html = line.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
        return <p key={i} dangerouslySetInnerHTML={{ __html: html }} />;
      })}
    </div>
  );
}

// ── 메인 패널 ────────────────────────────────────────────────────

export function DevicePanel() {
  const { phase, status, analysis, error, load, toggleCamera, toggleMic, toggleAppCamera, toggleAppMic, analyze } =
    useDeviceStore();

  const [activeDevice, setActiveDevice] = useState<"camera" | "mic">("camera");

  useEffect(() => {
    if (phase === "idle") void load();
  }, [phase, load]);

  const isLoading = phase === "loading";
  const isAnalyzing = phase === "analyzing";

  const camHighRisk = status ? status.camera_apps.filter((a) => classifyRisk(a) === "high" && a.allowed).length : 0;
  const micHighRisk = status ? status.mic_apps.filter((a) => classifyRisk(a) === "high" && a.allowed).length : 0;

  return (
    <div className="w-full max-w-[940px] space-y-6">
      {/* 헤더 */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-text">카메라·마이크 보호</h2>
          <p className="mt-0.5 text-sm text-text-2">카메라·마이크 접근 권한을 전역 및 앱별로 제어합니다.</p>
        </div>
        <button
          onClick={() => void load()}
          disabled={isLoading}
          className="flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-ink transition hover:brightness-105 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {isLoading ? (
            <>
              <span className="h-4 w-4 flex-shrink-0 animate-spin rounded-full border-2 border-white/40 border-t-white" style={{ willChange: "transform" }} />
              불러오는 중…
            </>
          ) : (
            "새로고침"
          )}
        </button>
      </div>

      {error && (
        <div className="rounded-lg border border-danger bg-danger-bg p-4 text-sm text-danger">{error}</div>
      )}

      {isLoading && !status && (
        <div className="animate-pulse space-y-4">
          <div className="h-24 rounded-[16px] bg-surface-2" />
          <div className="h-24 rounded-[16px] bg-surface-2" />
        </div>
      )}

      {status && (
        <>
          {/* 전역 장치 카드 */}
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <DeviceCard Icon={IconCamera} title="카메라" allowed={status.camera_allowed} onToggle={(v) => void toggleCamera(v)} appCount={status.camera_apps.length} highRiskCount={camHighRisk} />
            <DeviceCard Icon={IconMic} title="마이크" allowed={status.mic_allowed} onToggle={(v) => void toggleMic(v)} appCount={status.mic_apps.length} highRiskCount={micHighRisk} />
          </div>

          {(!status.camera_allowed || !status.mic_allowed) && (
            <div className="rounded-lg border border-info bg-info-bg p-3 text-xs text-info">
              전역 차단이 활성화된 경우 앱별 허용 설정과 관계없이 해당 장치에 접근할 수 없습니다.
            </div>
          )}

          {/* 앱별 접근 제어 */}
          <div className="rounded-[16px] border border-border bg-surface p-5">
            <div className="mb-4 flex gap-1 border-b border-border pb-3">
              <button
                onClick={() => setActiveDevice("camera")}
                className={`rounded px-4 py-1.5 text-sm font-medium transition ${activeDevice === "camera" ? "bg-accent-soft text-accent" : "text-text-2 hover:bg-hover"}`}
              >
                카메라 앱 <span className="ml-1 rounded-full bg-surface-2 px-2 py-0.5 text-xs text-text-2">{status.camera_apps.length}</span>
              </button>
              <button
                onClick={() => setActiveDevice("mic")}
                className={`rounded px-4 py-1.5 text-sm font-medium transition ${activeDevice === "mic" ? "bg-accent-soft text-accent" : "text-text-2 hover:bg-hover"}`}
              >
                마이크 앱 <span className="ml-1 rounded-full bg-surface-2 px-2 py-0.5 text-xs text-text-2">{status.mic_apps.length}</span>
              </button>
            </div>

            {activeDevice === "camera" ? (
              <AppTable apps={status.camera_apps} onToggle={(key, allow) => void toggleAppCamera(key, allow)} />
            ) : (
              <AppTable apps={status.mic_apps} onToggle={(key, allow) => void toggleAppMic(key, allow)} />
            )}
          </div>

          {/* AI 분석 카드 */}
          <div className="rounded-[16px] border border-border bg-surface p-5">
            <div className="mb-4 flex items-center justify-between">
              <h3 className="flex items-center gap-2 font-semibold text-text">
                <IconSparkle size={16} className="text-accent" /> AI 프라이버시 분석
              </h3>
              <button
                onClick={() => void analyze()}
                disabled={isAnalyzing || isLoading}
                className="flex items-center gap-2 rounded-lg bg-accent px-3 py-1.5 text-xs font-medium text-accent-ink transition hover:brightness-105 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isAnalyzing ? (
                  <>
                    <span className="h-3 w-3 flex-shrink-0 animate-spin rounded-full border-2 border-white/40 border-t-white" style={{ willChange: "transform" }} />
                    분석 중…
                  </>
                ) : (
                  "AI 분석"
                )}
              </button>
            </div>

            {isAnalyzing && (
              <div className="flex items-center gap-3 py-6 text-sm text-text-2">
                <span className="h-5 w-5 flex-shrink-0 animate-spin rounded-full border-2 border-accent-soft border-t-accent" style={{ willChange: "transform" }} />
                <span>AI가 카메라·마이크 접근 권한을 분석하고 있습니다…</span>
              </div>
            )}

            {analysis && !isAnalyzing && (
              <div className="rounded-lg border border-accent bg-accent-soft p-4">
                <AnalysisCard markdown={analysis} />
              </div>
            )}

            {!analysis && !isAnalyzing && (
              <p className="py-6 text-center text-sm text-text-3">AI 분석 버튼을 눌러 카메라·마이크 접근 권한을 분석하세요.</p>
            )}
          </div>

          {/* 안내 */}
          <div className="space-y-1 rounded-lg border border-border bg-surface-2 p-4 text-xs text-text-2">
            <p>• 변경 사항은 레지스트리(HKCU)에 즉시 반영되며 재부팅 없이 적용됩니다.</p>
            <p>• 일부 앱은 변경된 설정을 인식하려면 앱 재시작이 필요할 수 있습니다.</p>
            <p>• 전역 차단 시 Windows 설정 → 개인 정보 &amp; 보안 → 카메라/마이크에서도 확인할 수 있습니다.</p>
          </div>
        </>
      )}
    </div>
  );
}
