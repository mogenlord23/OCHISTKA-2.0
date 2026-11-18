//! Раздел «Программы»: список установленных программ, размер папки и автозагрузка. Только чтение.
//! Чистый код: реестр и папки читает адаптер платформы через `ProgramsPort` и `FsPort`;
//! здесь — разбор записей в DTO, фильтры, бюджет обхода и проверка путей.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

use serde::Serialize;

use crate::fs::{walk, Budget, CleanupError, FsPort, Limits};
use crate::system::CoreError;

/// Лимит для подсчёта размера папки программы. Больше — обход останавливается, результат неполный.
pub const FOLDER_LIMITS: Limits = Limits {
    max_entries: 200_000,
    max_depth: 24,
};

/// Откуда пришла запись установленной программы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawSource {
    /// Ключ `...\CurrentVersion\Uninstall` (HKLM 64/32-битный вид или HKCU).
    Uninstall,
    /// Реестр пакетов Microsoft Store. Чтение пока не подключено (см. docs/platform-status.md).
    Store,
}

/// Значение реестра: текст (REG_SZ, REG_EXPAND_SZ) или число (REG_DWORD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawValue {
    Text(String),
    Number(u32),
}

/// Одна запись из реестра, уже прочитанная адаптером. Значения — по имени.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProgramRecord {
    pub source: RawSource,
    pub values: BTreeMap<String, RawValue>,
}

/// Тип программы для интерфейса.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProgramKind {
    /// Классическая программа: есть строка удаления или установка через Windows Installer.
    Win32,
    /// Приложение из Microsoft Store.
    Store,
    /// Всё остальное.
    Other,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledProgram {
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    /// Оценка размера из реестра (`EstimatedSize`, в КБ), переведена в байты.
    pub size_bytes: Option<u64>,
    pub install_location: Option<String>,
    /// Буква диска вида `C:`, если известна из папки установки.
    pub disk_letter: Option<String>,
    pub kind: ProgramKind,
}

/// Источник записи автозагрузки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupSource {
    HkcuRun,
    HklmRun,
    UserStartupFolder,
    CommonStartupFolder,
}

impl StartupSource {
    /// Подпись для интерфейса.
    pub fn label(self) -> &'static str {
        match self {
            Self::HkcuRun => "Реестр: HKCU, Run",
            Self::HklmRun => "Реестр: HKLM, Run",
            Self::UserStartupFolder => "Папка «Автозагрузка» пользователя",
            Self::CommonStartupFolder => "Папка «Автозагрузка» всех пользователей",
        }
    }
}

/// Сырая запись автозагрузки: имя и команда (для папок команды нет).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawStartup {
    pub source: StartupSource,
    pub name: String,
    pub command: Option<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupEntry {
    pub name: String,
    pub command: Option<String>,
    pub location: String,
}

/// Порт чтения реестра и папок автозагрузки. Реализация на Windows — в `crates/platform`.
pub trait ProgramsPort: Send + Sync {
    /// Записи установленных программ из всех ключей Uninstall. Недоступный ключ пропускается.
    fn installed_records(&self) -> Result<Vec<RawProgramRecord>, CoreError>;
    /// Записи автозагрузки: ключи Run и папки «Автозагрузка». Только чтение.
    fn startup_records(&self) -> Result<Vec<RawStartup>, CoreError>;
}

/// Текстовое значение без пробелов по краям; пустая строка — отсутствие значения.
fn text(values: &BTreeMap<String, RawValue>, name: &str) -> Option<String> {
    match values.get(name)? {
        RawValue::Text(value) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        RawValue::Number(_) => None,
    }
}

/// Число: из REG_DWORD или из текста с цифрами (некоторые программы пишут так).
fn number(values: &BTreeMap<String, RawValue>, name: &str) -> Option<u64> {
    match values.get(name)? {
        RawValue::Number(value) => Some(u64::from(*value)),
        RawValue::Text(value) => value.trim().parse::<u64>().ok(),
    }
}

/// Буква диска из пути: `C:\Program Files\X` → `C:`. Путь без буквы (например, UNC) → `None`.
pub fn disk_letter_of(path: &str) -> Option<String> {
    let mut chars = path.trim().chars();
    let letter = chars.next()?;
    if !letter.is_ascii_alphabetic() || chars.next()? != ':' {
        return None;
    }
    Some(format!("{}:", letter.to_ascii_uppercase()))
}

/// Разбирает одну запись реестра в программу. `None` — записи, которые не показываем:
/// без имени, системные компоненты (`SystemComponent = 1`) и обновления (`ParentKeyName`).
pub fn parse_program(record: &RawProgramRecord) -> Option<InstalledProgram> {
    let values = &record.values;
    let name = text(values, "DisplayName")?;
    if number(values, "SystemComponent") == Some(1) || text(values, "ParentKeyName").is_some() {
        return None;
    }
    let install_location = text(values, "InstallLocation").map(|path| {
        let trimmed = path.trim_end_matches('\\');
        if trimmed.is_empty() {
            path
        } else {
            trimmed.to_string()
        }
    });
    let kind = match record.source {
        RawSource::Store => ProgramKind::Store,
        RawSource::Uninstall => {
            if text(values, "UninstallString").is_some()
                || number(values, "WindowsInstaller") == Some(1)
            {
                ProgramKind::Win32
            } else {
                ProgramKind::Other
            }
        }
    };
    Some(InstalledProgram {
        name,
        publisher: text(values, "Publisher"),
        version: text(values, "DisplayVersion"),
        size_bytes: number(values, "EstimatedSize").map(|kb| kb * 1024),
        disk_letter: install_location.as_deref().and_then(disk_letter_of),
        install_location,
        kind,
    })
}

/// Список программ: разбор, удаление повторов (имя, версия, издатель без учёта регистра), сортировка по имени.
pub fn build_inventory(records: &[RawProgramRecord]) -> Vec<InstalledProgram> {
    let mut programs: Vec<InstalledProgram> = records.iter().filter_map(parse_program).collect();
    programs.sort_by_key(|program| program.name.to_lowercase());
    let mut seen = std::collections::BTreeSet::new();
    programs.retain(|program| {
        seen.insert((
            program.name.to_lowercase(),
            program.version.clone().unwrap_or_default(),
            program.publisher.clone().unwrap_or_default().to_lowercase(),
        ))
    });
    programs
}

/// Записи автозагрузки: без пустых имён, без повторов (источник и имя), по имени.
pub fn build_startup(records: Vec<RawStartup>) -> Vec<StartupEntry> {
    let mut entries: Vec<StartupEntry> = records
        .into_iter()
        .filter_map(|record| {
            let name = record.name.trim().to_string();
            if name.is_empty() {
                return None;
            }
            Some(StartupEntry {
                name,
                command: record
                    .command
                    .map(|command| command.trim().to_string())
                    .filter(|command| !command.is_empty()),
                location: record.source.label().to_string(),
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.location.cmp(&b.location))
    });
    entries.dedup_by(|a, b| a.name == b.name && a.location == b.location);
    entries
}

/// Результат подсчёта размера папки. При `complete == false` цифры — нижняя оценка.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderMeasure {
    pub path: String,
    pub bytes: u64,
    pub files: u64,
    pub complete: bool,
}

/// Проверка пути для подсчёта: абсолютный путь с буквой диска или UNC, без `..` и управляющих символов.
/// Системные папки разрешены: здесь только чтение размеров.
pub fn validate_measure_path(path: &str) -> Result<String, CleanupError> {
    let trimmed = path.trim();
    if trimmed.chars().any(char::is_control) {
        return Err(CleanupError::Refused(format!(
            "управляющие символы: {trimmed:?}"
        )));
    }
    let has_drive = disk_letter_of(trimmed).is_some() && trimmed.as_bytes().get(2) == Some(&b'\\');
    if !(has_drive || trimmed.starts_with("\\\\")) {
        return Err(CleanupError::Refused(format!(
            "путь не абсолютный: {trimmed}"
        )));
    }
    if trimmed.split('\\').any(|part| part == "..") {
        return Err(CleanupError::Refused(format!(
            "путь содержит «..»: {trimmed}"
        )));
    }
    Ok(trimmed.trim_end_matches('\\').to_string())
}

/// Размер папки установки. Только чтение; обход ограничен `limits`, отмена — флагом `cancel`.
/// Путь проверяет вызывающий через `validate_measure_path` (граница команды).
/// Если папки нет, возвращает `NotFound`; если лимит записей исчерпан, `complete` будет `false`.
pub fn measure_folder(
    fs: &dyn FsPort,
    root: &str,
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<FolderMeasure, CleanupError> {
    let root = root.to_string();
    let meta = fs.metadata(&root)?;
    if !meta.is_dir {
        return Err(CleanupError::NotFound(root));
    }
    let mut budget = Budget::new(limits.max_entries);
    let mut bytes = 0u64;
    let mut files = 0u64;
    let complete = walk(
        fs,
        &root,
        cancel,
        &mut budget,
        limits.max_depth,
        &mut |entry| {
            if !entry.meta.is_dir {
                bytes += entry.meta.size;
                files += 1;
            }
        },
    )?;
    Ok(FolderMeasure {
        path: root,
        bytes,
        files,
        complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake_fs::FakeFs;
    use std::sync::atomic::Ordering;

    fn record(source: RawSource, pairs: &[(&str, RawValue)]) -> RawProgramRecord {
        RawProgramRecord {
            source,
            values: pairs
                .iter()
                .map(|(name, value)| ((*name).to_string(), value.clone()))
                .collect(),
        }
    }

    fn text_value(value: &str) -> RawValue {
        RawValue::Text(value.to_string())
    }

    #[test]
    fn parses_a_full_uninstall_record_into_the_dto() {
        let rec = record(
            RawSource::Uninstall,
            &[
                ("DisplayName", text_value("  Test Tool  ")),
                ("DisplayVersion", text_value("1.2.3")),
                ("Publisher", text_value("Acme")),
                ("EstimatedSize", RawValue::Number(2048)),
                (
                    "InstallLocation",
                    text_value("c:\\Program Files\\Test Tool\\"),
                ),
                ("UninstallString", text_value("\"C:\\x\\uninstall.exe\"")),
            ],
        );
        let program = parse_program(&rec).expect("program");
        assert_eq!(program.name, "Test Tool");
        assert_eq!(program.version.as_deref(), Some("1.2.3"));
        assert_eq!(program.publisher.as_deref(), Some("Acme"));
        assert_eq!(program.size_bytes, Some(2048 * 1024));
        assert_eq!(
            program.install_location.as_deref(),
            Some("c:\\Program Files\\Test Tool")
        );
        assert_eq!(program.disk_letter.as_deref(), Some("C:"));
        assert_eq!(program.kind, ProgramKind::Win32);
    }

    #[test]
    fn missing_optional_fields_become_none_and_kind_other() {
        let rec = record(
            RawSource::Uninstall,
            &[
                ("DisplayName", text_value("Bare")),
                ("Publisher", text_value("   ")),
            ],
        );
        let program = parse_program(&rec).expect("program");
        assert_eq!(program.publisher, None);
        assert_eq!(program.version, None);
        assert_eq!(program.size_bytes, None);
        assert_eq!(program.install_location, None);
        assert_eq!(program.disk_letter, None);
        assert_eq!(program.kind, ProgramKind::Other);
    }

    #[test]
    fn windows_installer_flag_marks_a_win32_program() {
        let rec = record(
            RawSource::Uninstall,
            &[
                ("DisplayName", text_value("Msi App")),
                ("WindowsInstaller", RawValue::Number(1)),
            ],
        );
        assert_eq!(parse_program(&rec).unwrap().kind, ProgramKind::Win32);
    }

    #[test]
    fn skips_nameless_system_components_and_updates() {
        let nameless = record(RawSource::Uninstall, &[("Publisher", text_value("X"))]);
        let system = record(
            RawSource::Uninstall,
            &[
                ("DisplayName", text_value("Runtime")),
                ("SystemComponent", RawValue::Number(1)),
            ],
        );
        let patch = record(
            RawSource::Uninstall,
            &[
                ("DisplayName", text_value("Update KB1")),
                ("ParentKeyName", text_value("App")),
            ],
        );
        assert!(parse_program(&nameless).is_none());
        assert!(parse_program(&system).is_none());
        assert!(parse_program(&patch).is_none());
    }

    #[test]
    fn store_source_is_kind_store() {
        let rec = record(RawSource::Store, &[("DisplayName", text_value("Calc"))]);
        assert_eq!(parse_program(&rec).unwrap().kind, ProgramKind::Store);
    }

    #[test]
    fn text_size_is_read_as_a_number() {
        let rec = record(
            RawSource::Uninstall,
            &[
                ("DisplayName", text_value("T")),
                ("EstimatedSize", text_value("10")),
            ],
        );
        assert_eq!(parse_program(&rec).unwrap().size_bytes, Some(10 * 1024));
    }

    #[test]
    fn inventory_sorts_by_name_and_drops_duplicates() {
        let a = record(
            RawSource::Uninstall,
            &[
                ("DisplayName", text_value("beta")),
                ("DisplayVersion", text_value("1")),
            ],
        );
        let b = record(
            RawSource::Uninstall,
            &[("DisplayName", text_value("Alpha"))],
        );
        let again = record(
            RawSource::Uninstall,
            &[
                ("DisplayName", text_value("BETA")),
                ("DisplayVersion", text_value("1")),
            ],
        );
        let names: Vec<String> = build_inventory(&[a, b, again])
            .into_iter()
            .map(|program| program.name)
            .collect();
        assert_eq!(names, vec!["Alpha", "beta"]);
    }

    #[test]
    fn disk_letter_needs_a_drive_prefix() {
        assert_eq!(disk_letter_of("d:\\Games"), Some("D:".to_string()));
        assert_eq!(disk_letter_of("\\\\server\\share"), None);
        assert_eq!(disk_letter_of(""), None);
    }

    #[test]
    fn startup_entries_are_sorted_trimmed_and_deduplicated() {
        let entries = build_startup(vec![
            RawStartup {
                source: StartupSource::HkcuRun,
                name: "Zeta".into(),
                command: Some("  z.exe ".into()),
            },
            RawStartup {
                source: StartupSource::HklmRun,
                name: "alpha".into(),
                command: Some(String::new()),
            },
            RawStartup {
                source: StartupSource::HkcuRun,
                name: "Zeta".into(),
                command: Some("z.exe".into()),
            },
            RawStartup {
                source: StartupSource::UserStartupFolder,
                name: "  ".into(),
                command: None,
            },
        ]);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "Zeta"]);
        assert_eq!(entries[0].command, None);
        assert_eq!(entries[1].command.as_deref(), Some("z.exe"));
        assert_eq!(entries[1].location, "Реестр: HKCU, Run");
    }

    #[test]
    fn measure_path_must_be_absolute_and_without_parent_references() {
        assert_eq!(
            validate_measure_path("C:\\Program Files\\App\\").unwrap(),
            "C:\\Program Files\\App"
        );
        assert!(validate_measure_path("App\\x").is_err());
        assert!(validate_measure_path("C:\\App\\..\\Windows").is_err());
        assert!(validate_measure_path("C:\\App\\\u{7}").is_err());
        assert!(validate_measure_path("\\\\server\\share\\x").is_ok());
    }

    #[test]
    fn measure_sums_files_below_the_folder_only() {
        let fs = FakeFs::new();
        fs.file("D:\\App\\a.bin", &[0u8; 100], 0);
        fs.file("D:\\App\\sub\\b.bin", &[0u8; 50], 0);
        fs.file("D:\\Other\\c.bin", &[0u8; 999], 0);
        let cancel = AtomicBool::new(false);
        let report = measure_folder(&fs, "D:\\App", &cancel, Limits::default()).unwrap();
        assert_eq!(
            (report.bytes, report.files, report.complete),
            (150, 2, true)
        );
    }

    #[test]
    fn measure_reports_partial_result_when_budget_runs_out() {
        let fs = FakeFs::new();
        fs.file("D:\\App\\a.bin", &[0u8; 1], 0);
        fs.file("D:\\App\\b.bin", &[0u8; 1], 0);
        fs.file("D:\\App\\c.bin", &[0u8; 1], 0);
        let cancel = AtomicBool::new(false);
        let limits = Limits {
            max_entries: 2,
            max_depth: 24,
        };
        let report = measure_folder(&fs, "D:\\App", &cancel, limits).unwrap();
        assert!(!report.complete);
        assert!(report.files < 3);
    }

    #[test]
    fn measure_stops_when_cancelled() {
        let fs = FakeFs::new();
        fs.file("D:\\App\\a.bin", &[0u8; 1], 0);
        let cancel = AtomicBool::new(true);
        assert_eq!(
            measure_folder(&fs, "D:\\App", &cancel, Limits::default()),
            Err(CleanupError::Cancelled)
        );
        cancel.store(false, Ordering::SeqCst);
    }

    #[test]
    fn measure_of_a_missing_folder_is_not_found() {
        let fs = FakeFs::new();
        let cancel = AtomicBool::new(false);
        assert!(matches!(
            measure_folder(&fs, "D:\\Nope", &cancel, Limits::default()),
            Err(CleanupError::NotFound(_))
        ));
    }
}
