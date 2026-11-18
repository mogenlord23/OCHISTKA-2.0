//! Адаптер настроек и Boost для Windows: реестр текущего пользователя (`winreg`) и
//! `powercfg.exe` по абсолютному пути. Записи выполняются только по команде интерфейса после
//! подтверждения; в тестах настоящий реестр и powercfg не вызываются.

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

#[cfg(windows)]
mod imp {
    use std::io;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use ochistka_core::system::CoreError;
    use ochistka_core::tweaks::{is_guid, parse_active_scheme, PowerPort, RegistryPort};
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    use winreg::RegKey;

    use super::{powercfg_path, WindowsTweaks};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn probe(err: io::Error) -> CoreError {
        CoreError::Probe(format!("реестр: {err}"))
    }

    fn hkcu() -> RegKey {
        RegKey::predef(HKEY_CURRENT_USER)
    }

    /// Чтение: отсутствие ключа или значения — это `None`, а не ошибка.
    fn read<T: winreg::types::FromRegValue>(key: &str, name: &str) -> Result<Option<T>, CoreError> {
        let opened = match hkcu().open_subkey_with_flags(key, KEY_READ) {
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
            let (opened, _) = hkcu().create_subkey(key).map_err(probe)?;
            opened.set_value(name, &value).map_err(probe)
        }

        fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, CoreError> {
            read::<String>(key, name)
        }

        fn write_string(&self, key: &str, name: &str, value: &str) -> Result<(), CoreError> {
            let (opened, _) = hkcu().create_subkey(key).map_err(probe)?;
            opened.set_value(name, &value).map_err(probe)
        }

        fn delete_value(&self, key: &str, name: &str) -> Result<(), CoreError> {
            let opened = match hkcu().open_subkey_with_flags(key, KEY_SET_VALUE) {
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
            match hkcu().delete_subkey_all(key) {
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
            powercfg(&["/setactive", guid]).map(|_| ())
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use ochistka_core::system::CoreError;
    use ochistka_core::tweaks::{PowerPort, RegistryPort};

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

    #[cfg(not(windows))]
    #[test]
    fn writes_are_refused_outside_windows() {
        use ochistka_core::tweaks::{PowerPort, RegistryPort};
        let port = WindowsTweaks::new();
        assert!(port.write_dword("a", "b", 1).is_err());
        assert!(port.set_active_scheme("x").is_err());
    }
}
