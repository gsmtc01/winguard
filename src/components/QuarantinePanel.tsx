// src/components/QuarantinePanel.tsx
//
// Fix 실행 전 격리(백업)된 레지스트리 값의 복원 기록 — SettingsPanel 내 섹션.

import { useEffect } from "react";
import { useQuarantineStore } from "@/store/quarantineStore";
import { IconHistory } from "./icons";

function fmtDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString("ko-KR");
}

export function QuarantinePanel() {
  const { records, isLoading, error, busyId, loadList, restore, remove } = useQuarantineStore();

  useEffect(() => {
    void loadList();
  }, [loadList]);

  return (
    <div className="space-y-3">
      <h3 className="flex items-center gap-1.5 text-sm font-semibold uppercase tracking-wide text-text-2">
        <IconHistory size={15} />
        복원 기록
      </h3>
      <p className="text-xs text-text-3">
        보안 항목을 "지금 조치"할 때 변경 전 값이 자동으로 백업됩니다. 문제가 생기면 원클릭으로 이전 상태로 되돌릴 수 있습니다.
      </p>

      {isLoading && records.length === 0 && (
        <p className="py-4 text-center text-sm text-text-3">불러오는 중…</p>
      )}

      {!isLoading && records.length === 0 && (
        <p className="rounded-lg border border-border bg-surface-2 px-4 py-3 text-xs text-text-3">
          아직 백업된 기록이 없습니다.
        </p>
      )}

      {records.length > 0 && (
        <div className="space-y-2">
          {records.map((r) => {
            const busy = busyId === r.metadata.id;
            return (
              <div
                key={r.metadata.id}
                className="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface-2 px-4 py-3"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <p className="truncate text-sm font-medium text-text">{r.metadata.action_label}</p>
                    {r.metadata.restored && (
                      <span className="flex-shrink-0 rounded-full bg-ok-bg px-1.5 py-0.5 text-xs font-semibold text-ok">
                        복원됨
                      </span>
                    )}
                  </div>
                  <p className="mt-0.5 font-mono text-xs text-text-3">
                    {fmtDate(r.metadata.created_at)} · {r.metadata.check_id}
                  </p>
                </div>
                <div className="flex flex-shrink-0 items-center gap-2">
                  <button
                    onClick={() => void restore(r.metadata.id)}
                    disabled={busy}
                    className="rounded-md border border-accent px-3 py-1.5 text-xs font-medium text-accent transition hover:bg-accent-soft disabled:opacity-50"
                  >
                    {busy ? "복원 중…" : "복원"}
                  </button>
                  <button
                    onClick={() => void remove(r.metadata.id)}
                    disabled={busy}
                    className="rounded-md border border-danger px-3 py-1.5 text-xs font-medium text-danger transition hover:bg-danger-bg disabled:opacity-50"
                  >
                    삭제
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {error && <p className="text-xs text-danger">{error}</p>}
    </div>
  );
}
