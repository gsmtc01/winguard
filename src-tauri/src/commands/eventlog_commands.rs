use crate::util::powershell;
use serde::{Deserialize, Serialize};

/// 보안 이벤트 ID → 한국어 설명
pub fn event_id_label(id: u32) -> &'static str {
    match id {
        4625 => "로그인 실패",
        4648 => "명시적 자격증명 로그온",
        4697 => "서비스 설치",
        4698 => "예약 작업 생성",
        4720 => "계정 생성",
        4732 => "보안 그룹 구성원 추가",
        4740 => "계정 잠금",
        7045 => "새 서비스 설치",
        _ => "기타",
    }
}

/// 보안 이벤트 ID → 위험도
pub fn event_id_level(id: u32) -> &'static str {
    match id {
        4697 | 4698 | 4720 | 7045 => "고위험",
        4625 | 4648 | 4732 | 4740 => "주의",
        _ => "정보",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLogEntry {
    pub event_id: u32,
    pub time_created: String,
    pub level: String, // "고위험" | "주의" | "정보"
    pub label: String, // 한국어 이벤트 설명
    pub source: String,
    pub message: String,
}

/// 보안 관련 Windows 이벤트 로그를 최근 24시간 기준으로 수집한다.
///
/// 대상 이벤트:
/// - Security: 4625(로그인 실패), 4648(명시적 자격증명), 4697(서비스 설치),
///             4698(예약 작업 생성), 4720(계정 생성), 4732(그룹 추가), 4740(계정 잠금)
/// - System:   7045(새 서비스 설치)
///
/// 보안 로그 읽기에는 관리자 권한이 필요할 수 있다.
#[tauri::command]
pub fn scan_eventlog() -> Result<Vec<EventLogEntry>, String> {
    let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$cutoff = (Get-Date).AddHours(-24)
$results = [System.Collections.Generic.List[object]]::new()

function Get-ShortMsg($ev) {
    try {
        $msg = $ev.Message
        if (-not $msg) { return "" }
        $lines = ($msg -split "`n") | Where-Object { $_.Trim() -ne "" }
        $head = ($lines | Select-Object -First 3) -join " "
        if ($head.Length -gt 200) { $head = $head.Substring(0,200) + "..." }
        return $head.Trim()
    } catch { return "" }
}

function Add-Events($logName, $ids, $maxCount) {
    try {
        $evts = Get-WinEvent -FilterHashtable @{
            LogName   = $logName
            Id        = $ids
            StartTime = $cutoff
        } -MaxEvents $maxCount -ErrorAction Stop
        foreach ($e in $evts) {
            $script:results.Add([PSCustomObject]@{
                event_id     = [int]$e.Id
                time_created = $e.TimeCreated.ToString("yyyy-MM-dd HH:mm:ss")
                source       = [string]$e.ProviderName
                message      = Get-ShortMsg $e
            })
        }
    } catch {}
}

Add-Events 'Security' @(4625, 4648, 4697, 4698, 4720, 4732, 4740) 120
Add-Events 'System'   @(7045) 30

if ($results.Count -eq 0) {
    Write-Output "[]"
} else {
    $results | Sort-Object time_created -Descending | Select-Object -First 150 | ConvertTo-Json -Depth 2
}
"#;

    let output = powershell::run(script).map_err(|e| format!("이벤트 로그 스캔 실패: {e}"))?;

    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(vec![]);
    }

    let json = if trimmed.starts_with('{') {
        format!("[{}]", trimmed)
    } else {
        trimmed.to_string()
    };

    let raw: Vec<serde_json::Value> =
        serde_json::from_str(&json).map_err(|e| format!("파싱 오류: {e}"))?;

    let entries = raw
        .into_iter()
        .filter_map(|v| {
            let id = v["event_id"].as_u64()? as u32;
            Some(EventLogEntry {
                event_id: id,
                time_created: v["time_created"].as_str().unwrap_or("").to_string(),
                level: event_id_level(id).to_string(),
                label: event_id_label(id).to_string(),
                source: v["source"].as_str().unwrap_or("").to_string(),
                message: v["message"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect();

    Ok(entries)
}
