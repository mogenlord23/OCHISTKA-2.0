//! Адаптер ClamAV: вызывает clamscan.exe из папки установки по абсолютному пути.

use std::path::{Path, PathBuf};
use std::process::Command;

use ochistka_core::scan::{parse_clamscan_output, EngineStatus, ScanReport, VirusScanPort};
use ochistka_core::system::CoreError;

/// Путь к clamscan внутри папки установки. Поиска по PATH нет.
pub fn clamscan_path(install_dir: &Path) -> PathBuf {
    install_dir.join("clamav").join("clamscan.exe")
}

/// Аргументы: только заражённые файлы, рекурсивно. Путь — последним отдельным аргументом.
pub fn scan_args(target: &str) -> Vec<String> {
    vec![
        "--stdout".to_string(),
        "--infected".to_string(),
        "--recursive".to_string(),
        target.to_string(),
    ]
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct ClamAvScanner {
    clamscan: Option<PathBuf>,
    reason: String,
}

impl ClamAvScanner {
    pub fn from_install_dir(install_dir: &Path) -> Self {
        let path = clamscan_path(install_dir);
        if path.is_file() {
            Self {
                clamscan: Some(path),
                reason: String::new(),
            }
        } else {
            Self {
                clamscan: None,
                reason: format!("{} не найден", path.display()),
            }
        }
    }

    pub fn unavailable(reason: &str) -> Self {
        Self {
            clamscan: None,
            reason: reason.to_string(),
        }
    }
}

impl VirusScanPort for ClamAvScanner {
    fn engine_status(&self) -> EngineStatus {
        EngineStatus {
            available: self.clamscan.is_some(),
            reason: self.reason.clone(),
        }
    }

    fn scan(&self, target: &str) -> Result<ScanReport, CoreError> {
        let Some(clamscan) = &self.clamscan else {
            return Err(CoreError::EngineMissing(self.reason.clone()));
        };
        let mut command = Command::new(clamscan);
        command.args(scan_args(target));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let output = command
            .output()
            .map_err(|err| CoreError::Probe(format!("не удалось запустить clamscan: {err}")))?;
        // Коды clamscan: 0 — заражений нет, 1 — найдены, 2 — ошибка.
        match output.status.code() {
            Some(0) | Some(1) => Ok(parse_clamscan_output(&String::from_utf8_lossy(
                &output.stdout,
            ))),
            _ => Err(CoreError::Probe(format!(
                "clamscan завершился с ошибкой: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamscan_path_is_inside_clamav_folder() {
        let path = clamscan_path(Path::new("C:/app"));
        assert!(path.ends_with(Path::new("clamav").join("clamscan.exe")));
    }

    #[test]
    fn scan_args_pass_target_as_last_argument() {
        let args = scan_args("C:\\Users\\Slava\\Downloads");
        assert!(args.contains(&"--infected".to_string()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("C:\\Users\\Slava\\Downloads")
        );
    }

    #[test]
    fn missing_engine_reports_unavailable_with_reason() {
        let scanner = ClamAvScanner::from_install_dir(Path::new("Z:/definitely/missing"));
        let status = scanner.engine_status();
        assert!(!status.available);
        assert!(status.reason.contains("clamscan.exe"));
    }

    #[test]
    fn scan_without_engine_returns_engine_missing() {
        let scanner = ClamAvScanner::from_install_dir(Path::new("Z:/definitely/missing"));
        assert!(matches!(
            scanner.scan("C:\\Users"),
            Err(CoreError::EngineMissing(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn scan_runs_engine_and_parses_its_output() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!("ochistka-clamav-{}", std::process::id()));
        let engine_dir = root.join("clamav");
        std::fs::create_dir_all(&engine_dir).unwrap();
        let engine = engine_dir.join("clamscan.exe");
        std::fs::write(
            &engine,
            "#!/bin/sh\necho '/data/invoice.pdf.exe: Win.Test.EICAR_HDB-1 FOUND'\necho 'Scanned files: 4'\necho 'Infected files: 1'\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&engine, std::fs::Permissions::from_mode(0o755)).unwrap();

        let scanner = ClamAvScanner::from_install_dir(&root);
        let report = scanner.scan("/data").expect("fake engine should run");
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(report.scanned_files, 4);
        assert_eq!(report.infected_files, 1);
        assert_eq!(report.findings[0].signature, "Win.Test.EICAR_HDB-1");
    }
}
