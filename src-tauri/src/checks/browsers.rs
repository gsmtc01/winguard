use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const ID: &str = "browsers";
const TITLE: &str = "웹 브라우저 업데이트";
const CACHE_TTL: Duration = Duration::from_secs(24 * 3600);
const API_TIMEOUT: Duration = Duration::from_secs(8);

/// 조회한 최신 버전. API 실패 시 None 이므로 항목별로 Option 이다.
type BrowserVersion = Option<String>;

/// (fetched_at, edge_latest, chrome_latest, firefox_latest)
type VersionCacheEntry = (Instant, BrowserVersion, BrowserVersion, BrowserVersion);

static VERSION_CACHE: Mutex<Option<VersionCacheEntry>> = Mutex::new(None);

struct InstalledBrowser {
    name: &'static str, // "Edge" | "Chrome" | "Firefox"
    version: String,
}

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let installed = detect_installed(&mut evidence);

    if installed.is_empty() {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Info,
            message: "Microsoft Edge, Chrome, Firefox가 설치되어 있지 않습니다.".into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    let (edge_latest, chrome_latest, firefox_latest) = fetch_latest_versions();

    let mut outdated_minor: Vec<&str> = Vec::new();
    let mut outdated_major: Vec<&str> = Vec::new();
    let mut any_known = false;

    for b in &installed {
        let latest = match b.name {
            "Edge" => edge_latest.as_deref(),
            "Chrome" => chrome_latest.as_deref(),
            "Firefox" => firefox_latest.as_deref(),
            _ => None,
        };

        evidence.insert(
            format!("{}_current", b.name.to_lowercase()),
            b.version.clone(),
        );

        match latest {
            None => {
                evidence.insert(
                    format!("{}_status", b.name.to_lowercase()),
                    "unknown".into(),
                );
            }
            Some(l) => {
                any_known = true;
                evidence.insert(format!("{}_latest", b.name.to_lowercase()), l.to_string());
                let diff = major_diff(&b.version, l);
                let status = if diff >= 3 {
                    outdated_major.push(b.name);
                    format!("outdated_major({diff})")
                } else if diff >= 1 {
                    outdated_minor.push(b.name);
                    format!("outdated_minor({diff})")
                } else {
                    "up_to_date".into()
                };
                evidence.insert(format!("{}_status", b.name.to_lowercase()), status);
            }
        }
    }

    if !outdated_major.is_empty() {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Danger,
            message: format!(
                "웹 브라우저 보안 업데이트가 크게 뒤처져 있습니다. 즉시 업데이트하세요. ({})",
                outdated_major.join(", ")
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    if !outdated_minor.is_empty() {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Warning,
            message: format!(
                "일부 웹 브라우저가 최신 버전이 아닙니다. 업데이트를 확인하세요. ({})",
                outdated_minor.join(", ")
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    if !any_known {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Info,
            message: "웹 브라우저 최신 버전 정보를 가져오지 못했습니다. 네트워크를 확인하세요."
                .into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Ok,
        message: "설치된 웹 브라우저가 모두 최신 버전입니다.".into(),
        action_uri: None,
        evidence,
        checked_at: now_ts(),
    }
}

fn detect_installed(evidence: &mut HashMap<String, String>) -> Vec<InstalledBrowser> {
    let script = r#"
$paths = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$all = Get-ItemProperty $paths -ErrorAction SilentlyContinue |
    Where-Object {
        $_.DisplayName -eq 'Microsoft Edge' -or
        $_.DisplayName -eq 'Google Chrome' -or
        ($_.DisplayName -match '^Mozilla Firefox')
    } |
    Select-Object DisplayName, DisplayVersion
if ($null -eq $all) {
    Write-Output '[]'
} elseif ($all -is [array]) {
    $all | ConvertTo-Json -Compress
} else {
    "[$(ConvertTo-Json $all -Compress)]"
}
"#;

    let output = match powershell::run(script) {
        Ok(o) => o,
        Err(e) => {
            evidence.insert("detection_error".into(), e);
            return Vec::new();
        }
    };

    let trimmed = output.trim();
    evidence.insert("raw_detection".into(), trimmed.to_string());
    parse_installed(trimmed)
}

fn parse_installed(json: &str) -> Vec<InstalledBrowser> {
    let value: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let items = match value {
        serde_json::Value::Array(arr) => arr,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return Vec::new(),
    };

    let mut browsers: Vec<InstalledBrowser> = Vec::new();
    let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();

    for item in &items {
        let display_name = item["DisplayName"].as_str().unwrap_or("");
        let version = item["DisplayVersion"].as_str().unwrap_or("").to_string();

        if let Some(name) = normalize_name(display_name) {
            if !version.is_empty() && seen.insert(name) {
                browsers.push(InstalledBrowser { name, version });
            }
        }
    }

    browsers
}

fn normalize_name(display_name: &str) -> Option<&'static str> {
    if display_name == "Microsoft Edge" {
        Some("Edge")
    } else if display_name == "Google Chrome" {
        Some("Chrome")
    } else if display_name.starts_with("Mozilla Firefox") {
        Some("Firefox")
    } else {
        None
    }
}

fn fetch_latest_versions() -> (Option<String>, Option<String>, Option<String>) {
    // 캐시 확인
    if let Ok(guard) = VERSION_CACHE.lock() {
        if let Some((fetched_at, edge, chrome, firefox)) = guard.as_ref() {
            if fetched_at.elapsed() < CACHE_TTL {
                return (edge.clone(), chrome.clone(), firefox.clone());
            }
        }
    }

    // 3개 API를 병렬 스레드로 동시에 요청
    let edge_h = std::thread::spawn(fetch_edge_latest);
    let chrome_h = std::thread::spawn(fetch_chrome_latest);
    let firefox_h = std::thread::spawn(fetch_firefox_latest);

    let edge = edge_h.join().ok().flatten();
    let chrome = chrome_h.join().ok().flatten();
    let firefox = firefox_h.join().ok().flatten();

    // 캐시 갱신
    if let Ok(mut guard) = VERSION_CACHE.lock() {
        *guard = Some((
            Instant::now(),
            edge.clone(),
            chrome.clone(),
            firefox.clone(),
        ));
    }

    (edge, chrome, firefox)
}

fn http_get(url: &str) -> Option<String> {
    reqwest::blocking::Client::builder()
        .timeout(API_TIMEOUT)
        .build()
        .ok()?
        .get(url)
        .send()
        .ok()?
        .text()
        .ok()
}

fn fetch_edge_latest() -> Option<String> {
    let body = http_get("https://edgeupdates.microsoft.com/api/products?view=enterprise")?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    let products = json.as_array()?;
    for product in products {
        if product["Product"].as_str() == Some("Stable") {
            for release in product["Releases"].as_array()? {
                if release["Platform"].as_str() == Some("Windows") {
                    return release["ProductVersion"].as_str().map(String::from);
                }
            }
        }
    }
    None
}

fn fetch_chrome_latest() -> Option<String> {
    let body = http_get(
        "https://versionhistory.googleapis.com/v1/chrome/platforms/win/channels/stable/versions",
    )?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    json["versions"][0]["version"].as_str().map(String::from)
}

fn fetch_firefox_latest() -> Option<String> {
    let body = http_get("https://product-details.mozilla.org/1.0/firefox_versions.json")?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    json["LATEST_FIREFOX_VERSION"].as_str().map(String::from)
}

/// 최신 메이저 버전과 설치 버전의 차이를 반환한다 (설치가 더 최신이면 0).
fn major_diff(installed: &str, latest: &str) -> u32 {
    let inst: u32 = installed
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let late: u32 = latest
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    late.saturating_sub(inst)
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
    fn normalize_name_works() {
        assert_eq!(normalize_name("Microsoft Edge"), Some("Edge"));
        assert_eq!(normalize_name("Google Chrome"), Some("Chrome"));
        assert_eq!(
            normalize_name("Mozilla Firefox 125.0.3 (x64 ko-KR)"),
            Some("Firefox")
        );
        assert_eq!(normalize_name("Microsoft Edge Update"), None);
        assert_eq!(normalize_name("Microsoft Edge WebView2 Runtime"), None);
        assert_eq!(normalize_name("Brave Browser"), None);
    }

    #[test]
    fn major_diff_calculation() {
        assert_eq!(major_diff("120.0.0.0", "124.0.0.0"), 4);
        assert_eq!(major_diff("124.0.0.0", "124.0.0.0"), 0);
        assert_eq!(major_diff("125.0.0.0", "124.0.0.0"), 0); // 설치가 더 최신 → 0
        assert_eq!(major_diff("", "124.0.0.0"), 124);
    }

    #[test]
    fn parse_installed_handles_array() {
        let json = r#"[{"DisplayName":"Microsoft Edge","DisplayVersion":"124.0.2478.97"},{"DisplayName":"Google Chrome","DisplayVersion":"120.0.6099.130"}]"#;
        let browsers = parse_installed(json);
        assert_eq!(browsers.len(), 2);
        assert!(browsers.iter().any(|b| b.name == "Edge"));
        assert!(browsers.iter().any(|b| b.name == "Chrome"));
    }

    #[test]
    fn parse_installed_handles_single_object() {
        let json = r#"{"DisplayName":"Google Chrome","DisplayVersion":"124.0.6367.82"}"#;
        let browsers = parse_installed(json);
        assert_eq!(browsers.len(), 1);
        assert_eq!(browsers[0].name, "Chrome");
        assert_eq!(browsers[0].version, "124.0.6367.82");
    }

    #[test]
    fn parse_firefox_with_locale_suffix() {
        let json =
            r#"[{"DisplayName":"Mozilla Firefox 125.0.3 (x64 ko-KR)","DisplayVersion":"125.0.3"}]"#;
        let browsers = parse_installed(json);
        assert_eq!(browsers.len(), 1);
        assert_eq!(browsers[0].name, "Firefox");
        assert_eq!(browsers[0].version, "125.0.3");
    }

    #[test]
    fn parse_deduplicates_same_browser() {
        let json = r#"[{"DisplayName":"Google Chrome","DisplayVersion":"124.0.6367.82"},{"DisplayName":"Google Chrome","DisplayVersion":"124.0.6367.82"}]"#;
        let browsers = parse_installed(json);
        assert_eq!(browsers.len(), 1);
    }

    #[test]
    fn parse_empty_array() {
        assert!(parse_installed("[]").is_empty());
        assert!(parse_installed("").is_empty());
    }

    #[test]
    #[ignore] // 실제 API 키 / 네트워크 필요
    fn fetch_real_api_versions() {
        let (edge, chrome, firefox) = fetch_latest_versions();
        assert!(edge.is_some(), "Edge API 응답 없음");
        assert!(chrome.is_some(), "Chrome API 응답 없음");
        assert!(firefox.is_some(), "Firefox API 응답 없음");
    }
}
