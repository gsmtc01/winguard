#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::unwrap_used)]

mod checks;
mod commands;
mod engine;
mod util;

use commands::browser_commands::scan_extensions;
use commands::device_commands::{
    get_device_status, get_startup_enabled, set_app_camera_access, set_app_mic_access,
    set_camera_access, set_mic_access, set_startup_enabled,
};
use commands::eventlog_commands::scan_eventlog;
use commands::llm_commands::{
    llm_analyze, llm_analyze_device, llm_analyze_eventlog, llm_analyze_extensions,
    llm_analyze_network, llm_analyze_processes, llm_ask, llm_default_model_dir,
    llm_default_model_path, llm_delete_model, llm_download_model, llm_explain_check,
    llm_generate_report, llm_is_model_loaded, llm_model_exists, llm_model_info, llm_model_path,
    llm_model_url, llm_roadmap, llm_set_model_path, llm_unload_model,
};
use commands::network_commands::scan_network;
use commands::process_commands::scan_processes;
use commands::quarantine_commands::{
    quarantine_delete, quarantine_list, quarantine_restore, quarantine_save, read_reg_dword,
};
use commands::update_commands::{check_for_update, update_repo};
use commands::vt_commands::{delete_vt_api_key, has_vt_api_key, save_vt_api_key, scan_file_vt};
use engine::{
    rules::apply_compound_rules,
    scorer::score,
    types::{now_ts, ScanReport},
};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

const TRAY_ID: &str = "main";

/// 관리자 권한(high integrity)으로 실행되면 Windows UIPI가 medium integrity인
/// 입력기(ctfmon/TSF)가 보내는 한/영 전환 요청 메시지를 창에 전달하지 못하게
/// 막는다. 한글 조합 자체는 되지만 전환만 안 되는 증상이 여기서 발생하므로,
/// 이 메시지 하나만 UIPI 필터에서 예외로 허용한다.
#[cfg(windows)]
fn allow_ime_language_switch(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        ChangeWindowMessageFilterEx, MSGFLT_ALLOW, WM_INPUTLANGCHANGEREQUEST,
    };
    unsafe {
        let _ = ChangeWindowMessageFilterEx(hwnd, WM_INPUTLANGCHANGEREQUEST, MSGFLT_ALLOW, None);
    }
}

#[tauri::command]
fn run_scan() -> ScanReport {
    let mut checks = vec![
        // ① 핵심 방어
        checks::defender::run(),
        checks::firewall::run(),
        checks::updates::run(),
        checks::eol::run(),
        checks::office_eol::run(),
        checks::browsers::run(),
        // ② 시스템 무결성
        checks::secure_boot::run(),
        checks::secure_boot_cert::run(),
        checks::tpm::run(),
        checks::bitlocker::run(),
        // ③ 접근 제어
        checks::account::run(),
        checks::uac::run(),
        checks::autolock::run(),
        checks::rdp::run(),
        // ④ 네트워크 노출
        checks::smb1::run(),
        checks::shares::run(),
        checks::dns_hijack::run(),
        // ⑤ 고급 탐지
        checks::hosts::run(),
        checks::autorun::run(),
        checks::ps_policy::run(),
        checks::login_failures::run(),
        checks::ransomware_ioc::run(),
        checks::pum_check::run(),
    ];

    apply_compound_rules(&mut checks);
    let total = score(&checks);

    ScanReport {
        score: total,
        checks,
        scanned_at: now_ts(),
    }
}

/// UAC 설정 창을 직접 연다 (UserAccountControlSettings.exe).
#[tauri::command]
fn open_uac_settings() -> Result<(), String> {
    std::process::Command::new("UserAccountControlSettings.exe")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 시스템 보호(System Protection) 설정 창을 직접 연다 (SystemPropertiesProtection.exe).
#[tauri::command]
fn open_system_protection() -> Result<(), String> {
    std::process::Command::new("SystemPropertiesProtection.exe")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// UTF-8 텍스트를 지정 경로에 파일로 저장한다 (CSV 내보내기용).
#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())
}

/// 보안 점수를 시스템 트레이 툴팁에 반영한다.
#[tauri::command]
fn update_tray_tooltip(score: u8, app: tauri::AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let label = if score >= 80 {
            "양호"
        } else if score >= 60 {
            "주의"
        } else {
            "위험"
        };
        let _ = tray.set_tooltip(Some(&format!(
            "WinGuard — 보안 점수: {}/100 ({})",
            score, label
        )));
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // ── 트레이 메뉴 ──────────────────────────────────────────
            let show_item = MenuItemBuilder::with_id("show", "열기").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "종료").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&quit_item)
                .build()?;

            // ── 트레이 아이콘 ────────────────────────────────────────
            // tauri.conf.json bundle.icon 에서 로드한 앱 기본 아이콘 사용
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("트레이 아이콘을 로드할 수 없습니다")?;

            TrayIconBuilder::with_id(TRAY_ID)
                .icon(icon)
                .menu(&menu)
                .tooltip("WinGuard")
                // 트레이 메뉴 클릭
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                // 트레이 아이콘 클릭 → 창 표시
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        }
                    ) {
                        if let Some(win) = tray.app_handle().get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            // ── 한/영 전환 UIPI 예외 등록 ──────────────────────────────
            #[cfg(windows)]
            if let Some(win) = app.get_webview_window("main") {
                if let Ok(hwnd) = win.hwnd() {
                    allow_ime_language_switch(hwnd);
                }
            }

            Ok(())
        })
        // 닫기 버튼 → 트레이로 최소화
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            run_scan,
            write_text_file,
            update_tray_tooltip,
            open_uac_settings,
            open_system_protection,
            scan_file_vt,
            save_vt_api_key,
            has_vt_api_key,
            delete_vt_api_key,
            llm_model_exists,
            llm_model_path,
            llm_default_model_path,
            llm_default_model_dir,
            llm_model_info,
            llm_model_url,
            llm_set_model_path,
            llm_download_model,
            llm_unload_model,
            llm_is_model_loaded,
            llm_delete_model,
            llm_analyze,
            llm_explain_check,
            llm_ask,
            llm_roadmap,
            llm_analyze_processes,
            llm_analyze_network,
            scan_processes,
            scan_network,
            scan_extensions,
            llm_analyze_extensions,
            scan_eventlog,
            llm_analyze_eventlog,
            llm_generate_report,
            get_device_status,
            set_camera_access,
            set_mic_access,
            set_app_camera_access,
            set_app_mic_access,
            llm_analyze_device,
            get_startup_enabled,
            set_startup_enabled,
            quarantine_save,
            quarantine_list,
            quarantine_restore,
            quarantine_delete,
            read_reg_dword,
            check_for_update,
            update_repo,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("WinGuard 실행 오류: {}", e);
            std::process::exit(1);
        });
}
