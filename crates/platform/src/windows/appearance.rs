//! Оформление Windows: запись в HKCU через `winreg`; обои, курсоры и рассылка о смене цветов —
//! через `win32_ffi.rs` (единственный модуль с unsafe). Реальные записи только под
//! `cfg(windows)` и только после подтверждения (проверяется в ядре); в тестах не выполняются.
//! На других ОС адаптеры возвращают `Unsupported`.

use ochistka_core::appearance::{
    AppearanceError, CursorRefreshPort, FilePort, RegValue, RegistryPort, WallpaperPort,
};

pub struct WindowsAppearanceRegistry;
pub struct WindowsWallpaper;
/// Папка `%LOCALAPPDATA%\OCHISTKA` и запись файлов в неё.
pub struct WindowsAppFiles;
/// Обновление курсоров: `SystemParametersInfoW(SPI_SETCURSORS)`; рассылка о смене цветов.
pub struct WindowsCursorRefresh;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::windows::win32_ffi;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_BINARY};
    use winreg::{RegKey, RegValue as RawValue};

    /// Ошибка с кодом ОС 0 («Операция успешно завершена») — успех, а не ошибка.
    fn err(e: std::io::Error) -> AppearanceError {
        AppearanceError::Registry(e.to_string())
    }

    fn checked(result: std::io::Result<()>) -> Result<(), AppearanceError> {
        crate::os_status::io_ok_if_code_zero(result).map_err(err)
    }

    impl RegistryPort for WindowsAppearanceRegistry {
        fn read(&self, key: &str, name: &str) -> Result<Option<RegValue>, AppearanceError> {
            let root = RegKey::predef(HKEY_CURRENT_USER);
            let Ok(sub) = root.open_subkey_with_flags(key, KEY_READ) else {
                return Ok(None);
            };
            if let Ok(raw) = sub.get_raw_value(name) {
                if raw.vtype == REG_BINARY {
                    return Ok(Some(RegValue::Binary(raw.bytes.into_owned())));
                }
            }
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
            checked(match value {
                RegValue::Dword(v) => sub.set_value(name, v),
                RegValue::Text(s) => sub.set_value(name, s),
                RegValue::Binary(bytes) => sub.set_raw_value(
                    name,
                    &RawValue {
                        bytes: bytes.clone().into(),
                        vtype: REG_BINARY,
                    },
                ),
            })
        }

        fn delete(&self, key: &str, name: &str) -> Result<(), AppearanceError> {
            let root = RegKey::predef(HKEY_CURRENT_USER);
            let Ok(sub) = root.open_subkey_with_flags(key, KEY_SET_VALUE) else {
                return Ok(());
            };
            match sub.delete_value(name) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => checked(Err(e)),
            }
        }
    }

    impl WallpaperPort for WindowsWallpaper {
        fn current(&self) -> Result<Option<String>, AppearanceError> {
            Ok(win32_ffi::current_wallpaper())
        }

        fn set(&self, path: &str) -> Result<(), AppearanceError> {
            win32_ffi::set_wallpaper(path).map_err(AppearanceError::Wallpaper)
        }

        fn screen_size(&self) -> Option<(u32, u32)> {
            win32_ffi::primary_screen_size()
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

        fn windows_dir(&self) -> String {
            Self::system_root()
        }

        fn write(&self, path: &str, bytes: &[u8]) -> Result<(), AppearanceError> {
            let p = std::path::Path::new(path);
            let io = |e: std::io::Error| AppearanceError::Wallpaper(e.to_string());
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::write(p, bytes).map_err(io)
        }

        fn copy(&self, from: &str, to: &str) -> Result<(), AppearanceError> {
            let target = std::path::Path::new(to);
            let io = |e: std::io::Error| AppearanceError::Wallpaper(e.to_string());
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::copy(from, target).map(|_| ()).map_err(io)
        }
    }

    impl CursorRefreshPort for WindowsCursorRefresh {
        fn refresh(&self) -> Result<(), AppearanceError> {
            win32_ffi::reload_cursors().map_err(AppearanceError::Registry)
        }

        fn colors_changed(&self) -> Result<(), AppearanceError> {
            win32_ffi::broadcast_color_change().map_err(AppearanceError::Registry)
        }
    }

    impl WindowsAppFiles {
        /// `%SystemRoot%` для пути к обоям Windows по умолчанию.
        pub(super) fn system_root() -> String {
            std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into())
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
        fn copy(&self, _: &str, _: &str) -> Result<(), AppearanceError> {
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
