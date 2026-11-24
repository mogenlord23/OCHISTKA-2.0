//! Проверка по признакам: инвентаризация, эвристика и локальная модель заголовков PE.
//!
//! Нужна, пока официальные базы ClamAV не загружены: движку не с чем сравнивать файлы, поэтому
//! приложение ищет подозрительные файлы по внешним признакам (двойное расширение, исполняемый
//! файл в загрузках, скрытый файл и т. п.) и честно называет их «по признакам», а не вирусами.

use std::path::Path;

use ochistka_core::assess::assess_path;
use ochistka_core::fs::FsPort;
use ochistka_core::scan::{EngineStatus, FileVerdict, ScanMode, VirusScanPort};
use ochistka_core::system::CoreError;

use super::files::file_facts;
use crate::pe_model::PeHeaderModel;

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
    fn mode(&self) -> ScanMode {
        ScanMode::Signs
    }

    fn engine_status(&self) -> EngineStatus {
        EngineStatus {
            available: true,
            reason: String::new(),
        }
    }

    fn scan_file(&self, path: &str) -> Result<FileVerdict, CoreError> {
        let facts = file_facts(Path::new(path)).map_err(|err| CoreError::Probe(err.to_string()))?;
        let suspicion = assess_path(&self.fs, path, &facts, &PeHeaderModel)?;
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

    #[test]
    fn synthetic_pe_header_surfaces_the_local_model_in_the_scan_result() {
        let root = temp_folder("model");
        let file = root.join("sample.exe");
        // Заголовок PE без исполняемого кода. Модель оценивает только признаки заголовка.
        let mut bytes = vec![0u8; 8192];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        bytes[84..86].copy_from_slice(&96u16.to_le_bytes());
        bytes[88..90].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes[92..96].copy_from_slice(&512u32.to_le_bytes());
        bytes[156..158].copy_from_slice(&2u16.to_le_bytes());
        std::fs::write(&file, bytes).unwrap();

        let engine = SuspicionEngine::new(WindowsFs);
        let verdict = engine.scan_file(file.to_str().unwrap()).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            matches!(verdict, FileVerdict::Found { signature } if signature.contains("ИИ-подсказка"))
        );
    }
}
