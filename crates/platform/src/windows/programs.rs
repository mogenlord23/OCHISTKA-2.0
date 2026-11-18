//! Адаптер «Программ» Windows: ключи Uninstall и Run реестра и папки «Автозагрузка». Только чтение.
//! Реестр читает `winreg` (безопасная обёртка) только под `cfg(windows)`; на других ОС методы
//! возвращают `Unsupported`. Папки читает `std::fs`, поэтому их разбор проверяется и на Linux.
//! Реестр пакетов Microsoft Store пока не читается (см. docs/platform-status.md).

use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;

use ochistka_core::programs::{ProgramsPort, RawProgramRecord, RawStartup, StartupSource};
use ochistka_core::system::CoreError;

#[cfg(windows)]
const UNINSTALL_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
#[cfg(windows)]
const RUN_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const USER_STARTUP_SUFFIX: &str = r"Microsoft\Windows\Start Menu\Programs\Startup";
#[cfg(windows)]
const COMMON_STARTUP_SUFFIX: &str = r"Microsoft\Windows\Start Menu\Programs\StartUp";

/// Адаптер реестра и автозагрузки Windows. Ничего не записывает и не меняет.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPrograms;

impl WindowsPrograms {
    pub fn new() -> Self {
        Self
    }
}

/// Имена файлов из папки «Автозагрузка». Ярлык `.lnk` показываем без расширения;
/// служебный `desktop.ini` пропускаем. Команды у папок нет: её не читаем.
pub fn startup_folder_records(dir: &Path, source: StartupSource) -> Vec<RawStartup> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if file_name.eq_ignore_ascii_case("desktop.ini") {
                return None;
            }
            let is_shortcut = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"));
            let name = if is_shortcut {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or(file_name)
            } else {
                file_name
            };
            Some(RawStartup {
                source,
                name,
                command: None,
            })
        })
        .collect()
}

/// Папки автозагрузки: профиль пользователя и общая для всех. Пути из переменных окружения.
#[cfg(windows)]
fn startup_folders() -> Vec<(PathBuf, StartupSource)> {
    let mut folders = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        folders.push((
            PathBuf::from(appdata).join(USER_STARTUP_SUFFIX),
            StartupSource::UserStartupFolder,
        ));
    }
    if let Some(common) = std::env::var_os("ProgramData") {
        folders.push((
            PathBuf::from(common).join(COMMON_STARTUP_SUFFIX),
            StartupSource::CommonStartupFolder,
        ));
    }
    folders
}

#[cfg(windows)]
mod registry {
    use std::collections::BTreeMap;

    use ochistka_core::programs::{
        RawProgramRecord, RawSource, RawStartup, RawValue, StartupSource,
    };
    use winreg::enums::{
        RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::types::FromRegValue;
    use winreg::RegKey;

    use super::{RUN_PATH, UNINSTALL_PATH};

    /// Значения ключа: текст и DWORD. Прочие типы не нужны и пропускаются.
    fn read_values(key: &RegKey) -> BTreeMap<String, RawValue> {
        let mut values = BTreeMap::new();
        for item in key.enum_values() {
            let Ok((name, value)) = item else { continue };
            let parsed = match value.vtype {
                RegType::REG_SZ | RegType::REG_EXPAND_SZ => {
                    String::from_reg_value(&value).ok().map(RawValue::Text)
                }
                RegType::REG_DWORD => u32::from_reg_value(&value).ok().map(RawValue::Number),
                _ => None,
            };
            if let Some(parsed) = parsed {
                values.insert(name, parsed);
            }
        }
        values
    }

    /// Все подключи Uninstall в одном виде реестра. Недоступный ключ даёт пустой список.
    fn uninstall_from(root: &RegKey, view: u32) -> Vec<RawProgramRecord> {
        let Ok(uninstall) = root.open_subkey_with_flags(UNINSTALL_PATH, KEY_READ | view) else {
            return Vec::new();
        };
        uninstall
            .enum_keys()
            .flatten()
            .filter_map(|name| uninstall.open_subkey_with_flags(&name, KEY_READ).ok())
            .map(|sub| RawProgramRecord {
                source: RawSource::Uninstall,
                values: read_values(&sub),
            })
            .collect()
    }

    pub fn installed() -> Vec<RawProgramRecord> {
        let local = RegKey::predef(HKEY_LOCAL_MACHINE);
        let user = RegKey::predef(HKEY_CURRENT_USER);
        let mut records = uninstall_from(&local, KEY_WOW64_64KEY);
        records.extend(uninstall_from(&local, KEY_WOW64_32KEY));
        records.extend(uninstall_from(&user, 0));
        records
    }

    fn run_from(root: &RegKey, source: StartupSource) -> Vec<RawStartup> {
        let Ok(run) = root.open_subkey_with_flags(RUN_PATH, KEY_READ) else {
            return Vec::new();
        };
        read_values(&run)
            .into_iter()
            .filter_map(|(name, value)| match value {
                RawValue::Text(command) => Some(RawStartup {
                    source,
                    name,
                    command: Some(command),
                }),
                RawValue::Number(_) => None,
            })
            .collect()
    }

    pub fn run_entries() -> Vec<RawStartup> {
        let mut entries = run_from(&RegKey::predef(HKEY_CURRENT_USER), StartupSource::HkcuRun);
        entries.extend(run_from(
            &RegKey::predef(HKEY_LOCAL_MACHINE),
            StartupSource::HklmRun,
        ));
        entries
    }
}

impl ProgramsPort for WindowsPrograms {
    #[cfg(windows)]
    fn installed_records(&self) -> Result<Vec<RawProgramRecord>, CoreError> {
        Ok(registry::installed())
    }

    #[cfg(not(windows))]
    fn installed_records(&self) -> Result<Vec<RawProgramRecord>, CoreError> {
        Err(unsupported())
    }

    #[cfg(windows)]
    fn startup_records(&self) -> Result<Vec<RawStartup>, CoreError> {
        let mut records = registry::run_entries();
        for (dir, source) in startup_folders() {
            records.extend(startup_folder_records(&dir, source));
        }
        Ok(records)
    }

    #[cfg(not(windows))]
    fn startup_records(&self) -> Result<Vec<RawStartup>, CoreError> {
        Err(unsupported())
    }
}

#[cfg(not(windows))]
fn unsupported() -> CoreError {
    CoreError::Unsupported {
        feature: "Программы".into(),
        os: std::env::consts::OS.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-startup-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn startup_folder_lists_shortcut_names_without_extension() {
        let dir = temp_dir("names");
        std::fs::write(dir.join("Messenger.lnk"), b"x").unwrap();
        std::fs::write(dir.join("Notes.txt"), b"x").unwrap();
        std::fs::write(dir.join("desktop.ini"), b"x").unwrap();

        let mut names: Vec<String> = startup_folder_records(&dir, StartupSource::UserStartupFolder)
            .into_iter()
            .map(|record| record.name)
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec!["Messenger".to_string(), "Notes.txt".to_string()]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_startup_folder_gives_no_entries() {
        let dir = std::env::temp_dir().join("ochistka-startup-missing-does-not-exist");
        assert!(startup_folder_records(&dir, StartupSource::CommonStartupFolder).is_empty());
    }

    fn folder_with_files(name: &str, count: usize) -> PathBuf {
        let dir = temp_dir(name);
        let app = dir.join("App");
        std::fs::create_dir_all(app.join("sub")).unwrap();
        for index in 0..count {
            std::fs::write(app.join(format!("f{index}.bin")), vec![0u8; 10]).unwrap();
        }
        std::fs::write(app.join("sub").join("deep.bin"), vec![0u8; 5]).unwrap();
        dir
    }

    #[test]
    fn measure_folder_sums_a_real_temp_directory() {
        let dir = folder_with_files("measure", 2);
        let app = dir.join("App").to_string_lossy().into_owned();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let report = ochistka_core::programs::measure_folder(
            &crate::windows_cleanup::WindowsFs,
            &app,
            &cancel,
            ochistka_core::programs::FOLDER_LIMITS,
        )
        .unwrap();
        assert_eq!((report.bytes, report.files, report.complete), (25, 3, true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn measure_folder_reports_partial_size_when_budget_is_exceeded() {
        let dir = folder_with_files("budget", 5);
        let app = dir.join("App").to_string_lossy().into_owned();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let limits = ochistka_core::fs::Limits {
            max_entries: 3,
            max_depth: 24,
        };
        let report = ochistka_core::programs::measure_folder(
            &crate::windows_cleanup::WindowsFs,
            &app,
            &cancel,
            limits,
        )
        .unwrap();
        assert!(!report.complete, "budget of 3 entries cannot cover 6 files");
        assert!(report.bytes < 55, "partial sum: {}", report.bytes);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn measure_folder_stops_on_cancel_in_a_real_temp_directory() {
        let dir = folder_with_files("cancel", 2);
        let app = dir.join("App").to_string_lossy().into_owned();
        let cancel = std::sync::atomic::AtomicBool::new(true);
        let result = ochistka_core::programs::measure_folder(
            &crate::windows_cleanup::WindowsFs,
            &app,
            &cancel,
            ochistka_core::programs::FOLDER_LIMITS,
        );
        assert_eq!(result, Err(ochistka_core::fs::CleanupError::Cancelled));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(not(windows))]
    #[test]
    fn registry_reads_report_unsupported_outside_windows() {
        assert!(matches!(
            WindowsPrograms::new().installed_records(),
            Err(CoreError::Unsupported { .. })
        ));
        assert!(matches!(
            WindowsPrograms::new().startup_records(),
            Err(CoreError::Unsupported { .. })
        ));
    }
}
