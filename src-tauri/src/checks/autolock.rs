use crate::engine::types::{now_ts, CheckResult, Severity};
use std::collections::HashMap;

#[cfg(windows)]
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

const ID: &str = "autolock";
const TITLE: &str = "화면 자동 잠금";
// HKCU\Control Panel\Desktop
const DESKTOP_PATH: &str = "Control Panel\\Desktop";
// 권장 최대 타임아웃: 15분 = 900초
const MAX_TIMEOUT_SECS: u32 = 900;

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    #[cfg(not(windows))]
    {
        evidence.insert("note".into(), "Windows 전용 점검".into());
        return info(evidence);
    }

    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = match hkcu.open_subkey(DESKTOP_PATH) {
            Ok(k) => k,
            Err(e) => {
                evidence.insert("error".into(), e.to_string());
                return info(evidence);
            }
        };

        // ScreenSaverIsSecure: "1" = 재개 시 암호 요구, "0" = 미요구
        let is_secure: String = key
            .get_value("ScreenSaverIsSecure")
            .unwrap_or_else(|_| "0".to_string());
        // ScreenSaveActive: "1" = 화면 보호기 활성, "0" = 비활성
        let is_active: String = key
            .get_value("ScreenSaveActive")
            .unwrap_or_else(|_| "0".to_string());
        // ScreenSaveTimeOut: 문자열로 저장된 초 단위 값
        let timeout_str: String = key
            .get_value("ScreenSaveTimeOut")
            .unwrap_or_else(|_| "0".to_string());

        let timeout_secs: u32 = timeout_str.trim().parse().unwrap_or(0);

        evidence.insert("ScreenSaverIsSecure".into(), is_secure.clone());
        evidence.insert("ScreenSaveActive".into(), is_active.clone());
        evidence.insert("ScreenSaveTimeOut".into(), timeout_str);

        let active = is_active.trim() == "1";
        let secure = is_secure.trim() == "1";

        if !active {
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Warning,
                message:
                    "화면 보호기가 비활성화되어 있습니다. 자리를 비울 때 화면이 자동으로 잠기지 않습니다."
                        .into(),
                action_uri: Some("ms-settings:lockscreen".into()),
                evidence,
                checked_at: now_ts(),
            };
        }

        if !secure {
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Warning,
                message:
                    "화면 보호기가 활성화되어 있지만, 재개 시 암호를 요구하지 않습니다. 잠금 설정을 켜세요."
                        .into(),
                action_uri: Some("ms-settings:lockscreen".into()),
                evidence,
                checked_at: now_ts(),
            };
        }

        if timeout_secs == 0 || timeout_secs > MAX_TIMEOUT_SECS {
            let msg = if timeout_secs == 0 {
                "화면 자동 잠금 타임아웃이 설정되어 있지 않습니다. 15분 이내로 설정하세요.".into()
            } else {
                format!(
                    "화면 자동 잠금 타임아웃이 {}분으로 너무 깁니다. (권장: 15분 이내)",
                    timeout_secs / 60
                )
            };
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Warning,
                message: msg,
                action_uri: Some("ms-settings:lockscreen".into()),
                evidence,
                checked_at: now_ts(),
            };
        }

        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: format!(
                "화면 자동 잠금이 {}분 후 활성화되며, 재개 시 암호를 요구합니다.",
                timeout_secs / 60
            ),
            action_uri: Some("ms-settings:lockscreen".into()),
            evidence,
            checked_at: now_ts(),
        }
    }
}

fn info(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "화면 자동 잠금 상태를 확인할 수 없습니다.".into(),
        action_uri: Some("ms-settings:lockscreen".into()),
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
