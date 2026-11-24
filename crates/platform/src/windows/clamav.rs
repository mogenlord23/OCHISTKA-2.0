//! Движок ClamAV (libclamav, встроенный в приложение по ADR 0004). Снаружи — только `VirusScanPort`.
//! Сборка линкует движок, когда рядом лежит официальная сборка ClamAV
//! (`tools/clamav-win/fetch.ps1`) и включена фича `libclamav`; тогда build.rs выставляет
//! `clamav_linked`. Без неё порт отвечает причиной «движок не собран», приложение не падает.

use ochistka_core::scan::{check_signatures_dir, EngineStatus, FileVerdict, VirusScanPort};
use ochistka_core::system::CoreError;

#[cfg(clamav_linked)]
use std::sync::{PoisonError, RwLock, RwLockReadGuard};

#[cfg(clamav_linked)]
use super::clamav_ffi::Engine;

/// Причина для интерфейса, пока libclamav не собран (статус: docs/status/clamav-build.md).
pub const NOT_BUILT_REASON: &str =
    "движок не собран: libclamav не встроен в эту сборку, см. docs/status/clamav-build.md";

/// Причина, пока собран движок, но сигнатуры не загружены.
#[cfg(clamav_linked)]
const NO_SIGNATURES_REASON: &str = "базы проверки не загружены: движок ещё не получил базу";

/// Встроен ли движок в эту сборку.
pub const fn engine_present() -> bool {
    cfg!(clamav_linked)
}

/// Папка баз в пользовательском каталоге данных текущей ОС.
pub fn default_signatures_dir() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    #[cfg(windows)]
    let base = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    #[cfg(target_os = "linux")]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".local/share")))?;
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    return Some(base.join("OCHISTKA").join("clamdb"));
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    None
}

/// Создаёт папку баз, если её нет, и возвращает путь.
pub fn ensure_signatures_dir() -> Result<std::path::PathBuf, String> {
    let dir = default_signatures_dir()
        .ok_or_else(|| "папка данных пользователя не найдена".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|err| format!("папка баз не создана: {err}"))?;
    Ok(dir)
}

/// Сколько файлов базы лежит в папке (0 — папка пуста или её нет).
pub fn signatures_files(dir: &std::path::Path) -> u32 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

pub struct ClamAvEngine {
    reason: String,
    #[cfg(clamav_linked)]
    native: RwLock<Option<(Engine, u32)>>,
}

impl ClamAvEngine {
    /// Движок для текущей сборки: с фичей — без сигнатур до загрузки, без фичи — заглушка.
    pub fn build() -> Self {
        #[cfg(clamav_linked)]
        {
            Self {
                reason: NO_SIGNATURES_REASON.to_string(),
                native: RwLock::new(None),
            }
        }
        #[cfg(not(clamav_linked))]
        {
            Self::not_built()
        }
    }

    /// Заглушка: движок в эту сборку не вошёл. Всегда недоступна, с причиной.
    pub fn not_built() -> Self {
        Self {
            reason: NOT_BUILT_REASON.to_string(),
            #[cfg(clamav_linked)]
            native: RwLock::new(None),
        }
    }

    #[cfg(clamav_linked)]
    fn read_native(&self) -> RwLockReadGuard<'_, Option<(Engine, u32)>> {
        self.native.read().unwrap_or_else(PoisonError::into_inner)
    }
}

impl VirusScanPort for ClamAvEngine {
    fn engine_status(&self) -> EngineStatus {
        #[cfg(clamav_linked)]
        let loaded = self.read_native().is_some();
        #[cfg(not(clamav_linked))]
        let loaded = false;
        EngineStatus {
            available: loaded,
            reason: if loaded {
                String::new()
            } else {
                self.reason.clone()
            },
        }
    }

    #[cfg(clamav_linked)]
    fn scan_workers(&self) -> usize {
        std::thread::available_parallelism().map_or(1, |count| (count.get() - 1).clamp(1, 4))
    }

    fn scan_file(&self, path: &str) -> Result<FileVerdict, CoreError> {
        #[cfg(clamav_linked)]
        {
            let guard = self.read_native();
            let (engine, _) = guard
                .as_ref()
                .ok_or_else(|| CoreError::EngineMissing(self.reason.clone()))?;
            #[cfg(windows)]
            let result = engine.scan(path);
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            let result = {
                let file = crate::computer_scan::open_regular(std::path::Path::new(path))
                    .map_err(|err| CoreError::Probe(err.to_string()))?;
                engine.scan_open_file(&file)
            };
            result.map_err(|err| CoreError::Probe(format!("движок не проверил файл: {err}")))
        }
        #[cfg(not(clamav_linked))]
        {
            let _ = path;
            Err(CoreError::EngineMissing(self.reason.clone()))
        }
    }

    #[cfg(all(clamav_linked, target_os = "linux"))]
    fn scan_file_with_identity(
        &self,
        path: &str,
    ) -> Result<(FileVerdict, Option<ochistka_core::quarantine::FileIdentity>), CoreError> {
        use crate::linux::file_identity::{open_regular_no_links, scan_open_file};
        let guard = self.read_native();
        let (engine, _) = guard
            .as_ref()
            .ok_or_else(|| CoreError::EngineMissing(self.reason.clone()))?;
        let file = open_regular_no_links(std::path::Path::new(path))
            .map_err(|err| CoreError::Probe(err.to_string()))?;
        let scan = |opened: &std::fs::File| {
            engine
                .scan_open_file(opened)
                .map_err(|err| CoreError::Probe(format!("движок не проверил файл: {err}")))
        };
        if matches!(scan(&file)?, FileVerdict::Clean) {
            return Ok((FileVerdict::Clean, None));
        }
        // Хеш идентичности и повторная проверка нужны только для находок.
        let (verdict, identity) = scan_open_file(&file, scan)?;
        let identity = matches!(verdict, FileVerdict::Found { .. }).then_some(identity);
        Ok((verdict, identity))
    }

    fn load_signatures(&self, dir: &str) -> Result<(), CoreError> {
        // Сначала папка: отсутствие сигнатур — отдельная понятная причина, не «движок не собран».
        check_signatures_dir(&crate::windows_cleanup::WindowsFs, dir)?;
        #[cfg(clamav_linked)]
        {
            let (engine, signatures) = Engine::load(dir)
                .map_err(|err| CoreError::Probe(format!("базы не загружены: {err}")))?;
            *self.native.write().unwrap_or_else(PoisonError::into_inner) =
                Some((engine, signatures));
            Ok(())
        }
        #[cfg(not(clamav_linked))]
        {
            Err(CoreError::EngineMissing(NOT_BUILT_REASON.to_string()))
        }
    }

    fn signature_count(&self) -> Option<u32> {
        #[cfg(clamav_linked)]
        {
            self.read_native()
                .as_ref()
                .map(|(_, signatures)| *signatures)
        }
        #[cfg(not(clamav_linked))]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("ochistka-clamav-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn not_built_engine_reports_reason_with_status_doc() {
        let status = ClamAvEngine::not_built().engine_status();
        assert!(!status.available);
        assert!(status.reason.contains("движок не собран"));
        assert!(status.reason.contains("docs/status/clamav-build.md"));
    }

    #[test]
    fn not_built_engine_refuses_to_scan_with_the_same_reason() {
        let err = ClamAvEngine::not_built()
            .scan_file("C:\\Users\\Slava\\Downloads\\a.exe")
            .unwrap_err();
        assert!(matches!(err, CoreError::EngineMissing(ref m) if m.contains("движок не собран")));
    }

    #[test]
    fn signatures_folder_missing_is_reported_before_engine_state() {
        let root = temp_root("missing");
        let missing = root.join("signatures");
        let err = ClamAvEngine::not_built()
            .load_signatures(&missing.to_string_lossy())
            .unwrap_err();
        assert!(matches!(err, CoreError::Probe(ref m) if m.contains("папка сигнатур не найдена")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    #[cfg(not(clamav_linked))]
    fn signatures_folder_present_but_not_built_reports_not_built() {
        let root = temp_root("present");
        let signatures = root.join("signatures");
        std::fs::create_dir_all(&signatures).unwrap();
        std::fs::write(signatures.join("main.cvd"), b"placeholder").unwrap();
        let err = ClamAvEngine::not_built()
            .load_signatures(&signatures.to_string_lossy())
            .unwrap_err();
        assert!(matches!(err, CoreError::EngineMissing(ref m) if m.contains("движок не собран")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn default_signatures_dir_lives_in_the_app_folder() {
        let dir = default_signatures_dir().expect("LOCALAPPDATA задан на Windows");
        assert!(dir.ends_with(std::path::Path::new("OCHISTKA").join("clamdb")));
        assert_eq!(signatures_files(&dir), 0, "свежая папка пуста или её нет");
    }

    #[cfg(clamav_linked)]
    #[test]
    fn built_engine_starts_without_signatures_and_says_so() {
        assert!(engine_present());
        let status = ClamAvEngine::build().engine_status();
        assert!(!status.available);
        assert!(status.reason.contains("базы проверки не загружены"));
    }

    /// Настоящий движок: тестовая база вместо официальной (её нет в сети).
    /// Вместо EICAR — собственный безвредный образец: Defender удаляет EICAR-файл на этой машине.
    #[cfg(all(clamav_linked, windows))]
    mod native {
        use super::*;

        const PATTERN: &str = "OCHISTKA-CLAMAV-TEST-PATTERN-0123456789ABCDEF";

        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|byte| format!("{byte:02x}")).collect()
        }

        fn base_folder(name: &str, body: &str) -> std::path::PathBuf {
            let root = temp_root(name);
            std::fs::write(root.join("test.ndb"), body).unwrap();
            root
        }

        #[test]
        fn test_base_finds_our_pattern_and_keeps_a_clean_file_clean() {
            let folder = base_folder(
                "native-clean",
                &format!("OCHISTKA.Test:0:*:{}\n", hex(PATTERN.as_bytes())),
            );
            let engine = ClamAvEngine::build();
            engine
                .load_signatures(&folder.to_string_lossy())
                .expect("тестовая база должна загрузиться");
            assert!(engine.engine_status().available);
            assert_eq!(engine.signature_count(), Some(1));

            let sample = folder.join("sample.bin");
            std::fs::write(&sample, PATTERN.as_bytes()).unwrap();
            let verdict = engine
                .scan_file(&sample.to_string_lossy())
                .expect("файл должен проверяться");
            // Локальная база без подписи ClamAV помечает суффиксом `.UNOFFICIAL`.
            match verdict {
                FileVerdict::Found { signature } => {
                    assert!(
                        signature.starts_with("OCHISTKA.Test"),
                        "имя сигнатуры: {signature}"
                    );
                }
                other => panic!("движок вернул {other:?}"),
            }

            let clean = folder.join("clean.txt");
            std::fs::write(&clean, b"just a note").unwrap();
            assert_eq!(
                engine.scan_file(&clean.to_string_lossy()).unwrap(),
                FileVerdict::Clean
            );

            let samples = folder.join("samples");
            let other = folder.join("other");
            std::fs::create_dir(&samples).unwrap();
            std::fs::create_dir(&other).unwrap();
            std::fs::rename(&sample, samples.join("sample.bin")).unwrap();
            std::fs::rename(&clean, other.join("clean.txt")).unwrap();
            for index in 0..126 {
                std::fs::write(other.join(format!("note-{index}.txt")), b"just a note").unwrap();
            }
            let started = std::time::Instant::now();
            let mut first_file = None;
            let report = ochistka_core::scan::sweep_computer(
                &crate::windows_cleanup::WindowsFs,
                &engine,
                &[
                    samples.to_string_lossy().into_owned(),
                    other.to_string_lossy().into_owned(),
                ],
                &std::sync::atomic::AtomicBool::new(false),
                &mut |progress| {
                    if progress.scanned_files == 1 && first_file.is_none() {
                        first_file = Some(started.elapsed());
                    }
                },
            )
            .unwrap();
            assert_eq!(report.scanned_files, 128);
            assert_eq!(report.infected_files, 1);
            assert!(!report.incomplete);
            println!(
                "128 temporary files: first result {:?}, total {:?}",
                first_file.unwrap(),
                started.elapsed()
            );
            std::fs::remove_dir_all(&folder).ok();
        }

        #[test]
        fn a_broken_base_is_reported_as_a_plain_reason() {
            let folder = base_folder("native-broken", "this is not a signature file\n");
            let engine = ClamAvEngine::build();
            let err = engine
                .load_signatures(&folder.to_string_lossy())
                .unwrap_err();
            assert!(matches!(err, CoreError::Probe(ref m) if m.contains("базы не загружены")));
            assert!(!engine.engine_status().available);
            assert_eq!(engine.signature_count(), None);
            std::fs::remove_dir_all(&folder).ok();
        }
    }
}
