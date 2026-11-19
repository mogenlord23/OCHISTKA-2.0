//! Оболочка Tauri: команды и управляющее состояние.

pub mod appearance;
pub mod commands;
pub mod disks;
pub mod drivers;
pub mod hardware;
pub mod network;
pub mod processes;
pub mod programs;
pub mod services;
pub mod tweaks;
pub mod vpn;

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
            ochistka_platform::current_icon_store(),
        ))
        .setup(|app| {
            use tauri::Manager;
            // Значок окна и панели задач задаём явно: иначе берётся самый маленький из списка.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_icon(tauri::include_image!("icons/icon.png"));
            }
            let (registry, wallpaper) = ochistka_platform::current_appearance_ports();
            let (files, refresh) = ochistka_platform::current_appearance_system_ports();
            let resources = app
                .path()
                .resource_dir()
                .ok()
                .map(|dir| appearance::plain_path(&dir));
            app.manage(appearance::AppearanceState::new(
                registry, wallpaper, files, refresh, resources,
            ));
            app.manage(vpn::build_service(app.path().resource_dir().ok()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            appearance::appearance_request,
            get_system_overview,
            scan_start,
            scan_cancel,
            clamav_status,
            assess_file,
            default_scan_target,
            disks::disks_request,
            processes::processes_request,
            programs::programs_request,
            services::services_request,
            hardware::get_hardware,
            drivers::drivers_catalog,
            drivers::driver_install,
            drivers::driver_open_page,
            drivers::driver_open_windows_update,
            tweaks::tweaks_overview,
            tweaks::tweak_set,
            tweaks::tweaks_revert,
            tweaks::boost_set_profile,
            tweaks::windows_activation,
            tweaks::open_activation_settings,
            network::get_network_adapters,
            network::get_tcp_connections,
            vpn::vpn_snapshot,
            vpn::vpn_status,
            vpn::vpn_import,
            vpn::vpn_select,
            vpn::vpn_set_rules,
            vpn::vpn_measure_latency,
            vpn::vpn_connect,
            vpn::vpn_disconnect
        ])
        .build(tauri::generate_context!());
    match result {
        // При выходе всегда останавливаем VPN-ядро и возвращаем системный прокси.
        Ok(app) => app.run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                if let Some(state) = handle.try_state::<vpn::VpnState>() {
                    state.0.shutdown();
                }
            }
        }),
        Err(err) => {
            eprintln!("не удалось запустить OCHISTKA: {err}");
            #[cfg(windows)]
            show_startup_error();
            std::process::exit(1);
        }
    }
}
