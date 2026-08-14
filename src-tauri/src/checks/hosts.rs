use crate::engine::types::{now_ts, CheckResult, Severity};
use std::collections::HashMap;
use std::fs;

const ID: &str = "hosts_file";
const TITLE: &str = "hosts 파일 변조";
const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

/// Windows 기본 hosts 항목 (IP, 호스트) — 이 항목은 정상으로 간주한다.
const SAFE_DEFAULTS: &[(&str, &str)] = &[
    ("127.0.0.1", "localhost"),
    ("::1", "localhost"),
    ("127.0.0.1", "ip6-localhost"),
    ("127.0.0.1", "ip6-loopback"),
    ("::1", "ip6-localhost"),
    ("::1", "ip6-loopback"),
    ("255.255.255.255", "broadcasthost"),
    ("0.0.0.0", "0.0.0.0"),
];

/// 이 도메인이 비정상 IP 로 리디렉션되면 Danger
const SENSITIVE_DOMAINS: &[&str] = &[
    "microsoft.com",
    "windows.com",
    "windowsupdate.com",
    "update.microsoft.com",
    "live.com",
    "google.com",
    "gmail.com",
    "apple.com",
    "icloud.com",
];

struct HostEntry {
    ip: String,
    host: String,
}

fn parse_hosts(content: &str) -> Vec<HostEntry> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            // 인라인 주석 제거
            let active = trimmed.split('#').next().unwrap_or("").trim();
            let mut parts = active.split_whitespace();
            let ip = parts.next()?.to_string();
            let host = parts.next()?.to_lowercase();
            Some(HostEntry { ip, host })
        })
        .collect()
}

fn is_safe_default(entry: &HostEntry) -> bool {
    SAFE_DEFAULTS.iter().any(|(ip, host)| {
        entry.ip.eq_ignore_ascii_case(ip) && entry.host.eq_ignore_ascii_case(host)
    })
}

fn is_sensitive_domain(host: &str) -> bool {
    SENSITIVE_DOMAINS
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{}", d)))
}

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let content = match fs::read_to_string(HOSTS_PATH) {
        Ok(c) => c,
        Err(e) => {
            evidence.insert("error".into(), e.to_string());
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Info,
                message: "hosts 파일을 읽을 수 없습니다.".into(),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            };
        }
    };

    let entries = parse_hosts(&content);
    let non_default: Vec<&HostEntry> = entries.iter().filter(|e| !is_safe_default(e)).collect();

    let suspicious: Vec<String> = non_default
        .iter()
        .filter(|e| is_sensitive_domain(&e.host))
        .map(|e| format!("{} → {}", e.host, e.ip))
        .collect();

    evidence.insert("total_entries".into(), entries.len().to_string());
    evidence.insert("non_default_count".into(), non_default.len().to_string());

    if !suspicious.is_empty() {
        evidence.insert("suspicious".into(), suspicious.join(", "));
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Danger,
            message: format!(
                "hosts 파일에서 신뢰할 수 있는 도메인이 변조되었습니다. {}",
                suspicious.join(", ")
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    if !non_default.is_empty() {
        let list: Vec<String> = non_default
            .iter()
            .map(|e| format!("{} {}", e.ip, e.host))
            .collect();
        evidence.insert("non_default_list".into(), list.join("; "));
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Warning,
            message: format!(
                "hosts 파일에 비표준 항목 {}개가 있습니다. 직접 추가한 항목이 아닐 경우 확인이 필요합니다.",
                non_default.len()
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Ok,
        message: "hosts 파일이 정상입니다. 비표준 항목이 없습니다.".into(),
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

    #[test]
    fn parse_ignores_comments_and_blank_lines() {
        let content = "# comment\n\n127.0.0.1 localhost\n192.168.1.1 example.com\n";
        let entries = parse_hosts(content);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn sensitive_domain_detection() {
        assert!(is_sensitive_domain("microsoft.com"));
        assert!(is_sensitive_domain("update.microsoft.com"));
        assert!(!is_sensitive_domain("example.com"));
    }

    #[test]
    fn safe_default_localhost_ignored() {
        let e = HostEntry {
            ip: "127.0.0.1".into(),
            host: "localhost".into(),
        };
        assert!(is_safe_default(&e));
    }
}
