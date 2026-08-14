use crate::util::powershell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConn {
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
    pub state: String,
    pub pid: u32,
    pub process_name: Option<String>,
}

/// 현재 TCP 연결 목록을 수집한다.
/// - ESTABLISHED 연결 + LISTEN 포트 포함
/// - 최대 100개 (루프백 제외)
/// - 각 연결의 소유 프로세스 이름 포함
#[tauri::command]
pub fn scan_network() -> Result<Vec<NetworkConn>, String> {
    let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$conns = Get-NetTCPConnection |
    Where-Object { $_.RemoteAddress -ne '0.0.0.0' -and $_.RemoteAddress -ne '::' } |
    Sort-Object State -Descending |
    Select-Object -First 100

$result = $conns | ForEach-Object {
    $procName = $null
    try {
        $proc = Get-Process -Id $_.OwningProcess -ErrorAction Stop
        $procName = $proc.Name
    } catch {}

    [PSCustomObject]@{
        local_address  = $_.LocalAddress
        local_port     = [int]$_.LocalPort
        remote_address = $_.RemoteAddress
        remote_port    = [int]$_.RemotePort
        state          = $_.State.ToString()
        pid            = [int]$_.OwningProcess
        process_name   = $procName
    }
}
$result | ConvertTo-Json -Depth 2
"#;

    let output = powershell::run(script).map_err(|e| format!("네트워크 수집 실패: {e}"))?;

    if output.trim().is_empty() {
        return Ok(vec![]);
    }

    let json = if output.trim_start().starts_with('{') {
        format!("[{}]", output)
    } else {
        output
    };

    let raw: Vec<serde_json::Value> =
        serde_json::from_str(&json).map_err(|e| format!("파싱 오류: {e}"))?;

    let conns = raw
        .into_iter()
        .filter_map(|v| {
            Some(NetworkConn {
                local_address: v["local_address"].as_str()?.to_string(),
                local_port: v["local_port"].as_u64()? as u16,
                remote_address: v["remote_address"].as_str()?.to_string(),
                remote_port: v["remote_port"].as_u64()? as u16,
                state: v["state"].as_str().unwrap_or("Unknown").to_string(),
                pid: v["pid"].as_u64()? as u32,
                process_name: v["process_name"].as_str().map(String::from),
            })
        })
        .collect();

    Ok(conns)
}
