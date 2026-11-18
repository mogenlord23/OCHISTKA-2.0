//! Команда «Программы»: установленные программы, автозагрузка и размер папки. Только чтение.
//! Один типизированный вход `programs_request` с полем `action`. Удаления нет: кнопка в интерфейсе выключена.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ochistka_core::fs::CleanupError;
use ochistka_core::layout::CleanupEnvPort;
use ochistka_core::programs::{
    build_inventory, build_startup, measure_folder, validate_measure_path, FolderMeasure,
    InstalledProgram, ProgramsPort, StartupEntry, FOLDER_LIMITS,
};
use ochistka_core::system::CoreError;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Ошибка для интерфейса: `kind` — тип для ветвления, `message` — текст на русском.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ProgramsError {
    pub kind: String,
    pub message: String,
}

impl From<CoreError> for ProgramsError {
    fn from(err: CoreError) -> Self {
        let kind = match err {
            CoreError::Unsupported { .. } => "unsupported",
            CoreError::Probe(_) => "probe",
            CoreError::EngineMissing(_) => "engineMissing",
            CoreError::InvalidTarget(_) => "refused",
        };
        Self {
            kind: kind.to_string(),
            message: err.to_string(),
        }
    }
}

impl From<CleanupError> for ProgramsError {
    fn from(err: CleanupError) -> Self {
        let kind = match err {
            CleanupError::Refused(_) => "refused",
            CleanupError::NotFound(_) => "notFound",
            CleanupError::Cancelled => "cancelled",
            CleanupError::Io(_) => "io",
            CleanupError::Unsupported(_) => "unsupported",
            _ => "io",
        };
        Self {
            kind: kind.to_string(),
            message: err.to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum ProgramsRequest {
    ListInstalled,
    ListStartup,
    /// Размер папки установки. Путь проверяется в ядре (абсолютный, без `..`).
    MeasureFolder {
        path: String,
    },
    StopMeasure,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProgramsResponse {
    Installed { programs: Vec<InstalledProgram> },
    Startup { entries: Vec<StartupEntry> },
    Folder { report: FolderMeasure },
    Stopped,
}

/// Состояние раздела: порт реестра, среда для файлов, флаг отмены и занятости подсчёта.
pub struct ProgramsState {
    registry: Arc<dyn ProgramsPort>,
    env: Arc<dyn CleanupEnvPort>,
    cancel: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
}

impl ProgramsState {
    pub fn new(registry: Box<dyn ProgramsPort>, env: Box<dyn CleanupEnvPort>) -> Self {
        Self {
            registry: Arc::from(registry),
            env: Arc::from(env),
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Освобождает флаг занятости при выходе, в том числе при панике.
struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Один подсчёт за раз; новый подсчёт сбрасывает прежнюю отмену.
fn begin_measure(busy: &Arc<AtomicBool>, cancel: &AtomicBool) -> Result<BusyGuard, ProgramsError> {
    busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| ProgramsError {
            kind: "busy".into(),
            message: "подсчёт размера уже идёт".into(),
        })?;
    cancel.store(false, Ordering::SeqCst);
    Ok(BusyGuard(Arc::clone(busy)))
}

/// Выполняет запрос. Отдельная функция: её можно проверить без окна Tauri.
pub fn run(
    registry: &dyn ProgramsPort,
    env: &dyn CleanupEnvPort,
    cancel: &AtomicBool,
    request: ProgramsRequest,
) -> Result<ProgramsResponse, ProgramsError> {
    match request {
        ProgramsRequest::ListInstalled => {
            let records = registry.installed_records()?;
            Ok(ProgramsResponse::Installed {
                programs: build_inventory(&records),
            })
        }
        ProgramsRequest::ListStartup => {
            let records = registry.startup_records()?;
            Ok(ProgramsResponse::Startup {
                entries: build_startup(records),
            })
        }
        ProgramsRequest::MeasureFolder { path } => {
            let root = validate_measure_path(&path)?;
            let fs = env.fs()?;
            let report = measure_folder(fs, &root, cancel, FOLDER_LIMITS)?;
            Ok(ProgramsResponse::Folder { report })
        }
        ProgramsRequest::StopMeasure => Ok(ProgramsResponse::Stopped),
    }
}

/// Все запросы раздела «Программы». Чтение реестра и папок идёт в фоновом потоке.
#[tauri::command]
pub async fn programs_request(
    state: State<'_, ProgramsState>,
    request: ProgramsRequest,
) -> Result<ProgramsResponse, ProgramsError> {
    if matches!(request, ProgramsRequest::StopMeasure) {
        state.cancel.store(true, Ordering::SeqCst);
        return Ok(ProgramsResponse::Stopped);
    }
    let guard = if matches!(request, ProgramsRequest::MeasureFolder { .. }) {
        Some(begin_measure(&state.busy, &state.cancel)?)
    } else {
        None
    };
    let registry = Arc::clone(&state.registry);
    let env = Arc::clone(&state.env);
    let cancel = Arc::clone(&state.cancel);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        run(registry.as_ref(), env.as_ref(), &cancel, request)
    })
    .await
    .map_err(|err| ProgramsError {
        kind: "io".into(),
        message: err.to_string(),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::fs::FsPort;
    use ochistka_core::layout::Layout;
    use ochistka_core::programs::{RawProgramRecord, RawSource, RawStartup, RawValue};
    use std::collections::BTreeMap;

    /// Реестр в памяти: одна программа и одна запись автозагрузки.
    struct FakeRegistry;

    impl ProgramsPort for FakeRegistry {
        fn installed_records(&self) -> Result<Vec<RawProgramRecord>, CoreError> {
            let mut values = BTreeMap::new();
            values.insert("DisplayName".into(), RawValue::Text("Tool".into()));
            values.insert("UninstallString".into(), RawValue::Text("u.exe".into()));
            Ok(vec![RawProgramRecord {
                source: RawSource::Uninstall,
                values,
            }])
        }

        fn startup_records(&self) -> Result<Vec<RawStartup>, CoreError> {
            Ok(vec![RawStartup {
                source: ochistka_core::programs::StartupSource::HkcuRun,
                name: "Helper".into(),
                command: Some("h.exe".into()),
            }])
        }
    }

    /// Среда без поддержки: раздел отвечает причиной, а не падает.
    struct NoEnv;

    impl CleanupEnvPort for NoEnv {
        fn layout(&self) -> Result<Layout, CleanupError> {
            Err(CleanupError::Unsupported("очистка дисков".into()))
        }

        fn fs(&self) -> Result<&dyn FsPort, CleanupError> {
            Err(CleanupError::Unsupported("очистка дисков".into()))
        }
    }

    #[test]
    fn lists_installed_programs_through_the_port() {
        let cancel = AtomicBool::new(false);
        let response = run(
            &FakeRegistry,
            &NoEnv,
            &cancel,
            ProgramsRequest::ListInstalled,
        )
        .unwrap();
        match response {
            ProgramsResponse::Installed { programs } => {
                assert_eq!(programs.len(), 1);
                assert_eq!(programs[0].name, "Tool");
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn lists_startup_entries_through_the_port() {
        let cancel = AtomicBool::new(false);
        let response = run(&FakeRegistry, &NoEnv, &cancel, ProgramsRequest::ListStartup).unwrap();
        match response {
            ProgramsResponse::Startup { entries } => {
                assert_eq!(entries[0].name, "Helper");
                assert_eq!(entries[0].command.as_deref(), Some("h.exe"));
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn measure_refuses_a_relative_path_before_touching_the_disk() {
        let cancel = AtomicBool::new(false);
        let err = run(
            &FakeRegistry,
            &NoEnv,
            &cancel,
            ProgramsRequest::MeasureFolder {
                path: "App\\x".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err.kind, "refused");
    }

    #[test]
    fn measure_without_file_access_reports_unsupported() {
        let cancel = AtomicBool::new(false);
        let err = run(
            &FakeRegistry,
            &NoEnv,
            &cancel,
            ProgramsRequest::MeasureFolder {
                path: "C:\\Apps\\Tool".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err.kind, "unsupported");
    }

    #[test]
    fn second_measure_is_rejected_while_one_runs_and_flag_is_released_on_drop() {
        let busy = Arc::new(AtomicBool::new(false));
        let cancel = AtomicBool::new(true);
        let first = begin_measure(&busy, &cancel).unwrap();
        assert!(
            !cancel.load(Ordering::SeqCst),
            "new measure clears a stale stop"
        );
        assert_eq!(begin_measure(&busy, &cancel).err().unwrap().kind, "busy");
        drop(first);
        assert!(begin_measure(&busy, &cancel).is_ok());
    }

    #[test]
    fn requests_parse_from_the_typed_wire_format() {
        let parsed: ProgramsRequest =
            serde_json::from_str(r#"{"action":"measureFolder","path":"C:\\Apps"}"#).unwrap();
        assert_eq!(
            parsed,
            ProgramsRequest::MeasureFolder {
                path: "C:\\Apps".into()
            }
        );
        let parsed: ProgramsRequest = serde_json::from_str(r#"{"action":"stopMeasure"}"#).unwrap();
        assert_eq!(parsed, ProgramsRequest::StopMeasure);
    }
}
