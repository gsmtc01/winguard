// src/commands/update_commands.rs
//
// 업데이트 확인 — GitHub Releases 를 업데이트 서버로 사용한다.
//
// 앱은 아무것도 자동으로 설치하지 않는다. 최신 버전·배포일·릴리즈 노트를 받아
// 사용자에게 보여주고, 다운로드는 사용자가 직접 눌러 브라우저로 연다.
// (설치 파일을 앱이 몰래 받아 실행하는 동작은 보안 도구로서 하지 않는다)

use serde::{Deserialize, Serialize};

/// 기본 저장소. 배포처가 정해지면 이 한 줄만 바꾸면 된다.
/// 재빌드 없이 바꾸려면 `%LOCALAPPDATA%\WinGuard\update_settings.json` 에
/// `{ "repo": "owner/repo" }` 를 넣는다.
const DEFAULT_REPO: &str = "gsmtc01/winguard";

const SETTINGS_FILENAME: &str = "update_settings.json";

/// GitHub API 는 User-Agent 가 없으면 403 을 돌려준다.
const USER_AGENT: &str = "WinGuard-Updater";

const TIMEOUT_SECS: u64 = 15;

// ── 설정 ───────────────────────────────────────────────────────

fn settings_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("WinGuard")
        .join(SETTINGS_FILENAME)
}

/// 설정 파일의 `repo` 값을 읽는다. 없거나 형식이 틀리면 기본값.
fn configured_repo() -> String {
    let from_file = std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["repo"].as_str().map(str::to_string));

    match from_file {
        Some(r) if is_valid_repo(&r) => r,
        _ => DEFAULT_REPO.to_string(),
    }
}

/// `owner/repo` 형태인지 확인한다. API URL 에 그대로 들어가므로 경로 조작을 막는다.
fn is_valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let ok = |s: &str| {
        !s.is_empty()
            && s.len() <= 100
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    };
    ok(owner) && ok(name)
}

// ── 버전 비교 ──────────────────────────────────────────────────

/// "v1.2.3" / "1.2.3-beta" → [1, 2, 3]. 숫자가 아닌 꼬리표는 무시한다.
fn parse_version(v: &str) -> Vec<u32> {
    v.trim()
        .trim_start_matches(['v', 'V'])
        .split(['.', '-', '+'])
        .map(|p| {
            let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<u32>().ok()
        })
        .take_while(Option::is_some)
        .flatten()
        .collect()
}

/// latest 가 current 보다 높으면 true.
fn is_newer(latest: &str, current: &str) -> bool {
    let (l, c) = (parse_version(latest), parse_version(current));
    if l.is_empty() {
        return false;
    }
    for i in 0..l.len().max(c.len()) {
        let (a, b) = (
            l.get(i).copied().unwrap_or(0),
            c.get(i).copied().unwrap_or(0),
        );
        if a != b {
            return a > b;
        }
    }
    false
}

// ── GitHub 응답 ────────────────────────────────────────────────

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
    #[serde(default)]
    prerelease: bool,
}

// ── 프런트엔드 응답 ────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    /// 현재 실행 중인 버전
    pub current_version: String,
    /// 서버가 알려준 최신 버전 (태그에서 v 제거)
    pub latest_version: String,
    /// 최신 버전이 현재보다 높은가
    pub update_available: bool,
    /// 릴리즈 제목
    pub title: String,
    /// 릴리즈 노트(마크다운)
    pub notes: String,
    /// 배포일 (ISO8601, 없으면 빈 문자열)
    pub published_at: String,
    /// 릴리즈 페이지 URL
    pub release_url: String,
    /// 이 PC 아키텍처에 맞는 설치 파일 URL (못 찾으면 None)
    pub download_url: Option<String>,
    pub download_name: Option<String>,
    pub download_size: Option<u64>,
    /// 미리보기(prerelease) 릴리즈인지
    pub prerelease: bool,
    /// 조회한 저장소 (설정 확인용)
    pub repo: String,
}

/// 현재 아키텍처에 맞는 설치 파일을 고른다.
/// 아키텍처 키워드가 붙은 설치 파일을 우선하고, 없으면 설치 파일 아무거나.
fn pick_asset(assets: &[GhAsset]) -> Option<&GhAsset> {
    let installers: Vec<&GhAsset> = assets
        .iter()
        .filter(|a| {
            let n = a.name.to_lowercase();
            n.ends_with(".msi") || n.ends_with(".exe")
        })
        .collect();

    let keywords: &[&str] = match std::env::consts::ARCH {
        "aarch64" => &["arm64", "aarch64"],
        "x86_64" => &["x64", "x86_64", "amd64"],
        _ => &[],
    };

    installers
        .iter()
        .find(|a| {
            let n = a.name.to_lowercase();
            keywords.iter().any(|k| n.contains(k))
        })
        .or_else(|| installers.first())
        .copied()
}

// ── 커맨드 ─────────────────────────────────────────────────────

/// 현재 설정된 업데이트 저장소를 돌려준다(설정 화면 표시용).
#[tauri::command]
pub fn update_repo() -> String {
    configured_repo()
}

/// 업데이트 서버(GitHub Releases)에서 최신 릴리즈를 조회한다.
#[tauri::command]
pub async fn check_for_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.package_info().version.to_string();
    let repo = configured_repo();
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");

    let release = tauri::async_runtime::spawn_blocking({
        let url = url.clone();
        let repo = repo.clone();
        move || -> Result<GhRelease, String> {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
                .build()
                .map_err(|e| format!("HTTP 클라이언트 생성 실패: {e}"))?;

            let resp = client
                .get(&url)
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .header(reqwest::header::ACCEPT, "application/vnd.github+json")
                .send()
                .map_err(|e| {
                    format!("업데이트 서버에 연결하지 못했습니다: {e}. 네트워크 연결을 확인하세요")
                })?;

            let status = resp.status();
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(format!(
                    "업데이트 정보를 찾을 수 없습니다: {repo} 저장소에 공개된 릴리즈가 없습니다"
                ));
            }
            if status == reqwest::StatusCode::FORBIDDEN || status.as_u16() == 429 {
                return Err(
                    "업데이트 서버 요청 한도를 초과했습니다. 잠시 후 다시 시도하세요".to_string(),
                );
            }
            if !status.is_success() {
                return Err(format!(
                    "업데이트 서버 응답 오류: url={url}, status={status}"
                ));
            }

            resp.json::<GhRelease>()
                .map_err(|e| format!("업데이트 정보를 해석하지 못했습니다: {e}"))
        }
    })
    .await
    .map_err(|e| format!("스레드 오류: {e}"))??;

    let latest_version = release
        .tag_name
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string();
    let asset = pick_asset(&release.assets);

    Ok(UpdateInfo {
        update_available: is_newer(&release.tag_name, &current_version),
        current_version,
        title: release
            .name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("WinGuard {latest_version}")),
        notes: release
            .body
            .map(|b| b.trim().to_string())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "이번 릴리즈에는 등록된 변경 내용이 없습니다.".to_string()),
        published_at: release.published_at.unwrap_or_default(),
        release_url: release.html_url,
        download_url: asset.map(|a| a.browser_download_url.clone()),
        download_name: asset.map(|a| a.name.clone()),
        download_size: asset.map(|a| a.size),
        prerelease: release.prerelease,
        latest_version,
        repo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_strips_prefix_and_suffix() {
        assert_eq!(parse_version("v1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version("1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version("v0.1.0-beta.2"), vec![0, 1, 0]);
        assert_eq!(parse_version("2.0"), vec![2, 0]);
        assert!(parse_version("nightly").is_empty());
    }

    #[test]
    fn is_newer_compares_numerically() {
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("v0.1.10", "0.1.9")); // 문자열 비교였다면 실패
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        // 자리수가 달라도 부족한 자리는 0 으로 본다
        assert!(!is_newer("1.0", "1.0.0"));
        assert!(is_newer("1.0.1", "1.0"));
    }

    #[test]
    fn is_newer_rejects_unparsable_tag() {
        // 태그를 못 읽으면 업데이트가 있다고 주장하지 않는다.
        assert!(!is_newer("nightly", "0.1.0"));
    }

    #[test]
    fn repo_validation_rejects_path_traversal() {
        assert!(is_valid_repo("gsmtc01/winguard"));
        assert!(is_valid_repo("Owner-1/repo_name.js"));
        assert!(!is_valid_repo("owner"));
        assert!(!is_valid_repo("owner/repo/extra"));
        assert!(!is_valid_repo("owner//repo"));
        assert!(!is_valid_repo("../../etc/passwd"));
        assert!(!is_valid_repo("owner/repo?query=1"));
        assert!(!is_valid_repo(""));
    }

    fn asset(name: &str) -> GhAsset {
        GhAsset {
            name: name.into(),
            browser_download_url: format!("https://example.test/{name}"),
            size: 1,
        }
    }

    #[test]
    fn pick_asset_prefers_matching_arch() {
        let assets = vec![
            asset("WinGuard_0.2.0_x64_ko-KR.msi"),
            asset("WinGuard_0.2.0_arm64_ko-KR.msi"),
            asset("winguard-source.zip"),
        ];
        let picked = pick_asset(&assets).expect("설치 파일이 있다");
        let expected = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            _ => "x64",
        };
        assert!(picked.name.contains(expected));
    }

    #[test]
    fn pick_asset_ignores_non_installers() {
        let assets = vec![asset("checksums.txt"), asset("winguard-source.zip")];
        assert!(pick_asset(&assets).is_none());
    }

    #[test]
    fn pick_asset_falls_back_to_any_installer() {
        let assets = vec![asset("WinGuard-setup.msi")];
        assert!(pick_asset(&assets).is_some());
    }
}
