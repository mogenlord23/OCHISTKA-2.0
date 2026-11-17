//! Типизированные команды Tauri. Интерфейс получает данные только отсюда.

use std::sync::Arc;

use std::path::Path;

use ochistka_core::scan::{
    run_scan, score_suspicion, validate_target, EngineStatus, ScanReport, Suspicion, VirusScanPort,
};
use ochistka_core::system::{CoreError, SystemInfoPort, SystemOverview};
use serde::Serialize;
use tauri::State;

/// Ошибка для интерфейса: `kind` — тип ошибки, `message` — текст на русском.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct CommandError {
    pub kind: String,
    pub message: String,
}

impl From<CoreError> for CommandError {
    fn from(err: CoreError) -> Self {
        let kind = match err {
            CoreError::Unsupported { .. } => "unsupported",
            CoreError::Probe(_) => "probe",
            CoreError::EngineMissing(_) => "engineMissing",
            CoreError::InvalidTarget(_) => "invalidTarget",
        };
        Self {
            kind: kind.to_string(),
            message: err.to_string(),
        }
    }
}

pub fn load_overview(port: &dyn SystemInfoPort) -> Result<SystemOverview, CommandError> {
    port.overview().map_err(CommandError::from)
}

pub fn scan_with(port: &dyn VirusScanPort, target: &str) -> Result<ScanReport, CommandError> {
    run_scan(port, target).map_err(CommandError::from)
}

/// Состояние приложения: адаптеры системных сведений и сканера.
pub struct AppState {
    pub system: Arc<dyn SystemInfoPort>,
    pub scanner: Arc<dyn VirusScanPort>,
}

/// Чтение выполняется в фоновом потоке, чтобы не блокировать главный поток окна.
#[tauri::command]
pub async fn get_system_overview(
    state: State<'_, AppState>,
) -> Result<SystemOverview, CommandError> {
    let port = Arc::clone(&state.system);
    tauri::async_runtime::spawn_blocking(move || load_overview(port.as_ref()))
        .await
        .map_err(|err| CommandError {
            kind: "probe".into(),
            message: err.to_string(),
        })?
}

/// Проверка идёт в фоне: окно остаётся отзывчивым, пока работает clamscan.
#[tauri::command]
pub async fn scan_start(
    state: State<'_, AppState>,
    target: String,
) -> Result<ScanReport, CommandError> {
    let scanner = Arc::clone(&state.scanner);
    tauri::async_runtime::spawn_blocking(move || scan_with(scanner.as_ref(), &target))
        .await
        .map_err(|err| CommandError {
            kind: "probe".into(),
            message: err.to_string(),
        })?
}

/// Оценка одного файла: проверка пути, сбор признаков и эвристика. Не вердикт.
pub fn assess_path(path: &str) -> Result<Suspicion, CommandError> {
    validate_target(path).map_err(CommandError::from)?;
    let facts = ochistka_platform::windows::files::file_facts(Path::new(path))
        .map_err(CommandError::from)?;
    Ok(score_suspicion(&facts))
}

#[tauri::command]
pub fn assess_file(path: String) -> Result<Suspicion, CommandError> {
    assess_path(&path)
}

#[tauri::command]
pub fn clamav_status(state: State<'_, AppState>) -> EngineStatus {
    state.scanner.engine_status()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::scan::Finding;
    use ochistka_core::system::{CpuStatus, MemoryStatus};

    enum Outcome {
        Ok,
        Unsupported,
        Probe,
    }

    struct FakePort {
        outcome: Outcome,
    }

    impl SystemInfoPort for FakePort {
        fn overview(&self) -> Result<SystemOverview, CoreError> {
            match self.outcome {
                Outcome::Ok => Ok(sample()),
                Outcome::Unsupported => Err(CoreError::Unsupported {
                    feature: "Сведения о системе".into(),
                    os: "macOS".into(),
                }),
                Outcome::Probe => Err(CoreError::Probe("x".into())),
            }
        }
    }

    fn sample() -> SystemOverview {
        SystemOverview {
            cpu: CpuStatus::new(12.0, 8),
            memory: MemoryStatus::new(100, 50),
            disks: vec![],
        }
    }

    #[test]
    fn load_overview_passes_through_snapshot() {
        let port = FakePort {
            outcome: Outcome::Ok,
        };
        assert_eq!(load_overview(&port), Ok(sample()));
    }

    #[test]
    fn load_overview_maps_unsupported_to_kind_unsupported() {
        let port = FakePort {
            outcome: Outcome::Unsupported,
        };
        let err = load_overview(&port).unwrap_err();
        assert_eq!(err.kind, "unsupported");
        assert_eq!(err.message, "Сведения о системе недоступно на macOS");
    }

    #[test]
    fn load_overview_maps_probe_error_to_kind_probe() {
        let port = FakePort {
            outcome: Outcome::Probe,
        };
        let err = load_overview(&port).unwrap_err();
        assert_eq!(err.kind, "probe");
        assert_eq!(err.message, "ошибка чтения системных данных: x");
    }

    struct FakeScanner {
        engine_ok: bool,
    }

    impl VirusScanPort for FakeScanner {
        fn engine_status(&self) -> EngineStatus {
            EngineStatus {
                available: self.engine_ok,
                reason: if self.engine_ok {
                    String::new()
                } else {
                    "clamscan.exe не найден".into()
                },
            }
        }

        fn scan(&self, _target: &str) -> Result<ScanReport, CoreError> {
            if !self.engine_ok {
                return Err(CoreError::EngineMissing("clamscan.exe не найден".into()));
            }
            Ok(ScanReport {
                scanned_files: 2,
                infected_files: 1,
                errors: 0,
                findings: vec![Finding {
                    path: "C:\\a.exe".into(),
                    signature: "Win.Test.X".into(),
                }],
            })
        }
    }

    #[test]
    fn scan_returns_engine_report() {
        let report = scan_with(&FakeScanner { engine_ok: true }, "C:\\Users").unwrap();
        assert_eq!(report.infected_files, 1);
        assert_eq!(report.findings[0].signature, "Win.Test.X");
    }

    #[test]
    fn scan_maps_missing_engine_to_kind_engine_missing() {
        let err = scan_with(&FakeScanner { engine_ok: false }, "C:\\Users").unwrap_err();
        assert_eq!(err.kind, "engineMissing");
        assert_eq!(err.message, "движок недоступен: clamscan.exe не найден");
    }

    #[test]
    fn scan_maps_invalid_target_to_kind_invalid_target() {
        let err = scan_with(&FakeScanner { engine_ok: true }, "Downloads").unwrap_err();
        assert_eq!(err.kind, "invalidTarget");
    }
    #[test]
    fn assess_rejects_relative_path() {
        assert_eq!(
            assess_path("invoice.exe").unwrap_err().kind,
            "invalidTarget"
        );
    }

    #[test]
    fn assess_reports_missing_file_as_probe() {
        assert_eq!(assess_path("Z:\\missing\\x.exe").unwrap_err().kind, "probe");
    }
}
