use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::registry;
use std::collections::HashMap;

const ID: &str = "rdp";
const TITLE: &str = "원격 데스크톱(RDP)";
const TS_PATH: &str = "SYSTEM\\CurrentControlSet\\Control\\Terminal Server";
const NLA_PATH: &str = "SYSTEM\\CurrentControlSet\\Control\\Terminal Server\\WinStations\\RDP-Tcp";
const ACTION: &str = "ms-settings:remotedesktop";

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    // fDenyTSConnections: 1(기본) = RDP 차단(안전), 0 = RDP 허용
    let deny = match registry::read_dword(TS_PATH, "fDenyTSConnections") {
        Ok(v) => v,
        Err(_) => {
            // 키 없음 = 기본값 차단
            evidence.insert(
                "fDenyTSConnections".into(),
                "absent (default=denied)".into(),
            );
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Ok,
                message: "원격 데스크톱(RDP)이(가) 비활성화되어 있습니다.".into(),
                action_uri: Some(ACTION.into()),
                evidence,
                checked_at: now_ts(),
            };
        }
    };
    evidence.insert("fDenyTSConnections".into(), deny.to_string());

    if deny != 0 {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Ok,
            message: "원격 데스크톱(RDP)이(가) 비활성화되어 있습니다.".into(),
            action_uri: Some(ACTION.into()),
            evidence,
            checked_at: now_ts(),
        };
    }

    // RDP 활성화됨 → NLA(네트워크 수준 인증) 확인
    // UserAuthentication: 1 = NLA 필수(권장), 0 = 사전 인증 없음(위험)
    let nla = registry::read_dword(NLA_PATH, "UserAuthentication").unwrap_or(0);
    evidence.insert("UserAuthentication".into(), nla.to_string());

    if nla == 1 {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Warning,
            message:
                "원격 데스크톱(RDP)이(가) 활성화되어 있습니다. 네트워크 수준 인증(NLA)이(가) 설정되어 있으나, 불필요하다면 비활성화하세요."
                    .into(),
            action_uri: Some(ACTION.into()),
            evidence,
            checked_at: now_ts(),
        }
    } else {
        CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Danger,
            message:
                "원격 데스크톱(RDP)이(가) 활성화되어 있고, 네트워크 수준 인증(NLA)이(가) 꺼져 있습니다. 사전 인증 공격에 취약합니다."
                    .into(),
            action_uri: Some(ACTION.into()),
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
        let _ = r.severity;
    }
}
