use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use serde::Deserialize;
use std::collections::HashMap;

const ID: &str = "bitlocker";
const TITLE: &str = "장치 암호화(BitLocker)";
const ACTION: &str = "ms-settings:deviceencryption";

/// WMI Win32_EncryptableVolume 결과.
/// ProtectionStatus: 0=Off, 1=On, 2=Unknown
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VolInfo {
    drive_letter: Option<String>,
    protection_status: Option<i64>,
}

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let script = r#"
        try {
            $vols = Get-WmiObject `
                -Namespace "root\cimv2\security\microsoftvolumeencryption" `
                -Class Win32_EncryptableVolume `
                -ErrorAction Stop |
                Select-Object DriveLetter, ProtectionStatus

            # 결과가 배열이 아닐 수 있으므로 @() 로 강제 변환
            $arr = @($vols)
            $arr | ConvertTo-Json -Compress
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

    // 단일 객체일 경우 JSON 배열로 파싱이 실패할 수 있으므로 양쪽 시도
    let vols: Vec<VolInfo> = if let Ok(v) = serde_json::from_str::<Vec<VolInfo>>(&raw) {
        v
    } else if let Ok(single) = serde_json::from_str::<VolInfo>(&raw) {
        vec![single]
    } else {
        evidence.insert("parse_error".into(), raw.clone());
        return unknown(evidence);
    };

    // C: 드라이브 찾기
    let c_drive = vols
        .iter()
        .find(|v| v.drive_letter.as_deref() == Some("C:"));

    match c_drive {
        None => {
            evidence.insert("note".into(), "C: 드라이브를 찾을 수 없습니다".into());
            unknown(evidence)
        }
        Some(vol) => {
            let status = vol.protection_status.unwrap_or(2);
            evidence.insert("C:ProtectionStatus".into(), status.to_string());

            match status {
                1 => CheckResult {
                    id: ID.into(),
                    title: TITLE.into(),
                    severity: Severity::Ok,
                    message: "C: 드라이브에 장치 암호화(BitLocker)가 적용되어 있습니다.".into(),
                    action_uri: Some(ACTION.into()),
                    evidence,
                    checked_at: now_ts(),
                },
                0 => CheckResult {
                    id: ID.into(),
                    title: TITLE.into(),
                    severity: Severity::Warning,
                    message:
                        "C: 드라이브에 장치 암호화(BitLocker)가 적용되어 있지 않습니다. 분실·도난 시 데이터가 노출될 수 있습니다."
                            .into(),
                    action_uri: Some(ACTION.into()),
                    evidence,
                    checked_at: now_ts(),
                },
                _ => unknown(evidence),
            }
        }
    }
}

fn unknown(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "장치 암호화(BitLocker) 상태를 확인할 수 없습니다.".into(),
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
