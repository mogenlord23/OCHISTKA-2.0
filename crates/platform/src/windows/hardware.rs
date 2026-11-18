//! Видеокарты и материнская плата через sysinfo (только чтение). Код собирается на всех ОС,
//! потому что sysinfo переносим; на других ОС `current_hardware_port` отдаёт заглушку.

use std::sync::OnceLock;

use ochistka_core::hardware::{
    build_gpu, build_motherboard, clean_text, HardwareOverview, HardwarePort,
};
use ochistka_core::system::CoreError;
use sysinfo::{CpuRefreshKind, Gpus, Motherboard, System};

/// Название процессора и модели дисков не меняются за сеанс: читаются один раз.
pub struct WindowsHardware {
    cpu_brand: OnceLock<Option<String>>,
    drive_models: OnceLock<Vec<String>>,
}

impl WindowsHardware {
    pub fn new() -> Self {
        Self {
            cpu_brand: OnceLock::new(),
            drive_models: OnceLock::new(),
        }
    }
}

fn read_cpu_brand() -> Option<String> {
    let mut system = System::new().ok()?;
    system.refresh_cpu_list(CpuRefreshKind::nothing());
    clean_text(system.cpus().first().map(|cpu| cpu.brand()))
}

/// Модели физических дисков из WMI (Win32_DiskDrive), только чтение. Сбой даёт пустой список:
/// интерфейс тогда честно показывает «SSD · C:».
#[cfg(target_os = "windows")]
fn read_drive_models() -> Vec<String> {
    use ochistka_core::hardware::clean_drive_models;

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct DiskDrive {
        model: Option<String>,
    }
    let Ok(connection) = wmi::WMIConnection::new() else {
        return Vec::new();
    };
    let rows: Vec<DiskDrive> = connection
        .raw_query("SELECT Model FROM Win32_DiskDrive")
        .unwrap_or_default();
    let raw: Vec<String> = rows.into_iter().filter_map(|row| row.model).collect();
    clean_drive_models(&raw)
}

#[cfg(not(target_os = "windows"))]
fn read_drive_models() -> Vec<String> {
    Vec::new()
}

impl Default for WindowsHardware {
    fn default() -> Self {
        Self::new()
    }
}

fn probe(err: sysinfo::Error) -> CoreError {
    CoreError::Probe(err.to_string())
}

impl HardwarePort for WindowsHardware {
    fn overview(&self) -> Result<HardwareOverview, CoreError> {
        let gpus = Gpus::new_with_refreshed_list().map_err(probe)?;
        let gpus = gpus
            .iter()
            .map(|gpu| {
                build_gpu(
                    gpu.vendor(),
                    gpu.model(),
                    gpu.usage(),
                    gpu.total_memory(),
                    gpu.used_memory(),
                )
            })
            .collect();
        let board = Motherboard::new().map_err(probe)?;
        Ok(HardwareOverview {
            gpus,
            motherboard: build_motherboard(
                board.vendor_name().as_deref(),
                board.name().as_deref(),
                board.version().as_deref(),
            ),
            cpu_brand: self.cpu_brand.get_or_init(read_cpu_brand).clone(),
            drive_models: self.drive_models.get_or_init(read_drive_models).clone(),
        })
    }
}
