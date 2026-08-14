# WinGuard

Windows 전용 보안 상태 점검 앱 — 내 PC의 보안 상태를 한눈에, 위협은 바로 조치.

트레이에 상주하는 경량 데스크톱 앱으로, 대부분의 기능이 관리자 권한 없이 동작합니다.

## 주요 기능

- **23개 상시 보안 점검** — DNS 하이재킹, 랜섬웨어 흔적, 레지스트리 변조 탐지 포함
- **보안 점수** — 점검 결과를 원형 게이지로 요약
- **온디바이스 AI 분석** — Gemma 기반 로컬 LLM으로 기기 내부에서만 추론. 분석 대상이 외부로 전송되지 않습니다
- **프로세스 / 네트워크 / 이벤트로그 / 확장 프로그램 분석**
- **카메라·마이크 접근 제어**
- **VirusTotal 연동 파일 검사**
- **자동 백업 및 복원(Quarantine)** — 문제를 수정하기 전 되돌릴 수 있도록 백업

## 설치

[Releases](../../releases)에서 아키텍처에 맞는 실행 파일을 받아 그대로 실행합니다. 설치 과정은 없습니다.

| 파일 | 대상 |
|---|---|
| `WinGuard-x64.exe` | Windows 11 (Intel/AMD) |
| `WinGuard-arm64.exe` | Windows 11 (ARM64) |

AI 분석 기능을 처음 사용할 때 약 3GB의 모델 파일을 내려받습니다. 저장 위치는 앱에서 지정할 수 있고, 앱 내에서 다시 삭제할 수 있습니다.

## 개발 환경

### 사전 요구사항

- Node.js 20+, pnpm 9+
- Rust (stable)
- Visual Studio 2022 — Desktop development with C++ (ARM64 빌드 도구 포함)
- LLVM/clang, CMake, Ninja — `llama-cpp-sys-2` 빌드에 필요

`.cargo/config.toml.example`을 같은 폴더에 `config.toml`로 복사한 뒤, 안에 적힌 툴체인 경로를 본인 머신에 맞게 수정하세요. 이 파일은 머신마다 경로가 달라 git에 포함되지 않습니다.

저장소 루트에 두는 이유가 있습니다. cargo는 설정 파일을 **현재 작업 디렉토리에서 위로 올라가며** 찾습니다. `src-tauri/` 안에 두면 루트에서 `cargo clippy --manifest-path src-tauri/Cargo.toml`처럼 실행할 때 설정이 적용되지 않아, llama.cpp 컴파일이 인코딩 오류로 실패합니다(pre-commit 훅이 이 방식으로 실행합니다).

### 실행

```bash
pnpm install
pnpm tauri dev
```

### 릴리스 빌드

```bash
./build.bat
```

x64와 ARM64를 차례로 빌드해 `release/WinGuard-x64.exe`, `release/WinGuard-arm64.exe`를 만듭니다. Visual Studio 설치 경로가 기본값과 다르면 `VCVARS` 환경변수로 `vcvarsall.bat` 위치를 지정하세요.

### 검사

```bash
pnpm lint && pnpm tsc && pnpm test --run
```

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## 기술 스택

React 18 · TypeScript · Tailwind CSS · Zustand · Tauri 2 · Rust · llama.cpp(`llama-cpp-2`)

## 라이선스

[MIT](LICENSE)
