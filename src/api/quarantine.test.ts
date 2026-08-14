import { describe, it, expect, vi, beforeEach } from "vitest";
import type { RegBackupEntry } from "@/store/quarantineStore";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { saveRegBackup } from "./quarantine";

const ENTRIES: RegBackupEntry[] = [
  {
    hive: "HKLM",
    path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System",
    name: "EnableLUA",
    value_type: "DWORD",
    original_value: 1,
  },
];

describe("api/quarantine", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("saveRegBackup 은 요청 객체를 그대로 quarantine_save 로 넘긴다", async () => {
    invokeMock.mockResolvedValue(undefined);
    const req = { entries: ENTRIES, checkId: "uac-settings", actionLabel: "UAC 설정 열기" };
    await saveRegBackup(req);
    expect(invokeMock).toHaveBeenCalledWith("quarantine_save", req);
  });

  it("백업 실패는 호출부가 수정을 중단할 수 있도록 reject 로 전파된다", async () => {
    invokeMock.mockRejectedValue("권한 없음");
    await expect(
      saveRegBackup({ entries: ENTRIES, checkId: "uac-settings", actionLabel: "x" }),
    ).rejects.toBe("권한 없음");
  });
});
