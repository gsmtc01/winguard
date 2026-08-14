// src/lib/reportPdf.ts
//
// 보고서 Markdown → 인쇄용 HTML 변환.
// window.open()으로 팝업을 열고 window.print()를 호출하면
// OS 인쇄 다이얼로그에서 "PDF로 저장" 할 수 있다.

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderInlineHtml(text: string): string {
  return escapeHtml(text).replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
}

function isTableSeparator(line: string): boolean {
  return /^\|[\s|:-]+\|$/.test(line.trim());
}

function parseTableCells(row: string): string[] {
  return row
    .split("|")
    .map((c) => c.trim())
    .filter((c) => c.length > 0);
}

/** Markdown 문자열을 HTML 본문으로 변환한다. */
export function markdownToHtml(markdown: string): string {
  const lines = markdown.split("\n");
  let html = "";
  let i = 0;

  while (i < lines.length) {
    const raw = lines[i];
    const line = raw.trim();

    if (!line) { i++; continue; }

    // --- 수평선
    if (/^-{3,}$/.test(line) || /^\*{3,}$/.test(line)) {
      html += "<hr/>\n";
      i++; continue;
    }

    // # H1 (## 아닌 것만)
    if (line.startsWith("# ") && !line.startsWith("## ")) {
      html += `<h1>${renderInlineHtml(line.slice(2))}</h1>\n`;
      i++; continue;
    }

    // ## H2 (### 아닌 것만)
    if (line.startsWith("## ") && !line.startsWith("### ")) {
      html += `<h2>${renderInlineHtml(line.slice(3))}</h2>\n`;
      i++; continue;
    }

    // ### H3
    if (line.startsWith("### ")) {
      html += `<h3>${renderInlineHtml(line.slice(4))}</h3>\n`;
      i++; continue;
    }

    // > 인용
    if (line.startsWith("> ")) {
      html += `<blockquote>${renderInlineHtml(line.slice(2))}</blockquote>\n`;
      i++; continue;
    }

    // | 표
    if (line.startsWith("|")) {
      const tableLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith("|")) {
        tableLines.push(lines[i].trim());
        i++;
      }
      const dataLines = tableLines.filter((l) => !isTableSeparator(l));
      if (dataLines.length === 0) continue;
      const [headerLine, ...bodyLines] = dataLines;
      const headers = parseTableCells(headerLine);
      const rows = bodyLines.map(parseTableCells);
      html += `<table><thead><tr>${headers.map((h) => `<th>${renderInlineHtml(h)}</th>`).join("")}</tr></thead>`;
      html += `<tbody>${rows
        .map(
          (row, ri) =>
            `<tr class="${ri % 2 === 0 ? "even" : "odd"}">${row
              .map((cell) => `<td>${renderInlineHtml(cell)}</td>`)
              .join("")}</tr>`
        )
        .join("")}</tbody></table>\n`;
      continue;
    }

    // 1. 번호 목록
    if (/^\d+\.\s+/.test(line)) {
      html += "<ol>\n";
      while (i < lines.length) {
        const m = /^\d+\.\s+(.+)/.exec(lines[i].trim());
        if (!m) break;
        html += `<li>${renderInlineHtml(m[1])}</li>\n`;
        i++;
      }
      html += "</ol>\n";
      continue;
    }

    // - 불릿 목록
    if (line.startsWith("- ") || line.startsWith("• ")) {
      html += "<ul>\n";
      while (i < lines.length) {
        const t = lines[i].trim();
        if (!t.startsWith("- ") && !t.startsWith("• ")) break;
        html += `<li>${renderInlineHtml(t.slice(2))}</li>\n`;
        i++;
      }
      html += "</ul>\n";
      continue;
    }

    // 일반 단락
    html += `<p>${renderInlineHtml(line)}</p>\n`;
    i++;
  }

  return html;
}

/** 인쇄/PDF용 완성 HTML 문서를 생성한다. */
export function buildPrintHtml(
  reportMarkdown: string,
  meta: { score: number; scannedAt: string }
): string {
  const scoreColor =
    meta.score >= 80 ? "#16a34a" : meta.score >= 60 ? "#ca8a04" : "#dc2626";
  const body = markdownToHtml(reportMarkdown);

  return `<!DOCTYPE html>
<html lang="ko">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width,initial-scale=1.0">
  <title>WinGuard 보안 보고서</title>
  <style>
    *{box-sizing:border-box;margin:0;padding:0}
    body{
      font-family:'Malgun Gothic','맑은 고딕','Apple SD Gothic Neo',
                  'Noto Sans KR','나눔고딕',sans-serif;
      font-size:11pt;line-height:1.75;color:#111827;
      background:#fff;padding:36px;max-width:800px;margin:0 auto;
    }
    .report-header{
      border-bottom:2.5px solid #2563eb;padding-bottom:16px;margin-bottom:28px;
    }
    .report-header .title{font-size:18pt;font-weight:800;color:#1d4ed8;margin-bottom:6px;}
    .report-header .meta{font-size:9pt;color:#6b7280;line-height:1.6;}
    .score-pill{
      display:inline-block;margin-top:10px;padding:4px 16px;border-radius:999px;
      font-size:13pt;font-weight:700;color:#fff;background:${scoreColor};
    }
    h1{font-size:15pt;font-weight:700;color:#111827;margin:24px 0 10px;}
    h2{
      font-size:12pt;font-weight:700;color:#1e40af;
      margin:22px 0 8px;padding-bottom:4px;
      border-bottom:1px solid #dbeafe;
    }
    h3{font-size:11pt;font-weight:600;color:#374151;margin:14px 0 6px;}
    p{margin:5px 0;word-break:keep-all;}
    ul,ol{margin:6px 0 6px 22px;}
    li{margin:3px 0;}
    strong{font-weight:700;color:#111827;}
    hr{border:none;border-top:1px solid #e5e7eb;margin:18px 0;}
    blockquote{
      border-left:3px solid #93c5fd;padding:6px 14px;
      color:#374151;background:#eff6ff;margin:8px 0;border-radius:0 4px 4px 0;
    }
    table{width:100%;border-collapse:collapse;margin:10px 0;font-size:9.5pt;}
    th{background:#1e40af;color:#fff;padding:6px 10px;text-align:left;font-weight:600;}
    td{padding:5px 10px;border-bottom:1px solid #e5e7eb;}
    tr.even{background:#f9fafb;}tr.odd{background:#fff;}
    .report-footer{
      margin-top:36px;padding-top:12px;border-top:1px solid #e5e7eb;
      font-size:8pt;color:#9ca3af;text-align:center;
    }
    @media print{
      body{padding:0;}
      @page{margin:18mm 14mm;size:A4;}
      h2{page-break-before:auto;}
      table,blockquote{page-break-inside:avoid;}
      .no-print{display:none!important;}
    }
  </style>
</head>
<body>
  <div class="report-header">
    <div class="title">🛡️ WinGuard 보안 점검 보고서</div>
    <div class="meta">
      점검 일시: ${meta.scannedAt}<br>
      생성: WinGuard On-Device AI 자동 분석
    </div>
    <div class="score-pill">보안 점수 &nbsp;${meta.score} / 100</div>
  </div>

  ${body}

  <div class="report-footer">
    이 보고서는 WinGuard 보안 점검 도구에 의해 자동 생성되었습니다. &nbsp;|&nbsp; ${meta.scannedAt}
  </div>

  <script>
    // 팝업 열린 직후 인쇄 다이얼로그 표시
    window.onload = function() { window.print(); };
  </script>
</body>
</html>`;
}

/** 팝업 창을 열어 PDF 인쇄 다이얼로그를 표시한다. */
export function openPrintWindow(html: string): void {
  const popup = window.open("", "_blank", "width=900,height=700,scrollbars=yes");
  if (!popup) {
    alert("팝업 차단이 활성화되어 있습니다. 팝업을 허용한 후 다시 시도하세요.");
    return;
  }
  popup.document.open();
  popup.document.write(html);
  popup.document.close();
}
