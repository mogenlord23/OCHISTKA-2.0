//! Оболочка Tauri: команды и управляющее состояние.

pub mod appearance;
pub mod bypass_sites;
pub mod commands;
pub mod desync_trial;
pub mod disks;
pub mod drivers;
pub mod hardware;
pub mod network;
pub mod processes;
pub mod programs;
pub mod service_check;
pub mod services;
pub mod speedtest;
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
        .manage(speedtest::SpeedState::default())
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
            let resources = appearance::pick_resource_dir(
                app.path()
                    .resource_dir()
                    .ok()
                    .map(|dir| appearance::plain_path(&dir)),
            );
            app.manage(appearance::AppearanceState::new(
                registry, wallpaper, files, refresh, resources,
            ));
            // Одно общее право на сетевой режим для VPN и обхода.
            let coord =
                std::sync::Arc::new(ochistka_core::desync::coordinator::NetCoordinator::new());
            let vpn_state = vpn::build_service(app.path().resource_dir().ok(), coord.clone());
            let desync_state = vpn::build_desync(&vpn_state, coord, app.path().resource_dir().ok());
            app.manage(vpn_state);
            app.manage(desync_state);
            app.manage(service_check::ProbeState::default());
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
            network::geo_consent,
            network::geo_set_consent,
            network::geo_lookup,
            speedtest::speedtest_start,
            speedtest::speedtest_cancel,
            bypass_sites::bypass_sites_list,
            bypass_sites::bypass_sites_add,
            bypass_sites::bypass_sites_remove,
            desync_trial::desync_trial_status,
            desync_trial::desync_trial_start,
            desync_trial::desync_trial_stop,
            desync_trial::desync_trial_note,
            service_check::service_check,
            service_check::service_access_plan,
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
                // Сначала останавливаем перехват, затем VPN.
                if let Some(state) = handle.try_state::<vpn::DesyncState>() {
                    // Если процесс не остановился, ложного успеха нет: ошибка остаётся в состоянии,
                    // а при следующем запуске приложения `recover_on_start` убирает остаток.
                    if let Err(err) = state.0.shutdown() {
                        eprintln!("не удалось остановить обход при выходе: {err}");
                    }
                }
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
