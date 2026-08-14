import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Theme = "light" | "dark" | "system";
export type Accent = "Forest" | "Ocean" | "Violet" | "Amber";

export const ACCENTS: Accent[] = ["Forest", "Ocean", "Violet", "Amber"];

export type ScheduleType = "daily" | "weekly" | "monthly" | "yearly";

export interface ScanSchedule {
  type: ScheduleType;
  time: string; // "HH:mm" (24-hour format)
  dayOfWeek?: number; // 0 (Sun) - 6 (Sat)
  dayOfMonth?: number; // 1 - 31
  month?: number; // 1 - 12
}

interface SettingsStore {
  theme: Theme;
  accent: Accent;
  scheduledScan: boolean;
  scanSchedule: ScanSchedule;
  setTheme: (theme: Theme) => void;
  setAccent: (accent: Accent) => void;
  setScheduledScan: (enabled: boolean) => void;
  setScanSchedule: (schedule: ScanSchedule) => void;
}

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set) => ({
      theme: "system",
      accent: "Forest",
      scheduledScan: false,
      scanSchedule: { type: "daily", time: "12:00" },
      setTheme: (theme) => set({ theme }),
      setAccent: (accent) => set({ accent }),
      setScheduledScan: (scheduledScan) => set({ scheduledScan }),
      setScanSchedule: (scanSchedule) => set({ scanSchedule }),
    }),
    { name: "winguard-settings" }
  )
);

// ── 디자인 토큰 팔레트 ────────────────────────────────────────────

const LIGHT: Record<string, string> = {
  "--bg": "#f4f4f1",
  "--surface": "#ffffff",
  "--surface-2": "#faf9f6",
  "--border": "#ecebe5",
  "--text": "#26251f",
  "--text-2": "#74726a",
  "--text-3": "#a7a59b",
  "--rail": "#ffffff",
  "--rail-border": "#ecebe5",
  "--hover": "#f1f0ea",
  "--score": "#c98a2e",
  "--score-track": "#edece7",
  "--danger": "#c2641f",
  "--danger-bg": "#fbeee2",
  "--warn": "#b5811f",
  "--warn-bg": "#f6efdb",
  "--info": "#3a6ea5",
  "--info-bg": "#e9eff0",
  "--ok": "#2f7a63",
  "--ok-bg": "#eef3f1",
};

const DARK: Record<string, string> = {
  "--bg": "#0b0e0d",
  "--surface": "#101513",
  "--surface-2": "#0d1211",
  "--border": "#1a201e",
  "--text": "#eef2ef",
  "--text-2": "#99a09a",
  "--text-3": "#6d756f",
  "--rail": "#0a0c0b",
  "--rail-border": "#1a201e",
  "--hover": "#14211c",
  "--score": "#d8a13e",
  "--score-track": "#1d2421",
  "--danger": "#d97b45",
  "--danger-bg": "#1f140e",
  "--warn": "#d8a13e",
  "--warn-bg": "#1c1710",
  "--info": "#4a86c4",
  "--info-bg": "#101820",
  "--ok": "#35c98a",
  "--ok-bg": "#0f1a15",
};

type AccentTriple = { a: string; s: string; i: string };
const ACCENT_MAP: Record<Accent, { light: AccentTriple; dark: AccentTriple }> = {
  Forest: { light: { a: "#2f7a63", s: "#eef3f1", i: "#ffffff" }, dark: { a: "#35c98a", s: "#14211c", i: "#07120d" } },
  Ocean: { light: { a: "#2563a5", s: "#eaf0f6", i: "#ffffff" }, dark: { a: "#4d9fe6", s: "#11202c", i: "#07131c" } },
  Violet: { light: { a: "#6d4bd1", s: "#f0ecfa", i: "#ffffff" }, dark: { a: "#a78bfa", s: "#1a1630", i: "#0d0a1f" } },
  Amber: { light: { a: "#b5701f", s: "#f7efe2", i: "#ffffff" }, dark: { a: "#e0a23c", s: "#241c10", i: "#1a1206" } },
};

function effectiveMode(theme: Theme): "light" | "dark" {
  if (theme === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return theme === "dark" ? "dark" : "light";
}

function buildTokens(theme: Theme, accent: Accent): Record<string, string> {
  const mode = effectiveMode(theme);
  const base = mode === "dark" ? DARK : LIGHT;
  const a = (ACCENT_MAP[accent] || ACCENT_MAP.Forest)[mode];
  return { ...base, "--accent": a.a, "--accent-soft": a.s, "--accent-ink": a.i };
}

/** 테마·액센트에 따라 :root 의 CSS 변수와 다크 클래스를 갱신한다.
 *  system 모드면 OS 테마 변경 리스너를 등록하고 cleanup 함수를 돌려준다. */
export function applyThemeTokens(theme: Theme, accent: Accent): (() => void) | undefined {
  const root = document.documentElement;

  const paint = () => {
    const mode = effectiveMode(theme);
    const tokens = buildTokens(theme, accent);
    for (const k in tokens) root.style.setProperty(k, tokens[k]);
    root.style.colorScheme = mode;
    root.classList.toggle("dark", mode === "dark");
  };

  paint();

  if (theme === "system") {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => paint();
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }
  return undefined;
}

/** @deprecated applyThemeTokens 사용. 기존 호출부 호환용. */
export function applyTheme(theme: Theme): (() => void) | undefined {
  const { accent } = useSettingsStore.getState();
  return applyThemeTokens(theme, accent);
}
