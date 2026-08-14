use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::registry;
use std::collections::HashMap;

const ID: &str = "smb1";
const TITLE: &str = "SMBv1 프로토콜";
const REG_PATH: &str = "SYSTEM\\CurrentControlSet\\Services\\LanmanServer\\Parameters";

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // SMB1 키가 없으면 Windows 10 1709+ 기본값 = 비활성화 (안전)
    let smb1_val = match registry::read_dword(REG_PATH, "SMB1") {
        Ok(v) => v,
        Err(_) => {
            evidence.insert("SMB1".into(), "absent (disabled by default)".into());
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Ok,
                message: "SMBv1이 비활성화되어 있습니다.".into(),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            };
        }
    };

    evidence.insert("SMB1".into(), smb1_val.to_string());

    if smb1_val == 0 {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: "SMBv1이 비활성화되어 있습니다.".into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        }
    } else {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Danger,
            message:
                "SMBv1이 활성화되어 있습니다. WannaCry·EternalBlue 계열 공격에 취약합니다. 즉시 비활성화하세요."
                    .into(),
            action_uri: Some("ms-settings:optionalfeatures".into()),
            evidence,
            checked_at: now_ts(),
        }
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
        if r.severity == Severity::Danger {
            assert!(r.message.contains("SMBv1"));
        }
    }
}
