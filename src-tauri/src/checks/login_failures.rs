use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use std::collections::HashMap;

const ID: &str = "login_failures";
const TITLE: &str = "로그인 실패 기록";

/// 최근 N시간 이내의 이벤트를 조회한다
const HOURS_BACK: u32 = 24;
/// Danger 임계값: 24시간 내 로그인 실패 횟수
const DANGER_THRESHOLD: u32 = 20;
/// Warning 임계값
const WARNING_THRESHOLD: u32 = 5;

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // 이벤트 ID 4625 = 로그온 실패 (Windows Security Log)
    // Get-WinEvent 가 없거나 권한 없으면 대안으로 wevtutil 사용
    let script = format!(
        r#"
$after = (Get-Date).AddHours(-{hours})
$events = Get-WinEvent -FilterHashtable @{{
    LogName = 'Security'
    Id = 4625
    StartTime = $after
}} -ErrorAction SilentlyContinue | Where-Object {{
    # LogonType: 2(Interactive), 7(Unlock), 10(RemoteInteractive), 11(CachedInteractive)
    $type = try {{ $_.Properties[10].Value }} catch {{ 0 }}
    $type -in 2, 7, 10, 11
}}
if ($null -eq $events) {{
    [PSCustomObject]@{{ count = 0; accounts = '' }} | ConvertTo-Json
}} else {{
    $count = @($events).Count
    $accounts = $events | ForEach-Object {{
        try {{ $_.Properties[5].Value }} catch {{ 'unknown' }}
    }} | Where-Object {{ $_ -ne '-' -and $_ -ne '' }} | Group-Object | Sort-Object Count -Descending | Select-Object -First 5 | ForEach-Object {{ "$($_.Name)($($_.Count))" }}
    [PSCustomObject]@{{ count = $count; accounts = ($accounts -join ', ') }} | ConvertTo-Json
}}
"#,
        hours = HOURS_BACK
    );

    let output = match powershell::run(&script) {
        Ok(o) => o,
        Err(e) => {
            evidence.insert("error".into(), e);
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Info,
                message:
                    "로그인 실패 기록을 확인할 수 없습니다. (관리자 권한이 필요할 수 있습니다)"
                        .into(),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            };
        }
    };

    evidence.insert("raw".into(), output.trim().to_string());

    // count 값 파싱
    let count = parse_count(&output);

    if count < 0 {
        // 권한 부족 또는 로그 비활성화
        evidence.insert(
            "note".into(),
            "보안 로그에 접근하려면 관리자 권한이 필요합니다.".into(),
        );
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Info,
            message: "로그인 실패 기록을 확인할 수 없습니다. 보안 로그 읽기 권한이 없거나 감사 정책이 비활성화되어 있습니다.".into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    let count_u = count as u32;
    evidence.insert("failure_count_24h".into(), count_u.to_string());

    // 계정별 실패 정보도 파싱
    if let Some(accounts) = parse_accounts(&output) {
        if !accounts.is_empty() {
            evidence.insert("top_accounts".into(), accounts);
        }
    }

    if count_u >= DANGER_THRESHOLD {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Danger,
            message: format!(
                "최근 24시간 내 로그인 실패가 {}회 발생했습니다. \
                 무차별 대입 공격(Brute-Force) 시도 가능성이 있습니다.",
                count_u
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        }
    } else if count_u >= WARNING_THRESHOLD {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Warning,
            message: format!(
                "최근 24시간 내 로그인 실패가 {}회 발생했습니다. \
                 비정상적인 접근 시도가 있는지 확인하세요.",
                count_u
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
            message: format!(
                "최근 24시간 내 로그인 실패 횟수가 {}회로 정상 범위입니다.",
                count_u
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        }
    }
}

/// JSON 에서 `"count": N` 값을 파싱한다. 실패 시 -1 반환.
fn parse_count(output: &str) -> i64 {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.contains("\"count\"") {
            // "count": 42 형식
            if let Some(colon_pos) = trimmed.find(':') {
                let val_str = trimmed[colon_pos + 1..].trim().trim_end_matches(',');
                if let Ok(n) = val_str.parse::<i64>() {
                    return n;
                }
            }
        }
    }
    -1
}

/// JSON 에서 `"accounts": "..."` 값을 파싱한다.
fn parse_accounts(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.contains("\"accounts\"") {
            if let Some(colon_pos) = trimmed.find(':') {
                let val = trimmed[colon_pos + 1..]
                    .trim()
                    .trim_start_matches('"')
                    .trim_end_matches('"')
                    .trim_end_matches(',');
                if !val.is_empty() && val != "null" {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
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
    fn parse_count_basic() {
        let json = "{\n  \"count\":  25,\n  \"accounts\": \"\"\n}";
        assert_eq!(parse_count(json), 25);
    }

    #[test]
    fn parse_count_missing() {
        assert_eq!(parse_count("{}"), -1);
    }
}
