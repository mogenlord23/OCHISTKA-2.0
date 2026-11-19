//! Видеокарты и материнская плата через sysinfo (только чтение). Код собирается на всех ОС,
//! потому что sysinfo переносим; на других ОС `current_hardware_port` отдаёт заглушку.

use std::sync::OnceLock;

use ochistka_core::hardware::{
    assign_gpu_loads, build_gpu, build_motherboard, clean_text, HardwareOverview, HardwarePort,
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

/// Загрузка 3D-движков видеокарт из счётчиков Windows (WMI, только чтение). Нет счётчиков или
/// сбой WMI дают пустой список: интерфейс тогда показывает прочерк с причиной.
#[cfg(target_os = "windows")]
fn read_gpu_loads() -> Vec<(String, f32)> {
    use ochistka_core::hardware::{aggregate_gpu_3d_load, GpuEngineSample};

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Engine {
        name: Option<String>,
        utilization_percentage: Option<u64>,
    }
    let Ok(connection) = wmi::WMIConnection::new() else {
        return Vec::new();
    };
    let rows: Vec<Engine> = connection
        .raw_query(
            "SELECT Name, UtilizationPercentage FROM \
             Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine WHERE Name LIKE '%engtype_3D'",
        )
        .unwrap_or_default();
    let samples: Vec<GpuEngineSample> = rows
        .into_iter()
        .filter_map(|row| {
            Some(GpuEngineSample {
                name: row.name?,
                utilization_percent: row.utilization_percentage?,
            })
        })
        .collect();
    aggregate_gpu_3d_load(&samples)
}

#[cfg(not(target_os = "windows"))]
fn read_gpu_loads() -> Vec<(String, f32)> {
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
        let gpu_list: Vec<_> = gpus.iter().collect();
        // sysinfo не отдаёт загрузку GPU на Windows: берём её из счётчиков производительности.
        let loads = assign_gpu_loads(&read_gpu_loads(), gpu_list.len());
        let gpus = gpu_list
            .iter()
            .zip(loads)
            .map(|(gpu, load)| {
                build_gpu(
                    gpu.vendor(),
                    gpu.model(),
                    gpu.usage().or(load),
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
