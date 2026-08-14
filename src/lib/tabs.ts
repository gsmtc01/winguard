// src/lib/tabs.ts
// 탭 식별자와 화면 메타(타이틀/서브타이틀) 공유 정의.

export type Tab =
  | "dashboard"
  | "process"
  | "network"
  | "eventlog"
  | "extension"
  | "device"
  | "virustotal";

export const SCREEN_META: Record<Tab, { title: string; sub: string }> = {
  dashboard: { title: "보안 대시보드", sub: "Windows 보안 상태 점검" },
  process: { title: "프로세스 분석", sub: "실행 중인 프로세스 위험도" },
  network: { title: "네트워크 분석", sub: "활성·외부 연결 모니터링" },
  eventlog: { title: "이벤트 로그 분석", sub: "최근 보안 이벤트" },
  extension: { title: "브라우저 확장 분석", sub: "설치된 확장 프로그램 점검" },
  device: { title: "카메라·마이크 보호", sub: "앱별 장치 접근 제어" },
  virustotal: { title: "파일 검사", sub: "VirusTotal 악성 여부 검사" },
};

export const SETTINGS_META = { title: "설정", sub: "WinGuard 환경설정" };
