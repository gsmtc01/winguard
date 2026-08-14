use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

const ID: &str = "windows_defender";
const TITLE: &str = "안티바이러스 보호 상태";
const ACTION: &str = "ms-settings:windowsdefender";

/// Windows Security Center 에 등록된 AV 제품 (WMI SecurityCenter2)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AvProduct {
    display_name: Option<String>,
    /// WMI productState: bits 12–15 가 실시간 보호 활성 여부를 나타낸다.
    product_state: Option<i64>,
}

/// Get-MpComputerStatus 에서 추출하는 Defender 전용 정보
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MpStatus {
    real_time_protection_enabled: Option<bool>,
    antivirus_signature_last_updated: Option<String>,
}

/// productState 에서 실시간 보호 활성 여부 추출
/// bit 12 (0x1000) = 1 이면 실시간 보호 활성
fn rtp_from_product_state(state: i64) -> bool {
    (state & 0x1000) != 0
}

/// Microsoft 제품(Defender) 여부 판별
fn is_microsoft_product(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("windows defender") || lower.contains("microsoft defender")
}

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // ── Step 1: Security Center 에 등록된 전체 AV 목록 조회 ──────
    let sc_script = r#"
        try {
            $avs = Get-WmiObject -Namespace "root\SecurityCenter2" -Class AntiVirusProduct `
                   -ErrorAction Stop |
                   Select-Object displayName, productState
            if ($avs) { $avs | ConvertTo-Json -Compress }
            else { '[]' }
        } catch {
            '[]'
        }
    "#;

    let av_raw = powershell::run(sc_script).unwrap_or_default();
    let av_products: Vec<AvProduct> = {
        let json = if av_raw.trim_start().starts_with('[') {
            av_raw.clone()
        } else if av_raw.trim().is_empty() || av_raw == "[]" {
            "[]".into()
        } else {
            format!("[{}]", av_raw.trim())
        };
        serde_json::from_str(&json).unwrap_or_default()
    };

    // AV 목록 evidence 기록
    let names: Vec<String> = av_products
        .iter()
        .filter_map(|p| p.display_name.clone())
        .collect();
    evidence.insert("RegisteredAV".into(), names.join(", "));

    // ── Step 2: 타사 AV 가 하나라도 활성화되어 있으면 OK ────────
    let third_party_active = av_products.iter().any(|p| {
        let name = p.display_name.as_deref().unwrap_or("");
        !is_microsoft_product(name) && p.product_state.map(rtp_from_product_state).unwrap_or(false)
    });

    if third_party_active {
        let active_name = av_products
            .iter()
            .find(|p| {
                let name = p.display_name.as_deref().unwrap_or("");
                !is_microsoft_product(name)
                    && p.product_state.map(rtp_from_product_state).unwrap_or(false)
            })
            .and_then(|p| p.display_name.clone())
            .unwrap_or_else(|| "타사 백신".into());

        evidence.insert("ActiveThirdPartyAV".into(), active_name.clone());
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: format!("{}이(가) 실시간으로 보호 중입니다.", active_name),
            action_uri: Some(ACTION.into()),
            evidence,
            checked_at: now_ts(),
        };
    }

    // ── Step 3: Defender 상태 상세 점검 ─────────────────────────
    let mp_script = r#"
        try {
            Get-MpComputerStatus |
            Select-Object RealTimeProtectionEnabled, AntivirusSignatureLastUpdated |
            ConvertTo-Json -Compress
        } catch {
            'ERROR:' + $_.Exception.Message
        }
    "#;

    let mp_raw = match powershell::run(mp_script) {
        Ok(s) if !s.starts_with("ERROR:") && !s.is_empty() => s,
        Ok(s) => {
            evidence.insert("DefenderError".into(), s);
            return no_av_critical(evidence);
        }
        Err(e) => {
            evidence.insert("DefenderError".into(), e);
            return no_av_critical(evidence);
        }
    };

    let mp: MpStatus = match serde_json::from_str(&mp_raw) {
        Ok(v) => v,
        Err(e) => {
            evidence.insert("MpParseError".into(), e.to_string());
            return unknown(evidence);
        }
    };

    let rtp = mp.real_time_protection_enabled.unwrap_or(false);
    evidence.insert("DefenderRTP".into(), rtp.to_string());

    if !rtp {
        // 타사 AV 도 없고 Defender 도 꺼진 상태
        return no_av_critical(evidence);
    }

    // ── Step 4: Defender 시그니처 최신화 확인 ────────────────────
    let sig_age_days = mp
        .antivirus_signature_last_updated
        .as_deref()
        .and_then(parse_ps_date_days_ago);

    if let Some(d) = sig_age_days {
        evidence.insert("SignatureAgeDays".into(), d.to_string());
    }

    let (severity, message) = match sig_age_days {
        Some(d) if d >= 30 => (
            Severity::Danger,
            format!(
                "Microsoft Defender 시그니처가 {}일째 갱신되지 않았습니다.",
                d
            ),
        ),
        Some(d) if d >= 7 => (
            Severity::Warning,
            format!("Microsoft Defender 시그니처가 {}일 전에 갱신되었습니다.", d),
        ),
        _ => (
            Severity::Ok,
            "Microsoft Defender가 실시간으로 보호 중입니다.".into(),
        ),
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

fn no_av_critical(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Critical,
        message: "실시간 보호가 비활성화되어 있습니다. 안티바이러스를 활성화하세요.".into(),
        action_uri: Some(ACTION.into()),
        evidence,
        checked_at: now_ts(),
    }
}

fn unknown(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "안티바이러스 보호 상태를 확인할 수 없습니다.".into(),
        action_uri: Some(ACTION.into()),
        evidence,
        checked_at: now_ts(),
    }
}

fn parse_ps_date_days_ago(s: &str) -> Option<i64> {
    let dt = DateTime::parse_from_rfc3339(s)
        .ok()
        .or_else(|| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%:z").ok())?;
    Some((Utc::now() - dt.with_timezone(&Utc)).num_days())
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

    #[test]
    fn rtp_bit_extraction() {
        // 0x61100 = 397312 → bit 12 set → enabled
        assert!(rtp_from_product_state(397312));
        // 0x41000 = 266240 → bit 12 set → enabled (Microsoft Defender disabled by 3rd party)
        // In practice Defender shows bit 12 clear when disabled; this tests the bitmask
        assert!(!rtp_from_product_state(0));
    }
}
