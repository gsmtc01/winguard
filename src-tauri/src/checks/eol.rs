use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::registry;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const ID: &str = "windows_eol";
const TITLE: &str = "Windows 지원 종료 여부";
/// endoflife.date simple JSON API (배열 반환)
const EOL_API: &str = "https://endoflife.date/api/windows.json";
/// API 결과를 24시간 동안 메모리 캐시한다.
const CACHE_TTL: Duration = Duration::from_secs(86_400);

// ── 타입 ────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
struct EolCycle {
    cycle: String,
    /// date string ("2026-10-13") 또는 false (EOL 미설정)
    #[serde(default)]
    eol: serde_json::Value,
}

// ── 전역 캐시 ───────────────────────────────────────────────────

static CACHE: Mutex<Option<(Instant, Vec<EolCycle>)>> = Mutex::new(None);

fn fetch_cycles(evidence: &mut HashMap<String, String>) -> Result<Vec<EolCycle>, String> {
    {
        let guard = CACHE.lock().map_err(|e| e.to_string())?;
        if let Some((ts, ref cycles)) = *guard {
            if ts.elapsed() < CACHE_TTL {
                evidence.insert("eol_source".into(), "cache".into());
                return Ok(cycles.clone());
            }
        }
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("WinGuard/0.1 (https://github.com/winguard)")
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    let resp = client
        .get(EOL_API)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("EOL API 요청 실패: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("EOL API 응답 오류: HTTP {}", resp.status()));
    }

    let cycles: Vec<EolCycle> = resp
        .json()
        .map_err(|e| format!("EOL API 응답 파싱 실패: {}", e))?;

    {
        let mut guard = CACHE.lock().map_err(|e| e.to_string())?;
        *guard = Some((Instant::now(), cycles.clone()));
    }

    evidence.insert("eol_source".into(), "api".into());
    Ok(cycles)
}

// ── 빌드 번호 보정 ──────────────────────────────────────────────

/// 빌드 번호를 기준으로 올바른 Windows 표시 이름을 반환한다.
/// 일부 에디션에서 ProductName 이 "Windows 10 Pro" 로 남아 있는 버그 대응.
fn correct_product_name(product: &str, build: &str) -> String {
    let build_num: u32 = build.parse().unwrap_or(0);
    if build_num >= 22000 && product.to_lowercase().contains("windows 10") {
        return product.replacen("Windows 10", "Windows 11", 1);
    }
    product.to_string()
}

// ── 에디션 감지 ─────────────────────────────────────────────────

/// ProductName 에서 에디션 구분자를 반환한다: "lts" | "e" | "w"
fn detect_edition(product: &str) -> &'static str {
    let lower = product.to_lowercase();
    if lower.contains("ltsc") || lower.contains("ltsb") || lower.contains("iot") {
        "lts"
    } else if lower.contains("enterprise") || lower.contains("education") {
        "e"
    } else {
        "w" // Home, Pro, Pro Education 등
    }
}

// ── 빌드 번호 → base cycle ID ───────────────────────────────────

/// 빌드 번호로 endoflife.date base cycle ID 를 반환한다 (에디션 접미사 없음, 소문자).
/// 예) "26100" → "11-24h2"
fn detect_cycle(build: &str, product: &str, display_ver: &str) -> String {
    let from_build: Option<&str> = match build {
        "26200" => Some("11-25h2"),
        "26100" | "26120" => Some("11-24h2"),
        "22631" => Some("11-23h2"),
        "22621" => Some("11-22h2"),
        "22000" => Some("11-21h2"),
        "19045" => Some("10-22h2"),
        "19044" => Some("10-21h2"),
        "19043" => Some("10-21h1"),
        "19042" => Some("10-20h2"),
        "17763" => Some("10-1809"),
        "14393" => Some("10-1607"),
        "19041" => Some("10-2004"),
        _ => None,
    };

    if let Some(base) = from_build {
        return base.to_string();
    }

    // 알 수 없는 빌드 → 빌드 수치로 OS 메이저 버전 추정
    let build_num: u32 = build.parse().unwrap_or(0);
    let major = if build_num >= 22000 || product.to_lowercase().contains("windows 11") {
        "11"
    } else {
        "10"
    };
    format!("{}-{}", major, display_ver.to_lowercase())
}

// ── cycle 조회 ──────────────────────────────────────────────────

/// cycle 목록에서 에디션을 고려해 가장 알맞은 항목을 찾는다.
///
/// endoflife.date 는 에디션별로 cycle ID 에 접미사를 붙인다:
///   -w  (Home / Pro)
///   -e  (Enterprise / Education)
///   -e-lts / -iot-lts  (LTSC / IoT LTSC)
/// 일부 오래된 버전은 접미사 없이 단일 항목만 존재한다.
fn find_cycle<'a>(cycles: &'a [EolCycle], base: &str, edition: &str) -> Option<&'a EolCycle> {
    let lookup = |id: &str| -> Option<&'a EolCycle> {
        cycles.iter().find(|c| c.cycle.eq_ignore_ascii_case(id))
    };

    match edition {
        "lts" => lookup(&format!("{}-e-lts", base))
            .or_else(|| lookup(&format!("{}-iot-lts", base)))
            .or_else(|| lookup(&format!("{}-e", base)))
            .or_else(|| lookup(base)),
        "e" => lookup(&format!("{}-e", base))
            .or_else(|| lookup(&format!("{}-e-lts", base)))
            .or_else(|| lookup(base)),
        _ => lookup(&format!("{}-w", base)).or_else(|| lookup(base)),
    }
}

// ── 메인 ────────────────────────────────────────────────────────

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let reg_path = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let product_raw = registry::read_string(reg_path, "ProductName").unwrap_or_default();
    let display_ver = registry::read_string(reg_path, "DisplayVersion").unwrap_or_default();
    let build = registry::read_string(reg_path, "CurrentBuild").unwrap_or_default();

    let product = correct_product_name(&product_raw, &build);
    let edition = detect_edition(&product);

    evidence.insert("ProductName".into(), product_raw.clone());
    evidence.insert("ProductNameCorrected".into(), product.clone());
    evidence.insert("DisplayVersion".into(), display_ver.clone());
    evidence.insert("CurrentBuild".into(), build.clone());
    evidence.insert("Edition".into(), edition.into());

    let cycle_base = detect_cycle(&build, &product, &display_ver);
    evidence.insert("cycle_base".into(), cycle_base.clone());

    let cycles = match fetch_cycles(&mut evidence) {
        Ok(v) => v,
        Err(e) => {
            evidence.insert("api_error".into(), e);
            return api_unavailable(evidence);
        }
    };

    let cycle = match find_cycle(&cycles, &cycle_base, edition) {
        Some(c) => c,
        None => {
            evidence.insert("cycle_not_found".into(), cycle_base.clone());
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Info,
                message: format!(
                    "{} ({})은(는) EOL 데이터베이스에 등록되지 않은 버전입니다. Microsoft 수명 주기 정책을 직접 확인하세요.",
                    product, display_ver
                ),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            };
        }
    };

    evidence.insert("cycle_id".into(), cycle.cycle.clone());

    let eol_str = match cycle.eol.as_str() {
        Some(s) => s.to_string(),
        None => {
            evidence.insert("eol_value".into(), cycle.eol.to_string());
            return CheckResult {
                id: ID.into(),
                title: TITLE.into(),
                severity: Severity::Ok,
                message: format!(
                    "{} {}은(는) 지원 중입니다. (종료 일시 미정)",
                    product, display_ver
                ),
                action_uri: None,
                evidence,
                checked_at: now_ts(),
            };
        }
    };

    evidence.insert("EolDate".into(), eol_str.clone());

    let eol_date = match NaiveDate::parse_from_str(&eol_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(e) => {
            evidence.insert("date_parse_error".into(), e.to_string());
            return unknown(evidence);
        }
    };

    let today = Utc::now().date_naive();
    let days_until = (eol_date - today).num_days();

    let (severity, message) = if days_until < 0 {
        (
            Severity::Danger,
            format!(
                "{} {}은(는) {}일 전 지원이 종료되었습니다. 업그레이드하세요.",
                product, display_ver, -days_until
            ),
        )
    } else if days_until <= 180 {
        (
            Severity::Warning,
            format!(
                "{} {}은(는) {}일 후 지원이 종료됩니다.",
                product, display_ver, days_until
            ),
        )
    } else {
        (
            Severity::Ok,
            format!(
                "{} {}은(는) 지원 중입니다. (종료까지 {}일 남았습니다.)",
                product, display_ver, days_until
            ),
        )
    };

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

fn api_unavailable(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "EOL API에 연결할 수 없어 지원 종료 여부를 확인할 수 없습니다. \
                  인터넷 연결 상태를 확인하세요."
            .into(),
        action_uri: None,
        evidence,
        checked_at: now_ts(),
    }
}

fn unknown(evidence: HashMap<String, String>) -> CheckResult {
    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity: Severity::Info,
        message: "Windows 버전 정보를 확인할 수 없습니다.".into(),
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
    fn detect_cycle_win11_24h2() {
        assert_eq!(detect_cycle("26100", "Windows 11 Pro", "24H2"), "11-24h2");
    }

    #[test]
    fn detect_cycle_win10_22h2() {
        assert_eq!(detect_cycle("19045", "Windows 10 Pro", "22H2"), "10-22h2");
    }

    #[test]
    fn detect_cycle_ltsc_base_same() {
        // LTSC 에디션 구분은 detect_edition 이 담당; base 는 동일
        assert_eq!(
            detect_cycle("17763", "Windows 10 Enterprise LTSC 2019", "1809"),
            "10-1809"
        );
        assert_eq!(
            detect_cycle("17763", "Windows 10 Enterprise", "1809"),
            "10-1809"
        );
    }

    #[test]
    fn detect_edition_works() {
        assert_eq!(detect_edition("Windows 11 Pro"), "w");
        assert_eq!(detect_edition("Windows 11 Home"), "w");
        assert_eq!(detect_edition("Windows 10 Enterprise"), "e");
        assert_eq!(detect_edition("Windows 10 Enterprise LTSC 2021"), "lts");
        assert_eq!(detect_edition("Windows 10 IoT Enterprise LTSC"), "lts");
    }

    #[test]
    fn detect_cycle_unknown_build_uses_major_from_build_num() {
        let cycle = detect_cycle("99999", "Windows 11 Pro", "99H2");
        assert!(
            cycle.starts_with("11-"),
            "build 99999 은 Windows 11 로 분류"
        );
    }

    #[test]
    fn find_cycle_prefers_edition_suffix() {
        let cycles = vec![
            EolCycle {
                cycle: "11-24h2-w".into(),
                eol: serde_json::Value::String("2026-10-13".into()),
            },
            EolCycle {
                cycle: "11-24h2-e".into(),
                eol: serde_json::Value::String("2027-10-12".into()),
            },
        ];
        let w = find_cycle(&cycles, "11-24h2", "w").unwrap();
        assert_eq!(w.cycle, "11-24h2-w");
        let e = find_cycle(&cycles, "11-24h2", "e").unwrap();
        assert_eq!(e.cycle, "11-24h2-e");
    }

    #[test]
    fn find_cycle_falls_back_to_bare_cycle() {
        // 10-22h2 는 접미사 없이 단일 항목
        let cycles = vec![EolCycle {
            cycle: "10-22h2".into(),
            eol: serde_json::Value::String("2025-10-14".into()),
        }];
        let c = find_cycle(&cycles, "10-22h2", "w").unwrap();
        assert_eq!(c.cycle, "10-22h2");
    }
}
