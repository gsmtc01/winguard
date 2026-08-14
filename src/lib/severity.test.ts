import { describe, it, expect } from "vitest";
import { severityConfig } from "./severity";
import type { Severity } from "./severity";

describe("severityConfig", () => {
  const allSeverities: Severity[] = ["ok", "info", "warning", "danger", "critical"];

  it("모든 Severity 값에 색상이 정의되어 있다", () => {
    allSeverities.forEach((s) => {
      expect(severityConfig[s].bg).toMatch(/^bg-/);
      expect(severityConfig[s].text).toMatch(/^text-/);
      expect(severityConfig[s].label).not.toBe("");
    });
  });

  it("모든 Severity 값에 border 가 정의되어 있다", () => {
    allSeverities.forEach((s) => {
      expect(severityConfig[s].border).toMatch(/^border-/);
    });
  });
});
