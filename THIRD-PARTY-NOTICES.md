# 서드파티 고지

WinGuard는 아래 오픈소스 구성 요소를 사용합니다. WinGuard 자체 코드는
[MIT License](LICENSE)로 배포됩니다.

## 실행 파일에 포함되는 구성 요소

### Pretendard

- 저작권: Copyright (c) 2021 Kil Hyung-jin
- 라이선스: SIL Open Font License 1.1
- 출처: https://github.com/orioncactus/pretendard
- 전문: https://scripts.sil.org/OFL

UI 기본 서체로 사용하며 실행 파일에 내장됩니다. OFL 1.1에 따라 상업적 사용과
재배포가 허용되며, 글꼴 자체를 단독 판매하지 않습니다.

### llama.cpp

- 저작권: Copyright (c) 2023-2024 The ggml authors
- 라이선스: MIT License
- 출처: https://github.com/ggerganov/llama.cpp

로컬 LLM 추론 엔진입니다. Rust 바인딩 `llama-cpp-2`(및 `llama-cpp-sys-2`)를
통해 정적 링크됩니다.

### Tauri

- 라이선스: MIT License 또는 Apache License 2.0
- 출처: https://github.com/tauri-apps/tauri

데스크톱 애플리케이션 프레임워크입니다.

### 그 밖의 의존성

Rust 크레이트와 npm 패키지는 대부분 MIT 또는 Apache License 2.0으로 배포됩니다.
전체 목록과 정확한 버전은 다음 파일에서 확인할 수 있습니다.

- Rust: [`src-tauri/Cargo.lock`](src-tauri/Cargo.lock)
- 프런트엔드: [`pnpm-lock.yaml`](pnpm-lock.yaml)

## 실행 중 내려받는 구성 요소

### Gemma 4 E2B Instruct

- 제작: Google DeepMind
- 라이선스: Apache License 2.0
- 원본 모델: https://huggingface.co/google/gemma-4-E2B-it
- 사용하는 양자화본: https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF
- 전문: https://www.apache.org/licenses/LICENSE-2.0

온디바이스 AI 분석에 사용하는 언어 모델입니다.

**이 모델은 WinGuard 실행 파일에 포함되어 있지 않습니다.** 사용자가 AI 분석
기능을 처음 사용할 때 위 HuggingFace 저장소에서 직접 내려받으며, 약 3GB입니다.
따라서 WinGuard 배포본은 모델 가중치를 재배포하지 않습니다.

모델은 기기 내부에서만 실행되며 분석 대상 데이터가 외부로 전송되지 않습니다.

## 연동 서비스

### VirusTotal

파일 검사 기능은 사용자가 직접 입력한 API 키로 VirusTotal API를 호출합니다.
이 기능을 사용하려면 VirusTotal의 이용약관을 따라야 합니다.

- https://docs.virustotal.com/docs/please-give-me-an-api-key
- https://www.virustotal.com/gui/terms-of-service

WinGuard는 VirusTotal과 제휴 관계가 없으며, API 키는 Windows 자격 증명 관리자에
저장되고 VirusTotal 외의 서버로 전송되지 않습니다.
