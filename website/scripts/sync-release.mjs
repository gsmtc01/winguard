// 최신 GitHub 릴리즈의 값을 사이트 HTML 에 주입한다.
//
//   node website/scripts/sync-release.mjs [--check]
//
// 주입 대상은 `data-rel="..."` 마커가 붙은 요소의 텍스트와,
// `data-rel-href="tag"` 가 붙은 링크의 href 뿐이다. 문서 구조는 건드리지 않는다.
//
//   version     릴리즈 태그            예: v0.1.0
//   date        배포일(KST)            예: 2026.08.15
//   size-x64    x64 실행 파일 크기      예: 약 11.6 MB
//   size-arm64  arm64 실행 파일 크기
//   sha-x64     x64 SHA-256            .sha256 자산에서 읽는다
//   sha-arm64   arm64 SHA-256
//
// 릴리즈 노트 본문은 편집이 필요한 글이라 자동 생성하지 않는다. 대신
// updates.html 에 해당 태그의 항목이 없으면 경고만 남긴다. 해시 정정이
// 릴리즈 노트 누락 때문에 막히면 안 되기 때문이다(옛 해시가 남는 쪽이 더 위험).

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const WEBSITE = join(dirname(fileURLToPath(import.meta.url)), "..");
const REPO = process.env.GITHUB_REPOSITORY || "gsmtc01/winguard";
const API = `https://api.github.com/repos/${REPO}`;
const CHECK_ONLY = process.argv.includes("--check");

const TARGET_FILES = ["index.html", "download.html", "updates.html"];

// ── GitHub API ───────────────────────────────────────────────

const headers = {
  "User-Agent": "winguard-site-sync",
  Accept: "application/vnd.github+json",
};
if (process.env.GITHUB_TOKEN) {
  headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
}

async function get(url, asText = false) {
  const res = await fetch(url, { headers });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} ${url}`);
  return asText ? res.text() : res.json();
}

// ── 값 만들기 ────────────────────────────────────────────────

/** 바이트를 "약 11.6 MB" 형태로. */
function formatSize(bytes) {
  return `약 ${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/** UTC ISO 문자열을 KST 기준 "2026.08.15" 로. */
function formatDateKst(iso) {
  const kst = new Date(new Date(iso).getTime() + 9 * 60 * 60 * 1000);
  const p = (n) => String(n).padStart(2, "0");
  return `${kst.getUTCFullYear()}.${p(kst.getUTCMonth() + 1)}.${p(kst.getUTCDate())}`;
}

/** "<hash>  WinGuard-x64.exe" 형식에서 해시만 뽑는다. */
function parseSha256(text, expectedFile) {
  const line = text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .find((l) => l.length > 0);
  if (!line) throw new Error(`${expectedFile}: .sha256 파일이 비어 있습니다`);

  const hash = line.split(/\s+/)[0];
  if (!/^[0-9a-f]{64}$/i.test(hash)) {
    throw new Error(`${expectedFile}: SHA-256 형식이 아닙니다 (${line.slice(0, 40)})`);
  }
  return hash.toLowerCase();
}

async function collect() {
  const rel = await get(`${API}/releases/latest`);
  const asset = (name) => {
    const a = rel.assets.find((x) => x.name === name);
    if (!a) throw new Error(`릴리즈 ${rel.tag_name} 에 ${name} 자산이 없습니다`);
    return a;
  };

  const x64 = asset("WinGuard-x64.exe");
  const arm = asset("WinGuard-arm64.exe");

  const [shaX64Raw, shaArmRaw] = await Promise.all([
    get(asset("WinGuard-x64.exe.sha256").browser_download_url, true),
    get(asset("WinGuard-arm64.exe.sha256").browser_download_url, true),
  ]);

  return {
    tag: rel.tag_name,
    values: {
      version: rel.tag_name,
      date: formatDateKst(rel.published_at),
      "size-x64": formatSize(x64.size),
      "size-arm64": formatSize(arm.size),
      "sha-x64": parseSha256(shaX64Raw, "WinGuard-x64.exe"),
      "sha-arm64": parseSha256(shaArmRaw, "WinGuard-arm64.exe"),
    },
    tagUrl: `https://github.com/${REPO}/releases/tag/${rel.tag_name}`,
  };
}

// ── 치환 ─────────────────────────────────────────────────────

const escapeHtml = (s) =>
  String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

/**
 * data-rel="KEY" 가 붙은 요소의 텍스트를 바꾼다.
 * 마커 요소는 자식 태그 없이 순수 텍스트만 담기로 약속되어 있으므로 [^<]* 로 충분하다.
 */
function replaceMarkers(html, values) {
  let changed = 0;
  for (const [key, value] of Object.entries(values)) {
    const re = new RegExp(`(<[^<>]*\\bdata-rel="${key}"[^<>]*>)([^<]*)`, "g");
    html = html.replace(re, (m, open, old) => {
      const next = escapeHtml(value);
      if (old !== next) changed++;
      return open + next;
    });
  }
  return { html, changed };
}

function replaceTagHref(html, tagUrl) {
  let changed = 0;
  html = html.replace(
    /(<a[^<>]*\bdata-rel-href="tag"[^<>]*\bhref=")([^"]*)(")/g,
    (m, a, old, c) => {
      if (old !== tagUrl) changed++;
      return a + escapeHtml(tagUrl) + c;
    }
  );
  return { html, changed };
}

// ── 실행 ─────────────────────────────────────────────────────

const { tag, values, tagUrl } = await collect();

console.log(`릴리즈 ${tag}`);
for (const [k, v] of Object.entries(values)) console.log(`  ${k.padEnd(11)} ${v}`);

let totalChanged = 0;
const touched = [];

for (const file of TARGET_FILES) {
  const path = join(WEBSITE, file);
  const before = readFileSync(path, "utf8");

  let r = replaceMarkers(before, values);
  const r2 = replaceTagHref(r.html, tagUrl);
  const after = r2.html;
  const changed = r.changed + r2.changed;

  if (changed > 0) {
    totalChanged += changed;
    touched.push(`${file} (${changed}곳)`);
    if (!CHECK_ONLY) writeFileSync(path, after, "utf8");
  }
}

// updates.html 에 이번 태그의 항목이 있는지 확인한다.
const anchorId = tag.replace(/\./g, "-");
const hasEntry = readFileSync(join(WEBSITE, "updates.html"), "utf8").includes(
  `id="${anchorId}"`
);

const summary = [];
if (totalChanged === 0) {
  console.log("\n변경 없음. 사이트가 이미 최신 릴리즈와 일치합니다.");
} else {
  console.log(`\n${CHECK_ONLY ? "변경 필요" : "갱신함"}: ${touched.join(", ")}`);
  summary.push(`- 릴리즈 값 ${totalChanged}곳 갱신: ${touched.join(", ")}`);
}

if (!hasEntry) {
  const msg = `updates.html 에 ${tag} 항목(id="${anchorId}")이 없습니다. 릴리즈 노트를 직접 추가하세요.`;
  console.log(`\n::warning::${msg}`);
  summary.push(`- :warning: ${msg}`);
} else {
  summary.push(`- updates.html 에 ${tag} 릴리즈 노트 있음`);
}

if (process.env.GITHUB_STEP_SUMMARY) {
  const { appendFileSync } = await import("node:fs");
  appendFileSync(
    process.env.GITHUB_STEP_SUMMARY,
    `## 릴리즈 동기화 (${tag})\n\n${summary.join("\n")}\n`
  );
}

// --check 는 CI 에서 "손댈 게 남았는지"만 보는 용도라 종료 코드로 알린다.
if (CHECK_ONLY && totalChanged > 0) process.exit(1);
