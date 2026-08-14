# src-tauri/src/checks/AGENTS.md

이 디렉토리는 **데이터 수집과 단일 항목 판정**만 담당한다.
복합 판정(두 항목 이상 결합)은 `engine/rules.rs` 에서 처리한다.

---

## 이 디렉토리의 역할

| 허용 | 금지 |
|------|------|
| Windows API / 레지스트리 / PowerShell 호출 | 다른 `checks/` 모듈 import |
| 단일 항목 Severity 판정 | 복합 조건 판정 로직 |
| `CheckResult` 반환 | `ScanReport` 직접 생성 |
| `util/` 헬퍼 사용 | `winreg` / `windows-sys` 직접 import |

---

## 모듈 작성 규칙

### 파일당 공개 함수는 `run()` 하나만

```rust
// 올바른 구조
pub fn run() -> CheckResult { ... }

// 금지: 여러 진입점
pub fn check_realtime() -> CheckResult { ... }
pub fn check_signature() -> CheckResult { ... }
// → 두 결과가 필요하면 별도 모듈 파일로 분리한다
```

### 레지스트리 접근은 반드시 util 경유

```rust
// 올바른 패턴
use crate::util::registry;
let val = registry::read_dword("SYSTEM\\...", "EnableFirewall")?;

// 금지 패턴
use winreg::RegKey;                     // 직접 import 금지
let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
```

### Severity 판정 기준표 (모든 모듈에서 동일하게 적용)

| Severity | 판정 조건 예시 |
|----------|----------------|
| `Ok` | 기능 정상 동작 |
| `Info` | 지원 불가 하드웨어, 중립 상태 |
| `Warning` | 권장 설정 미적용, 점검 기간 초과 |
| `Danger` | 보호 기능 비활성, 미업데이트 30일 이상 |
| `Critical` | 실시간 방어 완전 해제, 악성코드 감지 후 미조치 |

`Info` 와 `Warning` 을 혼용하지 않는다.
하드웨어 미지원(`NotSupported`) 은 항상 `Info` 다.

### evidence 맵에 항상 원시 값을 담는다

```rust
// 올바른 패턴: 판정 근거를 evidence 에 기록
evidence.insert("RealTimeProtectionEnabled".into(), "false".into());
evidence.insert("AntivirusSignatureLastUpdated".into(), "2025-03-01".into());

// 금지 패턴: evidence 비워두기
// → 고급 표시 모드에서 근거를 보여줄 수 없음
```

---

## 테스트 필수 항목

모든 `checks/*.rs` 파일은 아래 두 테스트를 반드시 포함한다.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_has_non_empty_id_and_positive_timestamp() {
        let r = run();
        assert!(!r.id.is_empty());
        assert!(r.checked_at > 0);
    }

    #[test]
    fn never_panics_regardless_of_system_state() {
        // 레지스트리 키 없음, PowerShell 타임아웃 등 어떤 상황에서도
        // run() 은 패닉 없이 CheckResult 를 반환해야 한다
        let r = run();
        // Info 이하로만 실패를 표현한다 (Critical 로 뜨면 오탐)
        if r.message.contains("확인하지 못했습니다") {
            assert!(r.severity == Severity::Info);
        }
    }
}
```
