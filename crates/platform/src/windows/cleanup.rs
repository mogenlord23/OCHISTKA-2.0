//! Адаптер файловой системы и раскладка папок Windows для очистки и карантина.
//! `std::fs` компилируется везде (тесты идут на temp-папках); раскладку по переменным окружения
//! строит только Windows, на прочих ОС `layout()` возвращает `Unsupported`.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

use ochistka_core::cleanup::CleanupLocation;
use ochistka_core::fs::{CleanupError, DirEntry, FsPort, Meta};
use ochistka_core::layout::{CleanupEnvPort, Layout};
use ochistka_core::readonly::PrivacyCandidate;

/// Флаг Windows: точка повторной обработки (junction, символическая ссылка). Такие пути не обходим.
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
/// Код ошибки Windows «ERROR_NOT_SAME_DEVICE» (17) и код Unix EXDEV (18): перенос между томами.
const CROSS_DEVICE_CODES: [i32; 2] = [17, 18];

/// Файловые операции через `std::fs`. Ссылки и точки повторной обработки помечаются как ссылки.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsFs;

fn io_error(path: &str, err: std::io::Error) -> CleanupError {
    if err.kind() == std::io::ErrorKind::NotFound {
        CleanupError::NotFound(path.to_string())
    } else {
        CleanupError::Io(err.to_string())
    }
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn meta_from(metadata: &fs::Metadata) -> Meta {
    let modified_unix = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    Meta {
        is_dir: metadata.is_dir(),
        is_symlink: metadata.file_type().is_symlink() || is_reparse_point(metadata),
        size: if metadata.is_file() {
            metadata.len()
        } else {
            0
        },
        modified_unix,
    }
}

impl FsPort for WindowsFs {
    fn metadata(&self, path: &str) -> Result<Meta, CleanupError> {
        fs::symlink_metadata(path)
            .map(|metadata| meta_from(&metadata))
            .map_err(|err| io_error(path, err))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>, CleanupError> {
        let reader = fs::read_dir(path).map_err(|err| io_error(path, err))?;
        let mut entries = Vec::new();
        for entry in reader {
            // Ошибку нельзя скрывать: вызывающий должен сообщить о неполном обходе.
            let entry = entry.map_err(|err| io_error(path, err))?;
            let entry_path = entry.path();
            let metadata = fs::symlink_metadata(&entry_path)
                .map_err(|err| io_error(&entry_path.to_string_lossy(), err))?;
            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry_path.to_string_lossy().into_owned(),
                meta: meta_from(&metadata),
            });
        }
        Ok(entries)
    }

    fn child(&self, dir: &str, name: &str) -> String {
        Path::new(dir).join(name).to_string_lossy().into_owned()
    }

    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, CleanupError> {
        fs::File::open(path)
            .map(|file| Box::new(file) as Box<dyn Read + Send>)
            .map_err(|err| io_error(path, err))
    }

    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), CleanupError> {
        fs::write(path, bytes).map_err(|err| io_error(path, err))
    }

    fn create_dir_all(&self, path: &str) -> Result<(), CleanupError> {
        fs::create_dir_all(path).map_err(|err| io_error(path, err))
    }

    /// Перенос файла. Между томами rename не работает: копия, проверка размера и удаление источника.
    fn rename(&self, from: &str, to: &str) -> Result<(), CleanupError> {
        match fs::rename(from, to) {
            Ok(()) => Ok(()),
            Err(err)
                if err
                    .raw_os_error()
                    .is_some_and(|code| CROSS_DEVICE_CODES.contains(&code)) =>
            {
                copy_then_remove(from, to)
            }
            Err(err) => Err(io_error(from, err)),
        }
    }

    fn remove_file(&self, path: &str) -> Result<(), CleanupError> {
        fs::remove_file(path).map_err(|err| io_error(path, err))
    }

    fn remove_dir(&self, path: &str) -> Result<(), CleanupError> {
        fs::remove_dir(path).map_err(|err| io_error(path, err))
    }
}

fn copy_then_remove(from: &str, to: &str) -> Result<(), CleanupError> {
    let expected = fs::metadata(from).map_err(|err| io_error(from, err))?.len();
    fs::copy(from, to).map_err(|err| io_error(to, err))?;
    let copied = fs::metadata(to).map_err(|err| io_error(to, err))?.len();
    if copied != expected {
        let _ = fs::remove_file(to);
        return Err(CleanupError::Io(
            "копия не совпала по размеру, исходный файл не тронут".into(),
        ));
    }
    fs::remove_file(from).map_err(|err| io_error(from, err))
}

/// Переменные окружения, из которых строится раскладка. Отдельная структура, чтобы проверять без реальной среды.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EnvVars {
    pub temp: Option<String>,
    pub tmp: Option<String>,
    pub local_app_data: Option<String>,
    pub app_data: Option<String>,
    pub user_profile: Option<String>,
}

impl EnvVars {
    pub fn from_process() -> Self {
        Self {
            temp: std::env::var("TEMP").ok(),
            tmp: std::env::var("TMP").ok(),
            local_app_data: std::env::var("LOCALAPPDATA").ok(),
            app_data: std::env::var("APPDATA").ok(),
            user_profile: std::env::var("USERPROFILE").ok(),
        }
    }
}

/// Склеивает базу и компоненты пути по правилам ОС.
fn join_all(fs: &dyn FsPort, base: &str, parts: &[&str]) -> String {
    parts
        .iter()
        .fold(base.to_string(), |acc, part| fs.child(&acc, part))
}

fn is_dir(fs: &dyn FsPort, path: &str) -> bool {
    fs.metadata(path)
        .is_ok_and(|meta| meta.is_dir && !meta.is_symlink)
}

/// Подпапки профилей Firefox: каждая папка внутри `Profiles`.
fn firefox_profiles(fs: &dyn FsPort, profiles_dir: &str) -> Vec<(String, String)> {
    fs.read_dir(profiles_dir)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.meta.is_dir && !entry.meta.is_symlink)
        .map(|entry| (entry.name, entry.path))
        .collect()
}

/// Строит раскладку по переменным окружения. Категории и корни попадают только если папка существует.
pub fn build_layout(vars: &EnvVars, fs: &dyn FsPort) -> Result<Layout, CleanupError> {
    let local = vars
        .local_app_data
        .as_deref()
        .ok_or_else(|| CleanupError::Unsupported("не задана папка LOCALAPPDATA".into()))?;
    let roaming = vars.app_data.as_deref().unwrap_or("");
    let profile = vars.user_profile.as_deref().unwrap_or("");

    let mut cleanup: Vec<CleanupLocation> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut add = |id: &str, title: &str, root: String| {
        let key = ochistka_core::policy::normalize(&root);
        if is_dir(fs, &root) && !seen.contains(&key) {
            seen.push(key);
            cleanup.push(CleanupLocation {
                id: id.to_string(),
                title: title.to_string(),
                root,
            });
        }
    };

    for (id, title, value) in [
        ("temp", "Временные файлы (TEMP)", vars.temp.as_deref()),
        ("tmp", "Временные файлы (TMP)", vars.tmp.as_deref()),
    ] {
        if let Some(root) = value.filter(|v| !v.trim().is_empty()) {
            add(id, title, root.to_string());
        }
    }
    add(
        "local-temp",
        "Временные файлы приложений",
        join_all(fs, local, &["Temp"]),
    );
    add(
        "chrome-cache",
        "Кэш Chrome",
        join_all(
            fs,
            local,
            &["Google", "Chrome", "User Data", "Default", "Cache"],
        ),
    );
    add(
        "edge-cache",
        "Кэш Edge",
        join_all(
            fs,
            local,
            &["Microsoft", "Edge", "User Data", "Default", "Cache"],
        ),
    );
    for (name, profile_path) in firefox_profiles(
        fs,
        &join_all(fs, local, &["Mozilla", "Firefox", "Profiles"]),
    ) {
        add(
            &format!("firefox-cache-{name}"),
            &format!("Кэш Firefox ({name})"),
            fs.child(&profile_path, "cache2"),
        );
    }

    let mut privacy = vec![
        PrivacyCandidate {
            id: "recent".into(),
            title: "Недавние файлы".into(),
            path: join_all(fs, roaming, &["Microsoft", "Windows", "Recent"]),
        },
        PrivacyCandidate {
            id: "chrome-history".into(),
            title: "История Chrome".into(),
            path: join_all(
                fs,
                local,
                &["Google", "Chrome", "User Data", "Default", "History"],
            ),
        },
        PrivacyCandidate {
            id: "edge-history".into(),
            title: "История Edge".into(),
            path: join_all(
                fs,
                local,
                &["Microsoft", "Edge", "User Data", "Default", "History"],
            ),
        },
    ];
    for (name, profile_path) in firefox_profiles(
        fs,
        &join_all(fs, roaming, &["Mozilla", "Firefox", "Profiles"]),
    ) {
        privacy.push(PrivacyCandidate {
            id: format!("firefox-history-{name}"),
            title: format!("История Firefox ({name})"),
            path: fs.child(&profile_path, "places.sqlite"),
        });
    }

    let drives = (b'A'..=b'Z')
        .map(|letter| format!("{}:\\", letter as char))
        .filter(|root| fs.metadata(root).is_ok())
        .collect();

    Ok(Layout {
        cleanup,
        downloads: if profile.is_empty() {
            String::new()
        } else {
            join_all(fs, profile, &["Downloads"])
        },
        privacy,
        quarantine_root: join_all(fs, local, &["OCHISTKA", "Quarantine"]),
        drives,
    })
}

/// Раскладка и файловая система Windows для одного запуска приложения.
pub struct WindowsCleanupEnv {
    fs: WindowsFs,
}

impl WindowsCleanupEnv {
    pub fn new() -> Self {
        Self { fs: WindowsFs }
    }
}

impl Default for WindowsCleanupEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl CleanupEnvPort for WindowsCleanupEnv {
    fn layout(&self) -> Result<Layout, CleanupError> {
        build_layout(&EnvVars::from_process(), &self.fs)
    }

    fn fs(&self) -> Result<&dyn FsPort, CleanupError> {
        Ok(&self.fs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("ochistka-cleanup-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn text(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn real_temp_tree_is_counted_and_moved_into_quarantine_and_back() {
        let root = scratch("roundtrip");
        let temp = root.join("Temp");
        fs::create_dir_all(temp.join("sub")).unwrap();
        fs::write(temp.join("a.tmp"), [1u8; 40]).unwrap();
        fs::write(temp.join("sub").join("b.tmp"), [2u8; 60]).unwrap();
        let temp_text = text(&temp);

        let adapter = WindowsFs;
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let policy = ochistka_core::policy::PathPolicy::new(&[&temp_text]).unwrap();
        let locations = vec![CleanupLocation {
            id: "temp".into(),
            title: "Temp".into(),
            root: temp_text.clone(),
        }];
        let analysis = ochistka_core::cleanup::analyze(
            &adapter,
            &policy,
            &locations,
            &cancel,
            ochistka_core::fs::Limits::default(),
        )
        .unwrap();
        assert_eq!(
            (
                analysis.categories[0].file_count,
                analysis.categories[0].bytes
            ),
            (2, 100)
        );

        let quarantine_root = text(&root.join("Quarantine"));
        let quarantine =
            ochistka_core::quarantine::Quarantine::new(&quarantine_root, policy).unwrap();
        let ids = vec!["temp".to_string()];
        let outcome =
            ochistka_core::cleanup::clean_selected(&adapter, &quarantine, &analysis, &ids, 77)
                .unwrap();
        assert_eq!((outcome.moved, outcome.failed), (2, 0));
        assert!(!temp.join("a.tmp").exists());

        let entries = quarantine.list(&adapter).unwrap();
        assert_eq!(entries.len(), 2);
        let entry = entries.iter().find(|e| e.size == 40).unwrap().clone();
        quarantine.restore(&adapter, &entry.id, None).unwrap();
        assert_eq!(fs::read(temp.join("a.tmp")).unwrap(), vec![1u8; 40]);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn restore_and_purge_work_on_real_files_without_touching_outside_roots() {
        let root = scratch("purge");
        let temp = root.join("Temp");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("x.tmp"), b"x").unwrap();
        let temp_text = text(&temp);
        let adapter = WindowsFs;
        let policy = ochistka_core::policy::PathPolicy::new(&[&temp_text]).unwrap();
        let quarantine =
            ochistka_core::quarantine::Quarantine::new(&text(&root.join("Q")), policy).unwrap();
        let source = text(&temp.join("x.tmp"));
        quarantine
            .move_many(&adapter, &[source], "тест", 1)
            .unwrap();
        let entry = quarantine.list(&adapter).unwrap().remove(0);

        assert!(quarantine
            .purge(&adapter, &entry.id, "purge:wrong")
            .is_err());
        quarantine
            .purge(&adapter, &entry.id, &entry.purge_token)
            .unwrap();
        assert!(quarantine.list(&adapter).unwrap().is_empty());
        assert!(!temp.join("x.tmp").exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn layout_finds_existing_caches_in_a_fake_profile_only() {
        let root = scratch("layout");
        let local = root.join("Local");
        let roaming = root.join("Roaming");
        let profile = root.join("Profile");
        fs::create_dir_all(local.join("Temp")).unwrap();
        fs::create_dir_all(local.join("Google/Chrome/User Data/Default/Cache")).unwrap();
        fs::create_dir_all(local.join("Mozilla/Firefox/Profiles/abc.default/cache2")).unwrap();
        fs::create_dir_all(roaming.join("Microsoft/Windows/Recent")).unwrap();
        fs::create_dir_all(profile.join("Downloads")).unwrap();
        let vars = EnvVars {
            temp: Some(text(&local.join("Temp"))),
            tmp: None,
            local_app_data: Some(text(&local)),
            app_data: Some(text(&roaming)),
            user_profile: Some(text(&profile)),
        };

        let layout = build_layout(&vars, &WindowsFs).unwrap();
        let ids: Vec<&str> = layout.cleanup.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["temp", "chrome-cache", "firefox-cache-abc.default"]
        );
        assert!(layout.downloads.ends_with("Downloads"));
        assert!(
            layout.quarantine_root.ends_with("OCHISTKA/Quarantine")
                || layout.quarantine_root.ends_with("OCHISTKA\\Quarantine")
        );
        assert!(layout.privacy.iter().any(|p| p.id == "recent"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn layout_without_local_app_data_is_unsupported() {
        let vars = EnvVars::default();
        assert!(matches!(
            build_layout(&vars, &WindowsFs),
            Err(CleanupError::Unsupported(_))
        ));
    }

    #[test]
    fn reparse_free_linux_symlink_is_reported_as_link() {
        let root = scratch("link");
        let target = root.join("target");
        fs::create_dir_all(&target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, root.join("link")).unwrap();
        #[cfg(unix)]
        {
            let meta = WindowsFs.metadata(&text(&root.join("link"))).unwrap();
            assert!(meta.is_symlink);
        }
        fs::remove_dir_all(&root).ok();
    }
}
