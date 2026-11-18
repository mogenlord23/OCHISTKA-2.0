//! Проверка на вирусы: разбор вывода ClamAV, эвристика и порт сканера. Без кода ОС.

use serde::Serialize;
use ts_rs::TS;

use crate::system::CoreError;

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "Finding.ts")]
pub struct Finding {
    pub path: String,
    pub signature: String,
}

#[derive(Serialize, TS, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "ScanReport.ts")]
pub struct ScanReport {
    pub scanned_files: u32,
    pub infected_files: u32,
    pub errors: u32,
    pub findings: Vec<Finding>,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "EngineStatus.ts")]
pub struct EngineStatus {
    pub available: bool,
    pub reason: String,
}

/// Разбирает стандартный вывод clamscan: строки «путь: сигнатура FOUND», строки «ERROR» и сводку.
pub fn parse_clamscan_output(output: &str) -> ScanReport {
    let mut report = ScanReport::default();
    let mut summary_infected: Option<u32> = None;
    for line in output.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_suffix(" FOUND") {
            if let Some((path, signature)) = rest.rsplit_once(": ") {
                report.findings.push(Finding {
                    path: path.to_string(),
                    signature: signature.to_string(),
                });
            }
        } else if line.ends_with(" ERROR") {
            report.errors += 1;
        } else if let Some(value) = line.strip_prefix("Scanned files: ") {
            report.scanned_files = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("Infected files: ") {
            summary_infected = value.trim().parse().ok();
        }
    }
    report.infected_files = summary_infected.unwrap_or(report.findings.len() as u32);
    report
}

/// Путь проверки должен быть абсолютным и без управляющих символов, чтобы его нельзя было
/// принять за ключ программы.
pub fn validate_target(target: &str) -> Result<(), CoreError> {
    let bytes = target.as_bytes();
    let drive_path = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    let absolute = drive_path || target.starts_with("\\\\") || target.starts_with('/');
    if !absolute || target.chars().any(|c| c.is_control()) {
        return Err(CoreError::InvalidTarget(target.to_string()));
    }
    Ok(())
}

/// Признаки файла для эвристической оценки. Заполняет адаптер платформы.
#[derive(Clone, Debug, PartialEq)]
pub struct FileFacts {
    pub file_name: String,
    pub size_bytes: u64,
    pub in_downloads: bool,
    pub in_startup: bool,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "Suspicion.ts")]
pub struct Suspicion {
    pub score: u8,
    pub reasons: Vec<String>,
}

const EXECUTABLE_EXTENSIONS: [&str; 10] = [
    "exe", "scr", "bat", "cmd", "ps1", "vbs", "js", "msi", "com", "pif",
];
const DOCUMENT_EXTENSIONS: [&str; 9] = [
    "pdf", "doc", "docx", "xls", "xlsx", "txt", "jpg", "jpeg", "png",
];
const CRACK_WORDS: [&str; 3] = ["crack", "keygen", "patch"];

/// Эвристическая подсказка. Не вердикт: файл решает пользователь.
pub fn score_suspicion(facts: &FileFacts) -> Suspicion {
    let name = facts.file_name.to_lowercase();
    let parts: Vec<&str> = name.split('.').collect();
    let extension = if parts.len() >= 2 {
        parts[parts.len() - 1]
    } else {
        ""
    };
    let is_executable = EXECUTABLE_EXTENSIONS.contains(&extension);

    let mut score: u32 = 0;
    let mut reasons = Vec::new();
    if is_executable && parts.len() >= 3 && DOCUMENT_EXTENSIONS.contains(&parts[parts.len() - 2]) {
        score += 40;
        reasons.push("двойное расширение: документ, замаскированный под программу".to_string());
    }
    if is_executable && facts.in_startup {
        score += 30;
        reasons.push("исполняемый файл в автозагрузке".to_string());
    } else if is_executable && facts.in_downloads {
        score += 30;
        reasons.push("исполняемый файл в загрузках".to_string());
    }
    if is_executable && facts.size_bytes < 1024 {
        score += 10;
        reasons.push("очень маленький исполняемый файл".to_string());
    }
    if is_executable && CRACK_WORDS.iter().any(|word| name.contains(word)) {
        score += 20;
        reasons.push("имя похоже на взломщик".to_string());
    }
    Suspicion {
        score: score.min(100) as u8,
        reasons,
    }
}

/// Порт сканера. Реализации живут в `crates/platform`.
pub trait VirusScanPort: Send + Sync {
    fn engine_status(&self) -> EngineStatus;
    fn scan(&self, target: &str) -> Result<ScanReport, CoreError>;
}

/// Проверяет путь до обращения к движку.
pub fn run_scan(port: &dyn VirusScanPort, target: &str) -> Result<ScanReport, CoreError> {
    validate_target(target)?;
    port.scan(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOUND_OUTPUT: &str =
        "C:\\Users\\Slava\\Downloads\\invoice.pdf.exe: Win.Test.EICAR_HDB-1 FOUND\n\
\n\
----------- SCAN SUMMARY -----------\n\
Known viruses: 8650000\n\
Scanned directories: 1\n\
Scanned files: 12\n\
Infected files: 1\n";

    #[test]
    fn parses_findings_and_summary() {
        let report = parse_clamscan_output(FOUND_OUTPUT);
        assert_eq!(report.scanned_files, 12);
        assert_eq!(report.infected_files, 1);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0].path,
            "C:\\Users\\Slava\\Downloads\\invoice.pdf.exe"
        );
        assert_eq!(report.findings[0].signature, "Win.Test.EICAR_HDB-1");
    }

    #[test]
    fn windows_path_with_colons_is_kept_whole() {
        let output = "C:\\Data\\a: b.exe: Win.Test.X FOUND\n";
        let report = parse_clamscan_output(output);
        assert_eq!(report.findings[0].path, "C:\\Data\\a: b.exe");
        assert_eq!(report.findings[0].signature, "Win.Test.X");
    }

    #[test]
    fn counts_error_lines() {
        let output = "C:\\locked.sys: lstat() failed: ERROR\n";
        let report = parse_clamscan_output(output);
        assert_eq!(report.errors, 1);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn clean_output_has_no_findings() {
        let output = "----------- SCAN SUMMARY -----------\nScanned files: 3\nInfected files: 0\n";
        let report = parse_clamscan_output(output);
        assert_eq!(report.scanned_files, 3);
        assert_eq!(report.infected_files, 0);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn infected_count_falls_back_to_findings_without_summary() {
        let output = "C:\\a.exe: Win.Test.X FOUND\nC:\\b.exe: Win.Test.Y FOUND\n";
        let report = parse_clamscan_output(output);
        assert_eq!(report.infected_files, 2);
    }

    #[test]
    fn validate_target_accepts_absolute_paths_only() {
        assert!(validate_target("C:\\Users\\Slava\\Downloads").is_ok());
        assert!(validate_target("/home/user/x").is_ok());
        assert!(validate_target("\\\\server\\share").is_ok());
        assert!(validate_target("Downloads").is_err());
        assert!(validate_target("-r").is_err());
        assert!(validate_target("").is_err());
        assert!(validate_target("C:\\x\0y").is_err());
    }

    #[test]
    fn plain_document_is_not_flagged() {
        let facts = FileFacts {
            file_name: "report.pdf".into(),
            size_bytes: 50_000,
            in_downloads: false,
            in_startup: false,
        };
        let suspicion = score_suspicion(&facts);
        assert_eq!(suspicion.score, 0);
        assert!(suspicion.reasons.is_empty());
    }

    #[test]
    fn double_extension_is_flagged() {
        let facts = FileFacts {
            file_name: "invoice.pdf.exe".into(),
            size_bytes: 5_000,
            in_downloads: false,
            in_startup: false,
        };
        let suspicion = score_suspicion(&facts);
        assert!(suspicion.score >= 40);
        assert!(suspicion.reasons[0].starts_with("двойное расширение"));
    }

    #[test]
    fn executable_in_downloads_is_flagged() {
        let facts = FileFacts {
            file_name: "setup.exe".into(),
            size_bytes: 5_000,
            in_downloads: true,
            in_startup: false,
        };
        let suspicion = score_suspicion(&facts);
        assert_eq!(suspicion.score, 30);
        assert_eq!(
            suspicion.reasons,
            vec!["исполняемый файл в загрузках".to_string()]
        );
    }

    #[test]
    fn all_signals_together_reach_one_hundred() {
        let facts = FileFacts {
            file_name: "crack.doc.exe".into(),
            size_bytes: 10,
            in_downloads: false,
            in_startup: true,
        };
        assert_eq!(score_suspicion(&facts).score, 100);
    }

    struct FakeScanner {
        scans: std::sync::atomic::AtomicU32,
    }

    impl VirusScanPort for FakeScanner {
        fn engine_status(&self) -> EngineStatus {
            EngineStatus {
                available: true,
                reason: String::new(),
            }
        }

        fn scan(&self, _target: &str) -> Result<ScanReport, CoreError> {
            self.scans.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(ScanReport::default())
        }
    }

    #[test]
    fn run_scan_rejects_invalid_target_without_calling_engine() {
        let scanner = FakeScanner {
            scans: std::sync::atomic::AtomicU32::new(0),
        };
        assert!(run_scan(&scanner, "Downloads").is_err());
        assert_eq!(scanner.scans.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn run_scan_passes_valid_target_to_engine() {
        let scanner = FakeScanner {
            scans: std::sync::atomic::AtomicU32::new(0),
        };
        assert!(run_scan(&scanner, "C:\\Users\\Slava\\Downloads").is_ok());
        assert_eq!(scanner.scans.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn engine_missing_message_is_russian() {
        let err = CoreError::EngineMissing("clamscan.exe не найден".into());
        assert_eq!(err.to_string(), "движок недоступен: clamscan.exe не найден");
    }
}
