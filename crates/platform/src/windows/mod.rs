//! Адаптеры Windows: sysinfo (только чтение) и ClamAV (запуск clamscan по абсолютному пути).

pub mod clamav;
pub mod files;

use std::sync::Mutex;

use ochistka_core::system::{
    CoreError, CpuStatus, DiskStatus, MemoryStatus, SystemInfoPort, SystemOverview,
};
use sysinfo::{Disks, System, MINIMUM_CPU_UPDATE_INTERVAL};

pub struct WindowsSystemInfo {
    system: Mutex<Option<System>>,
}

impl WindowsSystemInfo {
    /// Ничего не читает: ошибки чтения возвращаются из `overview()`.
    pub fn new() -> Self {
        Self {
            system: Mutex::new(None),
        }
    }
}

impl Default for WindowsSystemInfo {
    fn default() -> Self {
        Self::new()
    }
}

fn probe_error(err: sysinfo::Error) -> CoreError {
    CoreError::Probe(err.to_string())
}

/// Создаёт `System` и делает первый замер CPU. sysinfo отдаёт осмысленную загрузку
/// только между двумя замерами, поэтому замер ждёт `MINIMUM_CPU_UPDATE_INTERVAL`.
fn init_system() -> Result<System, CoreError> {
    let mut system = System::new().map_err(probe_error)?;
    system.refresh_cpu_usage();
    std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    Ok(system)
}

impl SystemInfoPort for WindowsSystemInfo {
    fn overview(&self) -> Result<SystemOverview, CoreError> {
        let mut guard = self
            .system
            .lock()
            .map_err(|_| CoreError::Probe("блокировка сведений о системе".into()))?;
        if guard.is_none() {
            *guard = Some(init_system()?);
        }
        let system = guard
            .as_mut()
            .ok_or_else(|| CoreError::Probe("сведения о системе не инициализированы".into()))?;

        system.refresh_cpu_usage();
        system.refresh_memory();

        let logical_cores = u32::try_from(system.cpus().len()).unwrap_or(u32::MAX);
        let cpu = CpuStatus::new(system.global_cpu_usage(), logical_cores);
        let memory = MemoryStatus::new(system.total_memory(), system.used_memory());

        let disks = Disks::new_with_refreshed_list()
            .map_err(probe_error)?
            .iter()
            .map(|disk| {
                DiskStatus::new(
                    &disk.name().to_string_lossy(),
                    &disk.mount_point().to_string_lossy(),
                    disk.total_space(),
                    disk.available_space(),
                )
            })
            .collect();

        Ok(SystemOverview { cpu, memory, disks })
    }
}
