# src/AGENTS.md

이 디렉토리는 **표시와 사용자 상호작용**만 담당한다.
비즈니스 로직과 시스템 접근은 Rust 백엔드에서 처리한다.

---

## 디렉토리 역할

| 경로 | 역할 | 규칙 |
|------|------|------|
| `api/checks.ts` | `invoke()` 래퍼 + 타입 정의 | 보안 스캔 IPC 호출은 여기만 |
| `api/virustotal.ts` | VT invoke() 래퍼 + 타입 | VT IPC 호출은 여기만 |
| `store/checkStore.ts` | Zustand: 보안 스캔 결과 | 컴포넌트에서 직접 상태 변경 금지 |
| `store/llmStore.ts` | LLM 탭 상태, 로드맵, Q&A 히스토리 | invoke() 직접 허용 |
| `store/processStore.ts` | 프로세스 스캔 + AI 분석 상태 | invoke() 직접 허용 |
| `store/networkStore.ts` | 네트워크 스캔 + AI 분석 상태 | invoke() 직접 허용 |
| `store/settingsStore.ts` | 테마 설정 | localStorage persist |
| `components/` | UI 컴포넌트 | 비즈니스 로직 포함 금지 |
| `lib/severity.ts` | Severity → 색상/문자열 매핑 | 컴포넌트에 색상 하드코딩 금지 |

---

## invoke() 호출 규칙

```typescript
// 올바른 패턴: api/checks.ts 를 경유
import { runScan } from "@/api/checks";
const report = await runScan();

// store/*.ts 는 직접 invoke 허용 (.eslintrc.json overrides 에 반영됨)
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("llm_analyze", { report });

// 금지: 일반 컴포넌트에서 직접 invoke
// (예외: CheckCard.tsx → llm_explain_check 단일 항목 설명)
//
// 컴포넌트가 쓰는 래퍼:
//   api/system.ts     시작 프로그램, 레지스트리 읽기, 설정 창 열기
//   api/quarantine.ts 수정 전 레지스트리 백업
//   api/llm.ts        스캔 리포트 요약
```

---

## AI 응답 렌더링

AI가 반환하는 마크다운 텍스트는 반드시 `MarkdownBlock` 컴포넌트로 렌더링한다.
인라인 파싱을 별도 구현하지 않는다.

```tsx
import { MarkdownBlock } from "./MarkdownBlock";
<MarkdownBlock text={summary} compact />   // compact: CheckCard 등 좁은 공간
<MarkdownBlock text={summary} />           // LlmPanel 등 넓은 공간
```

지원 문법: `## 헤더`, `**bold**`, `- 목록`, `1. 번호 목록`, `| 표 |`

---

## 색상 사용 규칙

```tsx
// 올바른 패턴
import { severityConfig } from "@/lib/severity";
<span className={severityConfig[check.severity].text}>...</span>

// 금지: 색상 클래스 하드코딩
<span className="text-red-500">...</span>
```

---

## FixButton — action_uri 안전 가드

| URI 형태 | 처리 |
|----------|------|
| `ms-settings:xxx` | plugin-opener openUrl() |
| `windowsdefender://xxx` | plugin-opener openUrl() |
| `cmd:xxx` | Tauri invoke() — CMD_MAP 조회 |
| `http://` 또는 기타 | 버튼 미표시 |

---

## 숫자 정렬

숫자에는 `tabular-nums` 클래스를 사용한다. `font-mono`는 Pretendard 폰트를 덮어쓰므로 금지.

---

## 테스트 필수 항목

```typescript
// lib/severity.test.ts
test("모든 Severity 값에 색상이 정의되어 있다")

// components/FixButton.test.tsx
test("http URI 가 들어오면 버튼을 렌더링하지 않는다")
test("ms-settings URI 는 버튼을 렌더링한다")
```
