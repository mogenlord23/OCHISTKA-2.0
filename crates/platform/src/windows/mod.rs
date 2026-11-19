//! Адаптеры Windows: sysinfo (только чтение), реестр и движок ClamAV (libclamav, см. ADR 0004).

pub mod appearance;
pub mod clamav;
#[cfg(feature = "libclamav")]
pub mod clamav_ffi;
pub mod files;
pub mod hardware;
pub mod network;
pub mod processes;
pub mod programs;
pub mod services;
pub mod tweaks;
#[cfg(windows)]
pub mod win32_ffi;

use std::sync::{Mutex, OnceLock};

use ochistka_core::system::{
    CoreError, CpuStatus, DiskKind, DiskStatus, MemoryStatus, SystemInfoPort, SystemOverview,
};
use sysinfo::{Disks, System, MINIMUM_CPU_UPDATE_INTERVAL};

pub struct WindowsSystemInfo {
    system: Mutex<Option<System>>,
    /// Название и версия ОС не меняются за сеанс: читаются один раз.
    os_identity: OnceLock<(String, String)>,
}

impl WindowsSystemInfo {
    /// Ничего не читает: ошибки чтения возвращаются из `overview()`.
    pub fn new() -> Self {
        Self {
            system: Mutex::new(None),
            os_identity: OnceLock::new(),
        }
    }

    fn os_identity(&self) -> &(String, String) {
        self.os_identity.get_or_init(|| {
            // Пустая строка означает «не удалось определить»; интерфейс показывает прочерк.
            let name = System::name().unwrap_or_default();
            let version = System::os_version().unwrap_or_default();
            (name, version)
        })
    }
}

/// Переводит тип носителя из sysinfo в тип ядра.
pub fn map_disk_kind(kind: sysinfo::DiskKind) -> DiskKind {
    match kind {
        sysinfo::DiskKind::SSD => DiskKind::Ssd,
        sysinfo::DiskKind::HDD => DiskKind::Hdd,
        sysinfo::DiskKind::Unknown(_) => DiskKind::Unknown,
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
        // Блокировка нужна только для CPU и памяти. Диски перечисляются после её
        // освобождения: зависший съёмный накопитель не блокирует остальные вызовы.
        let (cpu, memory) = {
            // Отравление мьютекса (паника в другом потоке) не фатально: в `System` лежит
            // только снимок, который каждый вызов полностью обновляет через refresh_*.
            // Частично обновлённое состояние после паники перезаписывается следующим замером.
            let mut guard = self
                .system
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if guard.is_none() {
                *guard = Some(init_system()?);
            }
            let system = guard
                .as_mut()
                .ok_or_else(|| CoreError::Probe("сведения о системе не инициализированы".into()))?;

            system.refresh_cpu_usage();
            system.refresh_memory();

            let logical_cores = u32::try_from(system.cpus().len()).unwrap_or(u32::MAX);
            (
                CpuStatus::new(system.global_cpu_usage(), logical_cores),
                MemoryStatus::new(system.total_memory(), system.used_memory()),
            )
        };

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
                .with_kind(map_disk_kind(disk.kind()))
            })
            .collect();

        let (os_name, os_version) = self.os_identity().clone();
        Ok(SystemOverview {
            cpu,
            memory,
            disks,
            os_name,
            os_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_sysinfo_disk_kinds_to_core_kinds() {
        assert_eq!(map_disk_kind(sysinfo::DiskKind::SSD), DiskKind::Ssd);
        assert_eq!(map_disk_kind(sysinfo::DiskKind::HDD), DiskKind::Hdd);
        assert_eq!(
            map_disk_kind(sysinfo::DiskKind::Unknown(7)),
            DiskKind::Unknown
        );
    }

    #[test]
    fn overview_recovers_from_poisoned_mutex() {
        let info = std::sync::Arc::new(WindowsSystemInfo::new());
        let poisoner = std::sync::Arc::clone(&info);
        let joined = std::thread::spawn(move || {
            let _guard = poisoner.system.lock().expect("lock");
            panic!("намеренная паника для отравления мьютекса");
        })
        .join();
        assert!(joined.is_err());
        assert!(info.system.is_poisoned());

        let overview = info.overview();
        assert!(overview.is_ok(), "overview after poison: {overview:?}");
    }
}
