import type { ScanReport } from "@/api/checks";

const SEVERITY_LABEL: Record<string, string> = {
  ok:       "정상",
  info:     "정보",
  warning:  "주의",
  danger:   "위험",
  critical: "심각",
};

/** 스캔 결과를 CSV 문자열로 변환한다. */
export function reportToCsv(report: ScanReport): string {
  const scannedAt = new Date(report.scanned_at * 1000).toLocaleString("ko-KR");

  const header = ["점검 항목", "등급", "상태", "메시지", "검사 일시"].join(",");

  const rows = report.checks.map((c) => {
    const cells = [
      c.title,
      SEVERITY_LABEL[c.severity] ?? c.severity,
      c.severity.toUpperCase(),
      c.message,
      scannedAt,
    ].map(csvEscape);
    return cells.join(",");
  });

  // BOM 추가: Excel 에서 한글 깨짐 방지
  return "﻿" + [header, ...rows].join("\r\n");
}

/** CSV 셀 이스케이프: 쉼표·줄바꿈·따옴표 포함 시 따옴표로 감쌈 */
function csvEscape(value: string): string {
  if (/[",\r\n]/.test(value)) {
    return '"' + value.replace(/"/g, '""') + '"';
  }
  return value;
}

/** CSV 문자열을 파일로 다운로드한다 (브라우저/Tauri WebView 공용). */
export function downloadCsv(csv: string, filename: string): void {
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.style.display = "none";
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/** 스캔 결과 파일명 생성: WinGuard_보안점검_20260503_142311.csv */
export function buildFilename(report: ScanReport): string {
  const d = new Date(report.scanned_at * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  const stamp =
    `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}` +
    `_${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`;
  return `WinGuard_보안점검_${stamp}.csv`;
}
