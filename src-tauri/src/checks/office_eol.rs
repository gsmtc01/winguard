use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const ID: &str = "office_eol";
const TITLE: &str = "오피스 소프트웨어 지원 종료";
const EOL_API: &str = "https://endoflife.date/api/office.json";
const CACHE_TTL: Duration = Duration::from_secs(86_400);
const WARN_DAYS: i64 = 365;

// ── 타입 ────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
struct OfficeCycle {
    cycle: String,
    #[serde(default)]
    eol: serde_json::Value,
}

static EOL_CACHE: Mutex<Option<(Instant, Vec<OfficeCycle>)>> = Mutex::new(None);

#[derive(Debug, PartialEq, Clone)]
enum OfficeGen {
    M365,
    V2024,
    V2021,
    V2019,
    V2016,
    V2013,
    Unknown,
}

impl OfficeGen {
    fn cycle_id(&self) -> Option<&'static str> {
        match self {
            OfficeGen::M365 => None,
            OfficeGen::V2024 => Some("2024"),
            OfficeGen::V2021 => Some("2021"),
            OfficeGen::V2019 => Some("2019"),
            OfficeGen::V2016 => Some("2016"),
            OfficeGen::V2013 => Some("2013"),
            OfficeGen::Unknown => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            OfficeGen::M365 => "Microsoft 365",
            OfficeGen::V2024 => "Office 2024",
            OfficeGen::V2021 => "Office 2021",
            OfficeGen::V2019 => "Office 2019",
            OfficeGen::V2016 => "Office 2016",
            OfficeGen::V2013 => "Office 2013",
            OfficeGen::Unknown => "Microsoft Office (버전 미확인)",
        }
    }
}

/// 설치된 소프트웨어 항목
struct DetectedProduct {
    label: String,
    /// None = M365(무기한) or Unknown, Some(date) = 지원 종료일
    eol_date: Option<NaiveDate>,
    /// 지원 종료일이 이미 지났음을 명시적으로 표시
    already_expired: bool,
}

// ── EOL API ─────────────────────────────────────────────────────

fn fetch_office_cycles(evidence: &mut HashMap<String, String>) -> Option<Vec<OfficeCycle>> {
    {
        if let Ok(guard) = EOL_CACHE.lock() {
            if let Some((ts, ref cycles)) = *guard {
                if ts.elapsed() < CACHE_TTL {
                    evidence.insert("office_eol_source".into(), "cache".into());
                    return Some(cycles.clone());
                }
            }
        }
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("WinGuard/0.1")
        .build()
        .ok()?;

    let resp = client
        .get(EOL_API)
        .header("Accept", "application/json")
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let cycles: Vec<OfficeCycle> = resp.json().ok()?;

    if let Ok(mut guard) = EOL_CACHE.lock() {
        *guard = Some((Instant::now(), cycles.clone()));
    }

    evidence.insert("office_eol_source".into(), "api".into());
    Some(cycles)
}

fn lookup_office_eol(cycles: &[OfficeCycle], cycle_id: &str) -> Option<NaiveDate> {
    let entry = cycles
        .iter()
        .find(|c| c.cycle.eq_ignore_ascii_case(cycle_id))?;
    let date_str = entry.eol.as_str()?;
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

// ── Office 탐지 ─────────────────────────────────────────────────

/// C2R ProductReleaseIds 또는 MSI DisplayName 에서 세대를 분류한다.
fn classify_c2r(product_release_ids: &str) -> OfficeGen {
    let v = product_release_ids.to_lowercase();
    if v.contains("o365") || v.contains("m365") || v.contains("subscription") {
        OfficeGen::M365
    } else if v.contains("2024") {
        OfficeGen::V2024
    } else if v.contains("2021") {
        OfficeGen::V2021
    } else if v.contains("2019") {
        OfficeGen::V2019
    } else if v.contains("2016") {
        OfficeGen::V2016
    } else if v.contains("2013") {
        OfficeGen::V2013
    } else {
        OfficeGen::Unknown
    }
}

fn classify_msi(display_name: &str) -> OfficeGen {
    let v = display_name.to_lowercase();
    if v.contains("2016") {
        OfficeGen::V2016
    } else if v.contains("2013") {
        OfficeGen::V2013
    } else {
        OfficeGen::Unknown
    }
}

// ── Hancom EOL 테이블 ────────────────────────────────────────────

fn hancom_eol_date(display_name: &str) -> Option<Result<NaiveDate, ()>> {
    let date = if display_name.contains("2024") || display_name.contains("2022") {
        "2028-12-31"
    } else if display_name.contains("2020") || display_name.contains("2018") {
        "2026-12-31"
    } else if display_name.contains("NEO")
        || display_name.contains("2014")
        || display_name.contains("2010")
    {
        return Some(Err(())); // 이미 종료
    } else {
        return None; // 알 수 없음
    };
    Some(Ok(
        NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("hardcoded date")
    ))
}

// ── PowerShell 탐지 ─────────────────────────────────────────────

const DETECT_SCRIPT: &str = r#"
$result = @{ office_c2r = ''; office_msi = @(); hancom = @() }

$c2rProps = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Office\ClickToRun\Configuration' -ErrorAction SilentlyContinue
if ($c2rProps) { $result.office_c2r = [string]$c2rProps.ProductReleaseIds }

$paths = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$all = Get-ItemProperty $paths -ErrorAction SilentlyContinue

$msi = $all |
    Where-Object { $_.DisplayName -match '^Microsoft Office (Standard|Professional|Home|Access|Word|Excel|PowerPoint|Outlook|Personal).*(2013|2016)' } |
    Select-Object -ExpandProperty DisplayName -Unique
if ($msi) { $result.office_msi = @($msi) }

$hc = $all |
    Where-Object { $_.DisplayName -match '한컴오피스|HNC Office|Hancom Office|ThinkFree' } |
    Where-Object { $_.DisplayName -notmatch '웹 컴포넌트|플러그인|Plugin|Runtime|Updater|업데이터' } |
    Select-Object -ExpandProperty DisplayName -Unique
if ($hc) { $result.hancom = @($hc) }

$result | ConvertTo-Json -Compress
"#;

struct Detection {
    office_c2r: String,
    office_msi: Vec<String>,
    hancom: Vec<String>,
}

fn detect(evidence: &mut HashMap<String, String>) -> Option<Detection> {
    let raw = match powershell::run(DETECT_SCRIPT) {
        Ok(o) => o,
        Err(e) => {
            evidence.insert("detect_error".into(), e);
            return None;
        }
    };

    evidence.insert("raw_detection".into(), raw.clone());

    let v: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;

    let office_c2r = v["office_c2r"].as_str().unwrap_or("").to_string();

    let office_msi = v["office_msi"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let hancom = v["hancom"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Some(Detection {
        office_c2r,
        office_msi,
        hancom,
    })
}

// ── 메인 ────────────────────────────────────────────────────────

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let det = match detect(&mut evidence) {
        Some(d) => d,
        None => return err_result(evidence),
    };

    let cycles = fetch_office_cycles(&mut evidence);

    let mut products: Vec<DetectedProduct> = Vec::new();

    // Microsoft Office (C2R)
    if !det.office_c2r.is_empty() {
        evidence.insert("c2r_release_ids".into(), det.office_c2r.clone());
        for token in det
            .office_c2r
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let gen = classify_c2r(token);
            if gen == OfficeGen::Unknown {
                continue;
            }
            if gen == OfficeGen::M365 {
                products.push(DetectedProduct {
                    label: gen.label().to_string(),
                    eol_date: None,
                    already_expired: false,
                });
                continue;
            }
            let eol = cycles
                .as_deref()
                .and_then(|cs| lookup_office_eol(cs, gen.cycle_id()?));
            products.push(DetectedProduct {
                label: gen.label().to_string(),
                eol_date: eol,
                already_expired: eol.map(|d| d < Utc::now().date_naive()).unwrap_or(false),
            });
        }
    }

    // Microsoft Office (MSI — 2013/2016만)
    for name in &det.office_msi {
        let gen = classify_msi(name);
        if gen == OfficeGen::Unknown {
            continue;
        }
        let eol = cycles
            .as_deref()
            .and_then(|cs| lookup_office_eol(cs, gen.cycle_id()?));
        products.push(DetectedProduct {
            label: format!("{} (MSI)", gen.label()),
            eol_date: eol,
            already_expired: eol.map(|d| d < Utc::now().date_naive()).unwrap_or(false),
        });
    }

    // 한컴오피스
    for name in &det.hancom {
        match hancom_eol_date(name) {
            None => {
                products.push(DetectedProduct {
                    label: name.clone(),
                    eol_date: None,
                    already_expired: false,
                });
            }
            Some(Err(())) => {
                products.push(DetectedProduct {
                    label: name.clone(),
                    eol_date: None,
                    already_expired: true,
                });
            }
            Some(Ok(date)) => {
                let today = Utc::now().date_naive();
                products.push(DetectedProduct {
                    label: name.clone(),
                    eol_date: Some(date),
                    already_expired: date < today,
                });
            }
        }
    }

    if products.is_empty() {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Info,
            message: "Microsoft Office 또는 한컴오피스가 설치되어 있지 않습니다.".into(),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    // 결과 집계
    let today = Utc::now().date_naive();
    let mut expired: Vec<String> = Vec::new();
    let mut warning: Vec<String> = Vec::new();

    for p in &products {
        if p.already_expired {
            expired.push(p.label.clone());
        } else if let Some(d) = p.eol_date {
            let days = (d - today).num_days();
            if days <= WARN_DAYS {
                warning.push(format!("{} ({}일 후 종료)", p.label, days));
            }
            evidence.insert(
                format!("{}_eol", p.label.to_lowercase().replace(' ', "_")),
                d.to_string(),
            );
        }
    }

    if !expired.is_empty() {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Danger,
            message: format!(
                "지원이 종료된 오피스 소프트웨어가 설치되어 있습니다. 보안 업데이트를 더 이상 받을 수 없습니다.\n즉시 최신 버전으로 업그레이드하세요. ({})",
                expired.join(", ")
            ),
            action_uri: None,
            evidence,
            checked_at: now_ts(),
        };
    }

    if !warning.is_empty() {
        return CheckResult {
            id: ID.into(),
            title: TITLE.into(),
            severity: Severity::Warning,
            message: format!(
                "곧 지원이 종료되는 오피스 소프트웨어가 있습니다. 업그레이드를 계획하세요. ({})",
                warning.join(", ")
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
        message: format!(
            "설치된 오피스 소프트웨어가 지원 중입니다. ({})",
            products
                .iter()
                .map(|p| p.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        action_uri: None,
        evidence,
        checked_at: now_ts(),
    }
}

fn err_result(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "오피스 소프트웨어 설치 정보를 확인할 수 없습니다.".into(),
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
        let _ = run();
    }

    #[test]
    fn classify_c2r_m365() {
        assert_eq!(classify_c2r("O365ProPlusRetail"), OfficeGen::M365);
        assert_eq!(classify_c2r("M365BusinessRetail"), OfficeGen::M365);
    }

    #[test]
    fn classify_c2r_versions() {
        assert_eq!(classify_c2r("ProPlus2021Volume"), OfficeGen::V2021);
        assert_eq!(classify_c2r("Standard2019Volume"), OfficeGen::V2019);
        assert_eq!(classify_c2r("ProPlus2024Retail"), OfficeGen::V2024);
    }

    #[test]
    fn classify_msi_versions() {
        assert_eq!(
            classify_msi("Microsoft Office Professional Plus 2016"),
            OfficeGen::V2016
        );
        assert_eq!(
            classify_msi("Microsoft Office Standard 2013"),
            OfficeGen::V2013
        );
        assert_eq!(
            classify_msi("Microsoft Office Professional 2010"),
            OfficeGen::Unknown
        );
    }

    #[test]
    fn hancom_eol_dates() {
        // 2024/2022 → 2028-12-31
        assert!(matches!(hancom_eol_date("한컴오피스 2024"), Some(Ok(_))));
        assert!(matches!(hancom_eol_date("한컴오피스 2022"), Some(Ok(_))));
        // 2020/2018 → 2026-12-31
        assert!(matches!(hancom_eol_date("한컴오피스 2020"), Some(Ok(_))));
        assert!(matches!(hancom_eol_date("한컴오피스 2018"), Some(Ok(_))));
        // NEO → expired
        assert!(matches!(hancom_eol_date("한컴오피스 NEO"), Some(Err(()))));
        // Unknown
        assert!(hancom_eol_date("Some Other Software").is_none());
    }

    #[test]
    fn office_gen_labels_are_non_empty() {
        for gen in [
            OfficeGen::M365,
            OfficeGen::V2024,
            OfficeGen::V2021,
            OfficeGen::V2019,
            OfficeGen::V2016,
            OfficeGen::V2013,
            OfficeGen::Unknown,
        ] {
            assert!(!gen.label().is_empty());
        }
    }
}
