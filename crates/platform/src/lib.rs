//! Системные адаптеры по ОС. Весь системный код живёт здесь, за трейтом порта.

pub mod linux;
pub mod macos;
pub mod windows;

/// Очистка дисков Windows: файловый адаптер и раскладка. Файл лежит в `windows/`, но модуль
/// объявлен здесь, потому что `windows/mod.rs` ведётся отдельно.
#[path = "windows/cleanup.rs"]
pub mod windows_cleanup;

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

/// Сканер ClamAV: на Windows — clamscan.exe из папки установки, на других ОС — заглушка с причиной.
#[cfg(target_os = "windows")]
pub fn current_virus_scanner() -> Box<dyn VirusScanPort> {
    use std::path::{Path, PathBuf};

    let install_dir: Option<PathBuf> = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    match install_dir {
        Some(dir) => Box::new(windows::clamav::ClamAvScanner::from_install_dir(&dir)),
        None => Box::new(windows::clamav::ClamAvScanner::unavailable(
            "не удалось определить папку установки",
        )),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn current_virus_scanner() -> Box<dyn VirusScanPort> {
    Box::new(windows::clamav::ClamAvScanner::unavailable(
        "проверка ClamAV доступна только в Windows",
    ))
}

/// Раскладка и файловые операции очистки для текущей ОС: Windows — адаптер, остальные — заглушка.
#[cfg(target_os = "windows")]
pub fn current_cleanup_env() -> Box<dyn CleanupEnvPort> {
    Box::new(windows_cleanup::WindowsCleanupEnv::new())
}

#[cfg(not(target_os = "windows"))]
pub fn current_cleanup_env() -> Box<dyn CleanupEnvPort> {
    Box::new(linux::cleanup::UnsupportedCleanupEnv)
}

/// Порт процессов: Windows — sysinfo, остальные ОС — заглушка `Unsupported`.
#[cfg(target_os = "windows")]
pub fn current_process_port() -> Box<dyn ochistka_core::processes::ProcessPort> {
    Box::new(windows::processes::WindowsProcessPort::new())
}

#[cfg(not(target_os = "windows"))]
pub fn current_process_port() -> Box<dyn ochistka_core::processes::ProcessPort> {
    Box::new(linux::processes::UnsupportedProcessPort)
}

/// Реестр и автозагрузка для раздела «Программы»: Windows — адаптер реестра (только чтение), остальные — заглушка.
#[cfg(target_os = "windows")]
pub fn current_programs_port() -> Box<dyn ProgramsPort> {
    Box::new(windows::programs::WindowsPrograms::new())
}

#[cfg(not(target_os = "windows"))]
pub fn current_programs_port() -> Box<dyn ProgramsPort> {
    Box::new(linux::programs::UnsupportedPrograms)
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
