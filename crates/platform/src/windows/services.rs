//! Службы Windows: чтение из реестра (`winreg`, безопасная обёртка) и управление через
//! `sc.exe` по абсолютному пути `C:\Windows\System32\sc.exe` с ограничением по времени.
//! Выбор и причины: docs/adr/0013-services-control.md. Правила (чёрный список, подтверждения,
//! проверка результата) живут в `ochistka_core::services`; здесь только вызовы ОС.

use std::collections::BTreeMap;

use ochistka_core::programs::RawValue;
use ochistka_core::services::{RawService, ScOutput, ServiceBackend, ServiceError};

/// Разбирает одну запись службы. Драйверы (`Type` без битов служб 0x10 и 0x20) и шаблоны
/// пользовательских служб (бит 0x40: настоящие службы создаются с суффиксом) пропускаем.
/// Отображаемое имя-ссылка (`@...`) или пустое заменяется именем ключа.
pub fn parse_service(key_name: &str, values: &BTreeMap<String, RawValue>) -> Option<RawService> {
    let service_type = match values.get("Type") {
        Some(RawValue::Number(value)) => *value,
        Some(RawValue::Text(text)) => text.trim().parse::<u32>().ok()?,
        None => return None,
    };
    if service_type & 0x30 == 0 || service_type & 0x40 != 0 {
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
    let delayed = matches!(values.get("DelayedAutostart"), Some(RawValue::Number(1)));
    Some(RawService {
        name: name.to_string(),
        display_name,
        start,
        delayed,
    })
}

/// Список служб из записей реестра: разбор и сортировка по имени без учёта регистра.
pub fn build_services(raw: &[(String, BTreeMap<String, RawValue>)]) -> Vec<RawService> {
    let mut services: Vec<RawService> = raw
        .iter()
        .filter_map(|(name, values)| parse_service(name, values))
        .collect();
    services.sort_by_key(|service| service.name.to_lowercase());
    services
}

/// Путь к `sc.exe` из папки Windows, без поиска по PATH.
pub fn sc_path(system_root: Option<&str>) -> String {
    let root = system_root
        .map(|root| root.trim_end_matches(['\\', '/']))
        .filter(|root| !root.is_empty())
        .unwrap_or(r"C:\Windows");
    format!(r"{root}\System32\sc.exe")
}

pub struct WindowsServices;

impl WindowsServices {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsServices {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    use winreg::enums::{RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
    use winreg::types::FromRegValue;
    use winreg::RegKey;

    const SERVICES_PATH: &str = r"SYSTEM\CurrentControlSet\Services";
    const BACKUP_ROOT: &str = r"Software\OCHISTKA\ServiceStartBackup";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    /// Одна команда `sc` не должна занимать больше этого времени.
    const SC_TIMEOUT: Duration = Duration::from_secs(20);

    impl ServiceBackend for WindowsServices {
        fn registry_entries(&self) -> Result<Vec<RawService>, ServiceError> {
            let root = RegKey::predef(HKEY_LOCAL_MACHINE);
            let services = root
                .open_subkey_with_flags(SERVICES_PATH, KEY_READ)
                .map_err(|err| ServiceError::Probe(err.to_string()))?;
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
                        RegType::REG_DWORD => {
                            u32::from_reg_value(&value).ok().map(RawValue::Number)
                        }
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

        fn sc(&self, args: &[&str]) -> Result<ScOutput, ServiceError> {
            let root = std::env::var("SystemRoot").ok();
            let mut child = Command::new(sc_path(root.as_deref()))
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|err| {
                    ServiceError::Failed(format!("Не удалось запустить sc.exe: {err}"))
                })?;
            let mut stdout = child.stdout.take();
            // Вывод читаем в отдельном потоке, чтобы переполнение трубы не остановило sc.
            let reader = std::thread::spawn(move || {
                let mut bytes = Vec::new();
                if let Some(pipe) = stdout.as_mut() {
                    let _ = pipe.read_to_end(&mut bytes);
                }
                bytes
            });
            let started = Instant::now();
            let status = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) if started.elapsed() > SC_TIMEOUT => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(ServiceError::Failed(
                            "Windows слишком долго не отвечает на команду службы.".into(),
                        ));
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                    Err(err) => return Err(ServiceError::Failed(format!("Ошибка sc.exe: {err}"))),
                }
            };
            let bytes = reader.join().unwrap_or_default();
            Ok(ScOutput {
                code: status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&bytes).into_owned(),
            })
        }

        fn is_elevated(&self) -> bool {
            // Открытие ключа служб на запись ничего не меняет: без прав администратора оно отклоняется.
            RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(SERVICES_PATH, KEY_WRITE)
                .is_ok()
        }

        fn saved_start(&self, name: &str) -> Option<u32> {
            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey_with_flags(BACKUP_ROOT, KEY_READ)
                .ok()?
                .get_value::<u32, _>(name)
                .ok()
        }

        fn save_start(&self, name: &str, code: u32) -> Result<(), ServiceError> {
            let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey(BACKUP_ROOT)
                .map_err(|err| {
                    ServiceError::Failed(format!("Не удалось запомнить прежнее значение: {err}"))
                })?;
            key.set_value(name, &code).map_err(|err| {
                ServiceError::Failed(format!("Не удалось запомнить прежнее значение: {err}"))
            })
        }

        fn clear_saved_start(&self, name: &str) {
            if let Ok(key) =
                RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(BACKUP_ROOT, KEY_WRITE)
            {
                let _ = key.delete_value(name);
            }
        }

        fn wait_ms(&self, ms: u64) {
            std::thread::sleep(Duration::from_millis(ms));
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    fn unsupported() -> ServiceError {
        ServiceError::Unsupported {
            feature: "Службы".into(),
            os: std::env::consts::OS.into(),
        }
    }

    impl ServiceBackend for WindowsServices {
        fn registry_entries(&self) -> Result<Vec<RawService>, ServiceError> {
            Err(unsupported())
        }
        fn sc(&self, _args: &[&str]) -> Result<ScOutput, ServiceError> {
            Err(unsupported())
        }
        fn is_elevated(&self) -> bool {
            false
        }
        fn saved_start(&self, _name: &str) -> Option<u32> {
            None
        }
        fn save_start(&self, _name: &str, _code: u32) -> Result<(), ServiceError> {
            Err(unsupported())
        }
        fn clear_saved_start(&self, _name: &str) {}
    }
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
    fn service_keeps_display_name_start_and_delay_flag() {
        let entry = values(&[
            ("Type", RawValue::Number(0x10)),
            ("Start", RawValue::Number(2)),
            ("DelayedAutostart", RawValue::Number(1)),
            ("DisplayName", text("Диспетчер печати")),
        ]);
        let service = parse_service("Spooler", &entry).expect("service");
        assert_eq!(service.name, "Spooler");
        assert_eq!(service.display_name, "Диспетчер печати");
        assert_eq!(service.start, Some(2));
        assert!(service.delayed);
    }

    #[test]
    fn indirect_display_name_falls_back_to_key_name() {
        let entry = values(&[
            ("Type", RawValue::Number(0x20)),
            ("DisplayName", text("@%SystemRoot%\\system32\\svc.dll,-101")),
        ]);
        let service = parse_service("WSearch", &entry).expect("service");
        assert_eq!(service.display_name, "WSearch");
        assert_eq!(service.start, None);
        assert!(!service.delayed);
    }

    #[test]
    fn drivers_and_user_service_templates_are_not_listed() {
        let driver = values(&[
            ("Type", RawValue::Number(1)),
            ("Start", RawValue::Number(3)),
        ]);
        assert!(parse_service("disk", &driver).is_none());
        let template = values(&[("Type", RawValue::Number(0x60))]);
        assert!(parse_service("CDPUserSvc", &template).is_none());
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

    #[test]
    fn sc_is_always_an_absolute_path_in_the_windows_folder() {
        assert_eq!(sc_path(None), r"C:\Windows\System32\sc.exe");
        assert_eq!(sc_path(Some(r"D:\WINNT\")), r"D:\WINNT\System32\sc.exe");
        assert_eq!(sc_path(Some("")), r"C:\Windows\System32\sc.exe");
    }

    #[cfg(not(windows))]
    #[test]
    fn service_backend_reports_unsupported_outside_windows() {
        let backend = WindowsServices::new();
        assert!(matches!(
            backend.registry_entries(),
            Err(ServiceError::Unsupported { .. })
        ));
        assert!(!backend.is_elevated());
    }
}
