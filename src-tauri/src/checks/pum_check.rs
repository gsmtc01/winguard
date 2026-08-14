use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::registry::{read_dword, read_dword_hkcu, read_string_hkcu};
use std::collections::HashMap;

const ID: &str = "pum_check";
const TITLE: &str = "잠재적 레지스트리 변조(PUM)";

enum Hive {
    Hklm,
    Hkcu,
}

struct PumEntry {
    hive: Hive,
    path: &'static str,
    name: &'static str,
    /// 이 값이어야 정상
    safe_value: u32,
    description: &'static str,
    severity_on_violation: Severity,
}

const PUM_TABLE: &[PumEntry] = &[
    // ── 보안 도구 접근 차단 ─────────────────────────────────────
    PumEntry {
        hive: Hive::Hklm,
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
        name: "DisableRegistryTools",
        safe_value: 0,
        description: "레지스트리 편집기(regedit) 접근 차단",
        severity_on_violation: Severity::Danger,
    },
    PumEntry {
        hive: Hive::Hkcu,
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
        name: "DisableRegistryTools",
        safe_value: 0,
        description: "레지스트리 편집기(regedit) 접근 차단 (사용자)",
        severity_on_violation: Severity::Danger,
    },
    PumEntry {
        hive: Hive::Hklm,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "DisableCMD",
        safe_value: 0,
        description: "명령 프롬프트(cmd) 접근 차단",
        severity_on_violation: Severity::Danger,
    },
    PumEntry {
        hive: Hive::Hkcu,
        path: r"SOFTWARE\Policies\Microsoft\Windows\System",
        name: "DisableCMD",
        safe_value: 0,
        description: "명령 프롬프트(cmd) 접근 차단 (사용자)",
        severity_on_violation: Severity::Danger,
    },
    // ── 보안 센터 알림 억제 ──────────────────────────────────────
    PumEntry {
        hive: Hive::Hklm,
        path: r"SOFTWARE\Microsoft\Security Center",
        name: "UpdatesDisableNotify",
        safe_value: 0,
        description: "Windows Update 알림 비활성화",
        severity_on_violation: Severity::Warning,
    },
    PumEntry {
        hive: Hive::Hklm,
        path: r"SOFTWARE\Microsoft\Security Center",
        name: "FirewallDisableNotify",
        safe_value: 0,
        description: "방화벽 알림 비활성화",
        severity_on_violation: Severity::Warning,
    },
    PumEntry {
        hive: Hive::Hklm,
        path: r"SOFTWARE\Microsoft\Security Center",
        name: "AntiVirusDisableNotify",
        safe_value: 0,
        description: "백신 알림 비활성화",
        severity_on_violation: Severity::Warning,
    },
    // ── 암호화 서비스 비활성화 ───────────────────────────────────
    PumEntry {
        hive: Hive::Hklm,
        path: r"SYSTEM\CurrentControlSet\Services\CryptSvc",
        name: "Start",
        safe_value: 2, // 2 = 자동 시작 (정상)
        description: "암호화 서비스(CryptSvc) 비활성화",
        severity_on_violation: Severity::Danger,
    },
    // ── 작업 관리자 비활성화 ─────────────────────────────────────
    PumEntry {
        hive: Hive::Hkcu,
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
        name: "DisableTaskMgr",
        safe_value: 0,
        description: "작업 관리자 접근 차단",
        severity_on_violation: Severity::Danger,
    },
];

const LOW_RISK_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Policies\Associations";
const LOW_RISK_NAME: &str = "LowRiskFileTypes";

/// 키가 존재하면 위반(악성 파일 실행 촉진 목적). 키 없음 = 정상.
fn check_low_risk_file_types() -> Option<String> {
    match read_string_hkcu(LOW_RISK_PATH, LOW_RISK_NAME) {
        Ok(value) => Some(format!(
            "위험도 낮음 파일 형식(LowRiskFileTypes) 정책이 설정되어 있습니다: {}",
            value
        )),
        Err(_) => None,
    }
}

pub fn run() -> CheckResult {
    let mut violations: Vec<String> = Vec::new();
    let mut max_severity = Severity::Ok;
    let mut evidence: HashMap<String, String> = HashMap::new();

    for entry in PUM_TABLE {
        let result = match entry.hive {
            Hive::Hklm => read_dword(entry.path, entry.name),
            Hive::Hkcu => read_dword_hkcu(entry.path, entry.name),
        };

        match result {
            Ok(val) if val != entry.safe_value => {
                violations.push(format!(
                    "{} ({}\\{}={})",
                    entry.description, entry.path, entry.name, val
                ));
                if entry.severity_on_violation > max_severity {
                    max_severity = entry.severity_on_violation;
                }
            }
            Ok(_) => {}  // 정상값
            Err(_) => {} // 키 없음 = 정상 (악성코드가 삭제한 것이 아니라 원래 없는 키)
        }
    }

    if let Some(msg) = check_low_risk_file_types() {
        violations.push(msg);
        if Severity::Danger > max_severity {
            max_severity = Severity::Danger;
        }
    }

    evidence.insert("violation_count".into(), violations.len().to_string());

    let (severity, message) = if violations.is_empty() {
        (
            Severity::Ok,
            "레지스트리 변조 흔적이 감지되지 않았습니다.".to_string(),
        )
    } else {
        evidence.insert("violations".into(), violations.join(" | "));
        (
            max_severity,
            format!(
                "레지스트리 변조 {}건 감지: {} (의도적으로 설정한 경우 허용 목록에 추가하세요)",
                violations.len(),
                violations.join(" | ")
            ),
        )
    };

    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity,
        message,
        action_uri: Some("ms-settings:windowsdefender".into()),
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
        let _ = run();
    }

    #[test]
    fn all_pum_entries_have_non_empty_description() {
        for e in PUM_TABLE {
            assert!(!e.description.is_empty());
        }
    }

    #[test]
    fn low_risk_file_types_check_never_panics() {
        // 실제 반환값은 테스트 머신의 레지스트리 상태에 의존하므로
        // (일반적으로 미설정 → None) panic 없이 완료되는지만 확인한다.
        let _ = check_low_risk_file_types();
    }
}
