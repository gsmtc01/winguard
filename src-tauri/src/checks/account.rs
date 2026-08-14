use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use serde::Deserialize;
use std::collections::HashMap;

const ID: &str = "local_accounts";
const TITLE: &str = "로컬 계정 보안";
const ACTION: &str = "ms-settings:signinoptions";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LocalUser {
    name: Option<String>,
    blank_password: Option<bool>,
}

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let script = r#"
        try {
            Add-Type -AssemblyName System.DirectoryServices.AccountManagement
            $ctx = [System.DirectoryServices.AccountManagement.PrincipalContext]::new('Machine')
            Get-LocalUser | Where-Object { $_.Enabled -eq $true } | ForEach-Object {
                $blank = $false
                try { $blank = $ctx.ValidateCredentials($_.Name, '') } catch {}
                [PSCustomObject]@{ Name = $_.Name; BlankPassword = $blank }
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

    let users: Vec<LocalUser> = if raw.trim_start().starts_with('[') {
        match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                evidence.insert("parse_error".into(), e.to_string());
                return unknown(evidence);
            }
        }
    } else {
        match serde_json::from_str::<LocalUser>(&raw) {
            Ok(u) => vec![u],
            Err(e) => {
                evidence.insert("parse_error".into(), e.to_string());
                return unknown(evidence);
            }
        }
    };

    let no_password: Vec<String> = users
        .iter()
        .filter(|u| u.blank_password.unwrap_or(false))
        .filter_map(|u| u.name.clone())
        .collect();

    evidence.insert("total_enabled_accounts".into(), users.len().to_string());
    evidence.insert("no_password_accounts".into(), no_password.join(", "));

    if no_password.is_empty() {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: "모든 활성 계정에 암호가 설정되어 있습니다.".into(),
            action_uri: Some(ACTION.into()),
            evidence,
            checked_at: now_ts(),
        }
    } else {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Danger,
            message: format!(
                "암호가 설정되지 않은 계정 {}이(가) 있습니다. 즉시 설정하세요.",
                no_password.join(", ")
            ),
            action_uri: Some(ACTION.into()),
            evidence,
            checked_at: now_ts(),
        }
    }
}

fn unknown(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "계정 보안 정보를 확인할 수 없습니다.".into(),
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
