// 마크다운 인라인 문법 변환. build-legal.mjs 와 build-updates.mjs 가 함께 쓴다.
//
// 처리하는 것은 `코드`, **굵게**, [글자](주소) 뿐이다. 블록 문법(제목·목록·표)은
// 각 스크립트가 문서 구조에 맞게 따로 다룬다.

export const escapeHtml = (s) =>
  String(s).replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);

/**
 * 백틱으로 잘라 홀수 번째 조각만 코드로 다룬다. 자리표시자를 쓰지 않으므로
 * 본문에 어떤 글자가 들어와도 충돌하지 않는다.
 *
 * @param {string} text
 * @param {{strict?: boolean}} [opts]
 *   strict 면 처리하지 못한 별표가 남았을 때 오류를 낸다. 법적 고지 문서처럼
 *   조용히 뭉개지면 곤란한 곳에 쓴다. 릴리즈 노트는 사람이 자유롭게 쓰는 글이라
 *   별표 하나에 배포가 멈추면 곤란하므로 strict:false 로 부른다.
 */
export function inline(text, { strict = true } = {}) {
  const parts = String(text).split("`");

  if (parts.length % 2 === 0) {
    if (strict) throw new Error(`닫히지 않은 백틱: ${String(text).slice(0, 60)}`);
    return escapeHtml(text); // 릴리즈 노트에서는 원문 그대로 두고 넘어간다
  }

  return parts
    .map((part, i) => {
      if (i % 2 === 1) return `<code>${escapeHtml(part)}</code>`;

      let s = escapeHtml(part);
      s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (m, t, u) => `<a href="${u}">${t}</a>`);
      s = s.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");

      if (strict && s.includes("*")) {
        throw new Error(`처리하지 못한 별표가 남았습니다: ${String(text).slice(0, 60)}`);
      }
      return s;
    })
    .join("");
}
