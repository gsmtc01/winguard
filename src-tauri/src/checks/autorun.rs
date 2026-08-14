use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::registry;
use std::collections::HashMap;

const ID: &str = "autorun";
const TITLE: &str = "이동식 드라이브(CD/DVD, USB 등) 자동 실행";

/// HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer
const REG_PATH_LM: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer";
/// HKCU 동일 경로
const REG_PATH_CU: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer";

/// NoDriveTypeAutoRun 비트 마스크
/// bit 2 (0x04) = Removable, bit 3 (0x08) = Fixed, bit 4 (0x10) = Network
/// 0xFF = 모든 드라이브 타입 자동실행 비활성화
const REMOVABLE_BIT: u32 = 0x04;

/// AutoRun 및 AutoPlay 완전 비활성화 여부를 판단한다.
///
/// 판단 우선순위: HKLM > HKCU (HKLM 정책이 있으면 그것을 사용)
/// 값이 없으면 Windows 기본값(Vista 이후 Removable 자동실행 비활성화)으로 처리한다.
pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // HKLM NoDriveTypeAutoRun
    let lm_val = registry::read_dword(REG_PATH_LM, "NoDriveTypeAutoRun").ok();
    // HKCU NoDriveTypeAutoRun
    let cu_val = registry::read_dword_hkcu(REG_PATH_CU, "NoDriveTypeAutoRun").ok();

    if let Some(v) = lm_val {
        evidence.insert("HKLM_NoDriveTypeAutoRun".into(), format!("0x{:02X}", v));
    }
    if let Some(v) = cu_val {
        evidence.insert("HKCU_NoDriveTypeAutoRun".into(), format!("0x{:02X}", v));
    }

    // 적용될 값 결정 (HKLM 우선)
    let effective = lm_val.or(cu_val);

    match effective {
        None => {
            // 레지스트리 키 없음 = Windows Vista 이후 기본값
            // (removable drive AutoRun 은 이미 비활성화)
            evidence.insert(
                "note".into(),
                "registry key absent; Windows default applies".into(),
            );
            CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Ok,
                message:
                    "이동식 드라이브(CD/DVD, USB 등) 자동 실행이 기본으로 비활성화되어 있습니다."
                        .into(),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            }
        }
        Some(val) => {
            let removable_disabled = (val & REMOVABLE_BIT) != 0;
            let all_disabled = val == 0xFF || val == 0x91;

            if !removable_disabled {
                // USB(이동식) 드라이브 AutoRun 이 명시적으로 활성화된 상태
                CheckResult {
                    id: ID.into(),
                    title: TITLE.into(),
                    severity: Severity::Danger,
                    message: format!(
                        "이동식 드라이브(CD/DVD, USB 등) 자동 실행이 활성화되어 있습니다. \
                         이동식 드라이브에 악성 소프트웨어가 있을 경우 자동 실행될 수 있습니다. \
                         (NoDriveTypeAutoRun = 0x{:02X})",
                        val
                    ),
                    action_uri: None,
                    evidence,
                    checked_at: now_ts(),
                }
            } else if !all_disabled {
                // USB는 막혔지만 다른 드라이브 타입은 허용 중
                CheckResult {
                    id: ID.into(),
                    title: TITLE.into(),
                    severity: Severity::Warning,
                    message: format!(
                        "일부 드라이브 타입에서 자동 실행이 허용되어 있습니다. \
                         (NoDriveTypeAutoRun = 0x{:02X})",
                        val
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
                    message:
                        "모든 이동식 드라이브(CD/DVD, USB 등)의 자동 실행이 비활성화되어 있습니다."
                            .into(),
                    action_uri: None,
                    evidence,
                    checked_at: now_ts(),
                }
            }
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
        let _ = r.severity;
    }

    #[test]
    fn removable_bit_detection() {
        // 0xFF → removable bit set → Ok
        assert!((0xFF_u32 & REMOVABLE_BIT) != 0);
        // 0x00 → removable bit NOT set → Danger
        assert!((0x00_u32 & REMOVABLE_BIT) == 0);
        // 0x04 → only removable bit set → Ok (removable disabled)
        assert!((0x04_u32 & REMOVABLE_BIT) != 0);
    }
}
