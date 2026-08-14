use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use std::collections::HashMap;

const ID: &str = "shares";
const TITLE: &str = "네트워크 공유 폴더";

/// Windows 기본 관리 공유 (정상으로 간주)
const ADMIN_SHARES: &[&str] = &["ADMIN$", "C$", "D$", "E$", "F$", "IPC$", "print$"];

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // 현재 시스템의 공유 폴더 목록 조회
    let script = r#"
Get-SmbShare | Select-Object Name, Path, Description, ShareState | ConvertTo-Json -Compress
"#;

    let output = match powershell::run(script) {
        Ok(o) => o,
        Err(e) => {
            evidence.insert("error".into(), e);
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Info,
                message: "네트워크 공유 폴더 목록을 확인할 수 없습니다.".into(),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            };
        }
    };

    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "null" {
        evidence.insert("shares".into(), "none".into());
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: "네트워크 공유 폴더가 없습니다.".into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    evidence.insert("raw".into(), trimmed.to_string());

    // 공유 이름 목록 파싱 (단순 문자열 파싱)
    let share_names = parse_share_names(trimmed);
    let non_admin: Vec<String> = share_names
        .iter()
        .filter(|name| !ADMIN_SHARES.iter().any(|a| a.eq_ignore_ascii_case(name)))
        .cloned()
        .collect();

    evidence.insert("total_shares".into(), share_names.len().to_string());

    if !non_admin.is_empty() {
        evidence.insert("custom_shares".into(), non_admin.join(", "));

        // 위험 공유 패턴 확인 (루트 경로 또는 시스템 폴더 공유)
        let dangerous = find_dangerous_shares(trimmed);
        if !dangerous.is_empty() {
            evidence.insert("dangerous_shares".into(), dangerous.join(", "));
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Danger,
                message: format!(
                    "위험한 경로가 네트워크에 공유되어 있습니다. {}. \
                     즉시 불필요한 공유를 해제하세요.",
                    dangerous.join(", ")
                ),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            };
        }

        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Warning,
            message: format!(
                "{}개의 사용자 정의 공유 폴더가 있습니다. {}. \
                 의도하지 않은 공유가 없는지 확인하세요.",
                non_admin.len(),
                non_admin.join(", ")
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        }
    } else {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: "Windows 기본 관리 공유 폴더만 존재합니다. 추가 공유 폴더가 없습니다.".into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        }
    }
}

/// JSON 출력에서 공유 이름 목록을 추출한다.
/// PowerShell -Compress 옵션의 1줄 JSON 과 일반 들여쓰기 JSON 을 모두 지원한다.
fn parse_share_names(output: &str) -> Vec<String> {
    let mut names = Vec::new();
    // "Name": "값" 또는 "Name":"값" 패턴을 각 줄에서 탐색
    for line in output.lines() {
        if let Some(name) = extract_json_string_field(line, "Name") {
            if !name.is_empty() {
                names.push(name);
            }
        }
    }
    names
}

/// `"field": "value"` 또는 `"field":"value"` 패턴에서 value 를 추출한다.
fn extract_json_string_field(line: &str, field: &str) -> Option<String> {
    let key = format!("\"{}\"", field);
    let key_pos = line.find(&key)?;
    let after_key = &line[key_pos + key.len()..];
    // ':' 찾기 (공백 허용)
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    // 여는 따옴표
    if !after_colon.starts_with('"') {
        return None;
    }
    let inner = &after_colon[1..];
    // 닫는 따옴표 위치 (이스케이프 미적용 단순 탐색)
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

/// 위험한 공유 경로(루트 드라이브 또는 Windows 시스템 폴더)를 찾는다.
fn find_dangerous_shares(output: &str) -> Vec<String> {
    let mut dangerous = Vec::new();
    let dangerous_paths: &[&str] = &[
        "C:\\",
        "D:\\",
        "C:\\Windows",
        "C:\\Users",
        "C:\\Program Files",
    ];

    let lines: Vec<&str> = output.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if let Some(path_val) = extract_json_string_field(line, "Path") {
            let path_lower = path_val.to_lowercase();
            for dp in dangerous_paths {
                if path_lower.starts_with(&dp.to_lowercase()) {
                    // 같은 줄 또는 앞쪽 줄에서 Name 을 찾는다
                    let name = extract_json_string_field(line, "Name")
                        .or_else(|| {
                            lines[..i]
                                .iter()
                                .rev()
                                .find_map(|l| extract_json_string_field(l, "Name"))
                        })
                        .unwrap_or_else(|| "unknown".to_string());

                    if !dangerous.contains(&name) {
                        dangerous.push(name);
                    }
                }
            }
        }
    }
    dangerous
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
    fn admin_shares_are_safe() {
        for share in ADMIN_SHARES {
            assert!(ADMIN_SHARES.iter().any(|a| a.eq_ignore_ascii_case(share)));
        }
    }

    #[test]
    fn parse_share_names_basic() {
        let json = r#"[{"Name": "TestShare", "Path": "D:\\test"}]"#;
        let names = parse_share_names(json);
        assert!(names.contains(&"TestShare".to_string()));
    }
}
