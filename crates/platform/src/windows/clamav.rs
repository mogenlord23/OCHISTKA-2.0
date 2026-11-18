//! Движок ClamAV (libclamav, встроенный в приложение по ADR 0004). Снаружи — только `VirusScanPort`.
//! Без фичи `libclamav` движка нет: порт отвечает причиной «движок не собран», приложение не падает.

use ochistka_core::scan::{check_signatures_dir, EngineStatus, FileVerdict, VirusScanPort};
use ochistka_core::system::CoreError;

#[cfg(feature = "libclamav")]
use std::sync::{Mutex, MutexGuard, PoisonError};

#[cfg(feature = "libclamav")]
use super::clamav_ffi::Engine;

/// Причина для интерфейса, пока libclamav не собран (статус: docs/status/clamav-build.md).
pub const NOT_BUILT_REASON: &str =
    "движок не собран: libclamav не встроен в эту сборку, см. docs/status/clamav-build.md";

/// Причина, пока собран движок, но сигнатуры не загружены.
#[cfg(feature = "libclamav")]
const NO_SIGNATURES_REASON: &str = "сигнатуры не загружены: движок ещё не получил базу";

pub struct ClamAvEngine {
    reason: String,
    #[cfg(feature = "libclamav")]
    native: Mutex<Option<Engine>>,
}

impl ClamAvEngine {
    /// Движок для текущей сборки: с фичей — без сигнатур до загрузки, без фичи — заглушка.
    pub fn build() -> Self {
        #[cfg(feature = "libclamav")]
        {
            Self {
                reason: NO_SIGNATURES_REASON.to_string(),
                native: Mutex::new(None),
            }
        }
        #[cfg(not(feature = "libclamav"))]
        {
            Self::not_built()
        }
    }

    /// Заглушка: движок в эту сборку не вошёл. Всегда недоступна, с причиной.
    pub fn not_built() -> Self {
        Self {
            reason: NOT_BUILT_REASON.to_string(),
            #[cfg(feature = "libclamav")]
            native: Mutex::new(None),
        }
    }

    #[cfg(feature = "libclamav")]
    fn lock_native(&self) -> MutexGuard<'_, Option<Engine>> {
        self.native.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl VirusScanPort for ClamAvEngine {
    fn engine_status(&self) -> EngineStatus {
        #[cfg(feature = "libclamav")]
        let loaded = self.lock_native().is_some();
        #[cfg(not(feature = "libclamav"))]
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

    fn scan_file(&self, path: &str) -> Result<FileVerdict, CoreError> {
        #[cfg(feature = "libclamav")]
        {
            let guard = self.lock_native();
            let engine = guard
                .as_ref()
                .ok_or_else(|| CoreError::EngineMissing(self.reason.clone()))?;
            engine
                .scan(path)
                .map_err(|err| CoreError::Probe(format!("движок не проверил файл: {err}")))
        }
        #[cfg(not(feature = "libclamav"))]
        {
            let _ = path;
            Err(CoreError::EngineMissing(self.reason.clone()))
        }
    }

    fn load_signatures(&self, dir: &str) -> Result<(), CoreError> {
        // Сначала папка: отсутствие сигнатур — отдельная понятная причина, не «движок не собран».
        check_signatures_dir(&crate::windows_cleanup::WindowsFs, dir)?;
        #[cfg(feature = "libclamav")]
        {
            let engine = Engine::load(dir)
                .map_err(|err| CoreError::Probe(format!("сигнатуры не загружены: {err}")))?;
            *self.lock_native() = Some(engine);
            Ok(())
        }
        #[cfg(not(feature = "libclamav"))]
        {
            Err(CoreError::EngineMissing(NOT_BUILT_REASON.to_string()))
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
}
