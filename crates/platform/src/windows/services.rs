//! Список служб Windows только для чтения. Источник — ветка `Services` реестра (HKLM), которую
//! читает `winreg`: безопасная обёртка без собственного unsafe. Состояние «работает» здесь
//! не видно: его отдаёт только Service Control Manager, а вызовы SCM потребовали бы FFI.

use std::collections::BTreeMap;

use ochistka_core::processes::{ProcessError, ServiceInfo};
use ochistka_core::programs::RawValue;

/// Подпись режима запуска по значению `Start` из реестра.
pub fn start_type_label(start: Option<u32>) -> &'static str {
    match start {
        Some(0) => "Загрузка драйвера",
        Some(1) => "Системная",
        Some(2) => "Автоматически",
        Some(3) => "Вручную",
        Some(4) => "Отключена",
        _ => "Неизвестно",
    }
}

/// Разбирает одну запись службы. Драйверы (`Type` без битов служб 0x10 и 0x20) пропускаем.
/// Отображаемое имя-ссылка (`@...`) или пустое заменяется именем ключа.
pub fn parse_service(key_name: &str, values: &BTreeMap<String, RawValue>) -> Option<ServiceInfo> {
    let service_type = match values.get("Type") {
        Some(RawValue::Number(value)) => *value,
        Some(RawValue::Text(text)) => text.trim().parse::<u32>().ok()?,
        None => return None,
    };
    if service_type & 0x30 == 0 {
        return None;
    }
    let name = key_name.trim();
    if name.is_empty() {
        return None;
    }
    let display_name = match values.get("DisplayName") {
        Some(RawValue::Text(text))
            if !text.trim().is_empty() && !text.trim_start().starts_with('@') =>
        {
            text.trim().to_string()
        }
        _ => name.to_string(),
    };
    let start = match values.get("Start") {
        Some(RawValue::Number(value)) => Some(*value),
        _ => None,
    };
    Some(ServiceInfo {
        name: name.to_string(),
        display_name,
        start_type: start_type_label(start).to_string(),
    })
}

/// Список служб: разбор, сортировка по имени без учёта регистра.
pub fn build_services(raw: &[(String, BTreeMap<String, RawValue>)]) -> Vec<ServiceInfo> {
    let mut services: Vec<ServiceInfo> = raw
        .iter()
        .filter_map(|(name, values)| parse_service(name, values))
        .collect();
    services.sort_by_key(|service| service.name.to_lowercase());
    services
}

#[cfg(windows)]
pub fn list() -> Result<Vec<ServiceInfo>, ProcessError> {
    use winreg::enums::{RegType, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::types::FromRegValue;
    use winreg::RegKey;

    const SERVICES_PATH: &str = r"SYSTEM\CurrentControlSet\Services";

    let root = RegKey::predef(HKEY_LOCAL_MACHINE);
    let services = root
        .open_subkey_with_flags(SERVICES_PATH, KEY_READ)
        .map_err(|err| ProcessError::Probe(err.to_string()))?;
    let mut raw = Vec::new();
    for key_name in services.enum_keys().flatten() {
        let Ok(sub) = services.open_subkey_with_flags(&key_name, KEY_READ) else {
            continue;
        };
        let mut values = BTreeMap::new();
        for item in sub.enum_values() {
            let Ok((value_name, value)) = item else {
                continue;
            };
            let parsed = match value.vtype {
                RegType::REG_SZ | RegType::REG_EXPAND_SZ => {
                    String::from_reg_value(&value).ok().map(RawValue::Text)
                }
                RegType::REG_DWORD => u32::from_reg_value(&value).ok().map(RawValue::Number),
                _ => None,
            };
            if let Some(parsed) = parsed {
                values.insert(value_name, parsed);
            }
        }
        raw.push((key_name, values));
    }
    Ok(build_services(&raw))
}

#[cfg(not(windows))]
pub fn list() -> Result<Vec<ServiceInfo>, ProcessError> {
    Err(ProcessError::Unsupported {
        feature: "Службы".into(),
        os: std::env::consts::OS.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, RawValue)]) -> BTreeMap<String, RawValue> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.clone()))
            .collect()
    }

    fn text(value: &str) -> RawValue {
        RawValue::Text(value.to_string())
    }

    #[test]
    fn start_values_map_to_russian_labels() {
        assert_eq!(start_type_label(Some(2)), "Автоматически");
        assert_eq!(start_type_label(Some(3)), "Вручную");
        assert_eq!(start_type_label(Some(4)), "Отключена");
        assert_eq!(start_type_label(Some(9)), "Неизвестно");
        assert_eq!(start_type_label(None), "Неизвестно");
    }

    #[test]
    fn service_keeps_display_name_and_start_label() {
        let entry = values(&[
            ("Type", RawValue::Number(0x10)),
            ("Start", RawValue::Number(3)),
            ("DisplayName", text("Диспетчер печати")),
        ]);
        let service = parse_service("Spooler", &entry).expect("service");
        assert_eq!(service.name, "Spooler");
        assert_eq!(service.display_name, "Диспетчер печати");
        assert_eq!(service.start_type, "Вручную");
    }

    #[test]
    fn indirect_display_name_falls_back_to_key_name() {
        let entry = values(&[
            ("Type", RawValue::Number(0x20)),
            ("DisplayName", text("@%SystemRoot%\\system32\\svc.dll,-101")),
        ]);
        let service = parse_service("WSearch", &entry).expect("service");
        assert_eq!(service.display_name, "WSearch");
        assert_eq!(service.start_type, "Неизвестно");
    }

    #[test]
    fn drivers_are_not_listed_as_services() {
        let driver = values(&[
            ("Type", RawValue::Number(1)),
            ("Start", RawValue::Number(3)),
        ]);
        assert!(parse_service("disk", &driver).is_none());
    }

    #[test]
    fn missing_type_is_not_a_service() {
        assert!(parse_service("Orphan", &values(&[("Start", RawValue::Number(2))])).is_none());
    }

    #[test]
    fn text_type_value_is_read_as_number() {
        let entry = values(&[("Type", text(" 16 "))]);
        assert!(parse_service("Text", &entry).is_some());
    }

    #[test]
    fn build_sorts_by_name_ignoring_case() {
        let raw = vec![
            (
                "zeta".to_string(),
                values(&[("Type", RawValue::Number(16))]),
            ),
            (
                "Alpha".to_string(),
                values(&[("Type", RawValue::Number(16))]),
            ),
        ];
        let names: Vec<String> = build_services(&raw).into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["Alpha".to_string(), "zeta".to_string()]);
    }

    #[cfg(not(windows))]
    #[test]
    fn service_list_reports_unsupported_outside_windows() {
        assert!(matches!(list(), Err(ProcessError::Unsupported { .. })));
    }
}
