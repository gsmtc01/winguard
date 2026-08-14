use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

const ID: &str = "windows_updates";
const TITLE: &str = "Windows 업데이트";
const ACTION: &str = "ms-settings:windowsupdate";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UpdateStatus {
    last_install: Option<String>,
    pending_count: Option<i64>,
}

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // Online=false → 로컬 WUA 캐시만 조회, 네트워크 없이 빠르게 실행
    let script = r#"
        try {
            $au      = New-Object -ComObject Microsoft.Update.AutoUpdate
            $session = New-Object -ComObject Microsoft.Update.Session
            $srch    = $session.CreateUpdateSearcher()
            $srch.Online = $false

            $pending = $srch.Search("IsInstalled=0 and Type='Software' and IsHidden=0")

            @{
                LastInstall  = $au.Results.LastInstallationSuccessDate.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
                PendingCount = $pending.Updates.Count
            } | ConvertTo-Json -Compress
        } catch {
            'ERROR:' + $_.Exception.Message
        }
    "#;

    let raw = match powershell::run(script) {
        Ok(s) => s,
        Err(e) => {
            evidence.insert("error".into(), e);
            return unknown(evidence);
        }
    };

    if raw.starts_with("ERROR:") || raw.is_empty() {
        evidence.insert("raw".into(), raw);
        return unknown(evidence);
    }

    let status: UpdateStatus = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            evidence.insert("parse_error".into(), e.to_string());
            return unknown(evidence);
        }
    };

    let pending = status.pending_count.unwrap_or(0);
    evidence.insert("PendingCount".into(), pending.to_string());

    let last_str = status.last_install.unwrap_or_default();
    evidence.insert("LastInstallationSuccessDate".into(), last_str.clone());

    let days_since = if last_str.is_empty() {
        None
    } else {
        DateTime::parse_from_rfc3339(&last_str)
            .ok()
            .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days())
    };

    if let Some(d) = days_since {
        evidence.insert("DaysSince".into(), d.to_string());
    }

    // ── 판정 로직 ──────────────────────────────────────────────
    // pending = 0 → 대기 업데이트 없음 → 정상
    // pending > 0 + days_since >= 30 → 위험 (오래됐고 업데이트도 쌓임)
    // pending > 0 + days_since < 30  → 경고 (업데이트 권장)
    // pending > 0 + 날짜 불명        → 경고

    let (severity, message) = if pending == 0 {
        (Severity::Ok, "설치 대기 중인 업데이트가 없습니다.".into())
    } else {
        match days_since {
            Some(d) if d >= 30 => (
                Severity::Danger,
                format!(
                    "설치 대기 중인 업데이트 {}개가 있으며, 마지막 업데이트로부터 {}일이 경과했습니다. 즉시 업데이트하세요.",
                    pending, d
                ),
            ),
            Some(d) => (
                Severity::Warning,
                format!(
                    "설치 대기 중인 업데이트 {}개가 있습니다. (마지막 업데이트 {}일 전)",
                    pending, d
                ),
            ),
            None => (
                Severity::Warning,
                format!("설치 대기 중인 업데이트 {}개가 있습니다. 업데이트를 권장합니다.", pending),
            ),
        }
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

fn unknown(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "업데이트 상태를 확인할 수 없습니다.".into(),
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
