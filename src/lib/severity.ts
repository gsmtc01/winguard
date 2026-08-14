export type Severity = "ok" | "info" | "warning" | "danger" | "critical";

export interface SeverityStyle {
  /** 연한 배경 (bg-*-bg 토큰) */
  bg: string;
  /** 텍스트/아이콘 색 (text-* 토큰) */
  text: string;
  /** 테두리 색 */
  border: string;
  /** 도트 배경 */
  dot: string;
  label: string;
  /** 이 심각도로 인한 점수 감점 (Ok/Info는 null) */
  deduction: string | null;
}

// 토큰 4색(ok/info/warn/danger)으로 매핑.
// critical 은 danger 계열을 공유하되 라벨로 강조한다.
export const severityConfig: Record<Severity, SeverityStyle> = {
  ok: {
    bg: "bg-ok-bg",
    text: "text-ok",
    border: "border-ok",
    dot: "bg-ok",
    label: "정상",
    deduction: null,
  },
  info: {
    bg: "bg-info-bg",
    text: "text-info",
    border: "border-info",
    dot: "bg-info",
    label: "정보",
    deduction: null,
  },
  warning: {
    bg: "bg-warn-bg",
    text: "text-warn",
    border: "border-warn",
    dot: "bg-warn",
    label: "경고",
    deduction: "-10점",
  },
  danger: {
    bg: "bg-danger-bg",
    text: "text-danger",
    border: "border-danger",
    dot: "bg-danger",
    label: "위험",
    deduction: "-20점",
  },
  critical: {
    bg: "bg-danger-bg",
    text: "text-danger",
    border: "border-danger",
    dot: "bg-danger",
    label: "심각",
    deduction: "-35점",
  },
};
