//! Модели дисков по буквам томов из WMI (только чтение): Win32_DiskDrive и связи с разделами.
//! Сбой даёт пустую карту: интерфейс тогда пишет «Модель неизвестна».

use std::collections::HashMap;
use std::sync::OnceLock;

/// Карта «C:» → короткая модель. Читается один раз за сеанс.
pub fn volume_models() -> &'static HashMap<String, String> {
    static MODELS: OnceLock<HashMap<String, String>> = OnceLock::new();
    MODELS.get_or_init(read)
}

#[cfg(target_os = "windows")]
fn read() -> HashMap<String, String> {
    use ochistka_core::disk_model::link_volumes_to_models;

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Drive {
        #[serde(rename = "DeviceID")]
        device_id: Option<String>,
        model: Option<String>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Link {
        antecedent: Option<String>,
        dependent: Option<String>,
    }
    let Ok(connection) = wmi::WMIConnection::new() else {
        return HashMap::new();
    };
    let drives: Vec<Drive> = connection
        .raw_query("SELECT DeviceID, Model FROM Win32_DiskDrive")
        .unwrap_or_default();
    let links = |query: &str| -> Vec<(String, String)> {
        connection
            .raw_query::<Link>(query)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| Some((row.antecedent?, row.dependent?)))
            .collect()
    };
    let drive_to_partition =
        links("SELECT Antecedent, Dependent FROM Win32_DiskDriveToDiskPartition");
    let partition_to_volume =
        links("SELECT Antecedent, Dependent FROM Win32_LogicalDiskToPartition");
    let drives: Vec<(String, String)> = drives
        .into_iter()
        .filter_map(|row| Some((row.device_id?, row.model.unwrap_or_default())))
        .collect();
    link_volumes_to_models(&drives, &drive_to_partition, &partition_to_volume)
}

#[cfg(not(target_os = "windows"))]
fn read() -> HashMap<String, String> {
    HashMap::new()
}
