// src-tauri/src/commands/device_commands.rs
//
// 카메라·마이크 접근 권한 제어 (레지스트리 HKCU 기반, 관리자 불필요)
//
// 레지스트리 경로:
//   HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\
//          CapabilityAccessManager\ConsentStore\webcam
//   HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\
//          CapabilityAccessManager\ConsentStore\microphone
//
// Value 값: "Allow" (허용) | "Deny" (차단)
// 하위 키:
//   NonPackaged\<경로_#구분>   → Win32 앱
//   <PackageFamilyName>        → UWP 앱

use serde::{Deserialize, Serialize};
use winreg::enums::*;
use winreg::RegKey;

const CAM_KEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\webcam";
const MIC_KEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";

// ── 데이터 구조 ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppAccess {
    /// 표시용 이름 (Win32: 실행 파일명, UWP: 패키지명)
    pub name: String,
    /// 레지스트리 하위 키 경로 (set_app_*_access 호출 시 사용)
    pub key: String,
    /// 실행 파일 전체 경로 (Win32 전용, UWP는 빈 문자열)
    pub path: String,
    /// true = Allow, false = Deny
    pub allowed: bool,
    /// UWP 앱 여부
    pub is_uwp: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceStatus {
    pub camera_allowed: bool,
    pub mic_allowed: bool,
    pub camera_apps: Vec<AppAccess>,
    pub mic_apps: Vec<AppAccess>,
}

// ── 내부 헬퍼 ────────────────────────────────────────────────────

fn is_allow(key: &RegKey) -> bool {
    key.get_value::<String, _>("Value")
        .map(|v| v == "Allow")
        .unwrap_or(true) // 값 없음 → 기본 허용
}

fn read_consent_key(subkey: &str) -> Result<(bool, Vec<AppAccess>), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(subkey)
        .map_err(|e| format!("레지스트리 열기 실패 ({subkey}): {e}"))?;

    let global_allowed = is_allow(&key);
    let mut apps: Vec<AppAccess> = Vec::new();

    for subkey_name in key.enum_keys().flatten() {
        if subkey_name.eq_ignore_ascii_case("NonPackaged") {
            // ── Win32 앱 ──────────────────────────────────────────
            if let Ok(np_key) = key.open_subkey(&subkey_name) {
                for app_key_name in np_key.enum_keys().flatten() {
                    if let Ok(app_key) = np_key.open_subkey(&app_key_name) {
                        let allowed = is_allow(&app_key);
                        // Win32 키 이름의 '/' 는 경로 구분자 '#' 로 인코딩됨
                        let path = app_key_name.replace('#', "\\");
                        let name = path
                            .split('\\')
                            .next_back()
                            .unwrap_or(&app_key_name)
                            .to_string();
                        apps.push(AppAccess {
                            name,
                            key: format!("NonPackaged\\{app_key_name}"),
                            path,
                            allowed,
                            is_uwp: false,
                        });
                    }
                }
            }
        } else {
            // ── UWP 앱 (패키지 패밀리 이름) ─────────────────────
            if let Ok(app_key) = key.open_subkey(&subkey_name) {
                let allowed = is_allow(&app_key);
                // 가독성을 위해 첫 '_' 이전 부분만 표시 이름으로 사용
                let display = subkey_name
                    .split('_')
                    .next()
                    .unwrap_or(&subkey_name)
                    .to_string();
                apps.push(AppAccess {
                    name: display,
                    key: subkey_name.clone(),
                    path: String::new(),
                    allowed,
                    is_uwp: true,
                });
            }
        }
    }

    // 정렬: 차단된 앱 → 허용된 앱, 그 내부에서 이름 순
    apps.sort_by(|a, b| {
        a.allowed
            .cmp(&b.allowed)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok((global_allowed, apps))
}

fn write_value(full_subkey: &str, allow: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(full_subkey)
        .map_err(|e| format!("레지스트리 쓰기 실패: {e}"))?;
    let value = if allow { "Allow" } else { "Deny" };
    key.set_value("Value", &value)
        .map_err(|e| format!("값 설정 실패: {e}"))
}

// ── Tauri 커맨드 ──────────────────────────────────────────────────

/// 카메라·마이크 전체 상태 + 앱별 접근 목록 조회
#[tauri::command]
pub fn get_device_status() -> Result<DeviceStatus, String> {
    let (camera_allowed, camera_apps) = read_consent_key(CAM_KEY)?;
    let (mic_allowed, mic_apps) = read_consent_key(MIC_KEY)?;
    Ok(DeviceStatus {
        camera_allowed,
        mic_allowed,
        camera_apps,
        mic_apps,
    })
}

/// 카메라 전역 허용/차단
#[tauri::command]
pub fn set_camera_access(allow: bool) -> Result<(), String> {
    write_value(CAM_KEY, allow)
}

/// 마이크 전역 허용/차단
#[tauri::command]
pub fn set_mic_access(allow: bool) -> Result<(), String> {
    write_value(MIC_KEY, allow)
}

/// 특정 앱의 카메라 접근 허용/차단 (app_key = AppAccess.key 필드)
#[tauri::command]
pub fn set_app_camera_access(app_key: String, allow: bool) -> Result<(), String> {
    write_value(&format!("{CAM_KEY}\\{app_key}"), allow)
}

/// 특정 앱의 마이크 접근 허용/차단 (app_key = AppAccess.key 필드)
#[tauri::command]
pub fn set_app_mic_access(app_key: String, allow: bool) -> Result<(), String> {
    write_value(&format!("{MIC_KEY}\\{app_key}"), allow)
}

// ── 시작 프로그램 등록 ────────────────────────────────────────────
//
// HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run 에 "WinGuard" 값을 추가/제거.
// HKCU 이므로 관리자 권한 불필요.

const RUN_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "WinGuard";

/// Windows 시작 프로그램 등록 여부 조회
#[tauri::command]
pub fn get_startup_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(RUN_VALUE))
        .is_ok()
}

/// Windows 시작 프로그램 등록(true) / 해제(false)
#[tauri::command]
pub fn set_startup_enabled(enable: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(RUN_KEY)
        .map_err(|e| format!("레지스트리 열기 실패: {e}"))?;

    if enable {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("실행 파일 경로 조회 실패: {e}"))?
            .to_string_lossy()
            .to_string();
        // 시작 시 트레이로 최소화되도록 --minimized 플래그 추가
        let value = format!("\"{exe_path}\" --minimized");
        key.set_value(RUN_VALUE, &value)
            .map_err(|e| format!("값 설정 실패: {e}"))
    } else {
        // 값이 없어도 오류로 처리하지 않음
        match key.delete_value(RUN_VALUE) {
            Ok(_) | Err(_) => Ok(()),
        }
    }
}
