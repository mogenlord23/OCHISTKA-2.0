//! Команда «Диски»: глубокая очистка, очистка файлов, анализ пространства и приватность.
//! Один типизированный вход `disks_request` с полем `action`: весь доступ к диску идёт через ядро и порт платформы.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use ochistka_core::cleanup::{self, CleanupAnalysis, CleanupCategory, CleanupLocation};
use ochistka_core::fs::{CleanupError, Limits};
use ochistka_core::layout::{CleanupEnvPort, Layout};
use ochistka_core::policy::{validate_drive, validate_root, PathPolicy};
use ochistka_core::quarantine::{MoveOutcome, Quarantine, QuarantineEntry};
use ochistka_core::readonly::{
    folder_sizes, large_old_files, privacy_report, LargeFilesQuery, LargeFilesReport, PrivacyItem,
    SpaceReport,
};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Ошибка для интерфейса: `kind` — тип для ветвления, `message` — текст на русском.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DisksError {
    pub kind: String,
    pub message: String,
}

impl From<CleanupError> for DisksError {
    fn from(err: CleanupError) -> Self {
        let kind = match err {
            CleanupError::Refused(_) => "refused",
            CleanupError::NotFound(_) => "notFound",
            CleanupError::Occupied(_) => "occupied",
            CleanupError::Cancelled => "cancelled",
            CleanupError::InvalidId => "invalidId",
            CleanupError::ConfirmationMismatch => "confirmationMismatch",
            CleanupError::HashMismatch(_) => "hashMismatch",
            CleanupError::EntryNotFound => "entryNotFound",
            CleanupError::ManifestCorrupt => "manifestCorrupt",
            CleanupError::NoPlan => "noPlan",
            CleanupError::Io(_) => "io",
            CleanupError::Unsupported(_) => "unsupported",
        };
        Self {
            kind: kind.to_string(),
            message: err.to_string(),
        }
    }
}

/// Запросы интерфейса. Пути из интерфейса не принимаются там, где можно обойтись id или категорией.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum DisksRequest {
    AnalyzeCleanup,
    StopAnalysis,
    CleanSelected {
        ids: Vec<String>,
    },
    ListQuarantine,
    Restore {
        id: String,
        alternative: Option<String>,
    },
    Purge {
        id: String,
        token: String,
    },
    LargeFiles,
    SpaceReport {
        drive: String,
    },
    PrivacyReport,
    ListDrives,
    /// Находки «Вирусов» в карантин. Пути проверяет белый список (папка «Загрузки»).
    QuarantineFindings {
        items: Vec<FindingItem>,
    },
}

/// Одна находка для карантина: путь и короткая причина (сигнатура или оценка).
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FindingItem {
    pub path: String,
    pub reason: String,
}

/// Больше находок за один запрос не переносим: интерфейс отправляет выбранное, а не весь список.
const MAX_FINDINGS_PER_REQUEST: usize = 500;

impl DisksRequest {
    /// Анализы идут под общим флагом «занято» и останавливаются кнопкой «Стоп».
    fn is_analysis(&self) -> bool {
        matches!(
            self,
            Self::AnalyzeCleanup
                | Self::LargeFiles
                | Self::SpaceReport { .. }
                | Self::PrivacyReport
        )
    }
}

/// Ответы интерфейсу: тип ответа в поле `type`.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DisksResponse {
    Cleanup { categories: Vec<CleanupCategory> },
    Stopped,
    Cleaned { outcome: MoveOutcome },
    Quarantine { entries: Vec<QuarantineEntry> },
    Restored { entry: QuarantineEntry },
    Purged { entry: QuarantineEntry },
    LargeFiles { report: LargeFilesReport },
    Space { report: SpaceReport },
    Privacy { items: Vec<PrivacyItem> },
    Drives { drives: Vec<String> },
}

/// Состояние раздела: порт среды, флаг отмены, флаг занятости и последний анализ очистки.
pub struct DisksState {
    env: Arc<dyn CleanupEnvPort>,
    cancel: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    plan: Arc<Mutex<Option<CleanupAnalysis>>>,
}

impl DisksState {
    pub fn new(env: Box<dyn CleanupEnvPort>) -> Self {
        Self {
            env: Arc::from(env),
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
            plan: Arc::new(Mutex::new(None)),
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

fn begin_analysis(busy: &Arc<AtomicBool>, cancel: &AtomicBool) -> Result<BusyGuard, DisksError> {
    busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| DisksError {
            kind: "busy".into(),
            message: "анализ уже идёт".into(),
        })?;
    // Остановка предыдущего анализа не должна задеть новый.
    cancel.store(false, Ordering::SeqCst);
    Ok(BusyGuard(Arc::clone(busy)))
}

fn lock_plan(plan: &Mutex<Option<CleanupAnalysis>>) -> MutexGuard<'_, Option<CleanupAnalysis>> {
    plan.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// Категории, у которых корень проходит правила. Остальные (например, системная TEMP) пропускаются.
fn usable_locations(layout: &Layout) -> Vec<CleanupLocation> {
    layout
        .cleanup
        .iter()
        .filter(|location| validate_root(&location.root).is_ok())
        .cloned()
        .collect()
}

fn policy_for(locations: &[CleanupLocation]) -> Result<PathPolicy, CleanupError> {
    let roots: Vec<&str> = locations
        .iter()
        .map(|location| location.root.as_str())
        .collect();
    PathPolicy::new(&roots)
}

/// Карантин и восстановление: белый список очистки плюс папка «Загрузки», откуда берутся находки.
fn quarantine_for(layout: &Layout) -> Result<Quarantine, CleanupError> {
    let mut roots: Vec<String> = usable_locations(layout)
        .into_iter()
        .map(|location| location.root)
        .collect();
    if validate_root(&layout.downloads).is_ok() {
        roots.push(layout.downloads.clone());
    }
    let refs: Vec<&str> = roots.iter().map(String::as_str).collect();
    Quarantine::new(&layout.quarantine_root, PathPolicy::new(&refs)?)
}

/// Выполняет запрос. Отдельная функция: её можно проверить без окна Tauri.
pub fn run(
    env: &dyn CleanupEnvPort,
    cancel: &AtomicBool,
    plan: &Mutex<Option<CleanupAnalysis>>,
    request: DisksRequest,
) -> Result<DisksResponse, CleanupError> {
    let limits = Limits::default();
    match request {
        DisksRequest::StopAnalysis => Ok(DisksResponse::Stopped),
        DisksRequest::AnalyzeCleanup => {
            let layout = env.layout()?;
            let fs = env.fs()?;
            let locations = usable_locations(&layout);
            let policy = policy_for(&locations)?;
            let analysis = cleanup::analyze(fs, &policy, &locations, cancel, limits)?;
            let categories = analysis.categories.clone();
            *lock_plan(plan) = Some(analysis);
            Ok(DisksResponse::Cleanup { categories })
        }
        DisksRequest::CleanSelected { ids } => {
            // План расходуется: повторный перенос тех же файлов невозможен без нового анализа.
            let analysis = lock_plan(plan).take().ok_or(CleanupError::NoPlan)?;
            let layout = env.layout()?;
            let fs = env.fs()?;
            let quarantine = quarantine_for(&layout)?;
            let outcome = cleanup::clean_selected(fs, &quarantine, &analysis, &ids, now_unix())?;
            Ok(DisksResponse::Cleaned { outcome })
        }
        DisksRequest::ListQuarantine => {
            let layout = env.layout()?;
            let entries = quarantine_for(&layout)?.list(env.fs()?)?;
            Ok(DisksResponse::Quarantine { entries })
        }
        DisksRequest::Restore { id, alternative } => {
            let layout = env.layout()?;
            let entry = quarantine_for(&layout)?.restore(env.fs()?, &id, alternative.as_deref())?;
            Ok(DisksResponse::Restored { entry })
        }
        DisksRequest::Purge { id, token } => {
            let layout = env.layout()?;
            let entry = quarantine_for(&layout)?.purge(env.fs()?, &id, &token)?;
            Ok(DisksResponse::Purged { entry })
        }
        DisksRequest::LargeFiles => {
            let layout = env.layout()?;
            if layout.downloads.is_empty() {
                return Err(CleanupError::NotFound("папка «Загрузки»".into()));
            }
            let report = large_old_files(
                env.fs()?,
                &layout.downloads,
                LargeFilesQuery::default(),
                now_unix(),
                cancel,
                limits,
            )?;
            Ok(DisksResponse::LargeFiles { report })
        }
        DisksRequest::SpaceReport { drive } => {
            let report = folder_sizes(env.fs()?, &validate_drive(&drive)?, cancel, limits)?;
            Ok(DisksResponse::Space { report })
        }
        DisksRequest::PrivacyReport => {
            let layout = env.layout()?;
            let items = privacy_report(env.fs()?, &layout.privacy, cancel, limits)?;
            Ok(DisksResponse::Privacy { items })
        }
        DisksRequest::ListDrives => {
            let layout = env.layout()?;
            Ok(DisksResponse::Drives {
                drives: layout.drives,
            })
        }
        DisksRequest::QuarantineFindings { items } => {
            if items.len() > MAX_FINDINGS_PER_REQUEST {
                return Err(CleanupError::Refused(format!(
                    "за один раз не больше {MAX_FINDINGS_PER_REQUEST} находок"
                )));
            }
            // Находка содержит только путь: связать его с проверенным объектом невозможно.
            // Обычный rename допускает подмену источника и перезапись назначения.
            Err(CleanupError::Refused(
                "перенос находок временно недоступен: безопасная проверка файла ещё не реализована"
                    .into(),
            ))
        }
    }
}

/// Все запросы раздела «Диски». Файловые операции идут в фоновом потоке, окно не блокируется.
#[tauri::command]
pub async fn disks_request(
    state: State<'_, DisksState>,
    request: DisksRequest,
) -> Result<DisksResponse, DisksError> {
    if matches!(request, DisksRequest::StopAnalysis) {
        state.cancel.store(true, Ordering::SeqCst);
        return Ok(DisksResponse::Stopped);
    }
    let guard = if request.is_analysis() {
        Some(begin_analysis(&state.busy, &state.cancel)?)
    } else {
        None
    };
    let env = Arc::clone(&state.env);
    let cancel = Arc::clone(&state.cancel);
    let plan = Arc::clone(&state.plan);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        run(env.as_ref(), &cancel, &plan, request)
    })
    .await
    .map_err(|err| DisksError {
        kind: "io".into(),
        message: err.to_string(),
    })?
    .map_err(DisksError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::fs::FsPort;

    /// Среда без поддержки: проверяет, что раздел отвечает причиной, а не падает.
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
    fn clean_without_analysis_is_refused_with_reason() {
        let plan = Mutex::new(None);
        let cancel = AtomicBool::new(false);
        let err = run(
            &NoEnv,
            &cancel,
            &plan,
            DisksRequest::CleanSelected { ids: vec![] },
        )
        .unwrap_err();
        assert_eq!(err, CleanupError::NoPlan);
        assert_eq!(DisksError::from(err).kind, "noPlan");
    }

    #[test]
    fn unsupported_environment_maps_to_unsupported_kind() {
        let plan = Mutex::new(None);
        let cancel = AtomicBool::new(false);
        let err = run(&NoEnv, &cancel, &plan, DisksRequest::AnalyzeCleanup).unwrap_err();
        assert_eq!(DisksError::from(err).kind, "unsupported");
    }

    #[test]
    fn busy_flag_rejects_second_analysis_and_releases_on_drop() {
        let busy = Arc::new(AtomicBool::new(false));
        let cancel = AtomicBool::new(true);
        let first = begin_analysis(&busy, &cancel).unwrap();
        assert!(
            !cancel.load(Ordering::SeqCst),
            "new analysis must clear a stale stop"
        );
        assert_eq!(begin_analysis(&busy, &cancel).err().unwrap().kind, "busy");
        drop(first);
        assert!(begin_analysis(&busy, &cancel).is_ok());
    }

    #[test]
    fn requests_parse_from_the_typed_wire_format() {
        let parsed: DisksRequest =
            serde_json::from_str(r#"{"action":"cleanSelected","ids":["temp"]}"#).unwrap();
        assert_eq!(
            parsed,
            DisksRequest::CleanSelected {
                ids: vec!["temp".into()]
            }
        );
        let parsed: DisksRequest = serde_json::from_str(
            r#"{"action":"restore","id":"0123456789abcdef","alternative":null}"#,
        )
        .unwrap();
        assert!(matches!(
            parsed,
            DisksRequest::Restore {
                alternative: None,
                ..
            }
        ));
    }
}

#[cfg(test)]
mod findings_tests {
    use super::*;
    use ochistka_core::fs::FsPort;
    use ochistka_platform::windows_cleanup::WindowsFs;

    /// Среда с раскладкой из строк: файловые операции — реальный адаптер, но пути вне белого списка.
    struct TestEnv {
        fs: WindowsFs,
    }

    impl CleanupEnvPort for TestEnv {
        fn layout(&self) -> Result<Layout, CleanupError> {
            Ok(Layout {
                cleanup: vec![],
                downloads: "C:\\Users\\Slava\\Downloads".into(),
                privacy: vec![],
                quarantine_root: "C:\\Users\\Slava\\AppData\\Local\\OCHISTKA\\Quarantine".into(),
                drives: vec![],
            })
        }

        fn fs(&self) -> Result<&dyn FsPort, CleanupError> {
            Ok(&self.fs)
        }
    }

    fn quarantine_request(paths: &[&str]) -> DisksRequest {
        DisksRequest::QuarantineFindings {
            items: paths
                .iter()
                .map(|path| FindingItem {
                    path: (*path).to_string(),
                    reason: "Win.Test.X".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn findings_are_refused_until_identity_can_be_verified() {
        let env = TestEnv { fs: WindowsFs };
        let plan = Mutex::new(None);
        let cancel = AtomicBool::new(false);
        let error = run(
            &env,
            &cancel,
            &plan,
            quarantine_request(&["C:\\Users\\Slava\\Downloads\\evil.exe"]),
        )
        .unwrap_err();
        assert!(matches!(error, CleanupError::Refused(_)));
    }

    #[test]
    fn too_many_findings_in_one_request_are_refused() {
        let env = TestEnv { fs: WindowsFs };
        let plan = Mutex::new(None);
        let cancel = AtomicBool::new(false);
        let paths: Vec<String> = (0..=MAX_FINDINGS_PER_REQUEST)
            .map(|i| format!("C:\\Users\\Slava\\Downloads\\f{i}.exe"))
            .collect();
        let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
        let err = run(&env, &cancel, &plan, quarantine_request(&refs)).unwrap_err();
        assert_eq!(DisksError::from(err).kind, "refused");
    }
}
