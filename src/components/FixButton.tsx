import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { readRegDword, runSettingsCommand } from "@/api/system";
import { saveRegBackup } from "@/api/quarantine";
import type { RegBackupEntry } from "@/store/quarantineStore";

/** cmd: 접두사 → Tauri 커맨드 매핑 */
const CMD_MAP: Record<string, string> = {
  "cmd:uac-settings": "open_uac_settings",
  "cmd:system-protection": "open_system_protection",
};

/** URI → 버튼 라벨 */
const LABEL_MAP: Record<string, string> = {
  "ms-settings:windowsupdate":           "Windows 업데이트 열기",
  "ms-settings:windowsdefender":         "보안 설정 열기",
  "ms-settings:signinoptions":           "로그인 설정 열기",
  "cmd:uac-settings":                    "UAC 설정 열기",
  "cmd:system-protection":               "시스템 보호 설정 열기",
  "windowsdefender://network":           "방화벽 설정 열기",
  "windowsdefender://devicesecurity":    "장치 보안 열기",
  "windowsdefender://securityprocessor": "보안 프로세서 열기",
};

const UAC_REG_PATH = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";

/** Fix 실행 전 자동 백업할 레지스트리 항목 (§4). fallback 은 키가 아직
 *  존재하지 않을 때(정상 상태) 사용할 기본값 — uac.rs 의 기본값과 동일하다. */
type BackupTemplate = Omit<RegBackupEntry, "original_value"> & { fallback: number };

const BACKUP_MAP: Record<string, BackupTemplate[]> = {
  "cmd:uac-settings": [
    { hive: "HKLM", path: UAC_REG_PATH, name: "EnableLUA", value_type: "DWORD", fallback: 1 },
    {
      hive: "HKLM",
      path: UAC_REG_PATH,
      name: "ConsentPromptBehaviorAdmin",
      value_type: "DWORD",
      fallback: 5,
    },
  ],
};

/** 레지스트리 값을 읽되, 키가 없으면(정상 상태) fallback 을 사용한다. */
async function readDwordOrFallback(t: BackupTemplate): Promise<RegBackupEntry> {
  const original_value = await readRegDword({
    hive: t.hive,
    path: t.path,
    name: t.name,
  }).catch(() => t.fallback);
  return { hive: t.hive, path: t.path, name: t.name, value_type: t.value_type, original_value };
}

export const FixButton = ({ uri }: { uri: string | null }) => {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!uri) return null;
  if (
    !uri.startsWith("ms-settings:") &&
    !uri.startsWith("windowsdefender://") &&
    !Object.prototype.hasOwnProperty.call(CMD_MAP, uri)
  ) return null;

  const handleClick = async () => {
    if (uri.startsWith("ms-settings:") || uri.startsWith("windowsdefender://")) {
      openUrl(uri).catch(() => {
        // opener 실패는 조용히 무시
      });
      return;
    }

    const command = CMD_MAP[uri];
    if (!command) return;

    const backupTemplates = BACKUP_MAP[uri];
    if (backupTemplates && backupTemplates.length > 0) {
      setBusy(true);
      setError(null);
      try {
        const entries = await Promise.all(backupTemplates.map(readDwordOrFallback));
        await saveRegBackup({
          entries,
          checkId: uri.replace(/^cmd:/, ""),
          actionLabel: LABEL_MAP[uri] ?? uri,
        });
      } catch (e) {
        // 백업 실패 → 수정 자체를 중단한다 (§4-11)
        setError(`백업 실패로 수정이 중단되었습니다: ${String(e)}`);
        setBusy(false);
        return;
      }
      setBusy(false);
    }

    runSettingsCommand(command).catch(() => {});
  };

  const label = LABEL_MAP[uri] ?? "설정 열기";

  return (
    <div className="flex flex-col items-end gap-1">
      <button
        onClick={() => void handleClick()}
        disabled={busy}
        className="whitespace-nowrap rounded-lg bg-accent px-3 py-1 text-xs font-semibold text-accent-ink transition hover:brightness-105 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {busy ? "백업 중…" : label}
      </button>
      {error && <p className="max-w-[220px] text-right text-xs text-danger">{error}</p>}
    </div>
  );
};
