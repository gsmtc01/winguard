use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::registry;
use std::collections::HashMap;

const ID: &str = "firewall";
const TITLE: &str = "Windows 방화벽";
const ACTION: &str = "windowsdefender://network";

const PROFILES: &[(&str, &str)] = &[
    (
        "Domain",
        r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\DomainProfile",
    ),
    (
        "Standard",
        r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\StandardProfile",
    ),
    (
        "Public",
        r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\PublicProfile",
    ),
];

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();
    let mut enabled_count = 0;
    let mut total = 0;

    for (name, path) in PROFILES {
        match registry::read_dword(path, "EnableFirewall") {
            Ok(v) => {
                evidence.insert(format!("{}Profile.EnableFirewall", name), v.to_string());
                total += 1;
                if v == 1 {
                    enabled_count += 1;
                }
            }
            Err(e) => {
                evidence.insert(format!("{}Profile.error", name), e);
            }
        }
    }

    if total == 0 {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Info,
            message: "방화벽 상태를 확인할 수 없습니다.".into(),
            action_uri: Some(ACTION.into()),
            evidence,
            checked_at: now_ts(),
        };
    }

    let (severity, message) = if enabled_count == total {
        (
            Severity::Ok,
            "모든 프로파일에서 방화벽이 활성화되어 있습니다.".into(),
        )
    } else if enabled_count == 0 {
        (
            Severity::Danger,
            "모든 프로파일에서 방화벽이 비활성화되어 있습니다.".into(),
        )
    } else {
        (
            Severity::Warning,
            format!(
                "{}/{} 프로파일에서만 방화벽이 활성화되어 있습니다.",
                enabled_count, total
            ),
        )
    };

    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity,
        message,
        action_uri: Some(ACTION.into()),
        evidence,
        checked_at: now_ts(),
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
        if r.message.contains("확인할 수 없습니다") {
            assert!(r.severity == Severity::Info);
        }
    }
}
