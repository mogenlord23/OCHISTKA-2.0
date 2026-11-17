//! Оболочка Tauri: команды и управляющее состояние.

pub mod commands;

use std::sync::Arc;

use commands::{assess_file, clamav_status, get_system_overview, scan_start, AppState};

pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            system: Arc::from(ochistka_platform::current_system_info()),
            scanner: Arc::from(ochistka_platform::current_virus_scanner()),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_overview,
            scan_start,
            clamav_status,
            assess_file
        ])
        .run(tauri::generate_context!())
        .expect("не удалось запустить OCHISTKA");
}
