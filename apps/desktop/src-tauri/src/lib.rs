//! Оболочка Tauri: команды и управляющее состояние.

pub mod commands;
pub mod disks;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use commands::{
    assess_file, clamav_status, default_scan_target, get_system_overview, scan_cancel, scan_start,
    AppState,
};

/// Показывает родное окно с сообщением об ошибке запуска (в релизе нет консоли).
/// Текст фиксированный: без деталей ошибки, путей и данных пользователя.
#[cfg(windows)]
fn show_startup_error() {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            hwnd: *mut core::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            utype: u32,
        ) -> i32;
    }
    const MB_OK: u32 = 0x0000_0000;
    const MB_ICONERROR: u32 = 0x0000_0010;

    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    let text = wide("Не удалось запустить OCHISTKA. Попробуйте перезапустить приложение или переустановить его.");
    let caption = wide("OCHISTKA");
    // SAFETY: обе строки заканчиваются нулём и живут до конца вызова; окно-владелец не задано.
    unsafe {
        MessageBoxW(
            core::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn run() {
    let result = tauri::Builder::default()
        .manage(AppState {
            system: Arc::from(ochistka_platform::current_system_info()),
            scanner: Arc::from(ochistka_platform::current_virus_scanner()),
            scan_running: Arc::new(AtomicBool::new(false)),
        })
        .manage(disks::DisksState::new(
            ochistka_platform::current_cleanup_env(),
        ))
        .invoke_handler(tauri::generate_handler![
            get_system_overview,
            scan_start,
            scan_cancel,
            clamav_status,
            assess_file,
            default_scan_target,
            disks::disks_request
        ])
        .run(tauri::generate_context!());
    if let Err(err) = result {
        eprintln!("не удалось запустить OCHISTKA: {err}");
        #[cfg(windows)]
        show_startup_error();
        std::process::exit(1);
    }
}
