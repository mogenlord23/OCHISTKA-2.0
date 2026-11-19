//! Адаптер «Программ» Windows: ключи Uninstall и Run реестра и папки «Автозагрузка». Только чтение.
//! Реестр читает `winreg` (безопасная обёртка) только под `cfg(windows)`; на других ОС методы
//! возвращают `Unsupported`. Папки читает `std::fs`, поэтому их разбор проверяется и на Linux.
//! Реестр пакетов Microsoft Store читается только на чтение: пакеты пользователя (HKCU) и
//! общий список (HKLM). Разбор имени пакета и записей вынесен в чистые функции.

use std::collections::BTreeMap;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;

use ochistka_core::programs::icons::FileStamp;
#[cfg(windows)]
use ochistka_core::programs::icons::{base64_encode, ICON_PX};
#[cfg_attr(not(windows), allow(unused_imports))]
use ochistka_core::programs::uninstall::UninstallCommand;
#[cfg_attr(not(windows), allow(unused_imports))]
use ochistka_core::programs::StartupToggle;
use ochistka_core::programs::{
    ProgramsPort, RawProgramRecord, RawSource, RawStartup, RawValue, StartupSource,
};
use ochistka_core::system::CoreError;

#[cfg(windows)]
const UNINSTALL_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
#[cfg(windows)]
const RUN_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const USER_STARTUP_SUFFIX: &str = r"Microsoft\Windows\Start Menu\Programs\Startup";
#[cfg(windows)]
const COMMON_STARTUP_SUFFIX: &str = r"Microsoft\Windows\Start Menu\Programs\StartUp";
/// Пакеты Store текущего пользователя: читается без прав администратора.
#[cfg(windows)]
const STORE_USER_PACKAGES: &str = r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";
/// Общий список пакетов Store (все пользователи).
#[cfg(windows)]
const STORE_ALL_PACKAGES: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Appx\PackageRepository\Packages";

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
                    .unwrap_or_else(|| file_name.clone())
            } else {
                file_name.clone()
            };
            Some(RawStartup {
                source,
                name,
                command: None,
                value_name: file_name,
                approved: None,
                backup: None,
            })
        })
        .collect()
}

/// Имя пакета Store вида `Имя_Версия_Архитектура__ИдентификаторИздателя`: имя и версия.
/// `None`, если имени нет.
pub fn parse_package_full_name(full_name: &str) -> Option<(String, Option<String>)> {
    let mut parts = full_name.split('_');
    let name = parts
        .next()
        .map(str::trim)
        .filter(|name| !name.is_empty())?;
    let version = parts
        .next()
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(str::to_string);
    Some((name.to_string(), version))
}

/// Запись пакета Store в виде записи программы. `None` — фреймворки, ресурсные пакеты
/// и записи без имени: их в списке программ не показываем.
/// Отображаемое имя из реестра бывает ссылкой (`ms-resource:`, `@...`), тогда берём имя пакета.
pub fn store_record(
    package_full_name: &str,
    values: &BTreeMap<String, RawValue>,
) -> Option<RawProgramRecord> {
    if matches!(values.get("IsFramework"), Some(RawValue::Number(1)))
        || matches!(values.get("IsResourcePackage"), Some(RawValue::Number(1)))
    {
        return None;
    }
    let (package_name, version) = parse_package_full_name(package_full_name)?;
    let display = match values.get("DisplayName") {
        Some(RawValue::Text(text))
            if !text.trim().is_empty()
                && !text.trim_start().starts_with("ms-resource:")
                && !text.trim_start().starts_with('@') =>
        {
            text.trim().to_string()
        }
        _ => package_name,
    };
    let mut out = BTreeMap::new();
    out.insert("DisplayName".to_string(), RawValue::Text(display));
    if let Some(version) = version {
        out.insert("DisplayVersion".to_string(), RawValue::Text(version));
    }
    for location in ["PackageRootFolder", "Path"] {
        if let Some(RawValue::Text(path)) = values.get(location) {
            out.insert("InstallLocation".to_string(), RawValue::Text(path.clone()));
            break;
        }
    }
    Some(RawProgramRecord {
        source: RawSource::Store,
        values: out,
    })
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
    use std::borrow::Cow;
    use winreg::enums::{
        RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, KEY_WOW64_32KEY,
        KEY_WOW64_64KEY,
    };

    use ochistka_core::programs::{startup_state, StartupToggle};
    use ochistka_core::system::CoreError;
    use winreg::types::FromRegValue;
    use winreg::{RegKey, RegValue};

    use super::{store_record, RUN_PATH, STORE_ALL_PACKAGES, STORE_USER_PACKAGES, UNINSTALL_PATH};

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
        records.extend(store());
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
                    value_name: name.clone(),
                    name,
                    command: Some(command),
                    approved: None,
                    backup: None,
                }),
                RawValue::Number(_) => None,
            })
            .collect()
    }

    /// Пакеты Store из одного корня реестра. Недоступный список даёт пустой результат.
    fn store_from(root: &RegKey, path: &str) -> Vec<RawProgramRecord> {
        let Ok(packages) = root.open_subkey_with_flags(path, KEY_READ) else {
            return Vec::new();
        };
        packages
            .enum_keys()
            .flatten()
            .filter_map(|name| {
                let sub = packages.open_subkey_with_flags(&name, KEY_READ).ok()?;
                store_record(&name, &read_values(&sub))
            })
            .collect()
    }

    pub fn store() -> Vec<RawProgramRecord> {
        let mut records = store_from(&RegKey::predef(HKEY_CURRENT_USER), STORE_USER_PACKAGES);
        records.extend(store_from(
            &RegKey::predef(HKEY_LOCAL_MACHINE),
            STORE_ALL_PACKAGES,
        ));
        records
    }

    const APPROVED_ROOT: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";
    const BACKUP_ROOT: &str = r"Software\OCHISTKA\StartupBackup";

    fn hive(source: StartupSource) -> RegKey {
        RegKey::predef(if startup_state::is_machine_wide(source) {
            HKEY_LOCAL_MACHINE
        } else {
            HKEY_CURRENT_USER
        })
    }

    fn approved_path(source: StartupSource) -> String {
        format!(
            r"{APPROVED_ROOT}\{}",
            startup_state::approved_subkey(source)
        )
    }

    /// Флаг `StartupApproved` записи; `None`, если флага нет.
    pub fn read_approved(source: StartupSource, name: &str) -> Option<Vec<u8>> {
        let key = hive(source)
            .open_subkey_with_flags(approved_path(source), KEY_READ)
            .ok()?;
        key.get_raw_value(name).ok().map(|v| v.bytes.to_vec())
    }

    /// Состояние до отключения из OCHISTKA (наша резервная копия в HKCU).
    pub fn read_backup(source: StartupSource, name: &str) -> Option<Vec<u8>> {
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(format!(r"{BACKUP_ROOT}\{}", source.id()), KEY_READ)
            .ok()?;
        key.get_raw_value(name).ok().map(|v| v.bytes.to_vec())
    }

    /// Записывает флаг. Прежнее значение сохраняется в резервную копию (один раз) и
    /// удаляется из неё, когда запись снова включена.
    pub fn set_enabled(
        source: StartupSource,
        name: &str,
        enable: bool,
    ) -> Result<StartupToggle, CoreError> {
        let denied = |err: std::io::Error| {
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                CoreError::InvalidTarget(
                    "для этой записи нужны права администратора, а приложение запущено без них"
                        .into(),
                )
            } else {
                CoreError::Probe(format!("реестр: {err}"))
            }
        };
        let (key, _) = hive(source)
            .create_subkey_with_flags(approved_path(source), KEY_READ | KEY_SET_VALUE)
            .map_err(denied)?;
        let current = key.get_raw_value(name).ok().map(|v| v.bytes.to_vec());
        let previous_enabled = current
            .as_deref()
            .and_then(startup_state::is_enabled)
            .unwrap_or(true);
        let (backup, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey_with_flags(
                format!(r"{BACKUP_ROOT}\{}", source.id()),
                KEY_READ | KEY_SET_VALUE,
            )
            .map_err(denied)?;
        if !enable && backup.get_raw_value(name).is_err() {
            let original = current
                .clone()
                .unwrap_or_else(|| startup_state::toggled_bytes(None, true, 0));
            backup
                .set_raw_value(
                    name,
                    &RegValue {
                        bytes: Cow::Owned(original),
                        vtype: RegType::REG_BINARY,
                    },
                )
                .map_err(denied)?;
        }
        let bytes = startup_state::toggled_bytes(current.as_deref(), enable, super::filetime_now());
        key.set_raw_value(
            name,
            &RegValue {
                bytes: Cow::Owned(bytes),
                vtype: RegType::REG_BINARY,
            },
        )
        .map_err(denied)?;
        if enable {
            let _ = backup.delete_value(name);
        }
        Ok(StartupToggle {
            previous_enabled,
            enabled: enable,
        })
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
        for record in &mut records {
            record.approved = registry::read_approved(record.source, &record.value_name);
            record.backup = registry::read_backup(record.source, &record.value_name);
        }
        Ok(records)
    }

    fn list_file_names(&self, dir: &str) -> Result<Vec<String>, CoreError> {
        list_file_names(dir)
    }

    fn read_small_file(&self, path: &str, max: usize) -> Result<Vec<u8>, CoreError> {
        read_small_file(path, max)
    }

    fn file_stamp(&self, path: &str) -> Option<FileStamp> {
        file_stamp(path)
    }

    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok().filter(|value| !value.is_empty())
    }

    #[cfg(windows)]
    fn icon_png_base64(&self, path: &str) -> Result<String, CoreError> {
        let safe = ochistka_core::programs::icons::validate_icon_path(path)
            .map_err(CoreError::InvalidTarget)?;
        if !Path::new(&safe).is_file() {
            return Err(CoreError::InvalidTarget("файл значка не найден".into()));
        }
        // Самый крупный доступный размер (256, затем 96 и 48), дальше плавное уменьшение
        // до ICON_PX (плитка x2 для HiDPI). Пустой значок пробуем следующим размером.
        for size in [
            windows_icons::IconSize::ExtraLarge,
            windows_icons::IconSize::Large,
            windows_icons::IconSize::Medium,
        ] {
            let Ok(image) = windows_icons::get_icon_by_path_with_size(&safe, size) else {
                continue;
            };
            if let Some(png) = crate::icon_image::icon_png(&image, ICON_PX) {
                return Ok(base64_encode(&png));
            }
        }
        Err(CoreError::Probe("у файла нет значка".into()))
    }

    fn reveal_in_explorer(&self, path: &str) -> Result<(), ochistka_core::reveal::RevealError> {
        super::explorer::reveal(path)
    }

    #[cfg(windows)]
    fn run_uninstaller(&self, command: &UninstallCommand) -> Result<(), CoreError> {
        use std::os::windows::process::CommandExt;
        if !Path::new(&command.exe).is_file() {
            return Err(CoreError::InvalidTarget(
                "программа удаления не найдена по указанному пути".into(),
            ));
        }
        let mut process = std::process::Command::new(&command.exe);
        if !command.raw_args.is_empty() {
            process.raw_arg(&command.raw_args);
        }
        let mut child = process
            .spawn()
            .map_err(|err| CoreError::Probe(format!("не удалось запустить удаление: {err}")))?;
        // Ждём завершения в фоне, чтобы не оставлять процесс-зомби; интерфейс не блокируется.
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }

    #[cfg(windows)]
    fn set_startup_enabled(
        &self,
        source: StartupSource,
        value_name: &str,
        enable: bool,
    ) -> Result<StartupToggle, CoreError> {
        if !ochistka_core::programs::startup_state::valid_value_name(value_name) {
            return Err(CoreError::InvalidTarget("недопустимое имя записи".into()));
        }
        registry::set_enabled(source, value_name, enable)
    }

    #[cfg(windows)]
    fn system_root(&self) -> String {
        std::env::var("SystemRoot")
            .ok()
            .filter(|root| !root.trim().is_empty())
            .unwrap_or_else(|| ochistka_core::programs::uninstall::DEFAULT_SYSTEM_ROOT.to_string())
    }

    #[cfg(not(windows))]
    fn startup_records(&self) -> Result<Vec<RawStartup>, CoreError> {
        Err(unsupported())
    }
}

/// Имена файлов папки (без вложенных папок), только чтение. Путь проверяется как путь установки.
pub fn list_file_names(dir: &str) -> Result<Vec<String>, CoreError> {
    let absolute =
        ochistka_core::programs::icons::valid_location(dir) || Path::new(dir).is_absolute();
    if !absolute || dir.contains('%') || dir.split(['\\', '/']).any(|part| part == "..") {
        return Err(CoreError::InvalidTarget("недопустимая папка".into()));
    }
    let entries =
        std::fs::read_dir(dir).map_err(|err| CoreError::Probe(format!("папка: {err}")))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect())
}

/// Читает небольшой обычный файл целиком; файл больше `max` байт не читается.
pub fn read_small_file(path: &str, max: usize) -> Result<Vec<u8>, CoreError> {
    use std::io::Read;
    if !Path::new(path).is_absolute() || path.split(['\\', '/']).any(|part| part == "..") {
        return Err(CoreError::InvalidTarget("недопустимый путь".into()));
    }
    let file = std::fs::File::open(path).map_err(|err| CoreError::Probe(format!("файл: {err}")))?;
    let meta = file
        .metadata()
        .map_err(|err| CoreError::Probe(format!("файл: {err}")))?;
    if !meta.is_file() || meta.len() > max as u64 {
        return Err(CoreError::InvalidTarget(
            "файл не подходит по размеру".into(),
        ));
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| CoreError::Probe(format!("файл: {err}")))?;
    Ok(bytes)
}

/// Время изменения (секунды) и размер файла для ключа кэша значков.
pub fn file_stamp(path: &str) -> Option<FileStamp> {
    let meta = std::fs::metadata(path).ok()?;
    let modified_secs = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(FileStamp {
        modified_secs,
        len: meta.len(),
    })
}

/// Текущее время в формате FILETIME (100-нс интервалы с 1601 года).
#[cfg_attr(not(windows), allow(dead_code))]
fn filetime_now() -> u64 {
    const UNIX_TO_FILETIME_SECS: u64 = 11_644_473_600;
    let since_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    (since_unix.as_secs() + UNIX_TO_FILETIME_SECS) * 10_000_000
        + u64::from(since_unix.subsec_nanos() / 100)
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
    use ochistka_core::programs::{parse_program, ProgramKind, RawSource, RawValue};
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-startup-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn filetime_is_after_2020_and_in_100ns_ticks() {
        // 2020-01-01 в FILETIME.
        assert!(filetime_now() > 132_223_104_000_000_000);
    }

    #[test]
    fn startup_folder_keeps_the_file_name_as_the_approved_key() {
        let dir = temp_dir("keys");
        std::fs::write(dir.join("Messenger.lnk"), b"x").unwrap();
        let records = startup_folder_records(&dir, StartupSource::UserStartupFolder);
        assert_eq!(records[0].name, "Messenger");
        assert_eq!(records[0].value_name, "Messenger.lnk");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(not(windows))]
    #[test]
    fn mutating_operations_are_unsupported_outside_windows() {
        let port = WindowsPrograms::new();
        assert!(port
            .set_startup_enabled(StartupSource::HkcuRun, "X", false)
            .is_err());
        assert!(port.icon_png_base64(r"C:\A\a.exe").is_err());
        let command = UninstallCommand {
            exe: r"C:\A\u.exe".into(),
            raw_args: String::new(),
        };
        assert!(port.run_uninstaller(&command).is_err());
    }

    #[test]
    fn small_file_reader_respects_the_limit_and_refuses_parent_references() {
        let dir = temp_dir("small-file");
        let file = dir.join("a.bin");
        std::fs::write(&file, b"12345").unwrap();
        let path = file.to_string_lossy().to_string();
        assert_eq!(read_small_file(&path, 10).unwrap(), b"12345");
        assert!(read_small_file(&path, 4).is_err());
        assert!(read_small_file(&format!("{}/../a.bin", dir.display()), 10).is_err());
        assert!(read_small_file("relative.bin", 10).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_listing_returns_files_only_and_refuses_bad_paths() {
        let dir = temp_dir("list-files");
        std::fs::write(dir.join("Tool.exe"), b"x").unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let listed = list_file_names(&dir.to_string_lossy()).unwrap();
        assert_eq!(listed, vec!["Tool.exe".to_string()]);
        assert!(list_file_names("relative").is_err());
        assert!(list_file_names(&format!("{}/../x", dir.display())).is_err());
        assert!(list_file_names("%ProgramFiles%").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_stamp_changes_with_content_size_and_is_none_for_missing_files() {
        let dir = temp_dir("stamp");
        let file = dir.join("a.exe");
        std::fs::write(&file, b"1").unwrap();
        let first = file_stamp(&file.to_string_lossy()).unwrap();
        std::fs::write(&file, b"12").unwrap();
        let second = file_stamp(&file.to_string_lossy()).unwrap();
        assert_ne!(first.len, second.len);
        assert!(file_stamp(&dir.join("none.exe").to_string_lossy()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
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

    fn store_values(pairs: &[(&str, RawValue)]) -> BTreeMap<String, RawValue> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn package_full_name_gives_name_and_version() {
        assert_eq!(
            parse_package_full_name("Microsoft.WindowsCalculator_11.2301.0.0_x64__8wekyb3d8bbwe"),
            Some((
                "Microsoft.WindowsCalculator".to_string(),
                Some("11.2301.0.0".to_string())
            ))
        );
        assert_eq!(
            parse_package_full_name("Plain"),
            Some(("Plain".to_string(), None))
        );
        assert_eq!(parse_package_full_name("_1.0"), None);
    }

    #[test]
    fn store_package_becomes_a_store_program_with_its_location() {
        let values = store_values(&[
            ("DisplayName", RawValue::Text("Калькулятор".into())),
            (
                "PackageRootFolder",
                RawValue::Text("C:\\Program Files\\WindowsApps\\Calc".into()),
            ),
        ]);
        let record = store_record("Microsoft.Calc_1.2.3.4_x64__8wekyb", &values).expect("record");
        assert_eq!(record.source, RawSource::Store);
        let program = parse_program(&record).expect("program");
        assert_eq!(program.name, "Калькулятор");
        assert_eq!(program.version.as_deref(), Some("1.2.3.4"));
        assert_eq!(program.kind, ProgramKind::Store);
        assert_eq!(program.disk_letter.as_deref(), Some("C:"));
    }

    #[test]
    fn store_name_falls_back_to_package_name_for_resource_links() {
        let values = store_values(&[("DisplayName", RawValue::Text("ms-resource:AppName".into()))]);
        let record = store_record("Contoso.App_2.0_x64__abc", &values).expect("record");
        let program = parse_program(&record).expect("program");
        assert_eq!(program.name, "Contoso.App");
        assert_eq!(program.install_location, None);
    }

    #[test]
    fn store_frameworks_and_resource_packages_are_skipped() {
        let framework = store_values(&[("IsFramework", RawValue::Number(1))]);
        let resource = store_values(&[("IsResourcePackage", RawValue::Number(1))]);
        assert!(store_record("Microsoft.VCLibs_14.0_x64__abc", &framework).is_none());
        assert!(store_record("Microsoft.Res_1.0_x64__abc", &resource).is_none());
    }

    #[test]
    fn store_uses_path_value_when_root_folder_is_missing() {
        let values = store_values(&[("Path", RawValue::Text("D:\\Apps\\Game".into()))]);
        let record = store_record("Game.Thing_1.0_x64__abc", &values).expect("record");
        let program = parse_program(&record).expect("program");
        assert_eq!(program.disk_letter.as_deref(), Some("D:"));
    }
}
