// src/lib/areas.ts
//
// 점검 항목을 "보안 영역"으로 묶는다.
//
// 대시보드 상단(헤드라인 + 조치 아이템)이 이미 심각도 기준으로 정보를 보여주므로,
// 영역별 상태 카드까지 심각도(정상/경고/위험/심각)로 나누면 같은 분류가 두 번
// 반복된다. 이 모듈은 "어느 분야가 약한가"라는 다른 축을 제공해 중복을 없앤다.

import type { Severity } from "@/lib/severity";

export type AreaId =
  | "malware"
  | "account"
  | "network"
  | "disk"
  | "update"
  | "policy"
  | "etc";

export interface SecurityArea {
  id: AreaId;
  /** 카드 제목 */
  label: string;
  /** 카드 부제 — 이 영역이 무엇을 지키는지 한 줄 설명 */
  desc: string;
}

export const SECURITY_AREAS: SecurityArea[] = [
  { id: "malware", label: "악성코드 방어", desc: "백신 · 랜섬웨어 흔적 · 레지스트리 변조" },
  { id: "account", label: "계정 · 로그인", desc: "암호 · UAC · 화면 잠금 · 로그인 실패" },
  { id: "network", label: "네트워크 · 원격", desc: "방화벽 · RDP · 공유 · DNS · hosts" },
  { id: "disk", label: "디스크 · 부팅", desc: "BitLocker · Secure Boot · TPM" },
  { id: "update", label: "업데이트 · 지원", desc: "Windows · 브라우저 · Office 최신성" },
  { id: "policy", label: "시스템 정책", desc: "PowerShell 실행 정책 · 자동 실행" },
  { id: "etc", label: "기타", desc: "위 영역에 속하지 않는 점검 항목" },
];

/** check.id → 영역. 여기에 없는 id 는 "기타"로 모인다. */
const AREA_OF_CHECK: Record<string, AreaId> = {
  // 악성코드 방어
  windows_defender: "malware",
  ransomware_ioc: "malware",
  pum_check: "malware",

  // 계정 · 로그인
  local_accounts: "account",
  uac: "account",
  autolock: "account",
  login_failures: "account",

  // 네트워크 · 원격
  firewall: "network",
  rdp: "network",
  smb1: "network",
  shares: "network",
  hosts_file: "network",
  dns_hijack: "network",

  // 디스크 · 부팅
  bitlocker: "disk",
  secure_boot: "disk",
  secure_boot_cert: "disk",
  tpm: "disk",

  // 업데이트 · 지원
  windows_updates: "update",
  windows_eol: "update",
  browsers: "update",
  office_eol: "update",

  // 시스템 정책
  ps_policy: "policy",
  autorun: "policy",
};

export function areaOf(checkId: string): AreaId {
  return AREA_OF_CHECK[checkId] ?? "etc";
}

/** 심각도 순위 — 클수록 나쁘다. 영역의 대표 상태를 고를 때 사용. */
const SEVERITY_RANK: Record<Severity, number> = {
  ok: 0,
  info: 1,
  warning: 2,
  danger: 3,
  critical: 4,
};

export interface AreaSummary {
  area: SecurityArea;
  /** 이 영역에서 가장 나쁜 심각도 */
  worst: Severity;
  /** 영역 전체 항목 수 */
  total: number;
  /** 조치가 필요한 항목 수 (warning 이상) */
  issues: number;
}

interface CheckLike {
  id: string;
  severity: Severity;
}

/**
 * 점검 결과를 영역별로 묶어 요약한다.
 * 항목이 하나도 없는 영역은 결과에서 제외한다(빈 카드 방지).
 */
export function summarizeByArea(checks: CheckLike[]): AreaSummary[] {
  const buckets = new Map<AreaId, CheckLike[]>();
  for (const c of checks) {
    const id = areaOf(c.id);
    const bucket = buckets.get(id);
    if (bucket) bucket.push(c);
    else buckets.set(id, [c]);
  }

  return SECURITY_AREAS.flatMap((area) => {
    const items = buckets.get(area.id);
    if (!items || items.length === 0) return [];

    let worst: Severity = "ok";
    let issues = 0;
    for (const c of items) {
      if (SEVERITY_RANK[c.severity] > SEVERITY_RANK[worst]) worst = c.severity;
      if (SEVERITY_RANK[c.severity] >= SEVERITY_RANK.warning) issues += 1;
    }
    return [{ area, worst, total: items.length, issues }];
  });
}
