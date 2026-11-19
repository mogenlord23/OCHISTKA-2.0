//! Системные сведения: DTO, расчёты процентов и порт. Без кода ОС.

use serde::Serialize;
use thiserror::Error;
use ts_rs::TS;

/// Ограничивает проценты диапазоном 0..=100; NaN превращает в 0.
pub fn clamp_percent(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 100.0)
    }
}

/// Процент занятого места; при нулевом объёме возвращает 0.
pub fn usage_percent(used_bytes: u64, total_bytes: u64) -> f32 {
    if total_bytes == 0 {
        return 0.0;
    }
    clamp_percent(used_bytes as f32 / total_bytes as f32 * 100.0)
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "CpuStatus.ts")]
pub struct CpuStatus {
    pub usage_percent: f32,
    pub logical_cores: u32,
}

impl CpuStatus {
    pub fn new(usage_percent: f32, logical_cores: u32) -> Self {
        Self {
            usage_percent: clamp_percent(usage_percent),
            logical_cores,
        }
    }
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "MemoryStatus.ts")]
pub struct MemoryStatus {
    #[ts(type = "number")]
    pub total_bytes: u64,
    #[ts(type = "number")]
    pub used_bytes: u64,
    pub usage_percent: f32,
}

impl MemoryStatus {
    pub fn new(total_bytes: u64, used_bytes: u64) -> Self {
        let used_bytes = used_bytes.min(total_bytes);
        Self {
            total_bytes,
            used_bytes,
            usage_percent: usage_percent(used_bytes, total_bytes),
        }
    }
}

/// Тип носителя: SSD, HDD или не определён. В JSON — строки `"ssd" | "hdd" | "unknown"`.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "DiskKind.ts")]
pub enum DiskKind {
    Ssd,
    Hdd,
    Unknown,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "DiskStatus.ts")]
pub struct DiskStatus {
    pub display_name: String,
    pub mount_point: String,
    #[ts(type = "number")]
    pub total_bytes: u64,
    #[ts(type = "number")]
    pub used_bytes: u64,
    pub usage_percent: f32,
    pub kind: DiskKind,
}

impl DiskStatus {
    /// Пустое имя заменяется путём монтирования; занято = объём − свободно, не ниже 0.
    pub fn new(name: &str, mount_point: &str, total_bytes: u64, available_bytes: u64) -> Self {
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let display_name = if name.trim().is_empty() {
            mount_point
        } else {
            name
        };
        Self {
            display_name: display_name.to_string(),
            mount_point: mount_point.to_string(),
            total_bytes,
            used_bytes,
            usage_percent: usage_percent(used_bytes, total_bytes),
            kind: DiskKind::Unknown,
        }
    }

    /// Задаёт тип носителя, который читает адаптер ОС.
    pub fn with_kind(mut self, kind: DiskKind) -> Self {
        self.kind = kind;
        self
    }
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "SystemOverview.ts")]
pub struct SystemOverview {
    pub cpu: CpuStatus,
    pub memory: MemoryStatus,
    pub disks: Vec<DiskStatus>,
    /// Название ОС; пустая строка, если ОС не удалось определить.
    pub os_name: String,
    /// Версия ОС; пустая строка, если версию не удалось определить.
    pub os_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("{feature} недоступно на {os}")]
    Unsupported { feature: String, os: String },
    #[error("ошибка чтения системных данных: {0}")]
    Probe(String),
    #[error("движок недоступен: {0}")]
    EngineMissing(String),
    #[error("недопустимый путь проверки: {0}")]
    InvalidTarget(String),
    #[error("проверка отменена")]
    Cancelled,
    /// Настройка не действует на этой версии Windows (причина готова для показа).
    #[error("{0}")]
    NotApplicable(String),
    /// Запись в HKLM без прав администратора.
    #[error("Нужны права администратора")]
    NeedsAdmin,
}

/// Порт системных сведений. Реализации живут в `crates/platform`.
pub trait SystemInfoPort: Send + Sync {
    fn overview(&self) -> Result<SystemOverview, CoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_percent_handles_nan_and_range() {
        assert_eq!(clamp_percent(f32::NAN), 0.0);
        assert_eq!(clamp_percent(-5.0), 0.0);
        assert_eq!(clamp_percent(120.0), 100.0);
        assert_eq!(clamp_percent(42.5), 42.5);
    }

    #[test]
    fn usage_percent_is_zero_for_zero_total() {
        assert_eq!(usage_percent(10, 0), 0.0);
    }

    #[test]
    fn disk_with_zero_total_reports_zero_percent() {
        let disk = DiskStatus::new("C:", "C:\\", 0, 0);
        assert_eq!(disk.usage_percent, 0.0);
        assert!(!disk.usage_percent.is_nan());
    }

    #[test]
    fn disk_available_above_total_saturates() {
        let disk = DiskStatus::new("D:", "D:\\", 100, 150);
        assert_eq!(disk.used_bytes, 0);
        assert_eq!(disk.usage_percent, 0.0);
    }

    #[test]
    fn disk_without_name_uses_mount_point_as_label() {
        assert_eq!(DiskStatus::new("", "E:\\", 1, 1).display_name, "E:\\");
    }

    #[test]
    fn memory_used_is_clamped_to_total() {
        let mem = MemoryStatus::new(100, 250);
        assert_eq!(mem.used_bytes, 100);
        assert_eq!(mem.usage_percent, 100.0);
    }

    #[test]
    fn overview_serializes_with_camel_case_keys() {
        let overview = SystemOverview {
            cpu: CpuStatus::new(12.0, 8),
            memory: MemoryStatus::new(100, 50),
            disks: vec![DiskStatus::new("C:", "C:\\", 100, 40).with_kind(DiskKind::Ssd)],
            os_name: "Windows 11".into(),
            os_version: "24H2".into(),
        };
        let json = serde_json::to_value(&overview).unwrap();
        assert_eq!(json["osName"], "Windows 11");
        assert_eq!(json["osVersion"], "24H2");
        assert!(json["cpu"]["usagePercent"].is_number());
        assert!(json["cpu"]["logicalCores"].is_number());
        assert!(json["memory"]["totalBytes"].is_number());
        assert!(json["memory"]["usedBytes"].is_number());
        let disk = &json["disks"][0];
        assert_eq!(disk["displayName"], "C:");
        assert_eq!(disk["mountPoint"], "C:\\");
        assert_eq!(disk["kind"], "ssd");
    }

    #[test]
    fn disk_kind_serializes_as_lowercase_string() {
        assert_eq!(serde_json::to_value(DiskKind::Hdd).unwrap(), "hdd");
        assert_eq!(serde_json::to_value(DiskKind::Unknown).unwrap(), "unknown");
    }

    #[test]
    fn disk_is_unknown_kind_until_adapter_sets_it() {
        let disk = DiskStatus::new("C:", "C:\\", 100, 40);
        assert_eq!(disk.kind, DiskKind::Unknown);
        assert_eq!(disk.with_kind(DiskKind::Hdd).kind, DiskKind::Hdd);
    }

    #[test]
    fn core_error_message_is_russian() {
        let err = CoreError::Unsupported {
            feature: "Сведения о системе".into(),
            os: "macOS".into(),
        };
        assert_eq!(err.to_string(), "Сведения о системе недоступно на macOS");
    }
}
