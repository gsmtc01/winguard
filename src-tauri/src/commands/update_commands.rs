// src/commands/update_commands.rs
//
// 업데이트 확인 — GitHub Releases 를 업데이트 서버로 사용한다.
//
// 앱은 아무것도 자동으로 설치하지 않는다. 최신 버전·배포일·릴리즈 노트를 받아
// 사용자에게 보여준다.
//
// 포터블 실행 파일로 배포하면서 앱 안에서 교체하는 경로(update_download /
// update_apply)를 추가했다. "몰래 받아 실행하지 않는다" 는 원칙은 그대로다:
//   - 자동으로 받거나 적용하지 않는다. 각 단계를 사용자가 눌러야 진행된다.
//   - 다운로드 주소는 GitHub 호스트로 제한한다.
//   - SHA-256 이 일치할 때만 교체하고, 불일치하면 받은 파일을 지운다.
// 브라우저로 직접 받아 수동으로 바꾸는 기존 방법도 그대로 쓸 수 있다.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use tauri::Emitter;

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
    /// 릴리즈에 `<자산명>.sha256` 이 함께 올라와 있으면 그 내용(16진 해시).
    /// 앱 내 업데이트는 이 값이 일치할 때만 실행 파일을 교체한다.
    pub download_sha256: Option<String>,
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

    // 같은 이름에 .sha256 이 붙은 자산이 있으면 내용을 받아둔다.
    // 앱 내 업데이트가 교체 전에 이 값과 대조한다. 없으면 None 이고,
    // 그때는 앱 내 업데이트 대신 브라우저 다운로드만 안내한다.
    let sha_asset_url = asset.and_then(|a| {
        let want = format!("{}.sha256", a.name.to_lowercase());
        release
            .assets
            .iter()
            .find(|x| x.name.to_lowercase() == want)
            .map(|x| x.browser_download_url.clone())
    });
    let download_sha256 = match sha_asset_url {
        Some(u) => fetch_sha256_text(&u).await.ok(),
        None => None,
    };

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
        download_sha256,
        prerelease: release.prerelease,
        latest_version,
        repo,
    })
}

// ── 자체 업데이트 ──────────────────────────────────────────────
//
// 포터블 실행 파일이라 설치 관리자가 없다. Windows 는 실행 중인 exe 를
// 덮어쓸 수 없지만 이름 변경은 허용하므로, 현재 exe 를 옆으로 치우고 새
// 파일을 원래 자리에 놓는 방식으로 교체한다.
//
// 이 파일 첫머리의 원칙("몰래 받아 실행하지 않는다")을 지키기 위해:
//   - 자동으로 받거나 적용하지 않는다. 사용자가 각 단계를 눌러야 진행된다.
//   - 다운로드 URL 은 GitHub 호스트로 제한한다.
//   - SHA-256 이 일치할 때만 교체한다. 불일치면 받은 파일을 지운다.

/// 실행 파일을 받아올 수 있는 호스트. 임의 URL 로 exe 를 받지 않도록 막는다.
/// (리다이렉트는 reqwest 가 따라가며, GitHub 이 자산 저장소로 넘긴다)
const ALLOWED_DOWNLOAD_HOSTS: &[&str] = &["github.com", "www.github.com"];

/// `.sha256` 자산의 내용을 받아 16진 해시만 뽑는다.
/// `<hash>  <filename>` 형식(sha256sum 출력)도 허용한다.
async fn fetch_sha256_text(url: &str) -> Result<String, String> {
    if !is_allowed_download_url(url) {
        return Err("허용되지 않은 주소".to_string());
    }
    let url = url.to_string();
    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let text = resp.text().map_err(|e| e.to_string())?;
        let hex = text.split_whitespace().next().unwrap_or_default();
        if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(hex.to_ascii_lowercase())
        } else {
            Err("sha256 형식이 아닙니다".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 받은 파일을 둘 폴더. 교체 전까지 여기 머문다.
fn update_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("WinGuard")
        .join("update")
}

#[derive(Serialize, Clone)]
struct UpdateProgress {
    downloaded: u64,
    total: u64,
    percent: u8,
}

/// https 이고 허용된 호스트인지 확인한다.
fn is_allowed_download_url(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "https" {
        return false;
    }
    parsed
        .host_str()
        .is_some_and(|h| ALLOWED_DOWNLOAD_HOSTS.contains(&h))
}

/// 경로 조작을 막는다. 파일명에 구분자나 상위 참조가 있으면 거부한다.
fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.contains(['/', '\\', ':'])
        && name != "."
        && name != ".."
        && name.to_ascii_lowercase().ends_with(".exe")
}

fn sha256_hex(path: &std::path::Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).map_err(|e| format!("파일 열기 실패: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("읽기 실패: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// 새 실행 파일을 내려받아 임시 폴더에 저장하고 그 경로를 돌려준다.
///
/// 여기서는 아무것도 교체하지 않는다. 교체는 `update_apply` 가 한다.
/// `expected_sha256` 을 주면 일치할 때만 성공한다(릴리즈의 .sha256 자산 값).
#[tauri::command]
pub async fn update_download(
    app: tauri::AppHandle,
    url: String,
    file_name: String,
    expected_sha256: Option<String>,
) -> Result<String, String> {
    if !is_allowed_download_url(&url) {
        return Err(format!(
            "허용되지 않은 다운로드 주소입니다: {url}. GitHub 릴리즈 자산만 받을 수 있습니다"
        ));
    }
    if !is_safe_file_name(&file_name) {
        return Err(format!("올바르지 않은 파일명입니다: {file_name}"));
    }

    let dir = update_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("임시 폴더를 만들 수 없습니다: {} ({e})", dir.display()))?;

    let dest = dir.join(&file_name);
    let tmp = dir.join(format!("{file_name}.part"));

    let app_c = app.clone();
    let dest_c = dest.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(1800))
            .build()
            .map_err(|e| e.to_string())?;

        let mut response = client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .map_err(|e| format!("다운로드 요청 실패: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("서버 응답 오류: HTTP {}", response.status()));
        }

        let total = response.content_length().unwrap_or(0);
        let mut file = std::fs::File::create(&tmp).map_err(|e| format!("파일 생성 실패: {e}"))?;

        let mut downloaded: u64 = 0;
        let mut buf = vec![0u8; 64 * 1024];

        let result = (|| -> Result<(), String> {
            loop {
                let n = response.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                downloaded += n as u64;
                let percent = if total > 0 {
                    ((downloaded as f64 / total as f64) * 100.0) as u8
                } else {
                    0
                };
                let _ = app_c.emit(
                    "update:download-progress",
                    UpdateProgress {
                        downloaded,
                        total,
                        percent,
                    },
                );
            }
            if total > 0 && downloaded != total {
                return Err(format!(
                    "다운로드가 중간에 끊겼습니다: {downloaded}/{total} bytes"
                ));
            }
            Ok(())
        })();

        drop(file);

        if let Err(e) = result {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }

        // 무결성 확인이 끝나기 전에는 최종 이름을 주지 않는다.
        if let Some(expected) = expected_sha256 {
            let actual = sha256_hex(&tmp)?;
            if !actual.eq_ignore_ascii_case(expected.trim()) {
                let _ = std::fs::remove_file(&tmp);
                return Err(format!(
                    "받은 파일의 해시가 일치하지 않습니다. 기대값 {expected}, 실제 {actual}. \
                     파일이 손상되었거나 변조되었을 수 있어 삭제했습니다"
                ));
            }
        }

        std::fs::rename(&tmp, &dest_c).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("파일 정리 실패: {e}")
        })
    })
    .await
    .map_err(|e| format!("스레드 오류: {e}"))??;

    Ok(dest.to_string_lossy().to_string())
}

/// 받아둔 파일로 현재 실행 파일을 교체하고 앱을 재시작한다.
///
/// Windows 는 실행 중인 exe 를 덮어쓸 수 없지만 이름 변경은 허용한다.
/// ① 현재 exe → `<이름>.old` ② 새 파일 → 원래 경로 ③ 새 exe 실행 ④ 현재 종료.
/// ②에서 실패하면 ①을 되돌린다.
#[tauri::command]
pub fn update_apply(app: tauri::AppHandle, downloaded_path: String) -> Result<(), String> {
    let new_exe = std::path::PathBuf::from(&downloaded_path);
    if !new_exe.is_file() {
        return Err(format!("받아둔 파일이 없습니다: {downloaded_path}"));
    }
    // 반드시 우리가 내려받아 둔 폴더 안이어야 한다.
    if new_exe.parent() != Some(update_dir().as_path()) {
        return Err("업데이트 폴더 밖의 파일로는 교체할 수 없습니다".to_string());
    }

    let current = std::env::current_exe().map_err(|e| format!("현재 실행 경로 확인 실패: {e}"))?;
    let backup = current.with_file_name(format!(
        "{}.old",
        current
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "winguard.exe".to_string())
    ));

    let _ = std::fs::remove_file(&backup);

    std::fs::rename(&current, &backup).map_err(|e| {
        format!("현재 실행 파일을 옮기지 못했습니다: {e}. 다른 곳에 설치되어 있는지 확인하세요")
    })?;

    // 다른 볼륨이면 rename 이 실패하므로 복사로 대체한다.
    let placed = std::fs::rename(&new_exe, &current)
        .or_else(|_| std::fs::copy(&new_exe, &current).map(|_| ()));

    if let Err(e) = placed {
        // 롤백: 원래 실행 파일을 제자리에 돌려놓는다.
        let _ = std::fs::rename(&backup, &current);
        return Err(format!(
            "새 실행 파일을 놓지 못했습니다: {e}. 업데이트를 취소했습니다"
        ));
    }

    std::process::Command::new(&current)
        .spawn()
        .map_err(|e| format!("새 버전을 시작하지 못했습니다: {e}"))?;

    app.exit(0);
    Ok(())
}

/// 이전 업데이트가 남긴 `<이름>.old` 를 지운다. 앱 시작 시 한 번 호출한다.
/// 교체 직후에는 이전 파일이 아직 잠겨 있을 수 있으므로 실패는 무시한다.
pub fn cleanup_previous_update() {
    let Ok(current) = std::env::current_exe() else {
        return;
    };
    let Some(name) = current.file_name().map(|s| s.to_string_lossy().to_string()) else {
        return;
    };
    let _ = std::fs::remove_file(current.with_file_name(format!("{name}.old")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_url_allows_only_https_github() {
        assert!(is_allowed_download_url(
            "https://github.com/gsmtc01/winguard/releases/download/v0.1.0/WinGuard-x64.exe"
        ));
        // http 는 중간자 공격에 노출되므로 거부한다.
        assert!(!is_allowed_download_url(
            "http://github.com/a/b/releases/download/v1/x.exe"
        ));
        // 남의 호스트에서 실행 파일을 받아오지 않는다.
        assert!(!is_allowed_download_url("https://evil.example.com/x.exe"));
        // github.com 을 접두사로만 흉내낸 도메인도 거부한다.
        assert!(!is_allowed_download_url("https://github.com.evil.io/x.exe"));
        assert!(!is_allowed_download_url("not a url"));
    }

    #[test]
    fn file_name_rejects_path_traversal() {
        assert!(is_safe_file_name("WinGuard-x64.exe"));
        assert!(!is_safe_file_name("..\\..\\Windows\\System32\\evil.exe"));
        assert!(!is_safe_file_name("sub/dir/x.exe"));
        assert!(!is_safe_file_name("C:x.exe"));
        assert!(!is_safe_file_name(".."));
        assert!(!is_safe_file_name(""));
        // exe 가 아니면 교체 대상이 될 수 없다.
        assert!(!is_safe_file_name("payload.dll"));
    }

    #[test]
    fn sha256_matches_known_vector() {
        use std::io::Write as _;
        let dir = std::env::temp_dir().join("winguard_sha_test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("abc.bin");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(b"abc").unwrap();
        drop(f);
        // NIST 표준 벡터: SHA-256("abc")
        assert_eq!(
            sha256_hex(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_file(&p);
    }

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
