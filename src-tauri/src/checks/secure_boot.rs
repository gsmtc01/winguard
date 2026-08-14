use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::registry;
use std::collections::HashMap;

const ID: &str = "secure_boot";
const TITLE: &str = "보안 부팅(Secure Boot)";

/// Secure Boot 상태를 레지스트리에서 직접 읽는다.
/// PowerShell `Confirm-SecureBootUEFI` 는 시스템 언어에 따라 오류 메시지가
/// 다른 인코딩으로 출력되어 파싱이 불안정하므로 레지스트리 방식을 사용한다.
///
/// 레지스트리 키:
///   HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State
///   값: UEFISecureBootEnabled (DWORD)  1=활성화, 0=비활성화
///   키 없음 → 레거시 BIOS (Secure Boot 미지원)
const REG_PATH: &str = r"SYSTEM\CurrentControlSet\Control\SecureBoot\State";
const REG_VALUE: &str = "UEFISecureBootEnabled";

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    match registry::read_dword(REG_PATH, REG_VALUE) {
        Ok(1) => {
            evidence.insert(REG_VALUE.into(), "1".into());
            CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Ok,
                message: "보안 부팅(Secure Boot)이(가) 활성화되어 있습니다.".into(),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            }
        }
        Ok(0) => {
            evidence.insert(REG_VALUE.into(), "0".into());
            CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Warning,
                message: "보안 부팅(Secure Boot)이(가) 비활성화되어 있습니다. UEFI 설정에서 활성화하세요."
                    .into(),
                action_uri: Some("windowsdefender://devicesecurity".into()),
                evidence,
                checked_at: now_ts(),
            }
        }
        Ok(v) => {
            // 예상치 못한 값
            evidence.insert(REG_VALUE.into(), v.to_string());
            CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Info,
                message: format!(
                    "보안 부팅(Secure Boot) 상태에 문제가 있습니다. (값={}.) Windows 보안에서 수동으로 확인하세요.",
                    v
                ),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            }
        }
        Err(e) => {
            evidence.insert("registry_error".into(), e.clone());
            // 키 자체가 없으면 레거시 BIOS 로 판단
            if e.contains("레지스트리 키 열기 실패") || e.contains("2") {
                CheckResult {
                    id: ID.into(),
                    title: TITLE.into(),
                    severity: Severity::Info,
                    message:
                        "이 시스템은 보안 부팅(Secure Boot)을(를) 지원하지 않는 BIOS 시스템입니다."
                            .into(),
                    action_uri: None,
                    evidence,
                    checked_at: now_ts(),
                }
            } else {
                unknown(evidence)
            }
        }
    }
}

fn unknown(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "보안 부팅(Secure Boot) 상태를 확인할 수 없습니다.".into(),
        action_uri: None,
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
