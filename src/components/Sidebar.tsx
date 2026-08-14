// src/components/Sidebar.tsx
// 접이식 좌측 사이드바 — 로고/토글 + 7개 내비 항목.

import type { ComponentType } from "react";
import type { Tab } from "@/lib/tabs";
import { useUIStore } from "@/store/uiStore";
import {
  IconShield,
  IconDashboard,
  IconProcess,
  IconNetwork,
  IconEvents,
  IconExtension,
  IconPrivacy,
  IconFiles,
} from "./icons";

type IconCmp = ComponentType<{ size?: number; className?: string; strokeWidth?: number }>;

const NAV_ITEMS: { id: Tab; label: string; icon: IconCmp }[] = [
  { id: "dashboard", label: "보안 대시보드", icon: IconDashboard },
  { id: "process", label: "프로세스 분석", icon: IconProcess },
  { id: "network", label: "네트워크 분석", icon: IconNetwork },
  { id: "eventlog", label: "이벤트 로그", icon: IconEvents },
  { id: "extension", label: "브라우저 확장", icon: IconExtension },
  { id: "device", label: "카메라·마이크", icon: IconPrivacy },
  { id: "virustotal", label: "파일 검사", icon: IconFiles },
];

export const Sidebar = ({ tab, onTab }: { tab: Tab; onTab: (t: Tab) => void }) => {
  const { sidebarExpanded: expanded, toggleSidebar } = useUIStore();

  return (
    <div
      className="flex flex-none flex-col overflow-hidden bg-rail border-r border-rail-border"
      style={{
        width: expanded ? 214 : 68,
        transition: "width .26s cubic-bezier(.4,0,.2,1)",
      }}
    >
      {/* 로고 + 토글 */}
      <div className="flex h-[60px] flex-none items-center gap-[11px] border-b border-rail-border px-5">
        <button
          onClick={toggleSidebar}
          title="사이드바 접기·펼치기"
          aria-label="사이드바 접기·펼치기"
          className="flex h-6 w-6 flex-none items-center justify-center text-accent"
        >
          <IconShield size={24} strokeWidth={1.8} />
        </button>
        {expanded && (
          <b className="whitespace-nowrap text-base tracking-tight text-text">WinGuard</b>
        )}
      </div>

      {/* 내비 목록 */}
      <div className="flex flex-col gap-[3px] p-3">
        {NAV_ITEMS.map(({ id, label, icon: Icon }) => {
          const active = tab === id;
          return (
            <button
              key={id}
              data-tab={id}
              onClick={() => onTab(id)}
              className={`flex h-10 items-center gap-[13px] overflow-hidden rounded-[9px] px-[11px] text-[13px] whitespace-nowrap transition-colors ${
                active
                  ? "bg-accent-soft text-accent font-semibold"
                  : "text-text-2 font-medium hover:bg-hover"
              }`}
            >
              <span className="flex flex-none">
                <Icon size={18} />
              </span>
              {expanded && <span className="whitespace-nowrap">{label}</span>}
            </button>
          );
        })}
      </div>
    </div>
  );
};
