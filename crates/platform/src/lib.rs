//! Системные адаптеры по ОС. Весь системный код живёт здесь, за трейтом порта.

pub mod linux;
pub mod macos;
pub mod windows;

pub mod icon_image;
pub mod icon_store;
pub mod os_status;
pub mod speedtest;
pub mod vpn;

/// Очистка дисков Windows: файловый адаптер и раскладка. Файл лежит в `windows/`, но модуль
/// объявлен здесь, потому что `windows/mod.rs` ведётся отдельно.
#[path = "windows/cleanup.rs"]
pub mod windows_cleanup;

use ochistka_core::hardware::HardwarePort;
use ochistka_core::layout::CleanupEnvPort;
use ochistka_core::programs::ProgramsPort;
use ochistka_core::scan::VirusScanPort;
use ochistka_core::system::SystemInfoPort;

/// Адаптер текущей ОС. На прочих ОС, кроме Windows и macOS, — заглушка Linux.
#[cfg(target_os = "windows")]
pub fn current_system_info() -> Box<dyn SystemInfoPort> {
    Box::new(windows::WindowsSystemInfo::new())
}

#[cfg(target_os = "macos")]
pub fn current_system_info() -> Box<dyn SystemInfoPort> {
    Box::new(macos::MacosSystemInfo)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn current_system_info() -> Box<dyn SystemInfoPort> {
    Box::new(linux::LinuxSystemInfo)
}

/// Движок ClamAV. Без фичи `libclamav` — заглушка с причиной «движок не собран» на любой ОС.
pub fn current_virus_scanner() -> Box<dyn VirusScanPort> {
    Box::new(windows::clamav::ClamAvEngine::build())
}

/// Раскладка и файловые операции очистки для текущей ОС: Windows — адаптер, остальные — заглушка.
#[cfg(target_os = "windows")]
pub fn current_cleanup_env() -> Box<dyn CleanupEnvPort> {
    Box::new(windows_cleanup::WindowsCleanupEnv::new())
}

#[cfg(target_os = "macos")]
pub fn current_cleanup_env() -> Box<dyn CleanupEnvPort> {
    Box::new(macos::cleanup::UnsupportedCleanupEnv)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn current_cleanup_env() -> Box<dyn CleanupEnvPort> {
    Box::new(linux::cleanup::UnsupportedCleanupEnv)
}

/// Порт процессов: Windows — sysinfo, остальные ОС — заглушка `Unsupported`.
#[cfg(target_os = "windows")]
pub fn current_process_port() -> Box<dyn ochistka_core::processes::ProcessPort> {
    Box::new(windows::processes::WindowsProcessPort::new())
}

#[cfg(target_os = "macos")]
pub fn current_process_port() -> Box<dyn ochistka_core::processes::ProcessPort> {
    Box::new(macos::processes::UnsupportedProcessPort)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn current_process_port() -> Box<dyn ochistka_core::processes::ProcessPort> {
    Box::new(linux::processes::UnsupportedProcessPort)
}

/// Порт служб: Windows — реестр и `sc.exe`, остальные ОС — заглушка `Unsupported`.
#[cfg(target_os = "windows")]
pub fn current_service_backend() -> Box<dyn ochistka_core::services::ServiceBackend> {
    Box::new(windows::services::WindowsServices::new())
}

#[cfg(target_os = "macos")]
pub fn current_service_backend() -> Box<dyn ochistka_core::services::ServiceBackend> {
    Box::new(macos::services::UnsupportedServiceBackend)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn current_service_backend() -> Box<dyn ochistka_core::services::ServiceBackend> {
    Box::new(linux::services::UnsupportedServiceBackend)
}

/// Комплектующие (видеокарты, плата): Windows — sysinfo, остальные ОС — заглушка `Unsupported`.
#[cfg(target_os = "windows")]
pub fn current_hardware_port() -> Box<dyn HardwarePort> {
    Box::new(windows::hardware::WindowsHardware::new())
}

#[cfg(target_os = "macos")]
pub fn current_hardware_port() -> Box<dyn HardwarePort> {
    Box::new(macos::hardware::UnsupportedHardware)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn current_hardware_port() -> Box<dyn HardwarePort> {
    Box::new(linux::hardware::UnsupportedHardware)
}

/// Реестр и автозагрузка для раздела «Программы»: Windows — адаптер реестра (только чтение), остальные — заглушка.
#[cfg(target_os = "windows")]
pub fn current_programs_port() -> Box<dyn ProgramsPort> {
    Box::new(windows::programs::WindowsPrograms::new())
}

#[cfg(target_os = "macos")]
pub fn current_programs_port() -> Box<dyn ProgramsPort> {
    Box::new(macos::programs::UnsupportedPrograms)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn current_programs_port() -> Box<dyn ProgramsPort> {
    Box::new(linux::programs::UnsupportedPrograms)
}

/// Установка драйверов: Windows — загрузка, проверка подписи и запуск установщика, остальные ОС — заглушка.
#[cfg(target_os = "windows")]
pub fn current_driver_host() -> Box<dyn ochistka_core::drivers::DriverHost> {
    Box::new(windows::drivers::WindowsDriverHost::new())
}

#[cfg(target_os = "macos")]
pub fn current_driver_host() -> Box<dyn ochistka_core::drivers::DriverHost> {
    Box::new(macos::drivers::UnsupportedDriverHost)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn current_driver_host() -> Box<dyn ochistka_core::drivers::DriverHost> {
    Box::new(linux::drivers::UnsupportedDriverHost)
}

/// Дисковый кэш значков программ и процессов (папка данных приложения).
pub fn current_icon_store() -> Box<dyn ochistka_core::programs::icons::IconStore> {
    Box::new(icon_store::DiskIconStore::default())
}

/// Сеть (адаптеры и TCP-соединения), только чтение: Windows — sysinfo и netstat2, остальные ОС — заглушка.
#[cfg(target_os = "windows")]
pub fn current_network_port() -> Box<dyn ochistka_core::network::NetworkPort> {
    Box::new(windows::network::WindowsNetwork::new())
}

#[cfg(target_os = "macos")]
pub fn current_network_port() -> Box<dyn ochistka_core::network::NetworkPort> {
    Box::new(macos::network::UnsupportedNetwork)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn current_network_port() -> Box<dyn ochistka_core::network::NetworkPort> {
    Box::new(linux::network::UnsupportedNetwork)
}

/// Настройки и Boost: на Windows — реестр HKCU и powercfg, на остальных ОС — те же методы с ответом
/// `Unsupported` (системного кода для них нет).
pub fn current_tweaks_port() -> Box<dyn ochistka_core::tweaks::TweaksPort> {
    Box::new(windows::tweaks::WindowsTweaks::new())
}

/// Порты оформления Windows (реестр HKCU и обои). Вне Windows методы возвращают `Unsupported`.
pub fn current_appearance_ports() -> (
    std::sync::Arc<dyn ochistka_core::appearance::RegistryPort>,
    std::sync::Arc<dyn ochistka_core::appearance::WallpaperPort>,
) {
    (
        std::sync::Arc::new(windows::appearance::WindowsAppearanceRegistry),
        std::sync::Arc::new(windows::appearance::WindowsWallpaper),
    )
}

/// Файлы оформления (`%LOCALAPPDATA%\OCHISTKA`) и обновление курсоров Windows.
pub fn current_appearance_system_ports() -> (
    std::sync::Arc<dyn ochistka_core::appearance::FilePort>,
    std::sync::Arc<dyn ochistka_core::appearance::CursorRefreshPort>,
) {
    (
        std::sync::Arc::new(windows::appearance::WindowsAppFiles),
        std::sync::Arc::new(windows::appearance::WindowsCursorRefresh),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::system::{CoreError, SystemInfoPort, SystemOverview};

    fn unsupported_os(result: Result<SystemOverview, CoreError>) -> String {
        match result {
            Err(CoreError::Unsupported { os, .. }) => os,
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn macos_and_linux_stubs_report_unsupported() {
        assert_eq!(unsupported_os(macos::MacosSystemInfo.overview()), "macOS");
        assert_eq!(unsupported_os(linux::LinuxSystemInfo.overview()), "Linux");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn current_system_info_is_unsupported_outside_windows() {
        assert!(matches!(
            current_system_info().overview(),
            Err(CoreError::Unsupported { .. })
        ));
    }

    #[test]
    fn windows_adapter_reads_live_system_overview() {
        let overview = windows::WindowsSystemInfo::new()
            .overview()
            .expect("sysinfo should answer on this host");
        assert!(overview.cpu.logical_cores >= 1);
        assert!(overview.memory.total_bytes > 0);
        assert!(overview.memory.used_bytes <= overview.memory.total_bytes);
        assert!((0.0..=100.0).contains(&overview.cpu.usage_percent));
        for disk in &overview.disks {
            assert!(disk.used_bytes <= disk.total_bytes);
            assert!((0.0..=100.0).contains(&disk.usage_percent));
        }
    }

    #[test]
    fn virus_scanner_reports_engine_status_on_this_os() {
        let status = current_virus_scanner().engine_status();
        assert!(!status.available);
        assert!(!status.reason.is_empty());
    }
}
