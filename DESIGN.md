# WinGuard UI 디자인 문서

> 현재 구현 기준 디자인 레퍼런스. 리디자인 시 이 문서를 출발점으로 사용한다.
> 2026.06 리디자인: 토큰 기반 테마 시스템 + 좌측 사이드바 + 우측 AI 슬라이드오버로 전면 개편.
> 2026.07: Tier 1 보안 점검 3종 + Quarantine 백업/복원 UI 추가.

---

## 0. 프로그램 개요

**WinGuard**는 Windows 전용 보안 상태 점검 데스크탑 앱이다.

- **플랫폼:** Windows 11 (ARM64/x64), 데스크탑 전용. 기본 창 **1100×720**, 최소 **800×600**, 리사이즈 가능(`tauri.conf.json`)
- **목적:** 일반 사용자가 자신의 PC 보안 상태를 한눈에 파악하고 위협을 조치하도록 돕는다
- **주요 기능:** Windows 보안 점검(23개 상시 항목 — DNS 하이재킹·랜섬웨어 흔적·레지스트리 변조 탐지 포함)·보안 점수·On-Device AI 분석·프로세스/네트워크/이벤트로그/확장 분석·카메라/마이크 제어·VirusTotal 파일 검사·Fix 백업 및 복원(Quarantine)
- **AI:** 인터넷 전송 없이 기기 내 로컬 추론(Gemma 4 GGUF)
- 트레이 상주, 대부분 기능이 HKCU 레지스트리 접근으로 관리자 권한 없이 동작

---

## 1. 기술 스택

| 항목 | 값 |
|------|-----|
| 프레임워크 | React 18 + TypeScript |
| 스타일링 | Tailwind CSS v3 (JIT) + **CSS 변수 디자인 토큰** |
| 테마 | light / dark / system × **액센트 4종**(Forest·Ocean·Violet·Amber) |
| 아이콘 | **스트로크 SVG** (`src/components/icons.tsx`, lucide 스타일) |
| 폰트 | Pretendard → Apple SD Gothic Neo → Malgun Gothic → system-ui |
| 빌드 | Tauri v2 + Vite |

---

## 2. 디자인 토큰 시스템 (핵심)

색은 **Tailwind 유틸이 아니라 CSS 변수**로 구동한다. 컴포넌트는 시맨틱 토큰 유틸만 쓰고, 실제 색은 JS가 `:root` 변수로 주입한다 → `dark:` 변형 불필요.

### 토큰 → Tailwind 유틸 (`tailwind.config.js`)

| 토큰 | 유틸 예시 | 용도 |
|------|----------|------|
| `--bg` | `bg-bg` | 앱 배경 |
| `--surface` / `--surface-2` | `bg-surface` / `bg-surface-2` | 카드 / 보조 표면 |
| `--border` | `border-border` | 테두리·구분선 |
| `--text` / `--text-2` / `--text-3` | `text-text` / `text-text-2` / `text-text-3` | 본문 / 보조 / 흐림 |
| `--accent` / `--accent-soft` / `--accent-ink` | `bg-accent` / `bg-accent-soft` / `text-accent-ink` | 주 액션·활성 |
| `--ok` `--warn` `--danger` `--info` (+`-bg`) | `text-danger` / `bg-danger-bg` | 심각도 색 |
| `--score` / `--score-track` | (게이지 SVG) | 점수 게이지 |
| `--hover` / `--rail`(+`-border`) | `hover:bg-hover` / `bg-rail` | 호버 / 사이드바 |

> **주의**: 위 토큰 유틸에 Tailwind opacity modifier(`border-danger/30` 등)를 붙이면 CSS 변수 색에는 알파가 합성되지 않아 **규칙 자체가 생성되지 않고 테두리·배경이 소실**된다. 항상 솔리드 토큰 클래스만 사용한다.

### 토큰 주입 (`src/store/settingsStore.ts`)

- `applyThemeTokens(theme, accent)` 가 LIGHT/DARK 팔레트 + 액센트 맵을 합쳐 `:root`에 set, `colorScheme`·`.dark` 토글, system 모드는 `matchMedia` 리스너 등록.
- `useSettingsStore`: `theme`·`accent` 상태(persist: `winguard-settings`). `App.tsx`의 `useEffect([theme, accent])`가 변경 시 재적용.

### 폰트

- 본문: Pretendard. 숫자 정렬은 `tabular-nums` (`font-mono` 금지 — Pretendard를 덮어씀).
- `font-mono`(PID·IP·타임스탬프·격리 기록의 날짜+ID 라벨 등 기술적 표기)는 `tailwind.config.js`의 `mono` 스택에 Pretendard 폴백이 포함되어 있어, 한글이 섞여도 시스템 기본 한글 폰트로 깨지지 않는다.

---

## 3. 전역 레이아웃 (`src/App.tsx`)

```
┌──────────┬───────────────────────────────────────┐
│ Sidebar  │ Topbar (타이틀·AI분석·테마세그·설정기어) │
│ (접이식)  ├───────────────────────────────────────┤
│ 214/68px │ [data-scroll] 중앙 정렬 콘텐츠 (탭 패널) │
│ 7개 내비  │                                        │
└──────────┴───────────────────────────────────────┘
  · AI 슬라이드오버: 우측 440px (uiStore.aiOpen, transform)
  · 설정 패널: 우측 320px 슬라이드 (uiStore.settingsOpen, fixed)
```

- **Sidebar** (`Sidebar.tsx`): 확장 214 / 축소 68px(`uiStore.sidebarExpanded`). 활성 항목 `bg-accent-soft text-accent`. 7개 탭은 `src/lib/tabs.ts`의 `Tab`·`SCREEN_META` 공유.
- **Topbar** (`Topbar.tsx`): 화면 타이틀/서브 + AI 분석 버튼 + 테마 세그먼티드(해/모니터/달 SVG) + 설정 기어.
- 콘텐츠: `[data-scroll]` 컨테이너, 탭 전환 시 `wg-screen` 페이드업 애니메이션(`index.css`). 패널별 max-width(대시보드 900 / 프로세스·네트워크·로그·확장 1040 / 장치 940 / 파일 900).
- 루트는 `flex h-screen`(뷰포트를 꽉 채움)이며 **고정 크기 카드가 아니다**. 실제 창 최소 크기는 800×600(§0) — 사이드바 폭·패널 max-width는 1100×720 기준으로 다듬어졌고, 최소 크기 근처의 레이아웃 붕괴는 별도 검증되지 않았다(§6).

---

## 4. 화면별 핵심

- **Dashboard**: 원형 점수 게이지(`report.score` → dasharray, 색 ok/warn/danger) + 헤드라인/조치 아이템(상위 위험 checks) + 영역별(심각도별) 상태 4칸 + 점수 추이 차트 + 전체 점검 항목(CheckCard). 내보내기(CSV/AI 보고서) 로직 유지. 첫 진입 시 자동 스캔은 **1회만 시도**하며 실패해도 재시도 루프에 빠지지 않는다(수동 "지금 다시 검사"로 재시도).
- **Process·Network·EventLog·Extension**: 헤더 + 스캔 버튼 + AI 결과 카드(`bg-accent-soft`) + 통계/필터/토큰 테이블.
- **Device**: 카메라/마이크 2-카드(SVG 아이콘) + 공통 `Toggle` + 탭별 앱 테이블 + AI 프라이버시 분석.
- **VtScanner**: 드롭/선택 존 + 진행/결과 카드 + 검사 이력.
- **AI 슬라이드오버**(`AiSlideOver.tsx`): Topbar "AI 분석"으로 열림, `LlmPanel`(종합분석/로드맵/Q&A) 호스팅. CheckCard 내 per-항목 AI 설명은 인라인 유지.
- **Settings**(`SettingsPanel.tsx`): 모양(테마 3종 + **액센트 4색 스와치**) → AI 모델 → VT 키 → 시작 프로그램 → **복원 기록**(`QuarantinePanel.tsx`) → 데이터 관리, 순서대로 세로 섹션 나열.

---

## 5. 공통 컴포넌트·패턴

- **심각도**(`lib/severity.ts`): ok/info/warning/danger/critical → 토큰 세트(`bg`=`bg-*-bg`, `text`=`text-*`, `border`, `dot`). critical은 danger 강조. `SeverityBadge`가 렌더.
- **Toggle**(`Toggle.tsx`): 토큰 슬라이드 스위치(켜짐 `bg-accent`). Device·Settings 공용.
- **버튼**: Primary `bg-accent text-accent-ink` / Outline `border-border text-text-2` / Danger `text-danger border-danger`(솔리드 토큰, opacity modifier 금지 — §2). 호버는 `hover:brightness-105` 또는 `hover:bg-hover`.
- **비동기 액션 버튼**(`FixButton.tsx` 패턴): 진행 중 `disabled` + 라벨을 "백업 중…" 류 텍스트로 교체, 실패 시 버튼 하단에 우측 정렬 `text-danger` 인라인 에러 텍스트(별도 토스트/모달 없이 같은 flex-col 안에 조건부 렌더).
- **스피너**: `border-accent-soft border-t-accent animate-spin` + `style={{willChange:"transform"}}`(WebView2 GPU 레이어 필수).
- **아이콘**: `icons.tsx`의 명명 컴포넌트(`IconHistory` 등 포함). `currentColor` 상속 → `text-*`로 색 제어.

---

## 6. 알려진 제약

| 항목 | 현황 |
|------|------|
| 컴포넌트 `invoke` 직접 호출 | `no-restricted-imports` 린트 위반 12개 파일(기존 부채, 2026-07 기준). `CheckCard/Dashboard/FixButton/SettingsPanel.tsx` + `deviceStore/eventlogStore/extensionStore/llmStore/modelStore/networkStore/processStore/quarantineStore.ts`. `api/*.ts` 래퍼화는 별도 과제(AGENTS.md §2) |
| 브라우저 단독 실행(프리뷰) | Tauri `invoke` 미존재로 각 화면의 초기 스캔이 1회 실패하고 에러 배너를 표시한다(무한 루프는 아님 — 수정 완료). 실제 앱에서는 정상 동작 |
| 반응형 | 실제 창은 800×600까지 축소 가능(resizable)하지만, 사이드바 폭·패널 max-width·Topbar 서브타이틀 숨김 등은 1100×720 기준으로만 다듬어졌다. 최소 크기 근처 레이아웃은 별도 검증되지 않음 |
