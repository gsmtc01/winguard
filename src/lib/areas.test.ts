import { describe, it, expect } from "vitest";
import { areaOf, summarizeByArea, SECURITY_AREAS } from "./areas";
import type { Severity } from "./severity";

const chk = (id: string, severity: Severity) => ({ id, severity });

describe("areaOf", () => {
  it("알려진 점검 id 를 해당 영역으로 매핑한다", () => {
    expect(areaOf("windows_defender")).toBe("malware");
    expect(areaOf("uac")).toBe("account");
    expect(areaOf("dns_hijack")).toBe("network");
    expect(areaOf("bitlocker")).toBe("disk");
    expect(areaOf("windows_eol")).toBe("update");
    expect(areaOf("ps_policy")).toBe("policy");
  });

  it("등록되지 않은 id 는 기타로 모은다", () => {
    expect(areaOf("점검_추가_예정")).toBe("etc");
  });
});

describe("summarizeByArea", () => {
  it("영역에서 가장 나쁜 심각도를 대표 상태로 고른다", () => {
    const [malware] = summarizeByArea([
      chk("windows_defender", "ok"),
      chk("ransomware_ioc", "critical"),
      chk("pum_check", "warning"),
    ]);
    expect(malware.area.id).toBe("malware");
    expect(malware.worst).toBe("critical");
    expect(malware.total).toBe(3);
    // warning 이상만 조치 대상
    expect(malware.issues).toBe(2);
  });

  it("info 는 조치 대상으로 세지 않는다", () => {
    const [disk] = summarizeByArea([chk("bitlocker", "info"), chk("tpm", "ok")]);
    expect(disk.worst).toBe("info");
    expect(disk.issues).toBe(0);
  });

  it("항목이 없는 영역은 카드를 만들지 않는다", () => {
    const result = summarizeByArea([chk("uac", "ok")]);
    expect(result).toHaveLength(1);
    expect(result[0].area.id).toBe("account");
  });

  it("SECURITY_AREAS 선언 순서를 유지한다", () => {
    const result = summarizeByArea([
      chk("ps_policy", "ok"),
      chk("windows_defender", "ok"),
      chk("uac", "ok"),
    ]);
    const order = result.map((r) => r.area.id);
    const expected = SECURITY_AREAS.map((a) => a.id).filter((id) => order.includes(id));
    expect(order).toEqual(expected);
  });
});
