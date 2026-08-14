# src-tauri/src/engine/AGENTS.md

이 디렉토리는 **판정 로직과 점수 계산**만 담당한다.
Windows API 를 직접 호출하지 않는다.

---

## 파일별 역할

| 파일 | 역할 | 변경 허용 여부 |
|------|------|----------------|
| `types.rs` | `CheckResult`, `ScanReport`, `Severity` 정의 | **구조 변경 금지** |
| `rules.rs` | 복합 판정 룰 (두 항목 이상 결합) | 룰 추가 가능 |
| `scorer.rs` | 전체 점수 계산 (0~100) | 가중치 표 참고 후 수정 |

---

## types.rs 변경 금지 이유

`CheckResult` 에 필드를 추가하면 모든 `checks/*.rs` 가 컴파일 오류를 낸다.
추가 데이터가 필요하면 `evidence: HashMap<String, String>` 을 활용한다.

```rust
// 필드 추가 — 금지
pub struct CheckResult {
    pub id: String,
    pub severity: Severity,
    pub raw_value: u32,       // ← 추가 금지. evidence 맵에 넣을 것
}

// 올바른 방법
evidence.insert("raw_value".into(), raw_value.to_string());
```

---

## rules.rs 작성 규칙

복합 룰은 단일 항목 판정이 모두 끝난 후 적용된다.
`checks/` 에서 두 모듈의 결과를 합쳐서 판정하고 싶다면 반드시 여기에 작성한다.

```rust
// 올바른 패턴: id 로 기존 결과를 찾아 severity 를 상향 조정
pub fn apply_compound_rules(checks: &mut Vec<CheckResult>) {
    let eol_danger = checks.iter()
        .any(|c| c.id == "windows_eol" && c.severity >= Severity::Danger);
    let boot_off = checks.iter()
        .any(|c| c.id == "secure_boot" && c.severity == Severity::Warning);

    if eol_danger && boot_off {
        if let Some(c) = checks.iter_mut().find(|c| c.id == "secure_boot") {
            c.severity = Severity::Danger;
            // 상향 이유를 message 에 명시한다
            c.message = format!("{} (지원 종료 Windows와 함께 사용 중 — 위험도 상향)", c.message);
        }
    }
}

// 금지 패턴: severity 를 낮추는 방향의 룰
// → 판정은 단방향으로만 상향 조정한다
```

---

## scorer.rs 가중치 (변경 시 이 표도 함께 수정)

| Severity | 감점 |
|----------|------|
| Ok | 0 |
| Info | 0 |
| Warning | 10 |
| Danger | 20 |
| Critical | 35 |

시작 점수 100, 합산 감점 후 `clamp(0, 100)`.
가중치를 변경할 때는 루트 `AGENTS.md` 의 표도 동기화한다.
