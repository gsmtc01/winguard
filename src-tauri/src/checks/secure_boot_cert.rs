use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use std::collections::HashMap;

const ID: &str = "secure_boot_cert";
const TITLE: &str = "보안 부팅(Secure Boot) 인증서";

/// 2011년 Secure Boot 인증서 만료일 (2026-06-25)
/// 만료 전 2023년 인증서로 업데이트하지 않으면 부팅 문제가 발생할 수 있다.
/// 참고: https://support.microsoft.com/en-us/topic/windows-secure-boot-certificate-expiration-and-ca-updates-7ff40d33-95dc-4c3c-8725-a9b95457578e
const CERT_EXPIRY_DATE: &str = "2026-06-25";

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // Get-SecureBootUEFI 는 Secure Boot 가 비활성화된 BIOS/레거시 환경에서 예외 발생
    // 영문 sentinel('UNAVAILABLE') 로 예외 처리 — 언어 의존 메시지 파싱 금지
    let script = r#"
try {
    $kekBytes = (Get-SecureBootUEFI kek -ErrorAction Stop).bytes
    $dbBytes  = (Get-SecureBootUEFI db  -ErrorAction Stop).bytes
    $kekOk = [bool]([System.Text.Encoding]::ASCII.GetString($kekBytes) -match 'Microsoft Corporation KEK 2K CA 2023')
    $dbOk  = [bool]([System.Text.Encoding]::ASCII.GetString($dbBytes)  -match 'Windows UEFI CA 2023')
    [PSCustomObject]@{ kek2023 = $kekOk; db2023 = $dbOk } | ConvertTo-Json -Compress
} catch {
    [PSCustomObject]@{ error = 'UNAVAILABLE' } | ConvertTo-Json -Compress
}
"#;

    let output = match powershell::run(script) {
        Ok(o) => o,
        Err(e) => {
            evidence.insert("error".into(), e);
            return unavailable(evidence);
        }
    };

    let trimmed = output.trim();
    evidence.insert("raw".into(), trimmed.to_string());

    // error = UNAVAILABLE → Secure Boot 비활성화 또는 BIOS 시스템
    if trimmed.contains("\"error\"") {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Info,
            message: "보안 부팅(Secure Boot) 인증서를 확인할 수 없습니다. \
                      Secure Boot가 비활성화되어 있거나 BIOS 시스템일 수 있습니다."
                .into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    let kek_ok = extract_bool(trimmed, "kek2023").unwrap_or(false);
    let db_ok = extract_bool(trimmed, "db2023").unwrap_or(false);

    evidence.insert("KEK_2023".into(), kek_ok.to_string());
    evidence.insert("DB_2023".into(), db_ok.to_string());
    evidence.insert("cert_expiry".into(), CERT_EXPIRY_DATE.into());

    if kek_ok && db_ok {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: "보안 부팅(Secure Boot) 인증서가 최신(2023) 상태입니다.".into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    // 2023 인증서 미적용 — 어느 쪽이 없는지 기록
    let mut missing: Vec<&str> = Vec::new();
    if !kek_ok {
        missing.push("KEK 2023");
    }
    if !db_ok {
        missing.push("DB 2023");
    }
    evidence.insert("missing".into(), missing.join(", "));

    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Danger,
        message: "보안 부팅(Secure Boot) 인증서가 오래되었습니다.\n\
                  Windows 업데이트를 실행하여 인증서를 업데이트하세요."
            .into(),
        action_uri: Some("ms-settings:windowsupdate".into()),
        evidence,
        checked_at: now_ts(),
    }
}

fn unavailable(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "보안 부팅(Secure Boot) 인증서를 확인할 수 없습니다.".into(),
        action_uri: None,
        evidence,
        checked_at: now_ts(),
    }
}

/// 단순 JSON 에서 boolean 필드를 파싱한다.
/// compact JSON `{"field":true,...}` 과 pretty-printed JSON 을 모두 지원한다.
fn extract_bool(output: &str, field: &str) -> Option<bool> {
    // "field": 까지 포함해 검색하면 콜론 위치 탐색이 불필요하다.
    let key = format!("\"{}\":", field);
    // 공백 있는 형태 `"field" :` 도 지원하기 위해 두 패턴 모두 시도
    let key_nospace = format!("\"{}\"", field);
    let start = output.find(&key).map(|p| p + key.len()).or_else(|| {
        output.find(&key_nospace).and_then(|p| {
            let after = &output[p + key_nospace.len()..];
            after.find(':').map(|cp| p + key_nospace.len() + cp + 1)
        })
    })?;
    let val_str = output[start..].trim_start();
    // 알파벳만 읽어서 true/false 를 추출한다 (뒤에 , } 등이 붙어도 안전)
    let end = val_str
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(val_str.len());
    match &val_str[..end] {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
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
    fn bool_extraction_compact_json() {
        let json = r#"{"kek2023":true,"db2023":false}"#;
        assert_eq!(extract_bool(json, "kek2023"), Some(true));
        assert_eq!(extract_bool(json, "db2023"), Some(false));
    }

    #[test]
    fn both_true_is_ok() {
        let json = r#"{"kek2023":true,"db2023":true}"#;
        assert_eq!(extract_bool(json, "kek2023"), Some(true));
        assert_eq!(extract_bool(json, "db2023"), Some(true));
    }
}
