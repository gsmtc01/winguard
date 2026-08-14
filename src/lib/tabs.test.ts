import { describe, it, expect } from "vitest";
import { SCREEN_META, SETTINGS_META, type Tab } from "./tabs";

const TABS: Tab[] = [
  "dashboard",
  "process",
  "network",
  "eventlog",
  "extension",
  "device",
  "virustotal",
];

describe("lib/tabs", () => {
  it("모든 탭에 화면 메타가 정의되어 있다", () => {
    for (const tab of TABS) {
      expect(SCREEN_META[tab]).toBeDefined();
    }
    expect(Object.keys(SCREEN_META)).toHaveLength(TABS.length);
  });

  it("타이틀과 서브타이틀이 비어 있지 않다", () => {
    for (const tab of TABS) {
      expect(SCREEN_META[tab].title.trim()).not.toBe("");
      expect(SCREEN_META[tab].sub.trim()).not.toBe("");
    }
  });

  it("타이틀은 탭마다 서로 다르다", () => {
    const titles = TABS.map((t) => SCREEN_META[t].title);
    expect(new Set(titles).size).toBe(titles.length);
  });

  it("설정 화면 메타도 채워져 있다", () => {
    expect(SETTINGS_META.title.trim()).not.toBe("");
    expect(SETTINGS_META.sub.trim()).not.toBe("");
  });
});
