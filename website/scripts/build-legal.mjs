// docs/PRIVACY.md, docs/TERMS.md 로부터 privacy.html, terms.html 을 만든다.
//
//   node website/scripts/build-legal.mjs [--check]
//
// 법적 고지 문서의 원본은 docs/*.md 하나뿐이다. HTML 은 생성물이므로 손으로
// 고치지 말 것. 고쳐도 다음 빌드에서 덮어써진다.
//
// 머리말(header)과 꼬리말(footer)은 같은 폴더의 다른 페이지에서 그대로 떼어
// 온다. 페이지마다 복제된 구조라 템플릿이 따로 없는데, 이렇게 하면 내비게이션을
// 고쳤을 때 이 두 페이지도 자동으로 따라온다.
//
// 지원하는 마크다운은 원본 두 문서가 실제로 쓰는 것뿐이다. 그 밖의 문법을
// 만나면 조용히 뭉개지 않고 오류를 내고 멈춘다. 법적 문서를 다루므로 애매하게
// 넘어가는 것보다 실패하는 편이 낫다.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { escapeHtml, inline } from "./md-inline.mjs";

const WEBSITE = join(dirname(fileURLToPath(import.meta.url)), "..");
const ROOT = join(WEBSITE, "..");
const CHECK_ONLY = process.argv.includes("--check");

const DOCS = [
  {
    md: join(ROOT, "docs/PRIVACY.md"),
    out: join(WEBSITE, "privacy.html"),
    title: "개인정보처리방침 | WinGuard",
    description:
      "WinGuard 개인정보 처리방침. 수집하지 않는 정보, 로컬 저장 위치, 제3자 서비스로의 통신 범위를 명시합니다.",
    canonical: "privacy.html",
  },
  {
    md: join(ROOT, "docs/TERMS.md"),
    out: join(WEBSITE, "terms.html"),
    title: "이용약관 | WinGuard",
    description:
      "WinGuard 이용약관. 이용 조건, 기능의 한계, 보증의 부인, 책임의 제한을 규정합니다.",
    canonical: "terms.html",
  },
];

const SITE = "https://gsmtc01.github.io/winguard/";

// ── 블록 변환 ────────────────────────────────────────────────

function table(rows, indent = "        ") {
  const cells = (line) =>
    line
      .trim()
      .replace(/^\|/, "")
      .replace(/\|$/, "")
      .split("|")
      .map((c) => c.trim());

  const head = cells(rows[0]);
  const body = rows.slice(2).map(cells); // rows[1] 은 |---| 구분선

  const tr = (cs, tag) =>
    `${indent}      <tr>` + cs.map((c) => `<${tag}>${inline(c)}</${tag}>`).join("") + "</tr>";

  return [
    `${indent}<div class="table-wrap">`,
    `${indent}  <table>`,
    `${indent}    <tbody>`,
    tr(head, "th"),
    ...body.map((cs) => tr(cs, "td")),
    `${indent}    </tbody>`,
    `${indent}  </table>`,
    `${indent}</div>`,
  ].join("\n");
}

function convert(md, file) {
  const lines = md.split(/\r?\n/);
  const out = [];
  let i = 0;
  let sawH1 = false;
  const P = "        ";

  const isTable = (l) => /^\s*\|/.test(l);
  const indentOf = (l) => l.match(/^ */)[0].length;

  while (i < lines.length) {
    const line = lines[i];
    const t = line.trim();

    if (t === "") {
      i++;
      continue;
    }

    // 구분선은 원본 그대로 살린다 (제목 아래, 부칙 앞 구획)
    if (/^-{3,}$/.test(t)) {
      out.push(`${P}<hr />`);
      i++;
      continue;
    }

    // 제목
    if (/^### /.test(t)) {
      out.push(`${P}<h3>${inline(t.slice(4))}</h3>`);
      i++;
      continue;
    }
    // "## 요약" 절만 눈에 띄게 상자로 감싼다. 문서 맨 앞의 요약은 본문보다
    // 먼저 읽히는 편이 낫기 때문이며, 이것이 유일한 예외 규칙이다.
    if (t === "## 요약") {
      i++;
      const paras = [];
      while (i < lines.length && !/^#{1,3} /.test(lines[i].trim())) {
        const l = lines[i].trim();
        if (l !== "" && !/^-{3,}$/.test(l)) paras.push(`${P}    <p>${inline(l)}</p>`);
        i++;
      }
      out.push(
        `${P}<div class="notice-box">`,
        `${P}  <h2>요약</h2>`,
        ...paras,
        `${P}</div>`
      );
      continue;
    }
    if (/^## /.test(t)) {
      out.push(`${P}<h2>${inline(t.slice(3))}</h2>`);
      i++;
      continue;
    }
    if (/^# /.test(t)) {
      if (sawH1) throw new Error(`${file}: H1 이 두 번 나왔습니다`);
      sawH1 = true;
      out.push(`${P}<h1>${inline(t.slice(2))}</h1>`);
      i++;
      // H1 뒤에 오는 굵은 글씨 줄들은 문서 메타(개정일·적용 버전)로 묶는다.
      // 사이에 빈 줄이 있으므로 먼저 건너뛴다.
      while (i < lines.length && lines[i].trim() === "") i++;
      const meta = [];
      while (i < lines.length && /^\*\*.+\*\*$/.test(lines[i].trim())) {
        meta.push(inline(lines[i].trim()).replace(/<\/?strong>/g, ""));
        i++;
      }
      if (meta.length) {
        out.push(`${P}<p class="doc-meta num">`);
        out.push(meta.map((m) => `${P}  ${m}`).join("<br />\n"));
        out.push(`${P}</p>`);
      }
      continue;
    }

    // 인용 (중요 안내)
    if (/^> /.test(t)) {
      const buf = [];
      while (i < lines.length && /^> /.test(lines[i].trim())) {
        buf.push(lines[i].trim().slice(2));
        i++;
      }
      out.push(
        `${P}<blockquote>`,
        `${P}  <p>${inline(buf.join(" "))}</p>`,
        `${P}</blockquote>`
      );
      continue;
    }

    // 코드 블록
    if (/^```/.test(t)) {
      i++;
      const buf = [];
      while (i < lines.length && !/^```/.test(lines[i].trim())) {
        buf.push(lines[i]);
        i++;
      }
      if (i >= lines.length) throw new Error(`${file}: 닫히지 않은 코드 블록`);
      i++;
      out.push(`${P}<pre class="code"><code>${escapeHtml(buf.join("\n"))}</code></pre>`);
      continue;
    }

    // 표
    if (isTable(line) && indentOf(line) === 0) {
      const buf = [];
      while (i < lines.length && isTable(lines[i]) && indentOf(lines[i]) === 0) {
        buf.push(lines[i]);
        i++;
      }
      out.push(table(buf));
      continue;
    }

    // 목록 (번호 / 글머리표). 들여쓴 하위 목록과 표를 품을 수 있다.
    const ordered = /^\d+\. /.test(t);
    if (ordered || /^- /.test(t)) {
      const tag = ordered ? "ol" : "ul";
      const items = [];

      while (i < lines.length) {
        const l = lines[i];
        const lt = l.trim();
        const isItem = indentOf(l) === 0 && (ordered ? /^\d+\. /.test(lt) : /^- /.test(lt));
        if (!isItem) break;

        const content = [inline(lt.replace(/^(\d+\.|-) /, ""))];
        i++;

        // 이 항목에 딸린 들여쓴 블록을 모은다
        const nested = [];
        while (i < lines.length) {
          const n = lines[i];
          if (n.trim() === "") {
            // 뒤에 들여쓴 줄이 이어지면 같은 항목으로 본다
            const next = lines[i + 1];
            if (next !== undefined && next.trim() !== "" && indentOf(next) >= 2) {
              i++;
              continue;
            }
            break;
          }
          if (indentOf(n) < 2) break;
          nested.push(n);
          i++;
        }

        if (nested.length) {
          let j = 0;
          while (j < nested.length) {
            const n = nested[j];
            if (isTable(n)) {
              const buf = [];
              while (j < nested.length && isTable(nested[j])) buf.push(nested[j++]);
              content.push("\n" + table(buf, `${P}    `) + `\n${P}  `);
            } else if (/^- /.test(n.trim())) {
              const buf = [];
              while (j < nested.length && /^- /.test(nested[j].trim())) {
                buf.push(inline(nested[j].trim().slice(2)));
                j++;
              }
              content.push(
                `\n${P}    <ul>\n` +
                  buf.map((b) => `${P}      <li>${b}</li>`).join("\n") +
                  `\n${P}    </ul>\n${P}  `
              );
            } else {
              throw new Error(`${file}: 목록 안에서 처리하지 못한 줄: ${n.trim().slice(0, 60)}`);
            }
          }
        }

        items.push(`${P}  <li>${content.join("")}</li>`);
      }

      out.push(`${P}<${tag}>`, ...items, `${P}</${tag}>`);
      continue;
    }

    // 그 밖은 문단
    if (/^[|>#`]/.test(t)) throw new Error(`${file}: 처리하지 못한 줄: ${t.slice(0, 60)}`);
    out.push(`${P}<p>${inline(t)}</p>`);
    i++;
  }

  return out.join("\n");
}

// ── 페이지 조립 ──────────────────────────────────────────────

/** 형제 페이지에서 머리말/꼬리말을 그대로 떼어 온다. */
function chrome() {
  const src = readFileSync(join(WEBSITE, "404.html"), "utf8");
  const header = src.match(/(    <header class="site-header">[\s\S]*?<\/header>)/);
  const footer = src.match(/(    <footer class="site-footer">[\s\S]*?<\/footer>)/);
  if (!header || !footer) throw new Error("404.html 에서 머리말/꼬리말을 찾지 못했습니다");
  // 404.html 은 어떤 메뉴도 현재 위치가 아니다. 정책 문서도 마찬가지다.
  if (/aria-current/.test(header[1])) throw new Error("404.html 머리말에 aria-current 가 있습니다");
  return { header: header[1], footer: footer[1] };
}

function page({ title, description, canonical, body }) {
  const { header, footer } = chrome();
  return `<!doctype html>
<html lang="ko">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${title}</title>
    <meta name="description" content="${description}" />
    <link rel="icon" type="image/svg+xml" href="./assets/favicon.svg" />

    <meta property="og:type" content="website" />
    <meta property="og:site_name" content="WinGuard" />
    <meta property="og:title" content="${title}" />
    <meta property="og:description" content="${description}" />
    <meta property="og:url" content="${SITE}${canonical}" />
    <meta property="og:image" content="${SITE}assets/og.png" />
    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta name="twitter:card" content="summary_large_image" />
    <link rel="canonical" href="${SITE}${canonical}" />

    <!-- 첫 페인트 전에 테마를 확정해 깜빡임을 막는다. -->
    <script>
      (function () {
        var theme = "light";
        try {
          var saved = localStorage.getItem("winguard-theme");
          if (saved === "light" || saved === "dark") theme = saved;
          else if (
            window.matchMedia &&
            window.matchMedia("(prefers-color-scheme: dark)").matches
          )
            theme = "dark";
        } catch (e) {}
        document.documentElement.dataset.theme = theme;
      })();
    </script>

    <link rel="preload" as="style" href="./assets/fonts.css" />
    <link rel="stylesheet" href="./assets/fonts.css" />
    <link rel="stylesheet" href="./assets/styles.css" />
    <script src="./assets/site.js" defer></script>
  </head>
  <body>
${header}

    <main>
      <!-- 이 페이지는 docs/${canonical === "privacy.html" ? "PRIVACY" : "TERMS"}.md 에서 생성됩니다.
           직접 고치지 말고 원본 마크다운을 고친 뒤
           node website/scripts/build-legal.mjs 를 실행하세요. -->
      <section class="page page--narrow prose">
${body}
      </section>
    </main>

${footer}
  </body>
</html>
`;
}

// ── 실행 ─────────────────────────────────────────────────────

let stale = 0;
for (const d of DOCS) {
  const md = readFileSync(d.md, "utf8");
  const html = page({ ...d, body: convert(md, d.md.split("/").pop()) });

  let current = "";
  try {
    current = readFileSync(d.out, "utf8");
  } catch {}

  const name = d.out.split(/[\\/]/).pop();
  if (current === html) {
    console.log(`${name.padEnd(13)} 최신`);
  } else {
    stale++;
    if (CHECK_ONLY) console.log(`${name.padEnd(13)} 원본과 다름 (빌드 필요)`);
    else {
      writeFileSync(d.out, html, "utf8");
      console.log(`${name.padEnd(13)} 생성함 (${html.length} bytes)`);
    }
  }
}

if (CHECK_ONLY && stale > 0) process.exit(1);
