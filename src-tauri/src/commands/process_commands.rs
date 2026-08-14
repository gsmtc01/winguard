use crate::util::powershell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub company: Option<String>,
    /// None = 서명 확인 불가, Some(true) = 유효 서명, Some(false) = 서명 없음/무효
    pub signed: Option<bool>,
    pub cpu: f64,
    pub memory_mb: f64,
}

/// 현재 실행 중인 프로세스를 수집한다.
/// - 경로가 있는 프로세스만 포함 (시스템/커널 제외)
/// - CPU 점유율: 1초 간격 두 번 샘플링 → 델타/코어수로 실제 % 계산 (0~100 clamp)
///   ($_.CPU 는 누적 CPU 시간(초)이므로 직접 %로 사용하면 100 초과)
/// - 메모리 상위 60개로 제한 후 CPU 기준 재정렬
/// - Get-AuthenticodeSignature 로 디지털 서명 여부 확인
#[tauri::command]
pub fn scan_processes() -> Result<Vec<ProcessInfo>, String> {
    let script = r#"
$cores = [Environment]::ProcessorCount

# 1차 샘플: PID → 누적 CPU 시간 맵
$snap = @{}
Get-Process | Where-Object { $_.Path -ne $null } | ForEach-Object {
    if ($_.CPU -ne $null) { $snap[$_.Id] = [double]$_.CPU }
}

Start-Sleep -Milliseconds 1000

# 2차 샘플: 실 데이터 수집 + CPU % 계산
$procs = Get-Process | Where-Object { $_.Path -ne $null }

$result = $procs | ForEach-Object {
    $cpuPct = 0.0
    if ($snap.ContainsKey($_.Id) -and $_.CPU -ne $null) {
        $delta   = [double]$_.CPU - $snap[$_.Id]
        $cpuPct  = [math]::Round($delta / 1.0 / $cores * 100, 1)
        $cpuPct  = [math]::Max(0.0, [math]::Min(100.0, $cpuPct))
    }

    $signed = $null
    try {
        $status = (Get-AuthenticodeSignature -FilePath $_.Path -ErrorAction Stop).Status
        $signed = ($status -eq 'Valid')
    } catch {}

    [PSCustomObject]@{
        pid        = $_.Id
        name       = $_.Name
        path       = $_.Path
        company    = if ($_.Company) { $_.Company } else { $null }
        signed     = $signed
        cpu        = $cpuPct
        memory_mb  = [math]::Round($_.WorkingSet64 / 1MB, 1)
    }
}

$result | Sort-Object cpu -Descending | Select-Object -First 60 | ConvertTo-Json -Depth 2
"#;

    let output = powershell::run(script).map_err(|e| format!("프로세스 수집 실패: {e}"))?;

    if output.trim().is_empty() {
        return Ok(vec![]);
    }

    // 단일 객체일 때 배열로 감싸기
    let json = if output.trim_start().starts_with('{') {
        format!("[{}]", output)
    } else {
        output
    };

    let raw: Vec<serde_json::Value> =
        serde_json::from_str(&json).map_err(|e| format!("파싱 오류: {e}"))?;

    let procs = raw
        .into_iter()
        .filter_map(|v| {
            Some(ProcessInfo {
                pid: v["pid"].as_u64()? as u32,
                name: v["name"].as_str()?.to_string(),
                path: v["path"].as_str().map(String::from),
                company: v["company"].as_str().map(String::from),
                signed: v["signed"].as_bool(),
                cpu: v["cpu"].as_f64().unwrap_or(0.0),
                memory_mb: v["memory_mb"].as_f64().unwrap_or(0.0),
            })
        })
        .collect();

    Ok(procs)
}
