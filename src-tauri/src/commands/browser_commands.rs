use crate::util::powershell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserExtension {
    pub browser: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    /// 콤마 구분 권한 문자열 (예: "tabs, webRequest, <all_urls>")
    pub permissions: String,
    pub enabled: bool,
}

/// Chrome / Edge / Firefox 에 설치된 확장 프로그램 목록을 수집한다.
///
/// - Chrome/Edge: 사용자 프로파일 디렉터리의 manifest.json 파싱
///   - __MSG_xxx__ 형식 이름은 _locales 에서 실제 문자열로 치환
/// - Firefox: profiles 디렉터리의 extensions.json 파싱
/// - 비활성 확장도 포함 (enabled: false 로 표시)
#[tauri::command]
pub fn scan_extensions() -> Result<Vec<BrowserExtension>, String> {
    let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$results = [System.Collections.Generic.List[object]]::new()

# ── Chrome / Edge ──────────────────────────────────────────────
$browserDefs = @(
    [PSCustomObject]@{ name = "Chrome"; base = "$env:LOCALAPPDATA\Google\Chrome\User Data" },
    [PSCustomObject]@{ name = "Edge";   base = "$env:LOCALAPPDATA\Microsoft\Edge\User Data" }
)

foreach ($b in $browserDefs) {
    if (-not (Test-Path $b.base)) { continue }

    # Default + 추가 프로파일 디렉터리
    $profiles = @("Default")
    Get-ChildItem $b.base -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^Profile \d+$' } |
        ForEach-Object { $profiles += $_.Name }

    foreach ($profile in $profiles) {
        $extBase = Join-Path $b.base "$profile\Extensions"
        if (-not (Test-Path $extBase)) { continue }

        Get-ChildItem $extBase -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -ne 'Temp' } |
            ForEach-Object {
                $extId = $_.Name

                # 버전 디렉터리 중 manifest.json 이 있는 최신 버전 선택
                $manifestPath = $null
                Get-ChildItem $_.FullName -Directory -ErrorAction SilentlyContinue |
                    Sort-Object Name -Descending |
                    ForEach-Object {
                        if (-not $manifestPath) {
                            $mp = Join-Path $_.FullName "manifest.json"
                            if (Test-Path $mp) { $manifestPath = $mp }
                        }
                    }

                if (-not $manifestPath) { return }

                try {
                    $m = Get-Content $manifestPath -Raw -Encoding UTF8 -ErrorAction Stop |
                         ConvertFrom-Json

                    # __MSG_xxx__ 이름 → _locales 에서 실제 이름으로 치환
                    $extName = [string]$m.name
                    if ($extName -match '^__MSG_(.+)__$') {
                        $msgKey  = $matches[1]
                        $locBase = Split-Path $manifestPath -Parent
                        foreach ($lang in @("en", "en_US", "ko")) {
                            $msgFile = Join-Path $locBase "_locales\$lang\messages.json"
                            if (Test-Path $msgFile) {
                                try {
                                    $msgs = Get-Content $msgFile -Raw -Encoding UTF8 |
                                            ConvertFrom-Json
                                    if ($msgs.$msgKey -and $msgs.$msgKey.message) {
                                        $extName = $msgs.$msgKey.message
                                        break
                                    }
                                } catch {}
                            }
                        }
                        if ($extName -match '^__MSG_') { $extName = $extId }
                    }

                    # description 역시 __MSG_ 처리
                    $desc = if ($m.description -and -not ($m.description -match '^__MSG_')) {
                        [string]$m.description
                    } else { $null }

                    # 권한 수집 (manifest v2: permissions, v3: permissions + host_permissions)
                    $perms = [System.Collections.Generic.List[string]]::new()
                    if ($m.permissions) {
                        $m.permissions | Where-Object { $_ -is [string] } |
                            ForEach-Object { $perms.Add($_) }
                    }
                    if ($m.host_permissions) {
                        $m.host_permissions | Where-Object { $_ -is [string] } |
                            ForEach-Object { $perms.Add($_) }
                    }

                    $results.Add([PSCustomObject]@{
                        browser     = $b.name
                        id          = $extId
                        name        = $extName
                        version     = if ($m.version) { [string]$m.version } else { "" }
                        description = $desc
                        permissions = ($perms | Select-Object -Unique) -join ", "
                        enabled     = $true
                    })
                } catch {}
            }
    }
}

# ── Firefox ────────────────────────────────────────────────────
$ffBase = "$env:APPDATA\Mozilla\Firefox\Profiles"
if (Test-Path $ffBase) {
    Get-ChildItem $ffBase -Directory -ErrorAction SilentlyContinue | ForEach-Object {
        $extJson = Join-Path $_.FullName "extensions.json"
        if (-not (Test-Path $extJson)) { return }
        try {
            $data = Get-Content $extJson -Raw -Encoding UTF8 | ConvertFrom-Json
            $data.addons |
                Where-Object { $_.type -eq 'extension' -and $_.id -notmatch '^@' -or $_.id -match '^{' } |
                ForEach-Object {
                    $addon = $_
                    $perms = [System.Collections.Generic.List[string]]::new()
                    if ($addon.userPermissions -and $addon.userPermissions.permissions) {
                        $addon.userPermissions.permissions | ForEach-Object { $perms.Add([string]$_) }
                    }
                    if ($addon.userPermissions -and $addon.userPermissions.origins) {
                        $addon.userPermissions.origins | ForEach-Object { $perms.Add([string]$_) }
                    }

                    $ffName = if ($addon.defaultLocale -and $addon.defaultLocale.name) {
                        [string]$addon.defaultLocale.name
                    } elseif ($addon.id) { [string]$addon.id } else { "Unknown" }

                    $results.Add([PSCustomObject]@{
                        browser     = "Firefox"
                        id          = [string]$addon.id
                        name        = $ffName
                        version     = if ($addon.version) { [string]$addon.version } else { "" }
                        description = if ($addon.defaultLocale -and $addon.defaultLocale.description) {
                                          [string]$addon.defaultLocale.description
                                      } else { $null }
                        permissions = ($perms | Select-Object -Unique) -join ", "
                        enabled     = [bool]$addon.active
                    })
                }
        } catch {}
    }
}

if ($results.Count -eq 0) {
    Write-Output "[]"
} else {
    $results | ConvertTo-Json -Depth 2
}
"#;

    let output = powershell::run(script).map_err(|e| format!("확장 프로그램 스캔 실패: {e}"))?;

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

    let exts = raw
        .into_iter()
        .filter_map(|v| {
            let name = v["name"].as_str()?.to_string();
            if name.is_empty() {
                return None;
            }
            Some(BrowserExtension {
                browser: v["browser"].as_str().unwrap_or("Unknown").to_string(),
                id: v["id"].as_str().unwrap_or("").to_string(),
                name,
                version: v["version"].as_str().unwrap_or("").to_string(),
                description: v["description"].as_str().map(String::from),
                permissions: v["permissions"].as_str().unwrap_or("").to_string(),
                enabled: v["enabled"].as_bool().unwrap_or(true),
            })
        })
        .collect();

    Ok(exts)
}
