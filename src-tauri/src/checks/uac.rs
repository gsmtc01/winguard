use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::registry;
use std::collections::HashMap;

const ID: &str = "uac";
const TITLE: &str = "사용자 계정 컨트롤(UAC)";
const REG_PATH: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let enable_lua = match registry::read_dword(REG_PATH, "EnableLUA") {
        Ok(v) => v,
        Err(_) => {
            // 키 없음 = 시스템 기본값 (활성화)
            evidence.insert("EnableLUA".into(), "absent (default=enabled)".into());
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Ok,
                message: "사용자 계정 컨트롤(UAC)이(가) 활성화되어 있습니다.".into(),
                action_uri: Some("cmd:uac-settings".into()),
                evidence,
                checked_at: now_ts(),
            };
        }
    };

    evidence.insert("EnableLUA".into(), enable_lua.to_string());

    // ConsentPromptBehaviorAdmin: 0=자동승인(위험), 2=동의프롬프트, 5=자격증명프롬프트
    let prompt_level = registry::read_dword(REG_PATH, "ConsentPromptBehaviorAdmin").unwrap_or(5);
    evidence.insert(
        "ConsentPromptBehaviorAdmin".into(),
        prompt_level.to_string(),
    );

    if enable_lua == 0 {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Critical,
            message:
                "사용자 계정 컨트롤(UAC)이(가) 비활성화되어 있습니다. 악성 소프트웨어가 시스템의 관리자 권한을 무단 획득할 수 있습니다."
                    .into(),
            action_uri: Some("cmd:uac-settings".into()),
            evidence,
            checked_at: now_ts(),
        }
    } else if prompt_level == 0 {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Warning,
            message:
                "사용자 계정 컨트롤(UAC)이(가) 활성화되어 있으나, 관리자 권한을 자동 승인하도록 설정되어 있습니다. 승인 프롬프트를 표시하도록 변경하세요."
                    .into(),
            action_uri: Some("cmd:uac-settings".into()),
            evidence,
            checked_at: now_ts(),
        }
    } else {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: "사용자 계정 컨트롤(UAC)이(가) 활성화되어 있습니다.".into(),
            action_uri: Some("cmd:uac-settings".into()),
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
        // UAC 상태는 시스템에 따라 다르지만 panic 이 없어야 한다
        let _ = r.severity;
    }
}
