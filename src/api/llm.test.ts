import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ScanReport } from "./checks";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { generateReport } from "./llm";

const REPORT: ScanReport = {
  score: 82,
  checks: [],
  scanned_at: 1_700_000_000,
};

describe("api/llm", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("generateReport 는 report 를 감싼 객체로 전달한다", async () => {
    invokeMock.mockResolvedValue({ summary: "요약" });
    await expect(generateReport(REPORT)).resolves.toEqual({ summary: "요약" });
    expect(invokeMock).toHaveBeenCalledWith("llm_generate_report", { report: REPORT });
  });

  it("generateReport 는 실패를 그대로 전파한다", async () => {
    invokeMock.mockRejectedValue("모델 없음");
    await expect(generateReport(REPORT)).rejects.toBe("모델 없음");
  });
});
