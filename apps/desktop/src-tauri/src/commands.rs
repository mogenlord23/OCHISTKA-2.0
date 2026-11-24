//! Типизированные команды Tauri. Интерфейс получает данные только отсюда.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use std::path::Path;

use ochistka_core::assess::{assess_path, SuspicionModel};
use ochistka_core::fs::{FsPort, Limits};
use ochistka_core::heuristic::FileFacts;
use ochistka_core::scan::{
    sweep, sweep_with_progress, EngineStatus, ScanProgress, ScanReport, SignaturesInfo,
    VirusScanPort,
};
use ochistka_core::system::{CoreError, SystemInfoPort, SystemOverview};
use ochistka_platform::pe_model::PeHeaderModel;
use serde::Serialize;
use tauri::{Emitter, State};
use tauri_plugin_notification::NotificationExt;

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
            CoreError::Cancelled => "cancelled",
            CoreError::NotApplicable(_) => "notApplicable",
            CoreError::NeedsAdmin => "needsAdmin",
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

/// Проверка папки через порт движка; обход и отмена — в ядре.
pub fn scan_with(
    fs: &dyn FsPort,
    port: &dyn VirusScanPort,
    cancel: &AtomicBool,
    target: &str,
) -> Result<ScanReport, CommandError> {
    sweep(fs, port, target, cancel, Limits::default()).map_err(CommandError::from)
}

/// Состояние приложения: адаптеры системных сведений, движка и файлов, модель и флаги проверки.
pub struct AppState {
    pub system: Arc<dyn SystemInfoPort>,
    pub scanner: Arc<dyn VirusScanPort>,
    /// Проверка по признакам: работает, пока нет базы сигнатур.
    pub suspicion: Arc<dyn VirusScanPort>,
    pub files: Arc<dyn FsPort>,
    /// ИИ-слой оценки файла локальной моделью.
    pub model: Arc<dyn SuspicionModel>,
    /// Признак идущей проверки: одновременно допускается только одна.
    pub scan_running: Arc<AtomicBool>,
    /// Запрос отмены текущей проверки; сбрасывается в начале каждой проверки.
    pub scan_cancel: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(
        system: Box<dyn SystemInfoPort>,
        scanner: Box<dyn VirusScanPort>,
        suspicion: Box<dyn VirusScanPort>,
        files: Arc<dyn FsPort>,
    ) -> Self {
        Self {
            system: Arc::from(system),
            scanner: Arc::from(scanner),
            suspicion: Arc::from(suspicion),
            files,
            model: Arc::new(PeHeaderModel),
            scan_running: Arc::new(AtomicBool::new(false)),
            scan_cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Занимает право на проверку и освобождает его при выходе (в том числе при панике).
pub struct ScanGuard(Arc<AtomicBool>);

impl Drop for ScanGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Пытается начать проверку; если одна уже идёт — возвращает ошибку.
pub fn try_begin_scan(flag: &Arc<AtomicBool>) -> Result<ScanGuard, CommandError> {
    if flag
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(CommandError {
            kind: "probe".into(),
            message: "проверка уже идёт".into(),
        });
    }
    Ok(ScanGuard(Arc::clone(flag)))
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

/// Проверка идёт в фоне: окно остаётся отзывчивым. Отмена — через `scan_cancel`.
#[tauri::command]
pub async fn scan_start(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    target: String,
) -> Result<ScanReport, CommandError> {
    run_scan(app, state, Some(target), None).await
}

#[tauri::command]
pub async fn scan_start_with_progress(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    target: String,
    on_progress: tauri::ipc::Channel<ScanProgress>,
) -> Result<ScanReport, CommandError> {
    run_scan(app, state, Some(target), Some(on_progress)).await
}

#[tauri::command]
pub async fn scan_computer(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    on_progress: tauri::ipc::Channel<ScanProgress>,
) -> Result<ScanReport, CommandError> {
    run_scan(app, state, None, Some(on_progress)).await
}

#[tauri::command]
pub async fn scan_pick_folder(window: tauri::Window) -> Result<Option<String>, CommandError> {
    rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .set_title("Выберите папку для проверки")
        .pick_folder()
        .await
        .map(|folder| {
            folder.path().to_str().map(str::to_owned).ok_or_else(|| {
                CoreError::Probe("в имени папки есть неподдерживаемые символы".into()).into()
            })
        })
        .transpose()
}

async fn run_scan(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    target: Option<String>,
    on_progress: Option<tauri::ipc::Channel<ScanProgress>>,
) -> Result<ScanReport, CommandError> {
    let guard = try_begin_scan(&state.scan_running)?;
    state.scan_cancel.store(false, Ordering::SeqCst);
    let _ = app.emit_to("tray-panel", "tray-scan-running", true);
    let tray_events = app.clone();
    // Без загруженной базы сигнатур проверяем по признакам: так проверка работает и честно
    // называет находки «по признакам», а не вирусами.
    let scanner = if state.scanner.engine_status().available {
        Arc::clone(&state.scanner)
    } else {
        Arc::clone(&state.suspicion)
    };
    let files = Arc::clone(&state.files);
    let cancel = Arc::clone(&state.scan_cancel);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let mut last_sent = std::time::Instant::now();
        let mut notified = false;
        let signature_scan = scanner.mode() == ochistka_core::scan::ScanMode::Signatures;
        let mut progress = |update: ScanProgress| {
            if signature_scan && update.infected_files > 0 && !notified {
                notified = true;
                let _ = app.emit_to("main", "scan-threat-found", ());
                let _ = app
                    .notification()
                    .builder()
                    .title("OCHISTKA · Обнаружена угроза")
                    .body("Проверка продолжается. Подробности доступны во вкладке «Находки».")
                    .show();
            }
            if update.processed_files == 0
                || update.processed_files == update.total_files
                || last_sent.elapsed() >= std::time::Duration::from_millis(100)
            {
                if let Some(channel) = &on_progress {
                    let _ = channel.send(update);
                }
                last_sent = std::time::Instant::now();
            }
        };
        match target {
            Some(target) => sweep_with_progress(
                files.as_ref(),
                scanner.as_ref(),
                &target,
                &cancel,
                Limits::default(),
                &mut progress,
            ),
            None => ochistka_platform::computer_scan::scan_computer(
                files.as_ref(),
                scanner.as_ref(),
                &cancel,
                &mut progress,
            ),
        }
        .map_err(CommandError::from)
    })
    .await
    .map_err(|err| CommandError {
        kind: "probe".into(),
        message: err.to_string(),
    });
    let _ = tray_events.emit_to(
        "tray-panel",
        "tray-scan-running",
        state.scan_running.load(Ordering::SeqCst),
    );
    result?
}

/// Просит остановить идущую проверку. Возвращает `true`, если проверка шла.
#[tauri::command]
pub fn scan_cancel(state: State<'_, AppState>) -> bool {
    let running = state.scan_running.load(Ordering::SeqCst);
    if running {
        state.scan_cancel.store(true, Ordering::SeqCst);
    }
    running
}

/// Оценка одного файла: проверка пути, сбор признаков, инвентаризация и эвристика. Не вердикт.
pub fn assess_file_with(
    fs: &dyn FsPort,
    model: &dyn SuspicionModel,
    path: &str,
) -> Result<ochistka_core::assess::Suspicion, CommandError> {
    ochistka_core::scan::validate_target(path).map_err(CommandError::from)?;
    let facts: FileFacts = ochistka_platform::windows::files::file_facts(Path::new(path))
        .map_err(CommandError::from)?;
    assess_path(fs, path, &facts, model).map_err(CommandError::from)
}

/// Чтение файла идёт в фоновом потоке: недоступный сетевой диск не блокирует окно.
#[tauri::command]
pub async fn assess_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<ochistka_core::assess::Suspicion, CommandError> {
    let files = Arc::clone(&state.files);
    let model = Arc::clone(&state.model);
    tauri::async_runtime::spawn_blocking(move || {
        assess_file_with(files.as_ref(), model.as_ref(), &path)
    })
    .await
    .map_err(|err| CommandError {
        kind: "probe".into(),
        message: err.to_string(),
    })?
}

#[tauri::command]
pub fn default_scan_target() -> String {
    ochistka_platform::windows::files::default_downloads().unwrap_or_default()
}

#[tauri::command]
pub fn clamav_status(state: State<'_, AppState>) -> EngineStatus {
    state.scanner.engine_status()
}

/// Где лежат базы и сколько их: экран проверки показывает путь и кнопку «Открыть папку баз».
#[tauri::command]
pub fn clamav_signatures(state: State<'_, AppState>) -> SignaturesInfo {
    let dir = ochistka_platform::windows::clamav::default_signatures_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let files = ochistka_platform::windows::clamav::signatures_files(std::path::Path::new(&dir));
    SignaturesInfo {
        dir,
        files,
        signatures: state.scanner.signature_count(),
        engine_present: ochistka_platform::windows::clamav::engine_present(),
    }
}

/// Загружает базы из папки (по умолчанию — из папки приложения). Возвращает новый статус.
#[tauri::command]
pub fn clamav_load_signatures(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<EngineStatus, CommandError> {
    let dir = match path {
        Some(path) if !path.trim().is_empty() => path,
        _ => ochistka_platform::windows::clamav::ensure_signatures_dir()
            .map_err(|message| CommandError {
                kind: "probe".into(),
                message,
            })?
            .to_string_lossy()
            .into_owned(),
    };
    state
        .scanner
        .load_signatures(&dir)
        .map_err(CommandError::from)?;
    Ok(state.scanner.engine_status())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::assess::NoModel;
    use ochistka_core::scan::{EngineStatus, FileVerdict};
    use ochistka_core::system::{CpuStatus, MemoryStatus};
    use ochistka_platform::windows::clamav::ClamAvEngine;
    use ochistka_platform::windows_cleanup::WindowsFs;

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
            os_name: "Windows 11".into(),
            os_version: "24H2".into(),
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

    /// Движок, который считает любой файл заражённым (проверяем проводку, не сигнатуры).
    struct AlwaysInfected;

    impl VirusScanPort for AlwaysInfected {
        fn engine_status(&self) -> EngineStatus {
            EngineStatus {
                available: true,
                reason: String::new(),
            }
        }

        fn scan_file(&self, _path: &str) -> Result<FileVerdict, CoreError> {
            Ok(FileVerdict::Found {
                signature: "Win.Test.X".into(),
            })
        }

        fn load_signatures(&self, _dir: &str) -> Result<(), CoreError> {
            Ok(())
        }
    }

    fn temp_folder_with_one_file(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("ochistka-cmd-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.exe"), b"x").unwrap();
        root
    }

    #[test]
    fn scan_returns_engine_report() {
        let root = temp_folder_with_one_file("report");
        let cancel = AtomicBool::new(false);
        let report = scan_with(
            &WindowsFs,
            &AlwaysInfected,
            &cancel,
            &root.to_string_lossy(),
        );
        std::fs::remove_dir_all(&root).ok();
        let report = report.unwrap();
        assert_eq!(report.infected_files, 1);
        assert_eq!(report.findings[0].signature, "Win.Test.X");
    }

    #[test]
    fn stub_engine_reports_engine_not_built_as_engine_missing() {
        let cancel = AtomicBool::new(false);
        let err =
            scan_with(&WindowsFs, &ClamAvEngine::not_built(), &cancel, "C:\\Users").unwrap_err();
        assert_eq!(err.kind, "engineMissing");
        assert!(err
            .message
            .starts_with("движок недоступен: движок не собран"));
    }

    #[test]
    fn scan_maps_invalid_target_to_kind_invalid_target() {
        let cancel = AtomicBool::new(false);
        let err = scan_with(&WindowsFs, &AlwaysInfected, &cancel, "Downloads").unwrap_err();
        assert_eq!(err.kind, "invalidTarget");
    }

    #[test]
    fn cancelled_scan_maps_to_kind_cancelled() {
        let root = temp_folder_with_one_file("cancel");
        let cancel = AtomicBool::new(true);
        let err = scan_with(
            &WindowsFs,
            &AlwaysInfected,
            &cancel,
            &root.to_string_lossy(),
        )
        .unwrap_err();
        std::fs::remove_dir_all(&root).ok();
        assert_eq!(err.kind, "cancelled");
        assert_eq!(err.message, "проверка отменена");
    }

    #[test]
    fn second_scan_is_rejected_while_one_runs_and_allowed_after() {
        let flag = Arc::new(AtomicBool::new(false));
        let first = try_begin_scan(&flag).unwrap();
        let err = try_begin_scan(&flag).err().unwrap();
        assert_eq!(err.kind, "probe");
        assert_eq!(err.message, "проверка уже идёт");
        drop(first);
        assert!(try_begin_scan(&flag).is_ok());
    }

    #[test]
    fn assess_rejects_relative_path() {
        let err = assess_file_with(&WindowsFs, &NoModel, "invoice.exe").unwrap_err();
        assert_eq!(err.kind, "invalidTarget");
    }

    #[test]
    fn assess_reports_missing_file_as_probe() {
        let err = assess_file_with(&WindowsFs, &NoModel, "Z:\\missing\\x.exe").unwrap_err();
        assert_eq!(err.kind, "probe");
    }
}
