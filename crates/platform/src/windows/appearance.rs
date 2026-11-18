//! Оформление Windows: запись в HKCU через `winreg` и смена обоев через крейт `wallpaper`.
//! Оба — безопасные обёртки, собственного unsafe здесь нет. Реальные записи только под
//! `cfg(windows)` и только после подтверждения (проверяется в ядре); в тестах не выполняются.
//! На других ОС адаптеры возвращают `Unsupported`.

use ochistka_core::appearance::{
    AppearanceError, CursorRefreshPort, FilePort, RegValue, RegistryPort, WallpaperPort,
};

pub struct WindowsAppearanceRegistry;
pub struct WindowsWallpaper;
/// Папка `%LOCALAPPDATA%\OCHISTKA` и запись файлов в неё.
pub struct WindowsAppFiles;
/// Обновление курсоров через `%SystemRoot%\System32\rundll32.exe` (полный путь, без PATH).
pub struct WindowsCursorRefresh;

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    use winreg::RegKey;

    fn err(e: std::io::Error) -> AppearanceError {
        AppearanceError::Registry(e.to_string())
    }

    impl RegistryPort for WindowsAppearanceRegistry {
        fn read(&self, key: &str, name: &str) -> Result<Option<RegValue>, AppearanceError> {
            let root = RegKey::predef(HKEY_CURRENT_USER);
            let Ok(sub) = root.open_subkey_with_flags(key, KEY_READ) else {
                return Ok(None);
            };
            if let Ok(v) = sub.get_value::<u32, _>(name) {
                return Ok(Some(RegValue::Dword(v)));
            }
            Ok(sub.get_value::<String, _>(name).ok().map(RegValue::Text))
        }

        fn write(&self, key: &str, name: &str, value: &RegValue) -> Result<(), AppearanceError> {
            let root = RegKey::predef(HKEY_CURRENT_USER);
            let (sub, _) = root
                .create_subkey_with_flags(key, KEY_SET_VALUE)
                .map_err(err)?;
            match value {
                RegValue::Dword(v) => sub.set_value(name, v),
                RegValue::Text(s) => sub.set_value(name, s),
            }
            .map_err(err)
        }

        fn delete(&self, key: &str, name: &str) -> Result<(), AppearanceError> {
            let root = RegKey::predef(HKEY_CURRENT_USER);
            let Ok(sub) = root.open_subkey_with_flags(key, KEY_SET_VALUE) else {
                return Ok(());
            };
            match sub.delete_value(name) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(err(e)),
            }
        }
    }

    impl WallpaperPort for WindowsWallpaper {
        fn current(&self) -> Result<Option<String>, AppearanceError> {
            Ok(wallpaper::get().ok().filter(|p| !p.is_empty()))
        }

        fn set(&self, path: &str) -> Result<(), AppearanceError> {
            wallpaper::set_from_path(path).map_err(|e| AppearanceError::Wallpaper(e.to_string()))
        }
    }
    impl FilePort for WindowsAppFiles {
        fn data_dir(&self) -> Result<String, AppearanceError> {
            let base = std::env::var("LOCALAPPDATA")
                .map_err(|_| AppearanceError::Registry("нет папки LOCALAPPDATA".into()))?;
            Ok(format!("{base}\\OCHISTKA"))
        }

        fn exists(&self, path: &str) -> bool {
            std::path::Path::new(path).is_file()
        }

        fn write(&self, path: &str, bytes: &[u8]) -> Result<(), AppearanceError> {
            let p = std::path::Path::new(path);
            let io = |e: std::io::Error| AppearanceError::Wallpaper(e.to_string());
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::write(p, bytes).map_err(io)
        }
    }

    impl CursorRefreshPort for WindowsCursorRefresh {
        fn refresh(&self) -> Result<(), AppearanceError> {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
            let exe = format!("{root}\\System32\\rundll32.exe");
            let status = std::process::Command::new(exe)
                .args(["user32.dll,UpdatePerUserSystemParameters"])
                .status()
                .map_err(|e| AppearanceError::Registry(e.to_string()))?;
            if status.success() {
                Ok(())
            } else {
                Err(AppearanceError::Registry(
                    "rundll32 завершился с ошибкой".into(),
                ))
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    impl RegistryPort for WindowsAppearanceRegistry {
        fn read(&self, _: &str, _: &str) -> Result<Option<RegValue>, AppearanceError> {
            Err(AppearanceError::Unsupported)
        }
        fn write(&self, _: &str, _: &str, _: &RegValue) -> Result<(), AppearanceError> {
            Err(AppearanceError::Unsupported)
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), AppearanceError> {
            Err(AppearanceError::Unsupported)
        }
    }

    impl WallpaperPort for WindowsWallpaper {
        fn current(&self) -> Result<Option<String>, AppearanceError> {
            Err(AppearanceError::Unsupported)
        }
        fn set(&self, _: &str) -> Result<(), AppearanceError> {
            Err(AppearanceError::Unsupported)
        }
    }
    impl FilePort for WindowsAppFiles {
        fn data_dir(&self) -> Result<String, AppearanceError> {
            Err(AppearanceError::Unsupported)
        }
        fn exists(&self, _: &str) -> bool {
            false
        }
        fn write(&self, _: &str, _: &[u8]) -> Result<(), AppearanceError> {
            Err(AppearanceError::Unsupported)
        }
    }

    impl CursorRefreshPort for WindowsCursorRefresh {
        fn refresh(&self) -> Result<(), AppearanceError> {
            Err(AppearanceError::Unsupported)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn adapters_are_unsupported_off_windows() {
        assert_eq!(
            WindowsAppearanceRegistry.read("k", "n"),
            Err(AppearanceError::Unsupported)
        );
        assert_eq!(
            WindowsWallpaper.set("/a.png"),
            Err(AppearanceError::Unsupported)
        );
    }
}
