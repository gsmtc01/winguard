/**
 * 경량 마크다운 렌더러
 *
 * 지원 문법:
 *   ## 제목          → <h4>
 *   **굵게**         → <strong>
 *   - 항목           → <ul>/<li>
 *   1. 항목          → <ol>/<li>
 *   | 표 | 헤더 |    → <table>
 *   일반 텍스트       → <p>
 *
 * Props:
 *   text    – 렌더링할 마크다운 문자열
 *   compact – true 이면 간격을 줄인 조밀한 레이아웃 (CheckCard 내부용)
 */
import React from "react";

interface Props {
  text: string;
  compact?: boolean;
}

// ── 인라인 렌더러 (**bold** 처리) ─────────────────────────────

export function renderInline(text: string): React.ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*)/g).map((part, i) =>
    part.startsWith("**") && part.endsWith("**") ? (
      <strong key={i} className="font-semibold text-text">
        {part.slice(2, -2)}
      </strong>
    ) : (
      <span key={i}>{part}</span>
    )
  );
}

// ── 표 파서 ───────────────────────────────────────────────────

function parseTableCells(row: string): string[] {
  // "| a | b | c |" → ["a", "b", "c"]
  return row
    .split("|")
    .map((c) => c.trim())
    .filter((c) => c.length > 0);
}

function isSeparatorRow(row: string): boolean {
  // "| --- | :---: | ---: |" 같은 구분선
  return /^\|[\s|:-]+\|$/.test(row);
}

// ── 메인 렌더러 ───────────────────────────────────────────────

export const MarkdownBlock: React.FC<Props> = ({ text, compact = false }) => {
  const lines = text.split("\n");
  const nodes: React.ReactNode[] = [];
  let i = 0;

  const gap = compact ? "mt-2 first:mt-0" : "mt-4 first:mt-0";
  const pClass = compact
    ? "text-xs text-text-2 leading-relaxed"
    : "text-sm text-text-2 leading-relaxed";
  const liClass = compact
    ? "text-xs text-text-2 leading-relaxed"
    : "text-sm text-text-2 leading-relaxed";

  while (i < lines.length) {
    const raw = lines[i];
    const line = raw.trim();

    // 빈 줄 스킵
    if (!line) { i++; continue; }

    // ── 헤더 (1~6단계) ──────────────────────────────────────────
    const headerMatch = /^(#{1,6})\s+(.+)$/.exec(line);
    if (headerMatch) {
      const level = headerMatch[1].length;
      const content = headerMatch[2];
      const hClass = `${gap} ${compact ? "text-xs" : "text-sm"} font-bold text-text`;
      nodes.push(
        level <= 3 ? <h3 key={i} className={hClass}>{content}</h3> : <h4 key={i} className={hClass}>{content}</h4>
      );
      i++; continue;
    }

    // ── 마크다운 표 ────────────────────────────────────────────
    if (line.startsWith("|")) {
      const tableLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith("|")) {
        tableLines.push(lines[i].trim());
        i++;
      }

      // 구분선 행 제거
      const dataLines = tableLines.filter((l) => !isSeparatorRow(l));
      if (dataLines.length === 0) continue;

      const [headerLine, ...bodyLines] = dataLines;
      const headers = parseTableCells(headerLine);
      const rows = bodyLines.map(parseTableCells);

      nodes.push(
        <div key={`tbl-${i}`} className={`${gap} overflow-x-auto`}>
          <table className="min-w-full text-xs border-collapse rounded-lg overflow-hidden">
            <thead>
              <tr className="bg-surface-2">
                {headers.map((h, j) => (
                  <th
                    key={j}
                    className="border border-border px-3 py-1.5 text-left font-semibold text-text-2 whitespace-nowrap"
                  >
                    {renderInline(h)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row, j) => (
                <tr
                  key={j}
                  className={j % 2 === 0 ? "bg-surface" : "bg-surface-2"}
                >
                  {row.map((cell, k) => (
                    <td
                      key={k}
                      className="border border-border px-3 py-1.5 text-text-2"
                    >
                      {renderInline(cell)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
      continue;
    }

    // ── 번호 목록  1. … ───────────────────────────────────────
    if (/^\d+\.\s+/.test(line)) {
      const items: React.ReactNode[] = [];
      while (i < lines.length) {
        const m = /^\d+\.\s+(.+)/.exec(lines[i].trim());
        if (!m) break;
        items.push(
          <li key={i} className={liClass}>
            {renderInline(m[1])}
          </li>
        );
        i++;
      }
      nodes.push(
        <ol key={`ol-${i}`} className={`${gap} list-decimal list-inside space-y-1 ml-1`}>
          {items}
        </ol>
      );
      continue;
    }

    // ── 불릿 목록  - … ────────────────────────────────────────
    if (line.startsWith("- ") || line.startsWith("• ") || line.startsWith("* ")) {
      const items: React.ReactNode[] = [];
      while (i < lines.length) {
        const t = lines[i].trim();
        if (!t.startsWith("- ") && !t.startsWith("• ") && !t.startsWith("* ")) break;
        items.push(
          <li key={i} className={liClass}>
            {renderInline(t.slice(2))}
          </li>
        );
        i++;
      }
      nodes.push(
        <ul key={`ul-${i}`} className={`${gap} list-disc list-inside space-y-1 ml-1`}>
          {items}
        </ul>
      );
      continue;
    }

    // ── 일반 단락 ─────────────────────────────────────────────
    nodes.push(
      <p key={i} className={`${compact ? "mt-1.5 first:mt-0" : gap} ${pClass}`}>
        {renderInline(line)}
      </p>
    );
    i++;
  }

  return <>{nodes}</>;
};
