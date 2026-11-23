//! Проверка по признакам: без базы сигнатур, только инвентаризация файла и эвристика.
//!
//! Нужна, пока официальные базы ClamAV не загружены: движку не с чем сравнивать файлы, поэтому
//! приложение ищет подозрительные файлы по внешним признакам (двойное расширение, исполняемый
//! файл в загрузках, скрытый файл и т. п.) и честно называет их «по признакам», а не вирусами.
//! Нейросети здесь нет: `NoModel` — заглушка ИИ-слоя из ADR 0004.

use std::path::Path;

use ochistka_core::assess::{assess_path, NoModel};
use ochistka_core::fs::FsPort;
use ochistka_core::scan::{EngineStatus, FileVerdict, VirusScanPort};
use ochistka_core::system::CoreError;

use super::files::file_facts;

/// С какого балла файл показываем как подозрительный (0–100).
pub const SUSPICION_THRESHOLD: u8 = 40;

/// Движок проверки по признакам. Всегда доступен: база не нужна.
pub struct SuspicionEngine<F: FsPort + Send + Sync> {
    fs: F,
}

impl<F: FsPort + Send + Sync> SuspicionEngine<F> {
    pub fn new(fs: F) -> Self {
        Self { fs }
    }
}

impl<F: FsPort + Send + Sync> VirusScanPort for SuspicionEngine<F> {
    fn engine_status(&self) -> EngineStatus {
        EngineStatus {
            available: true,
            reason: String::new(),
        }
    }

    fn scan_file(&self, path: &str) -> Result<FileVerdict, CoreError> {
        let facts = file_facts(Path::new(path)).map_err(|err| CoreError::Probe(err.to_string()))?;
        let suspicion = assess_path(&self.fs, path, &facts, &NoModel)?;
        if suspicion.score >= SUSPICION_THRESHOLD {
            Ok(FileVerdict::Found {
                signature: format!("по признакам: {}", suspicion.reasons.join("; ")),
            })
        } else {
            Ok(FileVerdict::Clean)
        }
    }

    fn load_signatures(&self, _dir: &str) -> Result<(), CoreError> {
        // Признакам база не нужна: загрузка сигнатур ничего не меняет.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::windows_cleanup::WindowsFs;

    fn temp_folder(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("ochistka-signs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn suspicious_name_is_reported_by_signs_not_as_a_virus() {
        let root = temp_folder("suspicious");
        let file = root.join("invoice.pdf.exe");
        std::fs::write(&file, b"x").unwrap();
        let engine = SuspicionEngine::new(WindowsFs);
        let verdict = engine.scan_file(&file.to_string_lossy()).unwrap();
        match verdict {
            FileVerdict::Found { signature } => {
                assert!(
                    signature.starts_with("по признакам:"),
                    "подпись: {signature}"
                );
            }
            other => panic!("ожидали подозрительный файл, получили {other:?}"),
        }
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_ordinary_text_file_is_clean() {
        let root = temp_folder("clean");
        let file = root.join("notes.txt");
        std::fs::write(&file, b"just a note").unwrap();
        let engine = SuspicionEngine::new(WindowsFs);
        assert_eq!(
            engine.scan_file(&file.to_string_lossy()).unwrap(),
            FileVerdict::Clean
        );
        assert!(engine.engine_status().available);
        std::fs::remove_dir_all(&root).ok();
    }
}
