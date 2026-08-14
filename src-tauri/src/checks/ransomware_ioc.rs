use crate::engine::types::{now_ts, CheckResult, Severity};
use crate::util::powershell;
use std::collections::HashMap;

const ID: &str = "ransomware_ioc";
const TITLE: &str = "랜섬웨어 침해 흔적(IOC)";

const SUSPICIOUS_EXTENSIONS: &[&str] = &[
    ".vbs", ".vbe", ".js", ".jse", ".wsh", ".wsf", ".ps1", ".bat", ".cmd", ".hta",
];

const SUSPICIOUS_PATHS: &[&str] = &[
    "\\temp\\",
    "\\tmp\\",
    "\\appdata\\local\\temp\\",
    "\\users\\public\\",
    "\\programdata\\",
];

/// 정상 복구 소프트웨어 등 허용 목록. 이 접두사를 포함하면 Run 키 점검에서 제외한다.
const SAFE_RUN_PATHS: &[&str] = &["c:\\program files\\", "c:\\program files (x86)\\"];

/// 파일 확장자 연결 프로그램이 신뢰 위치로 간주되는 경로 토큰.
const TRUSTED_HANDLER_TOKENS: &[&str] = &["system32", "program files", "program files (x86)"];

fn is_suspicious_extension(value: &str) -> bool {
    let lower = value.to_lowercase();
    SUSPICIOUS_EXTENSIONS.iter().any(|ext| lower.contains(ext))
}

fn is_suspicious_path(value: &str) -> bool {
    let lower = value.to_lowercase();
    SUSPICIOUS_PATHS.iter().any(|p| lower.contains(p))
}

fn is_safe_run_entry(value: &str) -> bool {
    let lower = value.to_lowercase();
    SAFE_RUN_PATHS.iter().any(|p| lower.contains(p))
}

fn is_trusted_handler(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    TRUSTED_HANDLER_TOKENS.iter().any(|t| lower.contains(t))
}

fn clean_json(raw: &str) -> String {
    raw.replace("\\u0000", "")
        .replace('\0', "")
        .trim()
        .to_string()
}

// ── 하위 점검 A: 볼륨 섀도 복사본 ──────────────────────────────

const SHADOW_COPY_SCRIPT: &str =
    "(Get-WmiObject Win32_ShadowCopy -ErrorAction SilentlyContinue | Measure-Object).Count";

/// 반환: (점수, 근거 메시지). 조회 실패 시 0점(안전 측 판단) + 근거 없음.
fn check_shadow_copies() -> (u8, Option<String>) {
    let raw = match powershell::run(SHADOW_COPY_SCRIPT) {
        Ok(o) => o,
        Err(_) => return (0, None),
    };
    match clean_json(&raw).parse::<u64>() {
        Ok(0) => (
            2,
            Some(
                "시스템 보호(볼륨 섀도 복사본)가 꺼져 있습니다 (복구 방지 목적 삭제 가능성)".into(),
            ),
        ),
        _ => (0, None),
    }
}

// ── 하위 점검 B: 시작 프로그램 의심 스크립트 등록 ──────────────

const RUN_KEYS_SCRIPT: &str = "$paths = @(\
     'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run', \
     'HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run', \
     'HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce' \
 ); \
 $results = @(); \
 foreach ($p in $paths) { \
     if (Test-Path $p) { \
         $key = Get-ItemProperty -Path $p -ErrorAction SilentlyContinue; \
         if ($key) { \
             $key.PSObject.Properties | \
                 Where-Object { $_.MemberType -eq 'NoteProperty' -and $_.Name -notlike 'PS*' } | \
                 ForEach-Object { $results += $_.Value } \
         } \
     } \
 }; \
 $results | ConvertTo-Json -Compress";

fn parse_run_values(raw: &str) -> Vec<String> {
    let cleaned = clean_json(raw);
    if cleaned.is_empty() {
        return Vec::new();
    }
    let value: serde_json::Value = match serde_json::from_str(&cleaned) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    match value {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        serde_json::Value::String(s) => vec![s],
        _ => Vec::new(),
    }
}

/// 반환: (총점, 위반 항목 메시지 목록)
fn check_run_keys() -> (u8, Vec<String>) {
    let raw = match powershell::run(RUN_KEYS_SCRIPT) {
        Ok(o) => o,
        Err(_) => return (0, Vec::new()),
    };
    let values = parse_run_values(&raw);

    let mut score: u32 = 0;
    let mut violations = Vec::new();

    for v in &values {
        if is_safe_run_entry(v) {
            continue;
        }
        let ext = is_suspicious_extension(v);
        let path = is_suspicious_path(v);
        let points = match (ext, path) {
            (true, true) => 3,
            (true, false) => 2,
            (false, true) => 1,
            (false, false) => 0,
        };
        if points > 0 {
            score += points;
            violations.push(format!("시작 프로그램 의심 항목: {}", v));
        }
    }

    (score.min(u8::MAX as u32) as u8, violations)
}

// ── 하위 점검 C: 주요 파일 확장자 연결 프로그램 변조 ───────────

const FILE_ASSOC_SCRIPT: &str = "$exts = @('.txt', '.docx', '.xlsx', '.pdf', '.jpg'); \
 $results = @(); \
 foreach ($ext in $exts) { \
     $prog = (Get-ItemProperty -Path \"Registry::HKEY_CLASSES_ROOT\\$ext\" -Name '(Default)' -ErrorAction SilentlyContinue).'(Default)'; \
     if ($prog) { \
         $cmd = (Get-ItemProperty -Path \"Registry::HKEY_CLASSES_ROOT\\$prog\\shell\\open\\command\" -Name '(Default)' -ErrorAction SilentlyContinue).'(Default)'; \
         if ($cmd) { \
             $results += [PSCustomObject]@{ ext = $ext; cmd = $cmd } \
         } \
     } \
 }; \
 $results | ConvertTo-Json -Compress";

fn parse_file_assoc(raw: &str) -> Vec<(String, String)> {
    let cleaned = clean_json(raw);
    if cleaned.is_empty() {
        return Vec::new();
    }
    let value: serde_json::Value = match serde_json::from_str(&cleaned) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr: Vec<serde_json::Value> = match value {
        serde_json::Value::Array(a) => a,
        other => vec![other],
    };
    arr.iter()
        .filter_map(|item| {
            let ext = item.get("ext")?.as_str()?.to_string();
            let cmd = item.get("cmd")?.as_str()?.to_string();
            Some((ext, cmd))
        })
        .collect()
}

/// 반환: (점수, 위반 항목 메시지 목록)
fn check_file_associations() -> (u8, Vec<String>) {
    let raw = match powershell::run(FILE_ASSOC_SCRIPT) {
        Ok(o) => o,
        Err(_) => return (0, Vec::new()),
    };
    let entries = parse_file_assoc(&raw);

    let suspicious: Vec<String> = entries
        .iter()
        .filter(|(_, cmd)| !is_trusted_handler(cmd))
        .map(|(ext, cmd)| format!("{} 연결 프로그램이 비표준 위치입니다: {}", ext, cmd))
        .collect();

    let score = if suspicious.len() >= 2 {
        4
    } else if suspicious.len() == 1 {
        2
    } else {
        0
    };

    (score, suspicious)
}

// ── 최종 Severity 산정 (§2-6) ──────────────────────────────────

fn severity_from_score(score: u8) -> Severity {
    match score {
        0 => Severity::Ok,
        1..=2 => Severity::Warning,
        3..=4 => Severity::Danger,
        _ => Severity::Critical,
    }
}

pub fn run() -> CheckResult {
    let mut evidence: HashMap<String, String> = HashMap::new();

    let (score_a, note_a) = check_shadow_copies();
    let (score_b, violations_b) = check_run_keys();
    let (score_c, violations_c) = check_file_associations();

    let total = score_a.saturating_add(score_b).saturating_add(score_c);

    let mut violations: Vec<String> = Vec::new();
    if let Some(n) = &note_a {
        violations.push(n.clone());
    }
    violations.extend(violations_b);
    violations.extend(violations_c);

    evidence.insert("score_shadow_copy".into(), score_a.to_string());
    evidence.insert("score_run_keys".into(), score_b.to_string());
    evidence.insert("score_file_assoc".into(), score_c.to_string());
    evidence.insert("total_score".into(), total.to_string());

    let severity = severity_from_score(total);
    let message = if violations.is_empty() {
        "랜섬웨어 침해 흔적(IOC)이 감지되지 않았습니다.".to_string()
    } else if severity == Severity::Warning && score_a > 0 && violations.len() == 1 {
        // 섀도 복사본 단독 위반: 안내성 메시지로 완화 (§2-9)
        "시스템 복원 지점(섀도 복사본)이 없습니다. 복원 지점을 생성하십시오.".to_string()
    } else {
        format!(
            "랜섬웨어 침해 흔적 {}건 감지 (위험도 점수 {}): {}",
            violations.len(),
            total,
            violations.join(" | ")
        )
    };

    CheckResult {
        id: ID.into(),
        title: TITLE.into(),
        severity,
        message,
        // 3개 하위 점검 중 사용자가 원클릭으로 조치할 수 있는 유일한 항목(시스템 보호)을 가리킨다.
        // Run 키/파일 연결 변조는 자동 수정이 위험해 별도 action_uri를 두지 않는다.
        action_uri: Some("cmd:system-protection".into()),
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
    fn score_zero_returns_ok() {
        assert_eq!(severity_from_score(0), Severity::Ok);
    }

    #[test]
    fn score_above_threshold_returns_correct_severity() {
        assert_eq!(severity_from_score(1), Severity::Warning);
        assert_eq!(severity_from_score(2), Severity::Warning);
        assert_eq!(severity_from_score(3), Severity::Danger);
        assert_eq!(severity_from_score(4), Severity::Danger);
        assert_eq!(severity_from_score(5), Severity::Critical);
        assert_eq!(severity_from_score(10), Severity::Critical);
    }

    #[test]
    fn suspicious_extension_detection_works() {
        assert!(is_suspicious_extension(
            "C:\\Users\\user\\AppData\\Local\\Temp\\run.vbs"
        ));
        assert!(!is_suspicious_extension("C:\\Program Files\\App\\app.exe"));
    }

    #[test]
    fn suspicious_path_detection_works() {
        assert!(is_suspicious_path("C:\\Users\\Public\\payload.exe"));
        assert!(is_suspicious_path("%APPDATA%\\Local\\Temp\\x.exe"));
        assert!(!is_suspicious_path("C:\\Program Files\\App\\app.exe"));
    }

    #[test]
    fn safe_run_entry_is_allowlisted() {
        assert!(is_safe_run_entry(
            "\"C:\\Program Files\\Backup\\backup.exe\" --silent"
        ));
        assert!(!is_safe_run_entry("C:\\Users\\Public\\evil.vbs"));
    }

    #[test]
    fn trusted_handler_detection_works() {
        assert!(is_trusted_handler(
            "\"C:\\Windows\\System32\\notepad.exe\" %1"
        ));
        assert!(is_trusted_handler(
            "\"C:\\Program Files\\Adobe\\Reader.exe\" %1"
        ));
        assert!(!is_trusted_handler("\"C:\\Users\\Public\\evil.exe\" %1"));
    }

    #[test]
    fn parse_run_values_handles_array_string_and_empty() {
        assert_eq!(parse_run_values("").len(), 0);
        assert_eq!(parse_run_values("\"single.exe\"").len(), 1);
        assert_eq!(parse_run_values("[\"a.exe\",\"b.exe\"]").len(), 2);
    }

    #[test]
    fn parse_file_assoc_handles_single_object_and_array() {
        let single = r#"{"ext":".txt","cmd":"C:\\Windows\\System32\\notepad.exe"}"#;
        assert_eq!(parse_file_assoc(single).len(), 1);
        let array = r#"[{"ext":".txt","cmd":"a"},{"ext":".pdf","cmd":"b"}]"#;
        assert_eq!(parse_file_assoc(array).len(), 2);
    }
}
