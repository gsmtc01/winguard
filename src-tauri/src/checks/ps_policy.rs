use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use std::collections::HashMap;

const ID: &str = "ps_policy";
const TITLE: &str = "PowerShell 실행 정책";

/// 위험한 정책 (스크립트 제한 없음)
const DANGEROUS_POLICIES: &[&str] = &["unrestricted", "bypass"];
/// 경고 수준 정책 (서명 없어도 로컬 스크립트 실행 허용)
const WARNING_POLICIES: &[&str] = &["remotesigned", "allsigned"];

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // 모든 범위의 실행 정책을 가져온다
    let script =
        "Get-ExecutionPolicy -List | Select-Object Scope, ExecutionPolicy | ConvertTo-Json";
    let output = match powershell::run(script) {
        Ok(o) => o,
        Err(e) => {
            evidence.insert("error".into(), e);
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Info,
                message: "PowerShell 실행 정책을 확인할 수 없습니다.".into(),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            };
        }
    };

    evidence.insert("raw".into(), output.trim().to_string());

    // 유효 정책(MachinePolicy > UserPolicy > Process > CurrentUser > LocalMachine)
    // 가장 제한적인 설정이 아닌, 가장 넓게 허용된 정책을 찾는다
    let effective = find_most_permissive_policy(&output);
    evidence.insert("effective".into(), effective.clone());

    let lower = effective.to_lowercase();

    if DANGEROUS_POLICIES.iter().any(|p| lower == *p) {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Danger,
            message: format!(
                "PowerShell 실행 정책이 '{}'로 설정되어 있습니다. \
                 서명되지 않은 모든 스크립트가 실행될 수 있어 악성 소프트웨어 실행 위험이 있습니다.",
                effective
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        }
    } else if WARNING_POLICIES.iter().any(|p| lower == *p) {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Warning,
            message: format!(
                "PowerShell 실행 정책이 '{}'로 설정되어 있습니다. \
                 로컬 스크립트는 서명 없이 실행 가능합니다. 보안 강화를 위해 'Restricted' 사용을 권장합니다.",
                effective
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        }
    } else {
        // Restricted / Undefined / AllSigned (strict)
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: format!(
                "PowerShell 실행 정책이 '{}'로 설정되어 있습니다.",
                effective
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        }
    }
}

/// JSON 출력에서 가장 허용 범위가 넓은 실행 정책을 찾는다.
/// 단순 문자열 파싱으로 처리한다 (serde_json 의존성 없이).
fn find_most_permissive_policy(output: &str) -> String {
    // 우선순위 (낮을수록 더 위험 / 더 허용적)
    let order: &[(&str, u8)] = &[
        ("unrestricted", 0),
        ("bypass", 1),
        ("remotesigned", 2),
        ("allsigned", 3),
        ("restricted", 4),
        ("undefined", 5),
    ];

    let mut best_score: u8 = 255;
    let mut best_name = "Restricted".to_string();

    for line in output.lines() {
        let lower = line.to_lowercase();
        for (name, score) in order {
            if lower.contains(name) && *score < best_score {
                best_score = *score;
                best_name = name
                    .chars()
                    .enumerate()
                    .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
                    .collect();
            }
        }
    }

    best_name
}

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
        let r = run();
        let _ = r.severity;
    }

    #[test]
    fn permissive_policy_detection() {
        let json = r#"[{"Scope":"LocalMachine","ExecutionPolicy":"Unrestricted"}]"#;
        assert_eq!(find_most_permissive_policy(json), "Unrestricted");
    }

    #[test]
    fn restricted_is_safe() {
        let json = r#"[{"Scope":"LocalMachine","ExecutionPolicy":"Restricted"}]"#;
        let policy = find_most_permissive_policy(json);
        assert_eq!(policy.to_lowercase(), "restricted");
    }
}
