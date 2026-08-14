# WinGuard

Windows 전용 보안 상태 점검 앱입니다. 내 PC의 보안 상태를 한눈에 확인하고, 발견된 위협은 그 자리에서 조치할 수 있습니다.

트레이에 상주하는 경량 데스크톱 앱이며, 설치 과정 없이 실행 파일 하나로 동작합니다.

[![CI](https://github.com/gsmtc01/winguard/actions/workflows/ci.yml/badge.svg)](https://github.com/gsmtc01/winguard/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

## 다운로드

| 파일 | 대상 |
|---|---|
| [WinGuard-x64.exe](https://github.com/gsmtc01/winguard/releases/latest/download/WinGuard-x64.exe) | Windows 11 (Intel / AMD) |
| [WinGuard-arm64.exe](https://github.com/gsmtc01/winguard/releases/latest/download/WinGuard-arm64.exe) | Windows 11 (ARM64) |

내려받은 파일은 SHA-256으로 확인할 수 있습니다. 릴리즈에 함께 올라간 `.sha256` 파일의 값과 비교하세요.

```powershell
Get-FileHash .\WinGuard-x64.exe -Algorithm SHA256
```

이 실행 파일에는 아직 코드 서명이 적용되지 않아, 처음 실행할 때 Windows SmartScreen 경고가 나타날 수 있습니다. 위 해시로 파일을 확인한 뒤 `추가 정보`, `실행` 순서로 진행하세요.

## 주요 기능

**23개 상시 보안 점검**
Windows Defender, 방화벽, BitLocker, UAC, Secure Boot, TPM 등 기본 보안 설정을 점검합니다. DNS 하이재킹, 랜섬웨어 흔적, hosts 파일 변조, SMBv1 활성화 같은 침해 징후도 함께 확인합니다.

**보안 점수**
점검 결과를 가중치로 환산해 원형 게이지로 보여줍니다. 심각도별로 색이 구분되며, 항목마다 무엇이 문제인지와 조치 방법이 붙습니다.

**온디바이스 AI 분석**
Gemma 4 기반 로컬 LLM이 점검 결과를 해석해 설명합니다. 추론이 전부 기기 안에서 이뤄지므로 분석 대상 데이터가 외부로 나가지 않습니다.

**심층 분석**
실행 중인 프로세스, 활성 네트워크 연결, 보안 이벤트 로그, 설치된 브라우저 확장 프로그램을 각각 살펴볼 수 있습니다.

**카메라와 마이크 접근 제어**
앱별로 장치 접근 권한을 확인하고 차단할 수 있습니다.

**VirusTotal 연동 파일 검사**
의심스러운 파일의 해시를 조회하거나 직접 업로드해 검사합니다. API 키는 Windows 자격 증명 관리자에 저장됩니다.

**수정 전 자동 백업**
레지스트리를 변경하는 조치는 원래 값을 먼저 백업합니다. 백업에 실패하면 조치 자체를 중단하며, 언제든 복원할 수 있습니다.

## AI 모델

AI 분석을 처음 사용할 때 약 3GB의 모델 파일을 내려받습니다.

| 항목 | 값 |
|---|---|
| 모델 | Gemma 4 E2B Instruct (Q4_K_M 양자화) |
| 기본 저장 위치 | `%LOCALAPPDATA%\WinGuard\models\` |
| 크기 | 약 2.89 GiB |

저장 폴더는 앱에서 다른 곳으로 지정할 수 있고, 필요 없어지면 설정에서 삭제할 수 있습니다. 모델은 실행 파일에 포함되어 있지 않으며 HuggingFace에서 직접 받습니다.

## 데이터 저장 위치

앱이 만드는 데이터는 모두 사용자 폴더 안에 있습니다. 실행 파일을 옮기거나 이름을 바꿔도 설정이 유지됩니다.

| 데이터 | 위치 |
|---|---|
| 검사 기록, 점수 히스토리, 테마 | `%LOCALAPPDATA%\com.winguard.security\` |
| 모델 경로, 업데이트 설정 | `%LOCALAPPDATA%\WinGuard\` |
| VirusTotal API 키 | Windows 자격 증명 관리자 |

## 개발 환경

### 사전 요구사항

- Node.js 24 이상, pnpm 10 이상
- Rust (stable)
- Visual Studio 2022 Build Tools (Desktop development with C++, ARM64 빌드 도구 포함)
- LLVM/clang, CMake, Ninja (`llama-cpp-sys-2` 빌드에 필요)

`.cargo/config.toml.example`을 같은 폴더에 `config.toml`로 복사한 뒤, 안에 적힌 툴체인 경로를 본인 머신에 맞게 수정하세요. 이 파일은 머신마다 경로가 달라 git에 포함되지 않습니다.

이 설정 파일이 `src-tauri/`가 아니라 저장소 루트에 있는 이유가 있습니다. cargo는 설정 파일을 현재 작업 디렉토리에서 위로 올라가며 찾습니다. `src-tauri/` 안에 두면 루트에서 `cargo clippy --manifest-path src-tauri/Cargo.toml`처럼 실행할 때 설정이 적용되지 않아, llama.cpp 컴파일이 인코딩 오류로 실패합니다. pre-commit 훅이 이 방식으로 실행합니다.

### 실행

```bash
pnpm install
```

```bash
pnpm tauri dev
```

### 릴리즈 빌드

```bash
./build.bat
```

x64와 ARM64를 차례로 빌드해 `release/WinGuard-x64.exe`와 `release/WinGuard-arm64.exe`를 만듭니다. 한쪽만 필요하면 `build.bat x64` 또는 `build.bat arm64`로 지정하세요.

Visual Studio 설치 경로는 vswhere로 자동 탐지합니다. 찾지 못하면 `VCVARS` 환경변수로 `vcvarsall.bat` 위치를 지정하세요.

빌드하려면 실행 중인 WinGuard를 먼저 닫아야 합니다. 앱이 관리자 권한으로 실행되므로 스크립트가 대신 종료하지 못합니다.

### 검사

프런트엔드는 CI와 동일한 순서로 확인합니다.

```bash
pnpm eslint src --ext .ts,.tsx --max-warnings 0
```

```bash
pnpm build
```

```bash
pnpm vitest run
```

타입 검사는 `pnpm build`가 겸합니다. `tsc --noEmit`은 루트 `tsconfig.json`이 참조만 담고 있어 아무 파일도 검사하지 않으니 사용하지 마세요.

Rust는 저장소 루트에서 실행합니다.

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## 구조

```
src/            프런트엔드 (React)
  api/          Tauri invoke 래퍼. IPC 호출은 여기를 거칩니다
  components/   UI 컴포넌트
  store/        Zustand 상태
  lib/          순수 함수 (심각도 매핑, 내보내기, 히스토리)
src-tauri/      백엔드 (Rust)
  src/checks/   개별 보안 점검 모듈
  src/commands/ Tauri 커맨드 핸들러
  src/engine/   점수 산출과 복합 규칙
```

설계 원칙과 코딩 규약은 [AGENTS.md](AGENTS.md)와 [src/AGENTS.md](src/AGENTS.md)에 있습니다.

## 기술 스택

React 18, TypeScript, Tailwind CSS, Zustand, Tauri 2, Rust, llama.cpp (`llama-cpp-2`)

## 라이선스

WinGuard는 [MIT License](LICENSE)로 배포됩니다.

사용 중인 오픈소스 구성 요소와 각각의 라이선스는 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)에 정리되어 있습니다. AI 모델인 Gemma 4 E2B는 Google DeepMind가 Apache License 2.0으로 배포한 것입니다.
