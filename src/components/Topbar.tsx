// src/components/Topbar.tsx
// 상단 바 — 화면 타이틀/서브 + AI 분석 버튼 + 테마 세그먼티드 + 설정 기어.

import type { Tab } from "@/lib/tabs";
import { SCREEN_META, SETTINGS_META } from "@/lib/tabs";
import { useSettingsStore } from "@/store/settingsStore";
import type { Theme } from "@/store/settingsStore";
import { useUIStore } from "@/store/uiStore";
import { IconSparkle, IconSun, IconMonitor, IconMoon, IconSettings } from "./icons";

const THEME_SEG: { value: Theme; label: string; Icon: typeof IconSun }[] = [
  { value: "light", label: "라이트", Icon: IconSun },
  { value: "system", label: "시스템", Icon: IconMonitor },
  { value: "dark", label: "다크", Icon: IconMoon },
];

export const Topbar = ({ tab, settingsActive }: { tab: Tab; settingsActive: boolean }) => {
  const { theme, setTheme } = useSettingsStore();
  const { openAi, openSettings } = useUIStore();
  const meta = settingsActive ? SETTINGS_META : SCREEN_META[tab];

  return (
    <div className="flex h-[60px] flex-none items-center gap-[14px] border-b border-border bg-surface px-[26px]">
      <div className="flex flex-none items-baseline gap-[10px]">
        <span className="whitespace-nowrap text-base font-bold tracking-tight text-text">{meta.title}</span>
        <span className="hidden whitespace-nowrap text-xs text-text-3 md:inline">{meta.sub}</span>
      </div>

      <div className="ml-auto flex items-center gap-3">
        {/* AI 분석 */}
        <button
          onClick={openAi}
          className="flex h-9 items-center gap-[7px] rounded-[9px] border border-accent-soft bg-accent-soft px-[14px] text-[13px] font-bold text-accent transition hover:brightness-105"
        >
          <IconSparkle size={15} />
          <span>AI 분석</span>
        </button>

        {/* 테마 세그먼티드 */}
        <div className="flex items-center gap-[2px] rounded-[9px] border border-border bg-surface-2 p-[3px]">
          {THEME_SEG.map(({ value, label, Icon }) => {
            const active = theme === value;
            return (
              <button
                key={value}
                onClick={() => setTheme(value)}
                title={label}
                aria-label={label}
                className={`flex h-[30px] w-9 items-center justify-center rounded-[7px] transition ${
                  active
                    ? "bg-surface text-accent shadow-[0_1px_2px_rgba(0,0,0,.15)]"
                    : "text-text-3"
                }`}
              >
                <Icon size={16} />
              </button>
            );
          })}
        </div>

        {/* 설정 */}
        <button
          onClick={openSettings}
          aria-label="설정 열기"
          className={`flex h-9 w-9 items-center justify-center rounded-[9px] border border-border transition ${
            settingsActive ? "bg-accent-soft text-accent" : "bg-surface text-text-2"
          }`}
        >
          <IconSettings size={17} />
        </button>
      </div>
    </div>
  );
};
