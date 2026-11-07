# WinGuard 웹사이트 디자인

소개, 다운로드, 지원, 정책 페이지가 공유하는 디자인 기준이다.
데스크톱 앱의 디자인은 저장소 루트 `DESIGN.md`를 따른다.

## 방향

- 목적: Windows 사용자가 점검 범위를 이해하고 실행 파일을 내려받게 한다.
- 분위기: 중립색 배경과 짙은 머리말 위에 로고의 초록색을 강조한 보안 제품 사이트.
- 소개: 큰 실제 대시보드 캡처를 앞세운 Split Studio 구성.
- 하위 페이지: 짧은 제목 영역 다음에 목록, 검색, 다운로드 또는 조문을 둔다.
- 새 이용자 수, 후기, 성능 수치 또는 인증 마크를 만들어 넣지 않는다.

## 원본과 토큰

`assets/tokens.css`가 색상, 글꼴, 간격, 크기의 원본이다.

| 역할 | 토큰 |
|---|---|
| 기본 배경과 단계별 표면 | `--color-paper`, `--color-paper-2`, `--color-paper-3` |
| 본문과 보조 글자 | `--color-ink`, `--color-ink-2`, `--color-muted` |
| 경계선 | `--color-rule`, `--color-rule-strong` |
| 앱 Forest 액센트와 그 위 글자 | `--color-accent`, `--color-accent-ink` |
| 머리말과 꼬리말 | `--color-slab`, `--color-slab-ink` |
| 포커스 | `--color-focus`, `--color-slab-accent` |

글꼴은 자체 호스팅 Pretendard Variable이며 코드는 시스템 고정폭 글꼴을 쓴다.
본문은 16px, 버튼은 44~48px 높이, 모서리는 입력 8px와 큰 면 12px를 쓴다.
`--container: 1440px` 안에서 폭을 제한하며 데스크톱의 좌우 여백은 64px이다.
960px 이하에서는 주요 그리드를 한 열로, 640px 이하에서는 여백을 16px로 바꾼다.

## 화면과 상호작용

- 라이트와 다크는 `:root[data-theme]`에서 같은 토큰을 덮어쓴다.
- 테마 선택은 `localStorage['winguard-theme']` 하나에만 저장한다.
- 넓은 배경은 라이트에서 밝은 회색, 다크에서 짙은 회색을 사용한다.
- 다운로드 버튼과 점수 게이지에 초록색 액센트를 사용한다.
- 화면 모드 탭은 선택된 버튼의 반전 채움색과 체크 표시로 구분한다.
- 탭은 클릭과 방향키, Home, End를 지원하고 메뉴는 Escape로 닫는다.
- 움직임 감소 설정을 따르며 스크립트가 없어도 본문과 FAQ를 읽을 수 있다.
- 스크린샷은 기존 앱 캡처 4장을 사용한다. 100점은 예시 상태이다.
- 보안 점수 설명은 확대 캡처 대신 접근성 설명을 포함한 SVG 게이지를 사용한다.

## 파일과 생성

소개는 `assets/styles.css`, 하위 페이지는 이를 가져오는 `assets/pages.css`를 읽는다.
공통 기본 규칙은 `assets/base.css`에 있다.
방침과 약관은 `scripts/build-legal.mjs`, 변경 이력은 `scripts/build-updates.mjs`로 만든다.
릴리즈 값은 기존 `data-rel` 마커를 유지하고 `scripts/sync-release.mjs`로 확인한다.
생성 HTML의 내용은 직접 고치지 않는다.
