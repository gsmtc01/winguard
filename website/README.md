# WinGuard 웹사이트

WinGuard 소개 웹사이트. 빌드 도구 없는 정적 HTML/CSS/JS 라서 그대로 정적 호스팅에 올릴 수 있다.

레이아웃은 Claude Design 프로젝트 `WinGuard 웹사이트 제작 완료` 의 시안을 따르고,
**콘텐츠는 실제 저장소와 GitHub 릴리즈에서 가져온 값으로 채워져 있다**
(v0.1.0 / 2026.08.15 기준).

## 구조

```
website/
├── index.html                  ← 소개
├── notices.html                ← 공지사항 목록
├── notice.html                 ← 공지 상세, ?id=N
├── updates.html                ← 업데이트 소식 (릴리즈 노트)
├── qna.html                    ← Q&A
├── download.html               ← 다운로드
├── privacy.html                ← 개인정보처리방침
├── terms.html                  ← 이용약관
├── 404.html                    ← 404 페이지 (호스팅이 자동으로 사용)
├── sitemap.xml                 ← 색인 대상 9개 URL
├── robots.txt
├── scripts/
│   └── sync-release.mjs        ← 릴리즈 값 주입 (아래 참고)
└── assets/
    ├── styles.css              ← 디자인 토큰 + 전 페이지 공통 스타일
    ├── site.js                 ← 테마 토글, 모바일 메뉴, 목록 필터, 공지 상세
    ├── notices.js              ← 공지사항 데이터 (목록·상세 공용 원본)
    ├── favicon.svg             ← 루트 app-icon.svg 사본
    ├── og.png                  ← 공유 카드 이미지 1200×630
    ├── fonts/PretendardVariable.woff2   ← src/assets/fonts 사본 (CDN 의존 없음)
    └── screenshots/            ← 앱 화면 캡처 4장
```

## 배포

`.github/workflows/pages.yml` 이 `website/` 를 GitHub Pages 로 배포한다.
주소는 `https://gsmtc01.github.io/winguard/` 이다.

**처음 한 번은 저장소 Settings > Pages 에서 Source 를 "GitHub Actions" 로 바꿔야
한다.** 워크플로는 Pages 를 대신 켜지 않는다. 켜기 전까지는 아무것도 공개되지 않는다.

| 트리거 | 동작 |
|---|---|
| `website/**` 를 main 에 push | 그대로 배포 |
| 릴리즈 published | 릴리즈 값 주입 → 커밋 → 배포 |
| 수동 실행 (workflow_dispatch) | `sync_release` 를 켜면 값 주입까지 함께 |

앱 CI(`ci.yml`)는 `website/**` 를 `paths-ignore` 로 빼두었다. 사이트만 고친
커밋에 Windows Rust 빌드가 도는 것을 막기 위해서다.

## 로컬 실행

```bash
npx vite website --port 5189
```

## 릴리즈가 바뀔 때

버전·배포일·파일 크기·SHA-256 은 `scripts/sync-release.mjs` 가 자동으로 채운다.
릴리즈를 publish 하면 워크플로가 알아서 돌지만, 손으로도 실행할 수 있다.

```bash
node website/scripts/sync-release.mjs
```

`--check` 를 붙이면 파일을 고치지 않고 어긋난 곳만 알려주며, 어긋나 있으면
종료 코드 1 을 낸다.

주입 지점은 `data-rel` 마커가 붙은 요소뿐이다. **마커를 지우면 그 값은 영원히
갱신되지 않으므로 지우지 말 것.**

| 마커 | 값 | 들어 있는 곳 |
|---|---|---|
| `data-rel="version"` | `v0.1.0` | index (히어로·CTA), download (머리말·해시 라벨), updates (배너·바로가기) |
| `data-rel="date"` | `2026.08.15` | index 히어로, download 머리말, updates 배너 |
| `data-rel="size-x64"` | `약 11.6 MB` | index CTA, download 카드 |
| `data-rel="size-arm64"` | `약 10.5 MB` | download 카드 |
| `data-rel="sha-x64"` | SHA-256 | download |
| `data-rel="sha-arm64"` | SHA-256 | download |
| `data-rel-href="tag"` | 릴리즈 태그 URL | updates |

**자동으로 채워지지 않는 것이 둘 있다.**

1. **릴리즈 노트** (`updates.html` 의 `.timeline`). 다듬어 쓰는 글이라 손으로
   넣는다. 새 태그 항목(`id="v0-2-0"` 형태)이 없으면 스크립트가 경고를 남기지만
   배포는 계속 진행한다. 해시 정정이 릴리즈 노트 누락 때문에 막히면 안 되기
   때문이다(옛 해시가 남는 쪽이 훨씬 위험하다).
2. **공지사항** (`assets/notices.js`).

법적 고지 문서의 "적용 버전: WinGuard 0.1.0 이상" 은 `0.1.0 및 그 이후` 라는
뜻이므로 릴리즈마다 바꾸지 않는다. 그래서 마커를 달지 않았다.

다운로드 버튼은 `releases/latest/download/…` 를 가리켜 링크 자체는 항상 최신
파일을 받는다.

## 정확성에 관해: 앱과 어긋나면 안 되는 사실

보안 제품 사이트라 아래 항목은 실제 동작과 어긋나면 신뢰를 잃는다. 앱을 고칠 때
사이트도 같이 확인할 것.

- **관리자 권한**: `src-tauri/app.manifest` 가 `requireAdministrator` 이고
  `build.rs` 가 release 빌드에만 이를 임베드한다. 따라서 배포된 exe 는 실행할 때마다
  UAC 확인 창을 띄운다. 사이트는 이를 시스템 요구사항 · 설치 가이드 · Q&A ·
  이용약관 제6조에 명시하고 있다. **매니페스트를 바꾸면 이 네 곳을 함께 고칠 것.**
- **코드 서명 없음**: SmartScreen 경고가 뜬다는 안내를 다운로드 페이지 · Q&A ·
  공지 · 이용약관 제6조 · 개인정보 처리방침 제8조에 넣어두었다. 서명을 적용하면
  이 문구를 걷어내야 한다.
- **AI 모델 다운로드**: 첫 사용 시 약 2.89 GiB 를 HuggingFace 에서 받는다.
  다운로드 페이지, Q&A, 방침 제5.2조에 명시되어 있다.
- **외부 통신 대상**: 방침 제5조가 VirusTotal · HuggingFace · GitHub ·
  `endoflife.date` · `versionhistory.googleapis.com` ·
  `product-details.mozilla.org` · `edgeupdates.microsoft.com` 를 열거한다.
  코드에서 호출하는 호스트가 늘면 방침도 함께 고칠 것.
- **격리 자동 정리 50개**: `src-tauri/src/commands/quarantine_commands.rs` 의
  `PRUNE_THRESHOLD` 기준이다. 방침 제4조, 약관 제8조, Q&A 에 나온다.
- **보안 점수 계산**: Q&A 의 "경고 −10, 위험 −20, 심각 −35" 는
  `src-tauri/src/engine/scorer.rs` 와 `src/lib/severity.ts` 기준이다.

## 문장 부호 규칙

**em dash(`—`)를 쓰지 않는다.** 동격이나 부연은 콜론, 괄호, 가운뎃점(`·`)으로
쓰고, 제목 구분자는 파이프(`|`)를 쓴다. 새 문구를 넣은 뒤 아래로 확인할 수 있다.

```bash
grep -rn '—' website
```

## 디자인 토큰

`assets/styles.css` 의 CSS 변수 이름은 데스크톱 앱(`src/index.css`)과 동일하다.
다크 모드는 `:root[data-theme="dark"]` 에서 같은 변수를 덮어쓴다.

| 역할 | 변수 | 라이트 | 다크 |
|---|---|---|---|
| 배경 | `--bg` | `#f4f4f1` | `#0b0e0d` |
| 카드/표면 | `--surface` | `#ffffff` | `#101513` |
| 테두리 | `--border` | `#ecebe5` | `#1a201e` |
| 본문 텍스트 | `--text` | `#26251f` | `#eef2ef` |
| 보조 텍스트 | `--text-2` | `#74726a` | `#99a09a` |
| 주 액센트 | `--accent` | `#2f7a63` | `#35c98a` |
| 액센트 소프트 | `--accent-soft` | `#eef3f1` | `#14211c` |
| 경고 (점검 안내) | `--warn` / `--warn-soft` | `#b5811f` / `#faf3e3` | `#d8a13e` / `#2a2213` |
| 정보 (이벤트) | `--info` / `--info-soft` | `#3a6ea5` / `#eaf1fa` | `#4a86c4` / `#152230` |

테마는 `localStorage['winguard-theme']` 에 저장하고, 값이 없으면
`prefers-color-scheme` 을 따른다. `<head>` 의 인라인 스크립트가 첫 페인트 전에
`data-theme` 을 확정해 깜빡임을 막으므로, 페이지를 새로 추가할 때 이 스크립트도
그대로 복사해야 한다.

모바일 브레이크포인트는 900px (디자인 원본의 JS 분기와 동일한 값).

## 헤더·푸터는 페이지마다 복제되어 있다

빌드 단계가 없으므로 헤더와 푸터 마크업이 9개 페이지에 각각 들어 있다.
**내비게이션 항목이나 푸터 링크를 고칠 때는 9개 파일을 함께 고쳐야 한다.**
현재 위치 표시는 각 페이지의 `<a aria-current="page">` 로만 다르다.

## 스크린샷

`assets/screenshots/` 의 4개 파일은 실제 앱 화면을 캡처한 것이다.
보안 점수는 **가상으로 100점을 만들어 찍은 것**이며(23개 항목 전부 정상),
개발 서버에 `localStorage` 를 직접 심어 렌더링한 뒤 헤드리스 브라우저로 캡처했다.

| 파일 | 위치 | 캡처 크기 |
|---|---|---|
| `hero-light.png` | 히어로 (라이트 테마일 때) | 1240×979 |
| `hero-dark.png` | 히어로 (다크 테마일 때) | 1240×979 |
| `dashboard-light.png` | 쇼케이스 좌측 | 1400×715 |
| `dashboard-dark.png` | 쇼케이스 우측 | 1400×715 |

히어로 이미지는 사이트 테마를 따라 교체된다(`.shot__light` / `.shot__dark`).
파일이 없으면 이미지를 숨기고 placeholder 문구를 대신 보여주므로(`site.js` 의
`error` 핸들러) 콘솔 오류 없이 넘어간다.

캡처 크기는 각 슬롯의 가로세로비에 맞춰둔 값이다. 다시 찍을 때 비율이 달라지면
`object-fit: cover` 로 잘린다.

## 공지사항 추가하기

`assets/notices.js` 의 배열 맨 앞에 항목을 추가한다. 목록과 상세가 같은 파일을
읽으므로 한 군데만 고치면 된다.

- `id` 는 한 번 발행하면 바꾸지 않는다 (`notice.html?id=N` 링크가 깨진다).
- `category` 는 `공지` / `점검 안내` / `이벤트` 중 하나여야 배지 색이 맞는다.
- 이전글·다음글은 배열 순서를 그대로 따른다.
- 현재 2건은 v0.1.0 릴리즈 노트를 근거로 작성한 것이다. 문구·날짜는 필요에 따라
  조정할 것.

## 법적 고지 문서

`privacy.html` 과 `terms.html` 은 별도로 작성된 `PRIVACY.md` · `TERMS.md` 원문을
그대로 옮긴 것이다(2026년 8월 15일 개정, WinGuard 0.1.0 이상 적용).

- 조문 번호와 표 구성을 원문과 1:1로 맞춰두었다. 원문이 개정되면 두 파일을 함께
  갱신하고, 페이지 상단의 개정일과 부칙 날짜도 바꿀 것.
- 방침 제11조의 제3자 사업자 목록은 원문대로 링크 없이 이름만 적었다. 각 사업자
  정책 URL 은 바뀌는 일이 잦아, 확인 없이 링크를 걸면 오히려 신뢰를 해친다.
- 방침 제5조의 외부 통신 범위는 홈 프라이버시 밴드와 Q&A 답변에도 반영되어 있다.
  한쪽만 고치면 서로 어긋나므로 세 곳을 함께 볼 것.

## SEO · 공유 카드

모든 절대 URL 은 `https://gsmtc01.github.io/winguard/` 를 기준으로 한다.
**커스텀 도메인을 붙이면 아래를 전부 새 주소로 바꿔야 한다.**

- 각 페이지의 `og:url` · `canonical` (9개 파일)
- 각 페이지의 `og:image`
- `sitemap.xml` 의 `<loc>` 9개
- `robots.txt` 의 `Sitemap:` 줄

`404.html` 만 `canonical` 없이 `<meta name="robots" content="noindex">` 를 단다.

`assets/og.png` (1200×630) 는 다크 테마 대시보드 캡처를 넣어 만든 공유 카드다.
다시 만들려면 사이트 톤에 맞춘 1200×630 HTML 을 헤드리스 브라우저로 캡처하면 된다.

## 남은 개선거리

- **Pretendard 2MB**: 사이트 용량의 대부분이다. 한글 서브셋으로 줄이면 첫 방문이
  크게 가벼워진다.
- **헤더·푸터 중복**: 페이지를 더 늘릴 계획이면 제너레이터를 도입해 정리하는 편이
  낫다.
