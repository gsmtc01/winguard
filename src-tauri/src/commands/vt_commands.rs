// src-tauri/src/commands/vt_commands.rs
//
// VirusTotal 관련 Tauri 커맨드.
// main.rs 의 tauri::generate_handler![] 에 등록해야 한다.
//
// API 키 저장:
//   OS 키체인(keyring crate)을 사용한다. Windows = Credential Manager.
//   평문 파일이나 tauri-plugin-store 에 저장하지 않는다.

use crate::checks::virustotal::{scan_file, VtScanResult};
use keyring::Entry;

const KEYRING_SERVICE: &str = "winguard";
const KEYRING_USER: &str = "virustotal_api_key";

// ── API 키 관리 커맨드 ─────────────────────────────────────────

/// API 키를 OS 키체인에 저장
#[tauri::command]
pub fn save_vt_api_key(api_key: String) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err("API 키가 비어 있습니다.".into());
    }

    Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("키체인 접근 실패: {}", e))?
        .set_password(&api_key)
        .map_err(|e| format!("API 키 저장 실패: {}", e))
}

/// API 키 등록 여부만 반환 (키 값 자체는 프론트엔드로 노출하지 않는다)
#[tauri::command]
pub fn has_vt_api_key() -> bool {
    Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .and_then(|e| e.get_password())
        .is_ok()
}

/// API 키 삭제
#[tauri::command]
pub fn delete_vt_api_key() -> Result<(), String> {
    Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("키체인 접근 실패: {}", e))?
        .delete_password()
        .map_err(|e| format!("API 키 삭제 실패: {}", e))
}

// ── 파일 검사 커맨드 ───────────────────────────────────────────

/// VT 파일 검사 커맨드
///
/// # Arguments
/// - `file_path`: 검사할 파일의 절대 경로
/// - `allow_upload`: 해시 미등록 시 VT에 업로드할지 여부
///   → 프론트엔드에서 사용자 동의 다이얼로그를 거친 후 true 로 설정
#[tauri::command]
pub fn scan_file_vt(file_path: String, allow_upload: bool) -> VtScanResult {
    // API 키는 여기서 꺼내서 전달. 프론트엔드로는 절대 노출하지 않는다.
    let api_key = match Entry::new(KEYRING_SERVICE, KEYRING_USER).and_then(|e| e.get_password()) {
        Ok(key) => key,
        Err(_) => {
            return VtScanResult {
                sha256: String::new(),
                status: crate::checks::virustotal::VtStatus::Error,
                total_engines: 0,
                malicious: 0,
                suspicious: 0,
                undetected: 0,
                detections: vec![],
                vt_link: String::new(),
                error_message: Some(
                    "VT API 키가 등록되어 있지 않습니다. 설정에서 API 키를 입력하세요.".into(),
                ),
                was_uploaded: false,
            };
        }
    };

    // 파일 검사 (blocking — Tauri 커맨드는 별도 스레드에서 실행됨)
    scan_file(&file_path, &api_key, allow_upload)
}

// ── main.rs 등록 예시 (참고용 주석) ───────────────────────────
//
// fn main() {
//     tauri::Builder::default()
//         .invoke_handler(tauri::generate_handler![
//             run_scan,
//             scan_file_vt,       // ← 추가
//             save_vt_api_key,    // ← 추가
//             has_vt_api_key,     // ← 추가
//             delete_vt_api_key,  // ← 추가
//         ])
//         .run(tauri::generate_context!())
//         .expect("error while running tauri application");
// }
