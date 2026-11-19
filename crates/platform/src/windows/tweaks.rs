//! Адаптер настроек и Boost для Windows: реестр текущего пользователя (`winreg`) и
//! `powercfg.exe` по абсолютному пути. Записи выполняются только по команде интерфейса после
//! подтверждения; в тестах настоящий реестр и powercfg не вызываются. Ключи с приставкой `HKLM\`
//! пишутся в HKLM и работают только при правах администратора (иначе `NeedsAdmin`).

pub struct WindowsTweaks;

impl WindowsTweaks {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsTweaks {
    fn default() -> Self {
        Self::new()
    }
}

/// Полный путь к powercfg без поиска по PATH.
pub fn powercfg_path(system_root: Option<&str>) -> String {
    let root = system_root
        .filter(|root| !root.trim().is_empty())
        .unwrap_or(r"C:\Windows");
    format!(r"{}\System32\powercfg.exe", root.trim_end_matches('\\'))
}

/// Полный путь к системной программе в System32 без поиска по PATH.
pub fn system32_path(system_root: Option<&str>, relative: &str) -> String {
    let root = system_root
        .filter(|root| !root.trim().is_empty())
        .unwrap_or(r"C:\Windows");
    format!(r"{}\System32\{relative}", root.trim_end_matches('\\'))
}

/// Проводник для открытия `ms-settings:` по абсолютному пути.
pub fn explorer_path(system_root: Option<&str>) -> String {
    let root = system_root
        .filter(|root| !root.trim().is_empty())
        .unwrap_or(r"C:\Windows");
    format!(r"{}\explorer.exe", root.trim_end_matches('\\'))
}

/// Запрос статуса лицензии: `LicenseStatus;GracePeriodRemaining` или `none`. Текст фиксирован.
pub const ACTIVATION_SCRIPT: &str = "$p = Get-CimInstance SoftwareLicensingProduct -Filter \"ApplicationId='55c92734-d682-4d71-983e-d6ec3f16059f' AND PartialProductKey IS NOT NULL\" | Select-Object -First 1; if ($p) { \"$($p.LicenseStatus);$($p.GracePeriodRemaining)\" } else { 'none' }";

/// Есть ли схема в выводе `powercfg /list` (язык системы не важен).
pub fn scheme_listed(list_output: &str, guid: &str) -> bool {
    list_output
        .to_ascii_lowercase()
        .contains(&guid.to_ascii_lowercase())
}

#[cfg(windows)]
mod imp {
    use std::io;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use ochistka_core::system::CoreError;
    use ochistka_core::tweaks::{
        is_guid, parse_activation, parse_active_scheme, ActivationInfo, PowerPort, RegistryPort,
        SystemPort,
    };
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, KEY_WRITE,
    };
    use winreg::RegKey;

    use super::{
        explorer_path, powercfg_path, scheme_listed, system32_path, WindowsTweaks,
        ACTIVATION_SCRIPT,
    };

    const ULTIMATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn probe(err: io::Error) -> CoreError {
        if err.kind() == io::ErrorKind::PermissionDenied {
            return CoreError::NeedsAdmin;
        }
        CoreError::Probe(format!("реестр: {err}"))
    }

    /// Корень и путь внутри него: приставка `HKLM\` выбирает HKLM, иначе HKCU.
    fn root(key: &str) -> (RegKey, &str) {
        match key.strip_prefix("HKLM\\") {
            Some(rest) => (RegKey::predef(HKEY_LOCAL_MACHINE), rest),
            None => (RegKey::predef(HKEY_CURRENT_USER), key),
        }
    }

    /// Чтение: отсутствие ключа или значения — это `None`, а не ошибка.
    fn read<T: winreg::types::FromRegValue>(key: &str, name: &str) -> Result<Option<T>, CoreError> {
        let (hive, path) = root(key);
        let opened = match hive.open_subkey_with_flags(path, KEY_READ) {
            Ok(opened) => opened,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(probe(err)),
        };
        match opened.get_value::<T, _>(name) {
            Ok(value) => Ok(Some(value)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(probe(err)),
        }
    }

    impl RegistryPort for WindowsTweaks {
        fn read_dword(&self, key: &str, name: &str) -> Result<Option<u32>, CoreError> {
            read::<u32>(key, name)
        }

        fn write_dword(&self, key: &str, name: &str, value: u32) -> Result<(), CoreError> {
            let (hive, path) = root(key);
            let (opened, _) = hive.create_subkey(path).map_err(probe)?;
            opened.set_value(name, &value).map_err(probe)
        }

        fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, CoreError> {
            read::<String>(key, name)
        }

        fn write_string(&self, key: &str, name: &str, value: &str) -> Result<(), CoreError> {
            let (hive, path) = root(key);
            let (opened, _) = hive.create_subkey(path).map_err(probe)?;
            opened.set_value(name, &value).map_err(probe)
        }

        fn delete_value(&self, key: &str, name: &str) -> Result<(), CoreError> {
            let (hive, path) = root(key);
            let opened = match hive.open_subkey_with_flags(path, KEY_SET_VALUE) {
                Ok(opened) => opened,
                Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(err) => return Err(probe(err)),
            };
            match opened.delete_value(name) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(probe(err)),
            }
        }

        fn delete_key(&self, key: &str) -> Result<(), CoreError> {
            // Удаляются только собственные копии программы.
            if !key.starts_with(r"Software\OCHISTKA\Backup") {
                return Err(CoreError::InvalidTarget(format!("чужой ключ: {key}")));
            }
            match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(key) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(probe(err)),
            }
        }
    }

    fn powercfg(args: &[&str]) -> Result<String, CoreError> {
        let path = powercfg_path(std::env::var("SystemRoot").ok().as_deref());
        let output = Command::new(path)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|err| CoreError::Probe(format!("не удалось запустить powercfg: {err}")))?;
        if !output.status.success() {
            return Err(CoreError::Probe(
                "эта схема электропитания недоступна на компьютере".into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    impl PowerPort for WindowsTweaks {
        fn active_scheme(&self) -> Result<String, CoreError> {
            let out = powercfg(&["/getactivescheme"])?;
            parse_active_scheme(&out)
                .ok_or_else(|| CoreError::Probe("не удалось определить схему питания".into()))
        }

        fn set_active_scheme(&self, guid: &str) -> Result<(), CoreError> {
            if !is_guid(guid) {
                return Err(CoreError::InvalidTarget(
                    "недопустимая схема питания".into(),
                ));
            }
            // «Максимальная производительность» по умолчанию скрыта: создаём её с тем же GUID.
            if guid.eq_ignore_ascii_case(ULTIMATE) && !scheme_listed(&powercfg(&["/list"])?, guid) {
                powercfg(&["/duplicatescheme", guid, guid])?;
            }
            powercfg(&["/setactive", guid]).map(|_| ())
        }
    }

    fn run_hidden(program: &str, args: &[&str]) -> Result<String, CoreError> {
        let output = Command::new(program)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|err| CoreError::Probe(format!("не удалось запустить {program}: {err}")))?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    impl SystemPort for WindowsTweaks {
        fn windows_build(&self) -> Option<u32> {
            RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", KEY_READ)
                .ok()?
                .get_value::<String, _>("CurrentBuildNumber")
                .ok()?
                .trim()
                .parse()
                .ok()
        }

        fn is_elevated(&self) -> bool {
            // Открытие на запись ничего не меняет: без прав администратора оно отклоняется.
            RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(r"SOFTWARE\Policies", KEY_WRITE)
                .is_ok()
        }

        fn activation(&self) -> Result<ActivationInfo, CoreError> {
            let root = std::env::var("SystemRoot").ok();
            let shell = system32_path(root.as_deref(), r"WindowsPowerShell\v1.0\powershell.exe");
            let out = run_hidden(
                &shell,
                &[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    ACTIVATION_SCRIPT,
                ],
            )?;
            Ok(parse_activation(&out))
        }

        fn open_activation_settings(&self) -> Result<(), CoreError> {
            let root = std::env::var("SystemRoot").ok();
            Command::new(explorer_path(root.as_deref()))
                .arg("ms-settings:activation")
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|err| CoreError::Probe(format!("не удалось открыть параметры: {err}")))
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use ochistka_core::system::CoreError;
    use ochistka_core::tweaks::{ActivationInfo, PowerPort, RegistryPort, SystemPort};

    use super::WindowsTweaks;

    fn unsupported() -> CoreError {
        CoreError::Unsupported {
            feature: "Настройки Windows".into(),
            os: "этой ОС".into(),
        }
    }

    impl RegistryPort for WindowsTweaks {
        fn read_dword(&self, _: &str, _: &str) -> Result<Option<u32>, CoreError> {
            Err(unsupported())
        }
        fn write_dword(&self, _: &str, _: &str, _: u32) -> Result<(), CoreError> {
            Err(unsupported())
        }
        fn read_string(&self, _: &str, _: &str) -> Result<Option<String>, CoreError> {
            Err(unsupported())
        }
        fn write_string(&self, _: &str, _: &str, _: &str) -> Result<(), CoreError> {
            Err(unsupported())
        }
        fn delete_value(&self, _: &str, _: &str) -> Result<(), CoreError> {
            Err(unsupported())
        }
        fn delete_key(&self, _: &str) -> Result<(), CoreError> {
            Err(unsupported())
        }
    }

    impl PowerPort for WindowsTweaks {
        fn active_scheme(&self) -> Result<String, CoreError> {
            Err(unsupported())
        }
        fn set_active_scheme(&self, _: &str) -> Result<(), CoreError> {
            Err(unsupported())
        }
    }

    impl SystemPort for WindowsTweaks {
        fn windows_build(&self) -> Option<u32> {
            None
        }
        fn is_elevated(&self) -> bool {
            false
        }
        fn activation(&self) -> Result<ActivationInfo, CoreError> {
            Err(unsupported())
        }
        fn open_activation_settings(&self) -> Result<(), CoreError> {
            Err(unsupported())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powercfg_path_is_absolute_and_falls_back() {
        assert_eq!(
            powercfg_path(Some(r"D:\Win\")),
            r"D:\Win\System32\powercfg.exe"
        );
        assert_eq!(powercfg_path(None), r"C:\Windows\System32\powercfg.exe");
        assert_eq!(
            powercfg_path(Some("  ")),
            r"C:\Windows\System32\powercfg.exe"
        );
    }

    #[test]
    fn system_programs_are_absolute_paths() {
        assert_eq!(
            system32_path(Some(r"D:\Win"), r"WindowsPowerShell\v1.0\powershell.exe"),
            r"D:\Win\System32\WindowsPowerShell\v1.0\powershell.exe"
        );
        assert_eq!(explorer_path(None), r"C:\Windows\explorer.exe");
    }

    #[test]
    fn scheme_is_found_in_a_localized_list() {
        let list =
            "Имя схемы: Максимальная производительность  (E9A42B02-D5DF-448D-AA00-03F14749EB61)";
        assert!(scheme_listed(list, "e9a42b02-d5df-448d-aa00-03f14749eb61"));
        assert!(!scheme_listed(list, "381b4222-f694-41f0-9685-ff5bb260df2e"));
    }

    #[test]
    fn activation_script_is_fixed_text_with_the_windows_product_id() {
        assert!(ACTIVATION_SCRIPT.contains("55c92734-d682-4d71-983e-d6ec3f16059f"));
    }

    #[cfg(not(windows))]
    #[test]
    fn writes_are_refused_outside_windows() {
        use ochistka_core::tweaks::{PowerPort, RegistryPort};
        let port = WindowsTweaks::new();
        assert!(port.write_dword("a", "b", 1).is_err());
        assert!(port.set_active_scheme("x").is_err());
    }
}
