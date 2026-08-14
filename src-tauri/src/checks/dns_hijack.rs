use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use std::collections::HashMap;
use std::net::Ipv4Addr;

const ID: &str = "dns_hijack";
const TITLE: &str = "DNS 서버 변조";

/// 활성 NIC 최대 검사 개수 (성능 보호, §1-9)
const MAX_NICS: usize = 10;

/// 공인 공용 DNS 허용 목록
const ALLOWED_DNS: &[&str] = &[
    // Google
    "8.8.8.8",
    "8.8.4.4", // Cloudflare
    "1.1.1.1",
    "1.0.0.1", // KT (한국)
    "168.126.63.1",
    "168.126.63.2", // SKT (한국)
    "210.220.163.82",
    "219.250.36.130", // LGU+ (한국)
    "164.124.101.2",
    "203.248.252.2", // Quad9
    "9.9.9.9",
    "149.112.112.112", // OpenDNS
    "208.67.222.222",
    "208.67.220.220", // Microsoft
    "4.4.4.4",
];

/// DNS IP 하나에 대한 위험 수준 분류
#[derive(Debug, PartialEq, Eq)]
enum DnsRisk {
    Ok,
    SuspiciousPublic,
}

/// 사설 IP 여부 판단 (RFC 1918 + loopback). IPv4 파싱 실패 시 false.
fn is_private_ip(ip: &str) -> bool {
    match ip.parse::<Ipv4Addr>() {
        Ok(addr) => addr.is_private() || addr.is_loopback(),
        Err(_) => false,
    }
}

fn classify_dns(ip: &str) -> DnsRisk {
    if ALLOWED_DNS.contains(&ip) {
        return DnsRisk::Ok;
    }
    if is_private_ip(ip) {
        // 공유기 DNS(사설 게이트웨이)는 허용
        return DnsRisk::Ok;
    }
    DnsRisk::SuspiciousPublic
}

const PS_SCRIPT: &str =
    "Get-WmiObject -Class Win32_NetworkAdapterConfiguration -Filter 'IPEnabled=TRUE' | \
     Where-Object { $_.DNSServerSearchOrder -ne $null } | \
     Select-Object Description, DNSServerSearchOrder | \
     ConvertTo-Json -Compress";

/// PowerShell JSON 출력을 NIC 목록으로 파싱한다.
/// ConvertTo-Json 은 NIC 이 1개면 객체, 여러 개면 배열을 반환하므로 둘 다 처리한다.
fn parse_nics(raw: &str) -> Vec<(String, Vec<String>)> {
    let cleaned = raw.replace("\\u0000", "").replace('\0', "");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return Vec::new();
    }
    let value: serde_json::Value = match serde_json::from_str(cleaned) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr: Vec<serde_json::Value> = match value {
        serde_json::Value::Array(a) => a,
        other => vec![other],
    };
    arr.iter()
        .map(|nic| {
            let desc = nic
                .get("Description")
                .and_then(|d| d.as_str())
                .unwrap_or("알 수 없는 NIC")
                .to_string();
            let dns = nic
                .get("DNSServerSearchOrder")
                .and_then(|d| d.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            (desc, dns)
        })
        .collect()
}

fn build(severity: Severity, message: String, evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity,
        message,
        action_uri: None,
        evidence,
        checked_at: now_ts(),
    }
}

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let raw = match powershell::run(PS_SCRIPT) {
        Ok(out) => out,
        Err(e) => {
            evidence.insert("error".into(), e);
            return build(
                Severity::Info,
                "DNS 서버 설정을 조회할 수 없습니다.".into(),
                evidence,
            );
        }
    };

    let nics = parse_nics(&raw);
    if nics.is_empty() {
        return build(
            Severity::Info,
            "활성 NIC 또는 DNS 설정을 찾을 수 없습니다.".into(),
            evidence,
        );
    }

    let mut suspicious: Vec<String> = Vec::new();
    let mut unknown_ipv6: Vec<String> = Vec::new();
    let mut total_dns = 0usize;

    for (desc, dns_list) in nics.iter().take(MAX_NICS) {
        for ip in dns_list {
            total_dns += 1;
            if ip.contains(':') {
                // IPv6: ::1(loopback)만 허용, 나머지는 Info 처리 (§1-9)
                if ip != "::1" {
                    unknown_ipv6.push(format!("{}: {}", desc, ip));
                }
                continue;
            }
            if classify_dns(ip) == DnsRisk::SuspiciousPublic {
                suspicious.push(format!("{}의 DNS 서버 {}", desc, ip));
            }
        }
    }

    evidence.insert("nic_count".into(), nics.len().to_string());
    evidence.insert("dns_count".into(), total_dns.to_string());

    if !suspicious.is_empty() {
        evidence.insert("suspicious".into(), suspicious.join(" | "));
        let message = format!(
            "DNS 변조 의심: {}이(가) 비공인 서버입니다. 공유기/시스템 DNS 설정을 확인하세요.",
            suspicious.join(", ")
        );
        return build(Severity::Danger, message, evidence);
    }

    if !unknown_ipv6.is_empty() {
        evidence.insert("ipv6_unknown".into(), unknown_ipv6.join(" | "));
        return build(
            Severity::Info,
            format!(
                "확인되지 않은 IPv6 DNS 서버 {}개가 설정되어 있습니다. 현재 버전은 IPv6 DNS 검증을 지원하지 않습니다.",
                unknown_ipv6.len()
            ),
            evidence,
        );
    }

    build(
        Severity::Ok,
        "모든 NIC의 DNS 서버가 정상입니다.".into(),
        evidence,
    )
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
    fn known_good_dns_classified_as_ok() {
        assert_eq!(classify_dns("8.8.8.8"), DnsRisk::Ok);
        assert_eq!(classify_dns("168.126.63.1"), DnsRisk::Ok);
    }

    #[test]
    fn private_gateway_classified_as_ok() {
        assert_eq!(classify_dns("192.168.1.1"), DnsRisk::Ok);
        assert_eq!(classify_dns("10.0.0.1"), DnsRisk::Ok);
        assert_eq!(classify_dns("172.16.0.1"), DnsRisk::Ok);
    }

    #[test]
    fn unknown_public_ip_classified_as_suspicious() {
        assert_eq!(classify_dns("203.0.113.99"), DnsRisk::SuspiciousPublic);
    }

    #[test]
    fn is_private_ip_handles_invalid() {
        assert!(!is_private_ip("not-an-ip"));
        assert!(!is_private_ip("8.8.8.8"));
        assert!(is_private_ip("127.0.0.1"));
    }

    #[test]
    fn parse_nics_handles_single_object_and_array() {
        let single = r#"{"Description":"Wi-Fi","DNSServerSearchOrder":["192.168.1.1"]}"#;
        assert_eq!(parse_nics(single).len(), 1);
        let array = r#"[{"Description":"A","DNSServerSearchOrder":["8.8.8.8"]},{"Description":"B","DNSServerSearchOrder":["1.1.1.1"]}]"#;
        assert_eq!(parse_nics(array).len(), 2);
        assert_eq!(parse_nics("").len(), 0);
    }
}
