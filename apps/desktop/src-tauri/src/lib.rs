//! Оболочка Tauri: команды и управляющее состояние.

pub mod commands;
pub mod disks;
pub mod hardware;
pub mod processes;
pub mod programs;

use std::sync::Arc;

use commands::{
    assess_file, clamav_status, default_scan_target, get_system_overview, scan_cancel, scan_start,
    AppState,
};

/// Показывает родное окно с сообщением об ошибке запуска (в релизе нет консоли).
/// Текст фиксированный: без деталей ошибки, путей и данных пользователя.
#[cfg(windows)]
fn show_startup_error() {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("OCHISTKA")
        .set_description(
            "Не удалось запустить OCHISTKA. Попробуйте перезапустить приложение или переустановить его.",
        )
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

pub fn run() {
    let result = tauri::Builder::default()
        .manage(AppState::new(
            ochistka_platform::current_system_info(),
            ochistka_platform::current_virus_scanner(),
            Arc::new(ochistka_platform::windows_cleanup::WindowsFs),
        ))
        .manage(disks::DisksState::new(
            ochistka_platform::current_cleanup_env(),
        ))
        .manage(programs::ProgramsState::new(
            ochistka_platform::current_programs_port(),
            ochistka_platform::current_cleanup_env(),
        ))
        .invoke_handler(tauri::generate_handler![
            get_system_overview,
            scan_start,
            scan_cancel,
            clamav_status,
            assess_file,
            default_scan_target,
            disks::disks_request,
            processes::processes_request,
            programs::programs_request,
            hardware::get_hardware
        ])
        .run(tauri::generate_context!());
    if let Err(err) = result {
        eprintln!("не удалось запустить OCHISTKA: {err}");
        #[cfg(windows)]
        show_startup_error();
        std::process::exit(1);
    }
}
