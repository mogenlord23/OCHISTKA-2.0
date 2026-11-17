//! Системные адаптеры по ОС. Весь системный код живёт здесь, за трейтом порта.

pub mod linux;
pub mod macos;
pub mod windows;

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
}
