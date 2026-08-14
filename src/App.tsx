import { useEffect, useState } from "react";
import { Dashboard } from "./components/Dashboard";
import { VtScanner } from "./components/VtScanner";
import { ProcessPanel } from "./components/ProcessPanel";
import { NetworkPanel } from "./components/NetworkPanel";
import { ExtensionPanel } from "./components/ExtensionPanel";
import { EventLogPanel } from "./components/EventLogPanel";
import { DevicePanel } from "./components/DevicePanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { Sidebar } from "./components/Sidebar";
import { Topbar } from "./components/Topbar";
import { AiSlideOver } from "./components/AiSlideOver";
import { useSettingsStore, applyThemeTokens } from "@/store/settingsStore";
import { useCheckStore } from "@/store/checkStore";
import { useUIStore } from "@/store/uiStore";
import type { Tab } from "@/lib/tabs";

export default function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const { settingsOpen, closeSettings } = useUIStore();
  const { theme, accent, scheduledScan, scanSchedule } = useSettingsStore();
  const { lastScannedAt, runScan } = useCheckStore();

  // 앱 시작 시 + 테마/액센트 변경 시 토큰 재적용 (system 모드 리스너 포함)
  useEffect(() => {
    const cleanup = applyThemeTokens(theme, accent);
    return cleanup;
  }, [theme, accent]);

  // 예약 검사 타이머 (매 1분마다 체크)
  useEffect(() => {
    if (!scheduledScan || !scanSchedule) return;

    const checkSchedule = () => {
      const now = new Date();
      
      // 예약 시간 파싱 (HH:mm)
      const [sh, sm] = scanSchedule.time.split(":").map(Number);
      
      let shouldRun = false;
      const tHour = now.getHours();
      const tMin = now.getMinutes();

      // 현재 시간이 예약 시간과 일치하는지 (분 단위까지)
      const isTimeMatch = tHour === sh && tMin === sm;

      // 주기에 따른 추가 조건 검사
      let isDayMatch = false;
      if (scanSchedule.type === "daily") {
        isDayMatch = true;
      } else if (scanSchedule.type === "weekly") {
        isDayMatch = now.getDay() === (scanSchedule.dayOfWeek ?? 0);
      } else if (scanSchedule.type === "monthly") {
        isDayMatch = now.getDate() === (scanSchedule.dayOfMonth ?? 1);
      } else if (scanSchedule.type === "yearly") {
        isDayMatch = now.getMonth() + 1 === (scanSchedule.month ?? 1) && now.getDate() === (scanSchedule.dayOfMonth ?? 1);
      }

      if (isDayMatch && isTimeMatch) {
        shouldRun = true;
      }

      // 밀린 검사(Catch-up) 처리: 
      // PC가 꺼져있어서 놓친 경우를 대비해, 오늘(또는 이번 주기)에 수행했어야 할 스케줄 시간이 지났는데
      // 아직 수행되지 않았다면 즉시 수행합니다.
      // (단순화를 위해, "마지막 검사 시간이 24시간 이전"이고 "오늘의 예약 시간이 지났다면" 실행하도록 구현)
      if (isDayMatch && !shouldRun) {
        const scheduledTimeToday = new Date(now.getFullYear(), now.getMonth(), now.getDate(), sh, sm).getTime();
        const nowMs = now.getTime();
        
        // 오늘 설정된 시간이 이미 지났고
        if (nowMs >= scheduledTimeToday) {
          // 마지막 검사가 오늘 설정된 시간 이전에 수행되었다면 (또는 아예 없다면)
          if (!lastScannedAt || lastScannedAt < scheduledTimeToday) {
            shouldRun = true;
          }
        }
      }

      if (shouldRun) {
        // 이미 1분 내에 실행되었는지 방지 (중복 방지)
        if (!lastScannedAt || Date.now() - lastScannedAt > 60000) {
          void runScan();
        }
      }
    };

    // 마운트 시 즉시 한 번 체크 (catch-up 등)
    checkSchedule();

    const timerId = window.setInterval(checkSchedule, 60 * 1000); // 1분
    return () => window.clearInterval(timerId);
  }, [scheduledScan, scanSchedule, lastScannedAt, runScan]);

  return (
    <div className="relative flex h-screen overflow-hidden bg-bg text-text">
      <Sidebar tab={tab} onTab={setTab} />

      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar tab={tab} settingsActive={settingsOpen} />

        <div data-scroll className="flex-1 overflow-y-auto">
          <div key={tab} className="wg-screen flex flex-col items-center gap-6 px-[34px] py-[30px]">
            {tab === "dashboard" && <Dashboard />}
            {tab === "virustotal" && <VtScanner />}
            {tab === "process" && <ProcessPanel />}
            {tab === "network" && <NetworkPanel />}
            {tab === "extension" && <ExtensionPanel />}
            {tab === "eventlog" && <EventLogPanel />}
            {tab === "device" && <DevicePanel />}
          </div>
        </div>
      </div>

      {/* AI 슬라이드오버 (항상 마운트 — transform 으로 열고 닫음) */}
      <AiSlideOver />

      {/* 설정 슬라이드 패널 */}
      {settingsOpen && <SettingsPanel onClose={closeSettings} />}
    </div>
  );
}
