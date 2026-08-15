// GitHub 릴리즈 본문에서 updates.html 의 변경 이력을 만든다.
//
//   node website/scripts/build-updates.mjs [--check]
//
// 릴리즈 노트의 원본은 GitHub 릴리즈 본문 하나뿐이다. updates.html 의
// <!-- build-updates:start --> ~ <!-- build-updates:end --> 사이는 생성물이므로
// 손으로 고치지 말 것. 고쳐도 다음 빌드에서 덮어써진다.
//
// ── 릴리즈 노트 쓰는 법 ─────────────────────────────────────────────────
//
// 릴리즈 본문에서 아래 제목이 붙은 절만 사이트로 옮긴다. 나머지 절(다운로드
// 표, 해시 확인 방법 등)은 이미 사이트의 다른 페이지에 있으므로 무시한다.
//
//   첫 ## 앞 문단      항목 도입 문구
//   ## 신규 | 주요 기능 | 추가     "신규" 태그 목록
//   ## 개선 | 변경                 "개선" 태그 목록
//   ## 수정 | 버그 수정            "수정" 태그 목록
//   ## 알아두실 점 | 주의 | 참고    경고 상자
//
// 목록은 "- " 로 시작하는 줄만 항목이 된다. 알아두실 점 절은 문단으로 다룬다.
// 아는 절이 하나도 없으면 도입 문단과 원문 링크만 남기고 경고를 띄운다.
//
// 초안(draft)과 사전 릴리즈(prerelease)는 싣지 않는다. 다운로드 링크가
// releases/latest 를 가리키는데 GitHub 도 그 대상에서 사전 릴리즈를 빼므로,
// 사이트가 받을 수 없는 버전을 이력에 올리지 않기 위해서다.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { inline } from "./md-inline.mjs";

const WEBSITE = join(dirname(fileURLToPath(import.meta.url)), "..");
const TARGET = join(WEBSITE, "updates.html");
const REPO = process.env.GITHUB_REPOSITORY || "gsmtc01/winguard";
const CHECK_ONLY = process.argv.includes("--check");

const START = "<!-- build-updates:start -->";
const END = "<!-- build-updates:end -->";

const SECTIONS = [
  { keys: ["신규", "주요 기능", "추가"], kind: "added", label: "신규", cls: "" },
  { keys: ["개선", "변경"], kind: "improved", label: "개선", cls: " tag--improved" },
  { keys: ["수정", "버그 수정"], kind: "fixed", label: "수정", cls: " tag--fixed" },
  { keys: ["알아두실 점", "주의", "참고"], kind: "note", label: null, cls: "" },
];

// ── GitHub ───────────────────────────────────────────────────

const headers = {
  "User-Agent": "winguard-site-updates",
  Accept: "application/vnd.github+json",
};
if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;

async function releases() {
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100`, {
    headers,
  });
  if (!res.ok) throw new Error(`릴리즈 목록 조회 실패: ${res.status} ${res.statusText}`);
  const all = await res.json();
  return all
    .filter((r) => !r.draft && !r.prerelease)
    .sort((a, b) => new Date(b.published_at) - new Date(a.published_at));
}

/** UTC ISO 를 KST 기준 "2026.08.15" 로. */
function dateKst(iso) {
  const k = new Date(new Date(iso).getTime() + 9 * 60 * 60 * 1000);
  const p = (n) => String(n).padStart(2, "0");
  return `${k.getUTCFullYear()}.${p(k.getUTCMonth() + 1)}.${p(k.getUTCDate())}`;
}

// ── 릴리즈 본문 해석 ─────────────────────────────────────────

/**
 * 사이트는 em dash 를 쓰지 않는다(website/README.md 의 문장 부호 규칙).
 * 릴리즈 본문은 GitHub 에서 자유롭게 쓰는 글이라 섞여 들어오므로 여기서 바꾼다.
 * 양옆이 빈칸인 em dash 는 동격/부연이므로 콜론으로, 그 밖은 하이픈으로 둔다.
 */
function stripEmDash(text, warn) {
  if (!text.includes("—")) return text;
  warn();
  return text.replace(/\s—\s/g, ": ").replace(/—/g, "-");
}

function parseBody(body) {
  const lines = String(body || "").replace(/\r\n/g, "\n").split("\n");
  const lede = [];
  const groups = { added: [], improved: [], fixed: [] };
  const note = [];

  let current = null; // null = 첫 ## 이전, "skip" = 관심 없는 절
  let matched = 0;

  for (const raw of lines) {
    const t = raw.trim();
    if (t === "" || /^-{3,}$/.test(t)) continue;

    const h = t.match(/^##+\s+(.+?)\s*$/);
    if (h) {
      const title = h[1].replace(/[:：]$/, "").trim();
      const hit = SECTIONS.find((s) => s.keys.includes(title));
      current = hit ? hit.kind : "skip";
      if (hit) matched++;
      continue;
    }

    if (current === null) {
      // 첫 제목 앞 문단은 도입 문구로 쓴다. 첫 문단만 취한다.
      if (lede.length === 0) lede.push(t);
      continue;
    }
    if (current === "skip") continue;

    if (current === "note") {
      note.push(t.replace(/^-\s+/, ""));
      continue;
    }

    if (/^[-*]\s+/.test(t)) groups[current].push(t.replace(/^[-*]\s+/, ""));
    // 목록이 아닌 줄(표, 코드 등)은 변경 이력에 어울리지 않으므로 버린다
  }

  return { lede: lede[0] || "", groups, note, matched };
}

// ── HTML 만들기 ──────────────────────────────────────────────

const anchorOf = (tag) => tag.replace(/\./g, "-");

function releaseHtml(rel, isLatest, warnings) {
  const tag = rel.tag_name;
  let hadEmDash = false;
  const body = stripEmDash(rel.body, () => (hadEmDash = true));
  if (hadEmDash) {
    warnings.push(
      `${tag}: 릴리즈 본문의 em dash 를 콜론/하이픈으로 바꿨습니다. 사이트는 em dash 를 쓰지 않습니다.`
    );
  }

  const { lede, groups, note, matched } = parseBody(body);
  const P = "            ";

  if (matched === 0) {
    warnings.push(
      `${tag}: 옮길 절을 찾지 못했습니다. 릴리즈 본문에 "## 신규" 같은 제목이 있는지 확인하세요.`
    );
  }

  const out = [];
  out.push(`${P}<article class="release" id="${anchorOf(tag)}">`);
  out.push(`${P}  <div class="release__card">`);
  out.push(`${P}    <div class="release__head">`);
  out.push(`${P}      <h2 class="release__version num">${inline(tag, { strict: false })}</h2>`);
  out.push(`${P}      <span class="release__date num">${dateKst(rel.published_at)}</span>`);
  if (isLatest) out.push(`${P}      <span class="badge badge--solid">최신 버전</span>`);
  out.push(`${P}    </div>`);

  if (lede) out.push(`${P}    <p class="release__lede">${inline(lede, { strict: false })}</p>`);

  for (const sec of SECTIONS) {
    if (sec.kind === "note") continue;
    const items = groups[sec.kind];
    if (!items.length) continue;
    out.push(`${P}    <div class="release__group">`);
    out.push(`${P}      <span class="tag${sec.cls}">${sec.label}</span>`);
    out.push(`${P}      <ul>`);
    for (const it of items) out.push(`${P}        <li>${inline(it, { strict: false })}</li>`);
    out.push(`${P}      </ul>`);
    out.push(`${P}    </div>`);
  }

  if (note.length) {
    out.push(`${P}    <div class="release__note">`);
    out.push(
      `${P}      <p><strong>알아두실 점.</strong> ${inline(note.join(" "), { strict: false })}</p>`
    );
    out.push(`${P}    </div>`);
  }

  out.push(`${P}    <p class="release__link">`);
  out.push(
    `${P}      <a href="https://github.com/${REPO}/releases/tag/${tag}">GitHub 릴리즈에서 원문 보기 →</a>`
  );
  out.push(`${P}    </p>`);
  out.push(`${P}  </div>`);
  out.push(`${P}</article>`);
  return out.join("\n");
}

function changelogHtml(list, warnings) {
  const P = "        ";
  const latest = list[0];

  const nav = [`${P}  <nav class="version-nav" aria-label="버전 바로가기">`];
  nav.push(`${P}    <h2>버전 바로가기</h2>`);
  nav.push(`${P}    <a href="#latest">${latest.tag_name} (최신)</a>`);
  for (const r of list.slice(1)) {
    nav.push(`${P}    <a href="#${anchorOf(r.tag_name)}">${r.tag_name}</a>`);
  }
  if (list.length === 1) {
    nav.push(`${P}    <p class="version-nav__note">`);
    nav.push(
      `${P}      첫 공개 릴리즈라 이전 버전이 아직 없습니다. 전체 릴리즈 목록은`
    );
    nav.push(`${P}      <a href="https://github.com/${REPO}/releases">GitHub</a> 에서 볼 수 있습니다.`);
    nav.push(`${P}    </p>`);
  }
  nav.push(`${P}  </nav>`);

  return [
    `${P}<div class="changelog">`,
    `${P}  <div class="timeline">`,
    ...list.map((r, i) => releaseHtml(r, i === 0, warnings)),
    `${P}  </div>`,
    "",
    ...nav,
    `${P}</div>`,
  ].join("\n");
}

// ── 실행 ─────────────────────────────────────────────────────

const list = await releases();
if (list.length === 0) {
  console.error("공개된 릴리즈가 없습니다.");
  process.exit(1);
}

const warnings = [];
const block = changelogHtml(list, warnings);

const html = readFileSync(TARGET, "utf8");
const a = html.indexOf(START);
const b = html.indexOf(END);
if (a === -1 || b === -1 || b < a) {
  console.error(`updates.html 에서 ${START} / ${END} 표시를 찾지 못했습니다.`);
  process.exit(1);
}

// 끝 표시 앞의 들여쓰기는 교체 구간에 들어가므로 다시 붙여준다.
const next =
  html.slice(0, a + START.length) + "\n" + block + "\n        " + html.slice(b);

console.log(`릴리즈 ${list.length}건: ${list.map((r) => r.tag_name).join(", ")}`);

let changed = next !== html;
if (changed && !CHECK_ONLY) writeFileSync(TARGET, next, "utf8");
console.log(changed ? (CHECK_ONLY ? "updates.html 갱신 필요" : "updates.html 갱신함") : "updates.html 최신");

for (const w of warnings) console.log(`::warning::${w}`);

if (process.env.GITHUB_STEP_SUMMARY) {
  const { appendFileSync } = await import("node:fs");
  appendFileSync(
    process.env.GITHUB_STEP_SUMMARY,
    `## 변경 이력 빌드\n\n- 릴리즈 ${list.length}건 반영: ${list.map((r) => r.tag_name).join(", ")}\n` +
      warnings.map((w) => `- :warning: ${w}\n`).join("")
  );
}

if (CHECK_ONLY && changed) process.exit(1);
