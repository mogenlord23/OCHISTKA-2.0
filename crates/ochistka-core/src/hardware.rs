//! Комплектующие: видеокарты и материнская плата. DTO, очистка строк и порт. Без кода ОС.
//! Адаптер платформы читает сведения (на Windows — sysinfo) и передаёт их сюда.

use serde::Serialize;
use ts_rs::TS;

use crate::system::CoreError;

/// Видеокарта для интерфейса. Все поля необязательны: драйвер может не отдать сведения.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "GpuInfo.ts")]
pub struct GpuInfo {
    pub vendor: Option<String>,
    pub model: Option<String>,
    /// Загрузка в процентах, если ОС её отдаёт.
    pub usage_percent: Option<f32>,
    #[ts(type = "number | null")]
    pub total_memory_bytes: Option<u64>,
    #[ts(type = "number | null")]
    pub used_memory_bytes: Option<u64>,
}

/// Материнская плата. Серийный номер намеренно не читается.
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "MotherboardInfo.ts")]
pub struct MotherboardInfo {
    pub vendor: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
}

/// Ответ раздела «Компьютер» на сведения о комплектующих.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "HardwareOverview.ts")]
pub struct HardwareOverview {
    pub gpus: Vec<GpuInfo>,
    pub motherboard: MotherboardInfo,
    /// Название процессора, например «Intel(R) Core(TM) i7-6700 CPU @ 3.40GHz».
    pub cpu_brand: Option<String>,
    /// Короткие названия моделей физических дисков; пусто, если ОС их не отдала.
    pub drive_models: Vec<String>,
}

/// Пустая строка и строка из пробелов означают «нет значения».
pub fn clean_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// Доля в процентах: NaN и выход за 0..=100 не показываем, а отбрасываем.
pub fn clean_percent(value: Option<f32>) -> Option<f32> {
    value
        .filter(|percent| percent.is_finite())
        .map(|percent| percent.clamp(0.0, 100.0))
}

/// Собирает видеокарту из сырых значений адаптера: строки очищаются, память не больше объёма.
pub fn build_gpu(
    vendor: Option<&str>,
    model: Option<&str>,
    usage_percent: Option<f32>,
    total_memory_bytes: Option<u64>,
    used_memory_bytes: Option<u64>,
) -> GpuInfo {
    let used_memory_bytes = match (used_memory_bytes, total_memory_bytes) {
        (Some(used), Some(total)) => Some(used.min(total)),
        (used, _) => used,
    };
    GpuInfo {
        vendor: clean_text(vendor),
        model: clean_text(model),
        usage_percent: clean_percent(usage_percent),
        total_memory_bytes,
        used_memory_bytes,
    }
}

/// Собирает материнскую плату из сырых значений адаптера.
pub fn build_motherboard(
    vendor: Option<&str>,
    name: Option<&str>,
    version: Option<&str>,
) -> MotherboardInfo {
    MotherboardInfo {
        vendor: clean_text(vendor),
        name: clean_text(name),
        version: clean_text(version),
    }
}

/// Суффиксы, которые Windows дописывает к модели диска и которые ничего не говорят человеку.
const DRIVE_NOISE: [&str; 5] = [
    " ATA Device",
    " SCSI Disk Device",
    " NVMe Device",
    " USB Device",
    " Disk Device",
];

/// Короткое название модели диска: без служебных хвостов и лишних пробелов.
pub fn shorten_drive_model(raw: &str) -> Option<String> {
    let mut text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    for noise in DRIVE_NOISE {
        if let Some(stripped) = text.strip_suffix(noise) {
            text = stripped.trim().to_string();
        }
    }
    (!text.is_empty()).then_some(text)
}

/// Названия дисков для интерфейса: сокращённые, без пустых и без повторов, порядок сохранён.
pub fn clean_drive_models<S: AsRef<str>>(raw: &[S]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for model in raw
        .iter()
        .filter_map(|item| shorten_drive_model(item.as_ref()))
    {
        if !out.contains(&model) {
            out.push(model);
        }
    }
    out
}

/// Порт комплектующих. Реализации живут в `crates/platform`.
pub trait HardwarePort: Send + Sync {
    fn overview(&self) -> Result<HardwareOverview, CoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_text_becomes_none_and_is_trimmed() {
        assert_eq!(clean_text(Some("  NVIDIA  ")), Some("NVIDIA".to_string()));
        assert_eq!(clean_text(Some("   ")), None);
        assert_eq!(clean_text(Some("")), None);
        assert_eq!(clean_text(None), None);
    }

    #[test]
    fn percent_is_clamped_and_non_finite_is_dropped() {
        assert_eq!(clean_percent(Some(42.5)), Some(42.5));
        assert_eq!(clean_percent(Some(130.0)), Some(100.0));
        assert_eq!(clean_percent(Some(-3.0)), Some(0.0));
        assert_eq!(clean_percent(Some(f32::NAN)), None);
        assert_eq!(clean_percent(Some(f32::INFINITY)), None);
        assert_eq!(clean_percent(None), None);
    }

    #[test]
    fn gpu_memory_used_is_capped_at_total() {
        let gpu = build_gpu(Some("AMD"), Some("RX"), None, Some(100), Some(250));
        assert_eq!(gpu.used_memory_bytes, Some(100));
        assert_eq!(gpu.total_memory_bytes, Some(100));
    }

    #[test]
    fn gpu_without_details_keeps_none_fields() {
        let gpu = build_gpu(Some(" "), None, None, None, Some(5));
        assert_eq!(gpu.vendor, None);
        assert_eq!(gpu.model, None);
        assert_eq!(gpu.usage_percent, None);
        assert_eq!(gpu.total_memory_bytes, None);
        assert_eq!(gpu.used_memory_bytes, Some(5));
    }

    #[test]
    fn drive_models_are_shortened_deduplicated_and_cleaned() {
        assert_eq!(
            shorten_drive_model("KINGSTON SHFS37A120G ATA Device"),
            Some("KINGSTON SHFS37A120G".to_string())
        );
        assert_eq!(
            shorten_drive_model("  WDC   WD10EZEX-21WN4A0  "),
            Some("WDC WD10EZEX-21WN4A0".to_string())
        );
        assert_eq!(shorten_drive_model("  "), None);
        assert_eq!(
            clean_drive_models(&["A ATA Device", "A", "", "B"]),
            vec!["A".to_string(), "B".to_string()]
        );
    }

    #[test]
    fn motherboard_fields_are_cleaned() {
        let board = build_motherboard(Some("ASUS"), Some(" ROG STRIX "), Some(""));
        assert_eq!(board.vendor.as_deref(), Some("ASUS"));
        assert_eq!(board.name.as_deref(), Some("ROG STRIX"));
        assert_eq!(board.version, None);
    }

    #[test]
    fn overview_serializes_with_camel_case_keys() {
        let overview = HardwareOverview {
            gpus: vec![build_gpu(None, Some("GPU"), Some(10.0), Some(8), Some(2))],
            motherboard: build_motherboard(None, Some("Board"), None),
            cpu_brand: Some("CPU".into()),
            drive_models: vec!["Disk".into()],
        };
        let json = serde_json::to_value(&overview).unwrap();
        assert_eq!(json["gpus"][0]["usagePercent"], 10.0);
        assert_eq!(json["gpus"][0]["totalMemoryBytes"], 8);
        assert_eq!(json["gpus"][0]["usedMemoryBytes"], 2);
        assert!(json["gpus"][0]["vendor"].is_null());
        assert_eq!(json["motherboard"]["name"], "Board");
        assert_eq!(json["cpuBrand"], "CPU");
        assert_eq!(json["driveModels"][0], "Disk");
    }
}
