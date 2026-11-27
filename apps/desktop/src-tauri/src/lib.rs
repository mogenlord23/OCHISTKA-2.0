//! Оболочка Tauri: команды и управляющее состояние.

pub mod appearance;
pub mod bypass_sites;
pub mod commands;
pub mod disks;
pub mod drivers;
pub mod hardware;
pub mod network;
pub mod processes;
pub mod programs;
pub mod services;
pub mod speedtest;
mod tray;
pub mod tweaks;
pub mod vpn;
#[cfg(windows)]
mod zapret;

#[cfg(windows)]
pub fn check_embedded_zapret() -> Result<(), String> {
    zapret::check_embedded()
}

use std::sync::Arc;

use commands::{
    assess_file, clamav_load_signatures, clamav_signatures, clamav_status, default_scan_target,
    get_system_overview, scan_cancel, scan_computer, scan_pick_folder, scan_start,
    scan_start_with_progress, AppState,
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
        .plugin(tauri_plugin_notification::init())
        .on_window_event(tray::window_event)
        .manage(AppState::new(
            ochistka_platform::current_system_info(),
            ochistka_platform::current_virus_scanner(),
            ochistka_platform::current_suspicion_scanner(),
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
            #[cfg(windows)]
            {
                let zapret = Arc::new(zapret::Zapret::new(coord));
                let guard = Arc::clone(&zapret);
                vpn_state.0.set_conflict_guard(Box::new(move || guard.stop().err()));
                app.manage(zapret);
            }
            app.manage(vpn_state);
            app.manage(bypass_sites::BypassSitesState(
                ochistka_core::desync::user_sites::UserSites::new(
                    &ochistka_platform::desync::desync_data_dir(),
                ),
            ));
            // Базы проверки: папку создаём всегда, а встроенный движок грузим, только если в ней
            // уже есть файлы. Пустая папка — честное «базы не загружены», приложение работает.
            if let Ok(dir) = ochistka_platform::windows::clamav::ensure_signatures_dir() {
                if ochistka_platform::windows::clamav::signatures_files(&dir) > 0 {
                    let scanner = std::sync::Arc::clone(&app.state::<AppState>().scanner);
                    std::thread::spawn(move || {
                        let _ = scanner.load_signatures(&dir.to_string_lossy());
                    });
                }
            }
            // WebView начинает вызывать команды сразу после создания окна.
            // Открываем окна после регистрации состояния VPN и остальных разделов.
            for config in &app.config().app.windows {
                let result = tauri::WebviewWindowBuilder::from_config(app, config)
                    .and_then(|builder| builder.build());
                if config.label == "main" {
                    let window = result?;
                    let _ = window.set_icon(tauri::include_image!("icons/icon.png"));
                } else if let Err(error) = result {
                    eprintln!("не удалось создать окно {}: {error}", config.label);
                }
            }
            if let Err(error) = tray::install(app.handle()) {
                eprintln!("не удалось создать значок OCHISTKA: {error}");
                if let Some(panel) = app.get_webview_window("tray-panel") {
                    let _ = panel.destroy();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            appearance::appearance_request,
            get_system_overview,
            scan_start,
            scan_computer,
            scan_pick_folder,
            tray::tray_action,
            tray::tray_hover,
            tray::tray_status,
            scan_start_with_progress,
            scan_cancel,
            clamav_status,
            clamav_signatures,
            clamav_load_signatures,
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
            #[cfg(windows)]
            zapret::zapret_status,
            #[cfg(windows)]
            zapret::zapret_set,
            vpn::vpn_snapshot,
            vpn::vpn_status,
            vpn::vpn_import,
            vpn::vpn_clear_subscription,
            vpn::vpn_select,
            vpn::vpn_set_rules,
            vpn::vpn_set_mode,
            vpn::vpn_measure_latency,
            vpn::vpn_connect,
            vpn::vpn_disconnect
        ])
        .build(tauri::generate_context!());
    match result {
        // При выходе — та же последовательность, что при выключении VPN: остановка ядра, ожидание,
        // снятие остатков нашего адаптера (VpnService::shutdown).
        Ok(app) => app.run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                #[cfg(windows)]
                if let Some(zapret) = handle.try_state::<Arc<zapret::Zapret>>() {
                    let _ = zapret.stop();
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
