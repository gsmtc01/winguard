use crate::engine::types::{CheckResult, Severity};

/// 단일 점검 결과만으로 판단하기 어려운 복합 위협 시나리오를 처리한다.
/// - 규칙은 severity 를 올리기만 하고 절대 낮추지 않는다.
pub fn apply_compound_rules(checks: &mut [CheckResult]) {
    // ── 스냅샷 ────────────────────────────────────────────────────
    let eol_danger = sev_gte(checks, "windows_eol", Severity::Danger);
    let boot_warn = sev_eq(checks, "secure_boot", Severity::Warning);
    let smb1_danger = sev_gte(checks, "smb1", Severity::Danger);
    let rdp_open = sev_gte(checks, "rdp", Severity::Warning);
    let fw_not_ok = sev_gte(checks, "firewall", Severity::Warning);
    let av_critical = sev_gte(checks, "windows_defender", Severity::Critical);
    let dns_hijack_danger = sev_gte(checks, "dns_hijack", Severity::Danger);
    let hosts_danger = sev_gte(checks, "hosts_file", Severity::Danger);
    let ransomware_danger = sev_gte(checks, "ransomware_ioc", Severity::Danger);
    let pum_danger = sev_gte(checks, "pum_check", Severity::Danger);

    // ── Rule 1: EOL + Secure Boot 경고 → Danger ───────────────────
    // 지원 종료 OS 에서 Secure Boot 도 꺼져 있으면 부트킷 공격 위험 상승
    if eol_danger && boot_warn {
        promote(
            checks,
            "secure_boot",
            Severity::Danger,
            " (지원 종료 Windows와 함께 사용 중 — 위험도 상향)",
        );
    }

    // ── Rule 2: SMBv1 활성 + EOL → Critical ──────────────────────
    // EternalBlue 계열 웜이 직접 침투 가능한 최악의 조합
    if smb1_danger && eol_danger {
        promote(
            checks,
            "smb1",
            Severity::Critical,
            " (지원 종료 Windows와 함께 사용 중 — EternalBlue 웜 위험 최고)",
        );
    }

    // ── Rule 3: RDP 활성 + 방화벽 비정상 → Critical ──────────────
    // 방화벽이 꺼진 상태에서 RDP 가 열리면 인터넷 직접 노출 가능
    if rdp_open && fw_not_ok {
        promote(
            checks,
            "rdp",
            Severity::Critical,
            " (방화벽 비정상 + RDP 오픈 — 직접 인터넷 노출 위험)",
        );
    }

    // ── Rule 4: AV 없음 + 방화벽 비정상 → 방화벽 Critical ────────
    // 실시간 보호도 없고 방화벽도 없으면 방화벽 Danger → Critical 승격
    if av_critical && fw_not_ok {
        promote(
            checks,
            "firewall",
            Severity::Critical,
            " (안티바이러스도 비활성 — 방어 계층 전무)",
        );
    }

    // ── Rule 5: DNS 변조 + hosts 변조 → DNS Critical ─────────────
    // 두 경로 동시 변조는 표적 공격 강력 지표
    if dns_hijack_danger && hosts_danger {
        promote(
            checks,
            "dns_hijack",
            Severity::Critical,
            " (hosts 파일도 함께 변조됨 — 표적 공격 강력 지표)",
        );
    }

    // ── Rule 6: 랜섬웨어 IOC + AV 비활성 → 랜섬웨어 IOC Critical ─
    // 백신 비활성 + 침해 흔적 동시 = 감염 완료 가능성 매우 높음
    if ransomware_danger && av_critical {
        promote(
            checks,
            "ransomware_ioc",
            Severity::Critical,
            " (안티바이러스도 비활성 — 감염 완료 가능성 매우 높음)",
        );
    }

    // ── Rule 7: 레지스트리 변조(PUM) + AV 비활성 → PUM Critical ──
    // 보안 도구 차단 + 백신 비활성 = 능동적 방어 회피 시도
    if pum_danger && av_critical {
        promote(
            checks,
            "pum_check",
            Severity::Critical,
            " (안티바이러스도 비활성 — 능동적 방어 회피 시도)",
        );
    }
}

// ── 헬퍼 ─────────────────────────────────────────────────────────

fn sev_gte(checks: &[CheckResult], id: &str, min: Severity) -> bool {
    checks.iter().any(|c| c.id == id && c.severity >= min)
}

fn sev_eq(checks: &[CheckResult], id: &str, target: Severity) -> bool {
    checks.iter().any(|c| c.id == id && c.severity == target)
}

/// severity 를 `new_sev` 로 상향하고 메시지 suffix 를 추가한다.
/// 이미 `new_sev` 이상이면 suffix 만 추가한다.
fn promote(checks: &mut [CheckResult], id: &str, new_sev: Severity, suffix: &str) {
    if let Some(c) = checks.iter_mut().find(|c| c.id == id) {
        if c.severity < new_sev {
            c.severity = new_sev;
        }
        c.message.push_str(suffix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::now_ts;
    use std::collections::HashMap;

    fn mk(id: &str, sev: Severity) -> CheckResult {
        CheckResult {
            id: id.into(),
            title: id.into(),
            severity: sev,
            message: "msg".into(),
            action_uri: None,
            evidence: HashMap::new(),
            checked_at: now_ts(),
        }
    }

    #[test]
    fn eol_plus_secureboot_warning_promotes_to_danger() {
        let mut checks = vec![
            mk("windows_eol", Severity::Danger),
            mk("secure_boot", Severity::Warning),
        ];
        apply_compound_rules(&mut checks);
        assert_eq!(checks[1].severity, Severity::Danger);
    }

    #[test]
    fn smb1_plus_eol_promotes_to_critical() {
        let mut checks = vec![
            mk("windows_eol", Severity::Danger),
            mk("smb1", Severity::Danger),
        ];
        apply_compound_rules(&mut checks);
        assert_eq!(
            checks.iter().find(|c| c.id == "smb1").map(|c| c.severity),
            Some(Severity::Critical)
        );
    }

    #[test]
    fn rdp_plus_firewall_warning_promotes_to_critical() {
        let mut checks = vec![
            mk("rdp", Severity::Warning),
            mk("firewall", Severity::Warning),
        ];
        apply_compound_rules(&mut checks);
        assert_eq!(
            checks.iter().find(|c| c.id == "rdp").map(|c| c.severity),
            Some(Severity::Critical)
        );
    }

    #[test]
    fn av_critical_plus_fw_warning_promotes_fw_to_critical() {
        let mut checks = vec![
            mk("windows_defender", Severity::Critical),
            mk("firewall", Severity::Warning),
        ];
        apply_compound_rules(&mut checks);
        assert_eq!(
            checks
                .iter()
                .find(|c| c.id == "firewall")
                .map(|c| c.severity),
            Some(Severity::Critical)
        );
    }

    #[test]
    fn rules_never_lower_severity() {
        let mut checks = vec![mk("secure_boot", Severity::Critical)];
        apply_compound_rules(&mut checks);
        assert_eq!(checks[0].severity, Severity::Critical);
    }

    #[test]
    fn independent_checks_not_affected() {
        // EOL Ok 이면 SMBv1 Danger 가 Critical 로 올라가지 않아야 함
        let mut checks = vec![
            mk("windows_eol", Severity::Ok),
            mk("smb1", Severity::Danger),
        ];
        apply_compound_rules(&mut checks);
        assert_eq!(
            checks.iter().find(|c| c.id == "smb1").map(|c| c.severity),
            Some(Severity::Danger)
        );
    }

    #[test]
    fn dns_hijack_plus_hosts_danger_promotes_to_critical() {
        let mut checks = vec![
            mk("dns_hijack", Severity::Danger),
            mk("hosts_file", Severity::Danger),
        ];
        apply_compound_rules(&mut checks);
        assert_eq!(
            checks
                .iter()
                .find(|c| c.id == "dns_hijack")
                .map(|c| c.severity),
            Some(Severity::Critical)
        );
    }

    #[test]
    fn ransomware_ioc_plus_av_critical_promotes_to_critical() {
        let mut checks = vec![
            mk("ransomware_ioc", Severity::Danger),
            mk("windows_defender", Severity::Critical),
        ];
        apply_compound_rules(&mut checks);
        assert_eq!(
            checks
                .iter()
                .find(|c| c.id == "ransomware_ioc")
                .map(|c| c.severity),
            Some(Severity::Critical)
        );
    }

    #[test]
    fn pum_check_plus_av_critical_promotes_to_critical() {
        let mut checks = vec![
            mk("pum_check", Severity::Danger),
            mk("windows_defender", Severity::Critical),
        ];
        apply_compound_rules(&mut checks);
        assert_eq!(
            checks
                .iter()
                .find(|c| c.id == "pum_check")
                .map(|c| c.severity),
            Some(Severity::Critical)
        );
    }
}
