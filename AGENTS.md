# AGENTS.md — WinGuard

Windows PC 보안 점검 도구.
Tauri v2 (Rust 백엔드 + React 프론트엔드), Windows 전용.

UI 디자인 토큰·레이아웃 상세는 `DESIGN.md` 참조.

---

## 목차

1. [프로젝트 구조](#1-프로젝트-구조)
2. [아키텍처 규칙](#2-아키텍처-규칙)
3. [자동 검사 — 코드 품질 강제](#3-자동-검사--코드-품질-강제)
4. [새 점검 항목 추가 절차](#4-새-점검-항목-추가-절차)
5. [구현된 점검 모듈 목록](#5-구현된-점검-모듈-목록)
6. [AI 기능 (llm_commands)](#6-ai-기능-llm_commands)
7. [내비게이션·설정 패널 및 공통 UI](#7-내비게이션설정-패널-및-공통-ui)
8. [VirusTotal 파일 검사 모듈](#8-virustotal-파일-검사-모듈)
9. [Quarantine 백업·복원 시스템](#9-quarantine-백업복원-시스템)
10. [점검 모듈 구현 규칙](#10-점검-모듈-구현-규칙)
11. [테스트 작성 가이드](#11-테스트-작성-가이드)
12. [오류 처리 규칙](#12-오류-처리-규칙)
13. [절대 금지](#13-절대-금지)
14. [빌드 및 실행 명령](#14-빌드-및-실행-명령)
15. [업데이트 확인 (update_commands)](#15-업데이트-확인-update_commands)
16. [데이터 초기화 (설정 > 데이터 관리)](#16-데이터-초기화-설정--데이터-관리)

---

## 1. 프로젝트 구조

```
winguard/
├── AGENTS.md
├── DESIGN.md                        ← UI 디자인 토큰·레이아웃 레퍼런스
├── src-tauri/
│   ├── Cargo.toml
│   ├── clippy.toml
│   ├── app.manifest            ← 관리자 권한 + Common Controls v6 (ASCII only, 한글 주석 금지)
│   ├── build.rs                ← release 빌드에서만 admin manifest 임베드
│   └── src/
│       ├── main.rs
│       ├── checks/             ← 상시 점검 모듈 (run_scan 에서 호출)
│       │   ├── AGENTS.md
│       │   ├── mod.rs
│       │   ├── defender.rs, updates.rs, firewall.rs, ...  (23개 상시 점검 모듈 + virustotal.rs)
│       │   └── (상세 목록은 §5 참조)
│       ├── commands/           ← on-demand Tauri 커맨드
│       │   ├── mod.rs
│       │   ├── llm_commands.rs        ← LLM 추론 (분석/설명/Q&A/로드맵/프로세스·네트워크·확장·이벤트로그·장치 분석/보고서)
│       │   ├── process_commands.rs    ← 실행 프로세스 스캔 (scan_processes)
│       │   ├── network_commands.rs    ← TCP 연결 스캔 (scan_network)
│       │   ├── browser_commands.rs    ← 브라우저 확장 스캔 (scan_extensions)
│       │   ├── eventlog_commands.rs   ← 보안 이벤트 로그 스캔 (scan_eventlog)
│       │   ├── device_commands.rs     ← 카메라·마이크 접근 제어 + 시작 프로그램 등록
│       │   ├── quarantine_commands.rs ← Fix 백업/복원 (격리 시스템, §9)
│       │   ├── update_commands.rs     ← 업데이트 확인 (GitHub Releases, §15)
│       │   └── vt_commands.rs         ← VT 파일 검사 + API 키 관리
│       ├── engine/
│       │   ├── AGENTS.md
│       │   ├── types.rs
│       │   ├── rules.rs
│       │   └── scorer.rs
│       └── util/
│           ├── powershell.rs        ← PowerShell 실행 유틸 (UTF-8 preamble + CREATE_NO_WINDOW)
│           └── registry.rs          ← HKLM/HKCU 레지스트리 읽기 유틸
├── src/
│   ├── AGENTS.md
│   ├── api/
│   │   ├── checks.ts                ← run_scan() invoke 래퍼 + 타입
│   │   ├── virustotal.ts            ← VT invoke() 래퍼 + 타입
│   │   ├── update.ts                ← 업데이트 확인 invoke 래퍼 + 자동 확인 주기 (§15)
│   │   └── files.ts                 ← 파일 저장 invoke 래퍼 (CSV/MD 내보내기)
│   ├── store/
│   │   ├── checkStore.ts            ← Zustand: 보안 스캔 결과 (persist: report/lastScannedAt 만)
│   │   ├── dataReset.ts             ← 데이터 관리 초기화 오케스트레이션 (§16)
│   │   ├── settingsStore.ts         ← 테마(light/dark/system) + 액센트 4종 + CSS 토큰 주입
│   │   ├── uiStore.ts               ← 설정 패널·AI 슬라이드오버·사이드바 접힘 상태
│   │   ├── llmStore.ts              ← LLM 탭 상태, 로드맵, Q&A 히스토리
│   │   ├── processStore.ts          ← 프로세스 스캔 + AI 분석 상태
│   │   ├── networkStore.ts          ← 네트워크 스캔 + AI 분석 상태
│   │   ├── extensionStore.ts        ← 브라우저 확장 스캔 + AI 분석 상태
│   │   ├── eventlogStore.ts         ← 이벤트 로그 스캔 + AI 분석 상태
│   │   ├── deviceStore.ts           ← 카메라·마이크 상태 + AI 분석 상태
│   │   ├── modelStore.ts            ← On-Device AI 모델 다운로드/경로 관리
│   │   └── quarantineStore.ts       ← 격리(백업) 기록 조회/복원/삭제 (§9)
│   ├── components/
│   │   ├── Sidebar.tsx              ← 접이식 좌측 내비게이션 (7탭)
│   │   ├── Topbar.tsx               ← 화면 타이틀 + AI 분석 버튼 + 테마 세그먼트 + 설정 기어
│   │   ├── AiSlideOver.tsx          ← 우측 AI 분석 슬라이드오버 (LlmPanel 호스팅)
│   │   ├── Dashboard.tsx            ← 보안 대시보드 (점수 게이지 + CheckCard 목록)
│   │   ├── CheckCard.tsx            ← 개별 점검 항목 카드 (AI 상세 설명 포함)
│   │   ├── LlmPanel.tsx             ← AI 보안 분석 콘텐츠 (종합분석/로드맵/Q&A 탭)
│   │   ├── ProcessPanel.tsx         ← 프로세스 이상 탐지 UI
│   │   ├── NetworkPanel.tsx         ← 네트워크 연결 분석 UI
│   │   ├── ExtensionPanel.tsx       ← 브라우저 확장 분석 UI
│   │   ├── EventLogPanel.tsx        ← 보안 이벤트 로그 분석 UI
│   │   ├── DevicePanel.tsx          ← 카메라·마이크 접근 제어 UI
│   │   ├── VtScanner.tsx            ← VirusTotal 파일 검사 UI
│   │   ├── SettingsPanel.tsx        ← 설정 슬라이드 패널 (모양/AI모델/VT키/시작프로그램/복원기록/데이터관리)
│   │   ├── QuarantinePanel.tsx      ← "복원 기록" 섹션 (SettingsPanel 내, §9)
│   │   ├── MarkdownBlock.tsx        ← 공용 마크다운 렌더러 (##/bold/list/table)
│   │   ├── SeverityBadge.tsx
│   │   ├── FixButton.tsx            ← action_uri 처리 + Fix 전 자동 백업 (§9)
│   │   ├── Toggle.tsx               ← 공용 토큰 슬라이드 스위치
│   │   ├── icons.tsx                ← 스트로크 SVG 아이콘 세트
│   │   └── ApiKeyModal.tsx          ← SettingsPanel re-export (하위 호환용)
│   └── lib/
│       ├── severity.ts              ← Severity → 토큰 클래스 매핑
│       ├── areas.ts                 ← 점검 id → 보안 영역 매핑 (대시보드 "영역별 상태")
│       ├── tabs.ts                  ← Tab 타입 + 화면 타이틀/서브 메타
│       ├── export.ts                ← CSV 내보내기
│       ├── history.ts               ← 점수 이력 (localStorage)
│       ├── vtHistory.ts             ← VT 검사 이력 (localStorage)
│       └── reportPdf.ts             ← AI 보고서 PDF 렌더링
├── .eslintrc.json
├── .npmrc                           ← shamefully-hoist=true (ARM64 rollup 호환)
└── .github/workflows/ci.yml
```

---

## 2. 아키텍처 규칙

### 레이어 의존 방향

```
checks/ → engine/types   (단방향)
engine/ → util/          (단방향)
commands/ → checks/, engine/   (단방향)
main.rs → checks/, engine/, commands/

프론트엔드 → src/api/*.ts → Tauri IPC → Rust
          예외: store/*.ts 에서 invoke() 직접 허용 (스캔·AI 계열)
```

### 프론트엔드 invoke() 사용 규칙 (의도된 아키텍처)

| 파일 | invoke() 허용 | 비고 |
|------|:---:|------|
| `src/api/*.ts` | ✅ | 기본 경로 — 신규 IPC 호출은 원칙적으로 여기 추가 |
| `src/store/*.ts` (스캔·AI 계열) | ✅ | on-demand 스캔 + AI 분석은 스트리밍/폴링 특성상 store에서 직접 허용: `llmStore`, `processStore`, `networkStore`, `extensionStore`, `eventlogStore`, `deviceStore`, `modelStore`, `quarantineStore` |
| `src/components/*.tsx` | ❌ | store/ 또는 api/ 경유가 원칙 |
| 예외: `CheckCard.tsx` | ✅ | `llm_explain_check` 단일 항목 설명 |
| 예외: `FixButton.tsx` | ✅ | Fix 실행 전 `read_reg_dword`/`quarantine_save` 백업 + 실제 커맨드 실행 (§9) |

> **알려진 이탈**: `Dashboard.tsx`(`llm_generate_report` 보고서 생성), `SettingsPanel.tsx`(AI 모델 관리) 도 현재 invoke() 를 직접 사용한다. 위 표는 지향점이며, `eslint --max-warnings 0` 기준으로 총 12개 파일이 `no-restricted-imports` 위반 상태다(2026-07 기준: `CheckCard/Dashboard/FixButton/SettingsPanel.tsx` + `deviceStore/eventlogStore/extensionStore/llmStore/modelStore/networkStore/processStore/quarantineStore.ts`). 새 코드는 표의 원칙을 따르고 기존 위반을 확산시키지 않는다. `api/*.ts` 래퍼화는 별도 정리 과제로 남아 있다.

컴포넌트가 `CheckResult` 타입이 필요하면 `@/api/checks` 대신 `@/store/checkStore` re-export를 사용한다.

### VT 모듈 위치 원칙

`checks/virustotal.rs` 는 on-demand 검사 전용이다.
`run_scan()` 이 호출하는 상시 점검 루프에 포함하지 않는다.

---

## 3. 자동 검사 — 코드 품질 강제

| 검사 | 도구 | 실행 시점 |
|------|------|-----------|
| Rust 린트 | `cargo clippy -- -D warnings` | pre-commit, CI |
| Rust 포맷 | `cargo fmt --check` | pre-commit, CI |
| Rust 타입 | `cargo check` | CI |
| TS 린트 | `eslint --max-warnings 0` | pre-commit, CI |
| TS 타입 | `tsc --noEmit` | CI |
| 단위 테스트 | `cargo test` + `vitest` | CI |

> ARM64 개발 머신에서 `cargo check`/`clippy`/`test`/`fmt` 를 실행하려면 §14의 ARM64 툴체인 환경변수(vcvarsall + LLVM)가 선행되어야 한다. 그렇지 않으면 `llama-cpp-sys-2` 네이티브 빌드가 기본 MSVC `cl.exe` 로 실패한다.

---

## 4. 새 점검 항목 추가 절차

```
Step 1. src-tauri/src/checks/{name}.rs   생성
Step 2. src-tauri/src/checks/mod.rs      pub mod 등록
Step 3. src-tauri/src/engine/rules.rs    복합 룰 추가 (필요 시)
Step 4. src-tauri/src/main.rs            run_scan() 에 호출 추가
Step 5. src/api/checks.ts                TypeScript 타입 확인
Step 6. src/lib/areas.ts                 AREA_OF_CHECK 에 id → 보안 영역 등록 (§7)
```

on-demand 기능(VT, LLM, 프로세스/네트워크/확장/이벤트로그/장치 스캔, Quarantine)은 `commands/` 에 별도 커맨드로 만든다.

---

## 5. 구현된 점검 모듈 목록

| 모듈 | ID | 데이터 소스 | 비고 |
|------|-----|------------|------|
| `defender.rs` | `windows_defender` | WMI SecurityCenter2 + Get-MpComputerStatus | 타사 백신 자동 감지 |
| `updates.rs` | `windows_updates` | Microsoft.Update.AutoUpdate COM | 대기 업데이트 수 + 마지막 설치일 |
| `firewall.rs` | `firewall` | 레지스트리 FirewallPolicy\*Profile | Domain/Standard/Public 3개 프로파일 |
| `secure_boot.rs` | `secure_boot` | 레지스트리 SecureBoot\State | PowerShell 대신 레지스트리 (인코딩 회피) |
| `eol.rs` | `windows_eol` | 레지스트리 CurrentVersion + endoflife.date API | 24h 캐시, 빌드번호 ≥ 22000 → Win11 보정 |
| `account.rs` | `local_accounts` | Get-LocalUser + ValidateCredentials | 암호 없는 활성 계정 감지 (ValidateCredentials로 실제 빈 암호 검증) |
| `bitlocker.rs` | `bitlocker` | WMI Win32_EncryptableVolume | Windows Home 미지원 → Info 폴백 |
| `smb1.rs` | `smb1` | 레지스트리 LanmanServer\Parameters\SMB1 | 키 없으면 기본값 비활성화(안전) |
| `uac.rs` | `uac` | 레지스트리 EnableLUA + ConsentPromptBehaviorAdmin | action_uri `cmd:uac-settings` (§9 백업 대상) |
| `rdp.rs` | `rdp` | 레지스트리 fDenyTSConnections + UserAuthentication | RDP+NLA꺼짐 → Danger |
| `autolock.rs` | `autolock` | HKCU\Control Panel\Desktop | 15분 초과 또는 잠금 없음 → Warning |
| `hosts.rs` | `hosts_file` | C:\Windows\System32\drivers\etc\hosts | 민감 도메인 리디렉션 → Danger |
| `autorun.rs` | `autorun` | 레지스트리 NoDriveTypeAutoRun | 이동식 드라이브 AutoRun → Danger |
| `ps_policy.rs` | `ps_policy` | Get-ExecutionPolicy -List | Unrestricted/Bypass → Danger |
| `login_failures.rs` | `login_failures` | 이벤트 로그 ID 4625 | 24h 내 20회 이상 → Danger |
| `shares.rs` | `shares` | Get-SmbShare PowerShell | 사용자 정의 공유 → Warning; 루트 공유 → Danger |
| `tpm.rs` | `tpm` | Get-Tpm PowerShell | 없음/비활성/1.2 → Warning; 2.0+ → Ok |
| `secure_boot_cert.rs` | `secure_boot_cert` | Get-SecureBootUEFI kek/db | 2023 인증서 미적용 → Danger |
| `browsers.rs` | `browsers` | 레지스트리 Uninstall + 공개 API | Edge/Chrome/Firefox; 메이저 3+ 뒤처짐 → Danger |
| `office_eol.rs` | `office_eol` | 레지스트리 Uninstall | Office 버전 EOL 여부 |
| `dns_hijack.rs` | `dns_hijack` | WMI Win32_NetworkAdapterConfiguration | 활성 NIC DNS가 허용목록(공용 DNS+사설 게이트웨이) 밖 공인 IP → Danger; IPv6 비-`::1` → Info; NIC 최대 10개 |
| `ransomware_ioc.rs` | `ransomware_ioc` | WMI Win32_ShadowCopy + Run 키 PowerShell 열거 + HKCR 파일 연결 | 3개 하위 점검(섀도 복사본·시작프로그램·확장자 연결) 점수 합산 → Ok/Warning/Danger/Critical; 메시지는 "시스템 보호" 용어 사용, action_uri `cmd:system-protection`(§9, BACKUP_MAP 미등록) |
| `pum_check.rs` | `pum_check` | 레지스트리 PUM_TABLE(regedit/cmd/작업관리자 차단, 보안센터 알림, CryptSvc) + LowRiskFileTypes | 값 변조 시 위반; 키 부재는 정상으로 처리(삭제가 아니라 변조가 위험 신호) |

`virustotal.rs` 는 위 목록과 별개로 on-demand 전용이다 (§8 참조, `run_scan()` 미포함).

### 복합 규칙 (engine/rules.rs)

| 규칙 | 조건 | 효과 |
|------|------|------|
| EOL + Secure Boot | EOL Danger + Secure Boot Warning | Secure Boot → Danger |
| SMBv1 + EOL | SMBv1 Danger + EOL Danger | SMBv1 → Critical |
| RDP + 방화벽 | RDP Warning+ + 방화벽 Warning+ | RDP → Critical |
| AV 없음 + 방화벽 | AV Critical + 방화벽 Warning+ | 방화벽 → Critical |
| DNS 변조 + hosts 변조 | dns_hijack Danger+ + hosts_file Danger+ | dns_hijack → Critical |
| 랜섬웨어 IOC + AV 없음 | ransomware_ioc Danger+ + AV Critical | ransomware_ioc → Critical |
| PUM 변조 + AV 없음 | pum_check Danger+ + AV Critical | pum_check → Critical |

규칙은 severity를 올리기만 하며 절대 낮추지 않는다.

---

## 6. AI 기능 (llm_commands)

### 모델

- **Gemma 4 E2B Instruct Q4_K_M** — HuggingFace `unsloth/gemma-4-E2B-it-GGUF`, 약 2.9 GB
- **로컬 파일명은 업스트림 이름을 쓰지 않는다.** `winguard-ai-gemma4-e2b-q4_k_m.gguf` 로 고정
  (`MODEL_FILENAME`). 배포처가 파일명을 바꿔도 로컬 경로가 흔들리지 않게 하려는 것이며,
  **사용자는 저장 폴더만 고르고 파일명은 고를 수 없다** (설정 UI는 폴더 선택 다이얼로그)
- 구버전이 업스트림 이름으로 저장해 둔 파일은 `LEGACY_MODEL_FILENAMES` 로 인식만 해서
  재다운로드를 막는다. 새로 저장할 때는 쓰지 않는다
- 다운로드는 `<파일명>.part` 로 받아 완료 시에만 rename 한다. 크기가 `Content-Length` 와
  다르면 오류로 처리하고 `.part` 를 지운다 (끊긴 파일을 "설치됨"으로 오인 방지)
- 파일명·표시명·용량은 `llm_model_info` 커맨드로 내려준다. **프런트엔드에 하드코딩 금지**
- HuggingFace에서 다운로드, 설치 디렉터리 또는 사용자 지정 폴더에 저장
- 경로 설정은 `%LOCALAPPDATA%\WinGuard\llm_settings.json` (다운로드 성공 후에만 기록)
- `llama-cpp-2 0.1` 크레이트 사용 (`LlamaBackend`, `LlamaModel`, `LlamaBatch`, `LlamaSampler`)
- 모델은 전역 `static Mutex<Option<ModelHolder>>` 로 캐싱 (첫 호출 시 로드, 이후 재사용)

### Tauri 커맨드 목록

| 커맨드 | 입력 | temperature | 용도 |
|--------|------|:-----------:|------|
| `llm_analyze` | `ScanReport` | 0.0 (greedy) | 종합 보안 분석 |
| `llm_explain_check` | `CheckResult` | 0.0 | 개별 항목 심층 설명 |
| `llm_roadmap` | `ScanReport` | 0.0 | 단계별 보안 로드맵 |
| `llm_ask` | `ScanReport` + `question` | 0.7 (샘플링) | 자연어 Q&A |
| `llm_analyze_processes` | `Vec<ProcessInfo>` | 0.0 | 프로세스 이상 탐지 분석 |
| `llm_analyze_network` | `Vec<NetworkConn>` | 0.0 | 네트워크 연결 분석 |
| `llm_analyze_extensions` | `Vec<BrowserExtension>` | 0.0 | 브라우저 확장 위험 분석 |
| `llm_analyze_eventlog` | `Vec<EventLogEntry>` | 0.0 | 이벤트 로그 이상 징후 분석 |
| `llm_analyze_device` | `DeviceStatus` | 0.0 | 카메라·마이크 프라이버시 분석 |
| `llm_generate_report` | `ScanReport` | 0.0 | Markdown 종합 보고서 생성 (대시보드 내보내기) |

### 추론 규칙

- `temperature == 0.0` → `LlamaSampler::greedy()` (결정적, 분석/로드맵/설명에 적합)
- `temperature > 0.0` → `LlamaSampler::temp(t) + dist(SystemTime::now().subsec_nanos())` (Q&A)
- **Q&A 프롬프트는 `[질문]`을 맨 앞에 배치** — 소형 2B 모델이 질문보다 컨텍스트에 집중하는 현상 방지
- N_CTX = 8192, N_GENERATE = 1024
- Gemma 2 chat 형식: `<start_of_turn>user\n…<end_of_turn>\n<start_of_turn>model\n`
- 응답은 `<end_of_turn>` 토큰(EOG) 감지 시 자동 종료

### 이벤트

| 이벤트 | payload | 시점 |
|--------|---------|------|
| `llm:phase` | `"loading"` | 모델 로드 시작 |
| `llm:phase` | `"analyzing"` | 토큰 생성 시작 |
| `llm:download-progress` | `{downloaded, total, percent}` | 모델 다운로드 중 |

### 프로세스 / 네트워크 / 확장 / 이벤트로그 / 장치 스캔 커맨드

**`scan_processes`** (`process_commands.rs`)
- `Get-Process | Where-Object {$_.Path -ne $null} | Sort CPU | Top 60`
- `Get-AuthenticodeSignature`로 서명 확인
- 반환: `Vec<ProcessInfo>` (`pid, name, path, company, signed, cpu, memory_mb`)

**`scan_network`** (`network_commands.rs`)
- `Get-NetTCPConnection | Where-Object {RemoteAddress != 루프백} | Top 100`
- 각 연결의 소유 프로세스 이름 포함
- 반환: `Vec<NetworkConn>` (`local_address, local_port, remote_address, remote_port, state, pid, process_name`)

**`scan_extensions`** (`browser_commands.rs`)
- Chrome/Edge/Firefox 설치 확장 목록 + 권한
- 반환: `Vec<BrowserExtension>`

**`scan_eventlog`** (`eventlog_commands.rs`)
- 최근 보안 이벤트 로그 조회. `event_id_label`/`event_id_level` 이 이벤트 ID → 설명/위험도를 매핑
- 반환: `Vec<EventLogEntry>`

**`get_device_status` / `set_camera_access` / `set_mic_access` / `set_app_camera_access` / `set_app_mic_access`** (`device_commands.rs`)
- 카메라·마이크 전역/앱별 접근 제어. HKCU 기반, 관리자 권한 불필요
- `get_startup_enabled` / `set_startup_enabled` 도 같은 파일 — Windows 시작 프로그램 등록

### 프론트엔드 AI 패널 구조

```
App.tsx
├── Topbar.tsx "AI 분석" → AiSlideOver.tsx → LlmPanel.tsx (탭: 종합분석 / 로드맵 / Q&A)
├── Dashboard.tsx → CheckCard.tsx (AiExplainPanel — 항목별 설명)
├── ProcessPanel.tsx   → processStore.ts   → scan_processes   + llm_analyze_processes
├── NetworkPanel.tsx   → networkStore.ts   → scan_network     + llm_analyze_network
├── ExtensionPanel.tsx → extensionStore.ts → scan_extensions  + llm_analyze_extensions
├── EventLogPanel.tsx  → eventlogStore.ts  → scan_eventlog    + llm_analyze_eventlog
└── DevicePanel.tsx    → deviceStore.ts    → get_device_status + llm_analyze_device
```

### MarkdownBlock.tsx (공용 렌더러)

모든 AI 응답 렌더링에 사용한다. 직접 마크다운 파싱을 중복 구현하지 않는다.

지원 문법: `## 헤더` → `<h4>`, `**bold**` → `<strong>`, `- 항목` → `<ul>`, `1. 항목` → `<ol>`, `| 표 |` → `<table>`

Props: `text: string`, `compact?: boolean`

---

## 7. 내비게이션·설정 패널 및 공통 UI

### 화면 구조 (App.tsx)

가로 탭 방식에서 **접이식 좌측 사이드바 + 상단 Topbar** 구조로 전환되었다(2026-06 리디자인). 상세 레이아웃·색상 토큰은 `DESIGN.md` 참조.

| Tab (`lib/tabs.ts`) | 컴포넌트 | 설명 |
|-------|----------|------|
| `dashboard` | `Dashboard.tsx` | 보안 점수 게이지 + 점검 결과 + AI 분석 |
| `process` | `ProcessPanel.tsx` | 실행 프로세스 이상 탐지 |
| `network` | `NetworkPanel.tsx` | 네트워크 연결 분석 |
| `eventlog` | `EventLogPanel.tsx` | 보안 이벤트 로그 분석 |
| `extension` | `ExtensionPanel.tsx` | 브라우저 확장 분석 |
| `device` | `DevicePanel.tsx` | 카메라·마이크 접근 제어 |
| `virustotal` | `VtScanner.tsx` | VirusTotal 파일 검사 |

새 화면 탭 추가 시: `lib/tabs.ts`(`Tab` + `SCREEN_META`) + `Sidebar.tsx`(내비 항목) + `App.tsx`(라우팅 분기) 3곳을 함께 수정한다.

설정은 Topbar 우측 기어 아이콘이 `SettingsPanel` 슬라이드 패널을 열며 활성 탭과 독립적이다. AI 분석은 Topbar의 "AI 분석" 버튼이 `AiSlideOver.tsx`(우측 슬라이드오버, `LlmPanel.tsx` 호스팅)를 연다.

### 테마 · 디자인 토큰 아키텍처

```
src/store/settingsStore.ts
  Theme  = "light" | "dark" | "system"
  Accent = "Forest" | "Ocean" | "Violet" | "Amber"   (ACCENTS 배열)
  useSettingsStore()   ← Zustand + localStorage persist ("winguard-settings")
  applyThemeTokens(theme, accent)
    → LIGHT/DARK 팔레트 + 액센트 맵을 합쳐 :root 에 CSS 변수로 주입
    → documentElement.style.colorScheme, classList.toggle("dark", ...)
    → theme === "system" 이면 matchMedia 리스너 등록/해제
```

- 색은 하드코딩 Tailwind 팔레트나 `dark:` prefix가 아니라 **CSS 변수 토큰**으로 구동한다 (`bg-surface`, `text-text-2`, `bg-accent` 등). 토큰 전체 목록·팔레트 값은 `DESIGN.md` §2 참조.
- 새 UI는 `tailwind.config.js` 에 매핑된 토큰 유틸만 사용한다. `text-red-500` 같은 raw Tailwind 팔레트 클래스 금지.
- **`border-danger/30` 같은 opacity modifier는 CSS 변수 색에는 적용되지 않아 규칙 자체가 생성되지 않는다** (Tailwind가 `var()` 색에 알파를 합성하지 못함) — 반드시 솔리드 토큰 클래스(`border-danger`)만 사용한다.
- `font-mono` 는 숫자뿐 아니라 시간/ID 라벨에도 쓰이므로 `tailwind.config.js` 의 `mono` 스택에 Pretendard 폴백이 포함되어 있다(한글 글리프 대비). 숫자 정렬 자체는 여전히 `tabular-nums` 를 우선한다.

### 시스템 트레이 (main.rs)

- 트레이 메뉴: 열기 / 종료
- 창 닫기 → `hide()` + `api.prevent_close()` (트레이로 최소화)
- `update_tray_tooltip(score)`: "WinGuard — 보안 점수: 87/100 (양호)" 형태로 업데이트

### cmd: URI 스킴 (FixButton.tsx)

| 형태 | 처리 방식 |
|------|-----------|
| `ms-settings:xxx` | plugin-opener `openUrl()` — 백업 없이 즉시 실행 |
| `windowsdefender://xxx` | plugin-opener `openUrl()` — 백업 없이 즉시 실행 |
| `cmd:xxx` | `BACKUP_MAP` 등록 항목을 `read_reg_dword`+`quarantine_save` 로 백업한 뒤 `CMD_MAP` 커맨드 실행 (§9) |

새 `cmd:` 항목 추가 시: `FixButton.tsx` 의 `CMD_MAP`+`LABEL_MAP` 등록 + `main.rs` 커맨드 등록. 그 커맨드가 레지스트리를 직접 수정한다면 `BACKUP_MAP` 에도 반드시 등록한다(§9, §13).

### Quarantine 복원 UI

Fix 실행 전 백업된 레지스트리 값은 SettingsPanel 내 `QuarantinePanel.tsx`("복원 기록" 섹션)에서 조회·복원·삭제한다. 상세는 §9.

### SettingsPanel 섹션 구성

`AppearanceSection` → `ModelSection`(§6) → `ApiKeySection`(§8) → `StartupSection` →
`ScheduleSection` → `QuarantinePanel`(§9) → `DataSection`(§16) → `UpdateSection`(§15) → `AppFooter`

### 대시보드 "영역별 상태" (Dashboard.tsx)

`AreaStatusGrid` 는 심각도(정상/경고/위험/심각)로 다시 나누지 않는다. 게이지·조치 목록·
전체 점검 항목이 이미 심각도 축을 보여주므로, 같은 분류를 반복하면 정보가 중복된다.
대신 `lib/areas.ts` 의 `summarizeByArea()` 로 **보안 분야**(악성코드 방어 / 계정·로그인 /
네트워크·원격 / 디스크·부팅 / 업데이트·지원 / 시스템 정책)별로 묶어 각 영역의 최악 심각도와
`정상 n/m` 을 보여준다.

새 점검 항목을 추가하면 `lib/areas.ts` 의 `AREA_OF_CHECK` 에도 id 를 등록한다.
등록하지 않으면 "기타" 카드로 모인다(동작은 하지만 분류가 의미를 잃는다).

### 점수 배점 UI

배점 기준: Warning −10 · Danger −20 · Critical −35 (100점에서 감점, 0 이하 clamp)

숫자에는 반드시 `tabular-nums` 클래스 사용. 숫자 정렬 목적으로 `font-mono` 를 쓰지 않는다.

---

## 8. VirusTotal 파일 검사 모듈

### API 흐름 (VT API v3)

```
사용자 파일 선택
    → SHA256 로컬 계산 (파일 내용 미전송)
    → GET /api/v3/files/{sha256}
        ├─ 200 OK → 즉시 결과 표시
        └─ 404 → 업로드 동의 다이얼로그
                   → POST /api/v3/files
                   → GET /api/v3/analyses/{id} 폴링 (5초 간격, 최대 60초)
```

### API 키 저장

```
Windows Credential Manager: 서비스 "winguard", 계정 "virustotal_api_key"
```

평문 파일·환경변수·코드 하드코딩 금지. API 키는 Rust 백엔드에서만 읽는다.
`delete_password()` 사용 (`delete_credential()` 아님 — keyring 2.x 기준).

### 판정 기준

| malicious | suspicious | 표시 |
|-----------|------------|------|
| > 0 | any | 악성 |
| 0 | > 0 | 의심 |
| 0 | 0 | 안전 |

---

## 9. Quarantine 백업·복원 시스템

Fix 버튼(`FixButton.tsx`)이 레지스트리를 수정하기 전에 원본 값을 자동 백업하고, 사용자가 원클릭으로 이전 상태로 되돌릴 수 있게 하는 안전장치다.

### 데이터 스키마 (평문 JSON, 디스크)

```
%LOCALAPPDATA%\WinGuard\quarantine\
└── {timestamp_ms}\
    ├── metadata.json   { id, check_id, action_label, created_at, restored }
    └── registry.json   [ { hive, path, name, value_type, original_value } ]
```

`hive` 는 `"HKLM"` | `"HKCU"`, `value_type` 은 `"DWORD"` | `"SZ"` 만 지원한다.

### Tauri 커맨드 (`commands/quarantine_commands.rs`)

| 커맨드 | 입력 | 반환 | 용도 |
|--------|------|------|------|
| `read_reg_dword` | `hive, path, name` | `Result<u32, String>` | Fix 실행 전 원본 DWORD 조회 (BACKUP_MAP 채우기용) |
| `quarantine_save` | `Vec<RegBackupEntry>, check_id, action_label` | `Result<String, String>` (quarantine_id) | 백업 저장 |
| `quarantine_list` | 없음 | `Vec<QuarantineRecord>` (최신순) | 복원 기록 조회. 개별 항목 읽기 실패는 건너뛰고 절대 실패하지 않음 |
| `quarantine_restore` | `quarantine_id` | `Result<(), String>` | 레지스트리 값 복원 + `restored=true` 갱신 |
| `quarantine_delete` | `quarantine_id` | `Result<(), String>` | 격리 폴더 삭제 |

- `quarantine_list` 호출 시 기록이 50개를 초과하면 오래된 것부터 10개를 자동 정리한다(`restored=true` 항목 우선 삭제).
- 저장 값은 보안 정책 DWORD(예: `EnableLUA`)뿐이다 — **API 키 등 민감정보는 절대 저장하지 않는다** (§13).

### FixButton.tsx 통합 흐름

```
cmd: URI 클릭
  → BACKUP_MAP[uri] 존재 시:
      1. 각 항목을 read_reg_dword 로 조회
         (레지스트리 키 없음 = 정상 상태 → 항목별 fallback 값 사용, 백업 자체를 막지 않음)
      2. quarantine_save 호출
         실패 시 → 수정을 즉시 중단하고 에러 표시 (백업 없이 레지스트리 직접 수정 금지, §13)
  → CMD_MAP[uri] 커맨드 실행
```

### 현재 등록된 cmd: 액션

| URI | 커맨드 | BACKUP_MAP |
|-----|--------|:---:|
| `cmd:uac-settings` | `open_uac_settings` (UserAccountControlSettings.exe) | ✅ `EnableLUA` + `ConsentPromptBehaviorAdmin` |
| `cmd:system-protection` | `open_system_protection` (SystemPropertiesProtection.exe) | ❌ (아래 참고) |

**`BACKUP_MAP` 등록 기준**: 그 값을 담당하는 `checks/*.rs` 모듈이 이미 코드베이스에 존재해 해당 레지스트리 경로·값이 검증되어 있을 때만 등록한다(`cmd:uac-settings` ← `uac.rs`). `cmd:system-protection` 은 이 액션이 여는 다이얼로그가 어떤 레지스트리 값을 어떻게 바꾸는지 코드베이스에서 검증한 바가 없어 **의도적으로 비워두었다** — 확인되지 않은 복원 로직을 추가하는 것이 백업을 아예 안 하는 것보다 위험할 수 있기 때문이다. 새 `cmd:` 액션을 추가할 때 레지스트리를 직접 수정한다면, 먼저 해당 값을 읽는 `checks/` 모듈을 두거나 최소한 정확한 레지스트리 경로를 검증한 뒤에만 `BACKUP_MAP` 을 채운다.

### 프론트엔드

- `quarantineStore.ts` — Zustand, invoke() 직접 허용(§2)
- `QuarantinePanel.tsx` — SettingsPanel 내 "복원 기록" 섹션. 목록/복원/삭제 UI, 빈 상태·오류 상태 모두 크래시 없이 렌더링

### 테스트 패턴

클릭이 비동기 invoke() 체인(백업→실행)을 트리거하는 컴포넌트는 렌더링 여부만 확인하는 테스트로는 부수효과를 검증할 수 없다. `FixButton.test.tsx` 처럼 `vi.mock("@tauri-apps/api/core", ...)` 로 모킹하고 `fireEvent.click` + `waitFor` 로 각 invoke 호출 인자를 단언한다 (§11).

---

## 10. 점검 모듈 구현 규칙

### PowerShell 인코딩

`util/powershell.rs`는 모든 스크립트 실행 전 자동으로 UTF-8 preamble을 삽입한다.

```rust
const UTF8_PREAMBLE: &str =
    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
     $OutputEncoding = [System.Text.Encoding]::UTF8; ";
```

**오류 메시지를 파싱해 조건 분기하는 코드는 작성하지 않는다.** 레지스트리/WMI로 대체하거나 영문 sentinel 값을 사용한다.

### 레지스트리 유틸 (util/registry.rs)

| 함수 | 하이브 | 반환 타입 |
|------|--------|-----------|
| `read_dword(path, name)` | HKLM | `Result<u32, String>` |
| `read_dword_hkcu(path, name)` | HKCU | `Result<u32, String>` |
| `read_string(path, name)` | HKLM | `Result<String, String>` |
| `read_string_hkcu(path, name)` | HKCU | `Result<String, String>` |

### 날짜 파싱

```powershell
# 권장: UTC + Z 접미사
$date.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')

# 금지: .ToString('o') → 7자리 소수 초, 파싱 실패
```

### Get-WinEvent 이벤트 없음 처리

이벤트가 없을 때 `-ErrorAction Stop`은 예외를 던진다. 반드시 `-ErrorAction SilentlyContinue`를 사용하고 `$null`을 명시적으로 확인한다.

### ConvertTo-Json null byte 처리

```rust
let value = raw.replace("\\u0000", "").replace('\0', "").trim().to_string();
```

두 패턴 모두 제거해야 한다 (ConvertTo-Json 인코딩 리터럴 + 실제 null char).

### Severity 판정 기준

| Severity | 조건 예시 |
|----------|-----------|
| `Ok` | 정상 동작 |
| `Info` | 하드웨어 미지원, 중립 상태 |
| `Warning` | 권장 설정 미적용 |
| `Danger` | 보호 기능 비활성 |
| `Critical` | 실시간 방어 완전 해제 |

하드웨어 미지원은 항상 `Info`. `Info`와 `Warning` 혼용 금지.

### 메시지 명칭 통일

한글 먼저, 영문 약칭을 괄호 안에. 계정 관련은 `암호` 사용 (`비밀번호` 금지).

---

## 11. 테스트 작성 가이드

### 모든 checks/*.rs 필수 테스트

```rust
#[test]
fn result_has_non_empty_id_and_positive_timestamp() {
    let r = run();
    assert!(!r.id.is_empty());
    assert!(r.checked_at > 0);
}

#[test]
fn never_panics_regardless_of_system_state() {
    let _ = run();   // panic 없이 완료되는지만 확인
}
```

### 프론트엔드 필수 테스트

```typescript
test("모든 Severity 값에 색상이 정의되어 있다")
test("http URI 가 들어오면 버튼을 렌더링하지 않는다")
test("ms-settings URI 는 버튼을 렌더링한다")
```

### invoke() 클릭 플로우 테스트 패턴

버튼 클릭이 비동기 invoke() 체인을 트리거하는 컴포넌트(예: `FixButton.tsx`)는 다음 패턴으로 검증한다.

```typescript
const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
// fireEvent.click(...) 후 waitFor(() => expect(invokeMock).toHaveBeenCalledWith(...))
```

렌더링 여부만 확인하는 테스트로는 백업/복원 같은 부수효과 로직의 회귀를 잡을 수 없다.

---

## 12. 오류 처리 규칙

- `unwrap()` 사용 금지 — `#![warn(clippy::unwrap_used)]` 가 차단 (단, `#[cfg(test)]` 블록 내 사용은 기존 관례상 허용 — `eol.rs`/`virustotal.rs`/`quarantine_commands.rs` 참조. `clippy` 는 기본적으로 테스트 타깃을 별도 분석하지 않으므로 `-D warnings` 게이트에 걸리지 않는다)
- VT API 오류는 `VtScanResult { status: Error, error_message: Some(...) }` 로 반환
- 오류 메시지는 원인과 해결 방법을 포함한다

```rust
// 금지: Err("네트워크 오류".into())
// 권장: Err(format!("VT 해시 조회 실패: url={}, status={}", url, code))
```

---

## 13. 절대 금지

| 금지 | 이유 |
|------|------|
| VT API 키를 코드에 하드코딩 | 즉시 키 폐기 필요 |
| VT API 키를 프론트엔드로 반환 | DevTools 노출 |
| 평문 파일/store에 API 키 저장 | 키체인이 유일한 허용 저장소 |
| 사용자 동의 없이 파일 업로드 | 프라이버시 침해 |
| run_scan() 에 VT/LLM 호출 포함 | 상시 점검 루프는 빨라야 함 |
| `checks/` 모듈 간 상호 import | 순환 의존 |
| `action_uri` 에 `http://` 스킴 | 외부 URL 직접 열기 금지 |
| `unwrap()` 사용 (프로덕션 코드) | 런타임 패닉 |
| PowerShell 오류 메시지 한글 파싱 | 시스템 언어 의존 |
| 날짜를 `.ToString('o')` 로 출력 | 파싱 실패 |
| `app.manifest` 에 비ASCII 문자 | mt.exe 인코딩 오류 → SxS 14001 크래시 |
| `font-mono` 로 숫자 정렬 | Pretendard 폰트 덮어씀 → `tabular-nums` 사용 |
| CSS 변수 토큰 클래스에 Tailwind opacity modifier(`/30` 등) 사용 | `var()` 색에 알파 합성 불가 → 규칙 자체가 생성 안 됨(테두리·배경 소실) |
| AI 응답을 인라인 마크다운 파싱으로 렌더링 | `MarkdownBlock.tsx` 공용 컴포넌트 사용 |
| LLM Q&A에서 greedy 샘플러 사용 | 동일 질문에 동일 답변 → `temperature=0.7` 필수 |
| Fix 가 `quarantine_save` 백업 없이 레지스트리를 직접 수정 | 사용자가 되돌릴 수단이 없어짐 |
| Quarantine 데이터에 API 키 등 민감정보 저장 | 평문 JSON 으로 디스크에 남음 |
| 컴포넌트에서 `clearHistory()`/`clearVtHistory()` 직접 호출 | 메모리 스토어·로컬 state 가 안 지워져 데이터가 남아 보임 → `store/dataReset.ts` 경유 (§16) |
| 모델 파일명·용량을 프런트엔드에 하드코딩 | `llm_model_info` 와 어긋나 잘못된 안내 → 백엔드 값 사용 (§6) |
| 업데이트 버전을 문자열로 비교 | `0.1.10 < 0.1.9` 로 판정 → `parse_version()` 숫자 비교 (§15) |
| 업데이트 설치 파일을 앱이 자동 다운로드·실행 | 보안 도구가 검증 없이 실행 파일을 받아 실행하는 셈 → 브라우저로 열어 사용자가 받는다 |

---

## 14. 빌드 및 실행 명령

### 개발 / 테스트

```bash
pnpm tauri dev                                          # 개발 서버
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
pnpm lint && pnpm tsc --noEmit && pnpm test
```

### ARM64 프로덕션 빌드 (Device Guard 환경)

vcvarsall.bat으로 VS 환경 초기화 후 LLVM을 PATH에 추가해야 한다. `cargo check`/`clippy`/`test`/`fmt` 도 `llama-cpp-sys-2` 네이티브 빌드 때문에 동일 환경변수가 필요하다(기본 MSVC `cl.exe` 로는 실패).

```bat
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" amd64_arm64
set PATH=C:\Program Files\LLVM\bin;%PATH%
set CL=/utf-8 /EHsc
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
set CC_aarch64_pc_windows_msvc=clang-cl
set CXX_aarch64_pc_windows_msvc=clang-cl
set AR_aarch64_pc_windows_msvc=llvm-lib
set CMAKE_GENERATOR=Ninja
node "C:\Program Files\nodejs\node_modules\npm\bin\npx-cli.js" tauri build --target aarch64-pc-windows-msvc --no-bundle
```

출력: `src-tauri/target/aarch64-pc-windows-msvc/release/winguard.exe`

빌드 전 반드시 `Stop-Process -Name winguard -Force -ErrorAction SilentlyContinue` 로 실행 중인 앱을 종료한다 (exe 잠금 → os error 5). 앱이 관리자 권한으로 실행 중이면 일반 권한 세션에서 `Stop-Process` 가 "Access is denied" 로 거부될 수 있다 — 이 경우 트레이 아이콘에서 직접 종료해야 한다.

### app.manifest + build.rs 주의사항

`app.manifest`는 순수 ASCII만 사용한다. 한글 주석 포함 시 SxS error 14001 크래시.

`build.rs`는 **release 빌드에서만** admin manifest를 임베드한다.
debug/test 빌드에서 임베드하면 `cargo test`가 Common Controls v6 SxS 오류로 실패한다.

### 환경 요구사항

- Rust stable 1.77+
- Node.js 20+ (x64 또는 ARM64)
- pnpm 9+
- Windows 10/11 (x64 또는 ARM64)
- Visual Studio Build Tools 2022 (MSVC toolchain)
- LLVM (`clang-cl`, `llvm-lib`) — ARM64 빌드 시 필수

### Cargo.toml 주요 의존성

```toml
tauri            = { version = "2", features = ["tray-icon"] }
tauri-plugin-dialog  = "2"
tauri-plugin-opener  = "2"
serde            = { version = "1", features = ["derive"] }
serde_json       = "1"
reqwest          = { version = "0.12", features = ["json", "blocking", "multipart", "native-tls"] }
sha2             = "0.10"
keyring          = "2"
chrono           = { version = "0.4", features = ["serde"] }
winreg           = "0.52"
dirs             = "5"
encoding_rs      = "0.8"
llama-cpp-2      = "0.1"

[dev-dependencies]
tempfile         = "3"
```

> `reqwest`는 `native-tls` 사용. `rustls-tls`는 clang이 필요해 BuildTools만 있는 환경에서 빌드 실패.
>
> `.npmrc`에 `shamefully-hoist=true` 필수 (ARM64 rollup 바이너리 링크 문제 회피).

---

## 15. 업데이트 확인 (update_commands)

앱은 **아무것도 자동으로 설치하지 않는다.** 최신 버전 정보를 보여주고, 설치 파일은
사용자가 눌러 브라우저로 연다. (보안 도구가 몰래 실행 파일을 받아 실행하지 않는다는 원칙)

### 업데이트 서버

GitHub Releases 를 업데이트 서버로 쓴다: `GET https://api.github.com/repos/{repo}/releases/latest`

| 항목 | 값 |
|------|-----|
| 기본 저장소 | `update_commands.rs` 의 `DEFAULT_REPO` 상수 (현재 `gsmtc01/winguard`) |
| 런타임 덮어쓰기 | `%LOCALAPPDATA%\WinGuard\update_settings.json` 의 `{ "repo": "owner/repo" }` |
| 필수 헤더 | `User-Agent` (없으면 GitHub 가 403) + `Accept: application/vnd.github+json` |
| 타임아웃 | 15초 |

`repo` 값은 `is_valid_repo()` 로 `owner/repo` 형식을 검증한 뒤에만 URL 에 넣는다.
검증에 실패하면 조용히 기본값으로 되돌린다 (설정 파일을 통한 경로 조작 차단).

### 커맨드

| 커맨드 | 반환 | 용도 |
|--------|------|------|
| `check_for_update` | `UpdateInfo` | 최신 릴리즈 조회 + 현재 버전과 비교 |
| `update_repo` | `String` | 현재 설정된 저장소 (설정 화면 표시용) |

`UpdateInfo` 에는 현재/최신 버전, `update_available`, 릴리즈 제목·노트(마크다운),
배포일, 릴리즈 URL, 아키텍처에 맞는 설치 파일 URL·이름·크기가 담긴다.

### 버전 비교

- `parse_version()` 은 `v` 접두사와 `-beta` 류 꼬리표를 떼고 숫자 배열로 만든다
- **문자열 비교 금지** — `0.1.10` 이 `0.1.9` 보다 높아야 한다
- 태그를 숫자로 못 읽으면 `is_newer()` 는 항상 `false` (업데이트가 있다고 주장하지 않는다)

### 설치 파일 선택

`pick_asset()` 이 `.msi`/`.exe` 자산 중 실행 아키텍처(`std::env::consts::ARCH`) 키워드가
들어간 것을 우선 고른다 (`aarch64` → `arm64`/`aarch64`, `x86_64` → `x64`/`x86_64`/`amd64`).
없으면 설치 파일 아무거나, 그것도 없으면 `None` → UI 는 릴리즈 페이지 링크만 보여준다.

### 프런트엔드

- `src/api/update.ts` — invoke 래퍼 + 자동 확인 주기(6시간, `winguard_update_last_check`)
- `SettingsPanel.tsx` 의 `UpdateSection` — 마운트 시 자동 확인(실패는 조용히 무시,
  오프라인일 수 있으므로) + "지금 확인" 수동 버튼. 릴리즈 노트는 `MarkdownBlock` 으로 렌더
- 외부 링크는 `opener` 플러그인을 쓰므로 `capabilities/default.json` 의 `opener:allow-open-url`
  허용 목록에 `https://github.com/*`, `https://objects.githubusercontent.com/*` 가 있어야 한다

---

## 16. 데이터 초기화 (설정 > 데이터 관리)

"초기화" 는 **세 곳**을 함께 지워야 한다. 한 곳만 지우면 화면에 데이터가 그대로 남는다.

| 위치 | 예시 | 처리 |
|------|------|------|
| ① localStorage | `winguard_score_history`, `winguard_vt_history`, `winguard-checkstore` | `clear*()` + 스토어 초기화 |
| ② 메모리 스토어 | 마지막 리포트, AI 분석·로드맵·Q&A, 각 스캔 패널 결과 | `getState().reset()` 호출 |
| ③ 컴포넌트 로컬 state | 마운트 시 한 번만 읽어 둔 이력 배열 | `DATA_RESET_EVENT` 로 다시 읽게 함 |

모든 초기화는 `src/store/dataReset.ts` 를 거친다. **컴포넌트가 `clearHistory()` 를 직접
부르지 않는다** — ③이 빠져 "지웠는데 남아 있는" 증상이 생긴다.

| 함수 | 지우는 것 |
|------|-----------|
| `resetScoreHistory()` | 점수 이력만 |
| `resetVtHistory()` | VT 검사 이력만 |
| `resetAllData()` | 위 둘 + 마지막 검사 리포트 + AI 분석/로드맵/Q&A + 프로세스·네트워크·확장·이벤트로그·장치 패널 상태 |

`resetAllData()` 는 **격리(Quarantine) 백업을 지우지 않는다** — 수정한 설정을 되돌리는
수단이라 함께 지우면 복원이 불가능해진다. 격리 항목은 전용 섹션에서 개별 삭제한다.

마운트 시 localStorage 를 한 번만 읽는 컴포넌트는 `onDataReset()` 을 구독한다
(현재: `Dashboard.tsx` 점수 추이 차트, `VtScanner.tsx` 검사 이력 패널).

> `checkStore` 의 persist 는 `partialize` 로 `report`/`lastScannedAt` 만 저장한다.
> `isLoading` 까지 저장하면 검사 도중 앱이 죽었을 때 다음 실행이 "검사 중" 으로 굳어
> 자동 검사가 시작되지 않는다.
