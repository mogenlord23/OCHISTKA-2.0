//! Команда «Программы»: установленные программы, автозагрузка, размер папки, значки,
//! удаление штатным деинсталлятором и флаг автозагрузки. Один типизированный вход
//! `programs_request` с полем `action`. Изменяющие действия требуют `confirmed: true`
//! (подтверждение в диалоге интерфейса) и в тестах не выполняются.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ochistka_core::fs::CleanupError;
use ochistka_core::layout::CleanupEnvPort;
use ochistka_core::programs::icons::{png_data_url, validate_icon_path, IconCache};
use ochistka_core::programs::startup_state::valid_value_name;
use ochistka_core::programs::{
    build_inventory, build_startup, measure_folder, uninstall_command_for, validate_measure_path,
    FolderMeasure, InstalledProgram, ProgramsPort, StartupEntry, StartupSource, FOLDER_LIMITS,
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
            CoreError::Cancelled => "cancelled",
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
    /// Значок файла (exe, dll, ico, lnk) в виде PNG-адреса. Только чтение.
    Icon {
        path: String,
    },
    /// Штатное удаление по `id` программы. Без `confirmed` ядро не вызывается.
    Uninstall {
        id: String,
        confirmed: bool,
    },
    /// Включить или отключить запись автозагрузки (флаг `StartupApproved`).
    SetStartupEnabled {
        source: String,
        key: String,
        enabled: bool,
        confirmed: bool,
    },
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProgramsResponse {
    Installed {
        programs: Vec<InstalledProgram>,
    },
    Startup {
        entries: Vec<StartupEntry>,
    },
    Folder {
        report: FolderMeasure,
    },
    Stopped,
    /// Адрес PNG или `null`, если значок взять не удалось (интерфейс рисует букву).
    Icon {
        data_url: Option<String>,
    },
    /// Удаление запущено: окно деинсталлятора открыто, результат он покажет сам.
    UninstallStarted,
    StartupChanged {
        previous_enabled: bool,
        enabled: bool,
    },
}

/// Состояние раздела: порт реестра, среда для файлов, флаг отмены и занятости подсчёта.
pub struct ProgramsState {
    registry: Arc<dyn ProgramsPort>,
    env: Arc<dyn CleanupEnvPort>,
    cancel: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    icons: Arc<IconCache>,
}

impl ProgramsState {
    pub fn new(registry: Box<dyn ProgramsPort>, env: Box<dyn CleanupEnvPort>) -> Self {
        Self {
            registry: Arc::from(registry),
            env: Arc::from(env),
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
            icons: Arc::new(IconCache::new(512)),
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
    icons: &IconCache,
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
        ProgramsRequest::Icon { path } => {
            let path = validate_icon_path(&path).map_err(|message| ProgramsError {
                kind: "refused".into(),
                message,
            })?;
            if let Some(cached) = icons.get(&path) {
                return Ok(ProgramsResponse::Icon { data_url: cached });
            }
            // Неудача (нет файла, нет значка, чужая ОС) не ошибка: кэшируем и рисуем букву.
            let data_url = registry
                .icon_png_base64(&path)
                .ok()
                .map(|png| png_data_url(&png));
            icons.put(&path, data_url.clone());
            Ok(ProgramsResponse::Icon { data_url })
        }
        ProgramsRequest::Uninstall { id, confirmed } => {
            if !confirmed {
                return Err(not_confirmed("удаление"));
            }
            let records = registry.installed_records()?;
            let command = uninstall_command_for(&records, &id, &registry.system_root())?;
            registry.run_uninstaller(&command)?;
            Ok(ProgramsResponse::UninstallStarted)
        }
        ProgramsRequest::SetStartupEnabled {
            source,
            key,
            enabled,
            confirmed,
        } => {
            if !confirmed {
                return Err(not_confirmed("изменение автозагрузки"));
            }
            let source = StartupSource::from_id(&source).ok_or_else(|| ProgramsError {
                kind: "refused".into(),
                message: "неизвестный источник автозагрузки".into(),
            })?;
            if !valid_value_name(&key) {
                return Err(ProgramsError {
                    kind: "refused".into(),
                    message: "недопустимое имя записи автозагрузки".into(),
                });
            }
            let toggle = registry.set_startup_enabled(source, &key, enabled)?;
            Ok(ProgramsResponse::StartupChanged {
                previous_enabled: toggle.previous_enabled,
                enabled: toggle.enabled,
            })
        }
    }
}

fn not_confirmed(what: &str) -> ProgramsError {
    ProgramsError {
        kind: "notConfirmed".into(),
        message: format!("{what} не подтверждено пользователем"),
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
    let icons = Arc::clone(&state.icons);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        run(registry.as_ref(), env.as_ref(), &cancel, &icons, request)
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
                value_name: "Helper".into(),
                approved: None,
                backup: None,
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
            &IconCache::new(4),
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
        let response = run(
            &FakeRegistry,
            &NoEnv,
            &cancel,
            &IconCache::new(4),
            ProgramsRequest::ListStartup,
        )
        .unwrap();
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
            &IconCache::new(4),
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
            &IconCache::new(4),
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

    /// Порт, который считает опасные вызовы: в тестах настоящий запуск не происходит.
    #[derive(Default)]
    struct Counting {
        uninstalls: std::sync::atomic::AtomicUsize,
        toggles: std::sync::atomic::AtomicUsize,
        icon_calls: std::sync::atomic::AtomicUsize,
    }

    impl ProgramsPort for Counting {
        fn installed_records(&self) -> Result<Vec<RawProgramRecord>, CoreError> {
            let mut values = BTreeMap::new();
            values.insert("DisplayName".into(), RawValue::Text("Tool".into()));
            values.insert(
                "UninstallString".into(),
                RawValue::Text("\"C:\\Tool\\unins.exe\" /S".into()),
            );
            Ok(vec![RawProgramRecord {
                source: RawSource::Uninstall,
                values,
            }])
        }

        fn startup_records(&self) -> Result<Vec<RawStartup>, CoreError> {
            Ok(Vec::new())
        }

        fn icon_png_base64(&self, _path: &str) -> Result<String, CoreError> {
            self.icon_calls.fetch_add(1, Ordering::SeqCst);
            Ok("QUJD".into())
        }

        fn run_uninstaller(
            &self,
            command: &ochistka_core::programs::uninstall::UninstallCommand,
        ) -> Result<(), CoreError> {
            assert_eq!(command.exe, "C:\\Tool\\unins.exe");
            self.uninstalls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn set_startup_enabled(
            &self,
            _source: StartupSource,
            _value_name: &str,
            enable: bool,
        ) -> Result<ochistka_core::programs::StartupToggle, CoreError> {
            self.toggles.fetch_add(1, Ordering::SeqCst);
            Ok(ochistka_core::programs::StartupToggle {
                previous_enabled: !enable,
                enabled: enable,
            })
        }
    }

    fn call(port: &Counting, request: ProgramsRequest) -> Result<ProgramsResponse, ProgramsError> {
        run(
            port,
            &NoEnv,
            &AtomicBool::new(false),
            &IconCache::new(4),
            request,
        )
    }

    #[test]
    fn uninstall_needs_confirmation_and_never_runs_without_it() {
        let port = Counting::default();
        let id = "tool||".to_string();
        let err = call(
            &port,
            ProgramsRequest::Uninstall {
                id: id.clone(),
                confirmed: false,
            },
        )
        .unwrap_err();
        assert_eq!(err.kind, "notConfirmed");
        assert_eq!(port.uninstalls.load(Ordering::SeqCst), 0);
        let ok = call(
            &port,
            ProgramsRequest::Uninstall {
                id,
                confirmed: true,
            },
        )
        .unwrap();
        assert_eq!(ok, ProgramsResponse::UninstallStarted);
        assert_eq!(port.uninstalls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn uninstall_of_an_unknown_id_is_refused_without_running() {
        let port = Counting::default();
        let err = call(
            &port,
            ProgramsRequest::Uninstall {
                id: "nothing||".into(),
                confirmed: true,
            },
        )
        .unwrap_err();
        assert_eq!(err.kind, "refused");
        assert_eq!(port.uninstalls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn startup_toggle_needs_confirmation_and_valid_names() {
        let port = Counting::default();
        let make = |key: &str, confirmed: bool| ProgramsRequest::SetStartupEnabled {
            source: "hkcuRun".into(),
            key: key.into(),
            enabled: false,
            confirmed,
        };
        assert_eq!(
            call(&port, make("Helper", false)).unwrap_err().kind,
            "notConfirmed"
        );
        assert_eq!(call(&port, make("a\\b", true)).unwrap_err().kind, "refused");
        assert_eq!(port.toggles.load(Ordering::SeqCst), 0);
        let bad_source = ProgramsRequest::SetStartupEnabled {
            source: "nope".into(),
            key: "x".into(),
            enabled: true,
            confirmed: true,
        };
        assert_eq!(call(&port, bad_source).unwrap_err().kind, "refused");
        let ok = call(&port, make("Helper", true)).unwrap();
        assert_eq!(
            ok,
            ProgramsResponse::StartupChanged {
                previous_enabled: true,
                enabled: false
            }
        );
    }

    #[test]
    fn icons_are_validated_cached_and_failures_are_not_errors() {
        let port = Counting::default();
        let cache = IconCache::new(4);
        let run_icon = |path: &str| {
            run(
                &port,
                &NoEnv,
                &AtomicBool::new(false),
                &cache,
                ProgramsRequest::Icon { path: path.into() },
            )
        };
        assert_eq!(run_icon("calc.exe").unwrap_err().kind, "refused");
        let first = run_icon("C:\\A\\a.exe").unwrap();
        assert_eq!(
            first,
            ProgramsResponse::Icon {
                data_url: Some("data:image/png;base64,QUJD".into())
            }
        );
        run_icon("c:\\a\\A.exe").unwrap();
        assert_eq!(
            port.icon_calls.load(Ordering::SeqCst),
            1,
            "second call is cached"
        );
        // Порт по умолчанию (чужая ОС) даёт `null`, а не ошибку.
        let fallback = run(
            &FakeRegistry,
            &NoEnv,
            &AtomicBool::new(false),
            &cache,
            ProgramsRequest::Icon {
                path: "C:\\B\\b.exe".into(),
            },
        )
        .unwrap();
        assert_eq!(fallback, ProgramsResponse::Icon { data_url: None });
    }
}
