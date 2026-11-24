//! Проверка папки: обход, вызов движка по файлу через порт и сбор находок. Без кода ОС.
//! Обход, ограничения, очередь файлов и отмена управляются ядром.

use std::sync::atomic::AtomicBool;

use serde::Serialize;
use ts_rs::TS;

use crate::fs::{walk, Budget, CleanupError, FsPort, Limits};
use crate::quarantine::FileIdentity;
use crate::system::CoreError;

/// Файлы больше этого размера движок не читает: они попадают в `skipped_files`.
pub const MAX_FILE_BYTES: u64 = 200 * 1024 * 1024;
/// Предохранитель для патологически глубоких деревьев при проверке компьютера.
const MAX_COMPUTER_DEPTH: u32 = 128;

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "Finding.ts")]
pub struct Finding {
    pub path: String,
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub identity: Option<FileIdentity>,
}

#[derive(Serialize, TS, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "ScanMode.ts")]
pub enum ScanMode {
    #[default]
    Signatures,
    Signs,
}

#[derive(Serialize, TS, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "ScanReport.ts")]
pub struct ScanReport {
    pub scanned_files: u32,
    pub infected_files: u32,
    // Файлы, которые движок не смог проверить (повреждённые, занятые и т. п.).
    pub errors: u32,
    // Файлы больше `MAX_FILE_BYTES`: не проверялись, но учтены в отчёте.
    pub skipped_files: u32,
    // Часть дерева не проверена из-за лимитов обхода или недоступных папок.
    pub incomplete: bool,
    pub findings: Vec<Finding>,
    pub mode: ScanMode,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "EngineStatus.ts")]
pub struct EngineStatus {
    pub available: bool,
    pub reason: String,
}

/// Что приложение знает о базах сигнатур: где они лежат и сколько их загружено.
/// Базы наполняет владелец сам; приложение только читает папку и загружает её в движок.
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "SignaturesInfo.ts")]
pub struct SignaturesInfo {
    /// Папка баз, которую смотрит приложение.
    pub dir: String,
    /// Сколько файлов базы лежит в папке (0 — папка пуста).
    pub files: u32,
    /// Сколько сигнатур загружено, если библиотека сообщила число.
    pub signatures: Option<u32>,
    /// Встроен ли движок в эту сборку вообще.
    pub engine_present: bool,
}

/// Результат движка по одному файлу.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileVerdict {
    Clean,
    Found { signature: String },
}

/// Порт движка. Реализации живут в `crates/platform`; без `unsafe` снаружи FFI-модуля.
pub trait VirusScanPort: Send + Sync {
    fn engine_status(&self) -> EngineStatus;
    fn mode(&self) -> ScanMode {
        ScanMode::Signatures
    }
    /// Число одновременных вызовов, которое поддерживает движок.
    fn scan_workers(&self) -> usize {
        1
    }
    /// Проверяет один файл. Вызывается только при доступном движке.
    fn scan_file(&self, path: &str) -> Result<FileVerdict, CoreError>;
    /// Если платформа не поддерживает проверку открытого объекта, идентичность отсутствует.
    fn scan_file_with_identity(
        &self,
        path: &str,
    ) -> Result<(FileVerdict, Option<FileIdentity>), CoreError> {
        self.scan_file(path).map(|verdict| (verdict, None))
    }
    /// Загружает и компилирует сигнатуры из папки. Без встроенного движка возвращает ошибку.
    fn load_signatures(&self, dir: &str) -> Result<(), CoreError>;

    /// Сколько сигнатур загружено, если движок знает число. Иначе `None`.
    fn signature_count(&self) -> Option<u32> {
        None
    }
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

/// Проверяет папку с сигнатурами: она должна существовать и содержать файлы.
/// Возвращает число файлов. Ошибка объясняет причину для интерфейса.
pub fn check_signatures_dir(fs: &dyn FsPort, dir: &str) -> Result<u32, CoreError> {
    let meta = fs
        .metadata(dir)
        .map_err(|_| CoreError::Probe(format!("папка сигнатур не найдена: {dir}")))?;
    if !meta.is_dir {
        return Err(CoreError::Probe(format!(
            "папка сигнатур не найдена: {dir}"
        )));
    }
    let entries = fs
        .read_dir(dir)
        .map_err(|err| CoreError::Probe(format!("папка сигнатур не читается: {err}")))?;
    let files = entries.iter().filter(|entry| !entry.meta.is_dir).count();
    if files == 0 {
        return Err(CoreError::Probe(format!("папка сигнатур пуста: {dir}")));
    }
    u32::try_from(files).map_err(|_| CoreError::Probe("слишком много файлов сигнатур".into()))
}

/// Реальный ход обработки найденных файлов после обхода дерева.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub total_files: u32,
    pub processed_files: u32,
    pub scanned_files: u32,
    pub infected_files: u32,
    pub errors: u32,
    pub skipped_files: u32,
}

/// Проверяет папку `target`. Движок недоступен — ошибка `EngineMissing` до обхода.
/// Ошибка одного файла считается в `errors` и не останавливает проверку.
/// `cancel` останавливает проверку ошибкой `Cancelled`.
pub fn sweep(
    fs: &dyn FsPort,
    port: &dyn VirusScanPort,
    target: &str,
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<ScanReport, CoreError> {
    sweep_with_progress(fs, port, target, cancel, limits, &mut |_| {})
}

/// Проверка с уведомлением о фактически обработанных файлах.
pub fn sweep_with_progress(
    fs: &dyn FsPort,
    port: &dyn VirusScanPort,
    target: &str,
    cancel: &AtomicBool,
    limits: Limits,
    progress: &mut dyn FnMut(ScanProgress),
) -> Result<ScanReport, CoreError> {
    validate_target(target)?;
    let status = port.engine_status();
    if !status.available {
        return Err(CoreError::EngineMissing(status.reason));
    }
    let meta = fs
        .metadata(target)
        .map_err(|_| CoreError::Probe(format!("папка недоступна: {target}")))?;
    if !meta.is_dir {
        return Err(CoreError::Probe(format!("не папка: {target}")));
    }

    let mut files: Vec<(String, u64)> = Vec::new();
    let mut budget = Budget::new(limits.max_entries);
    let complete = walk(
        fs,
        target,
        cancel,
        &mut budget,
        limits.max_depth,
        &mut |entry| {
            if !entry.meta.is_dir {
                files.push((entry.path.clone(), entry.meta.size));
            }
        },
    )
    .map_err(into_core)?;

    let mut report = ScanReport {
        incomplete: !complete,
        mode: port.mode(),
        ..ScanReport::default()
    };
    let total = u32::try_from(files.len()).unwrap_or(u32::MAX);
    progress(report_progress(&report, total));
    with_scan_queue(port, cancel, |queue| {
        for (path, size) in files {
            if is_cancelled(cancel) {
                return Err(CoreError::Cancelled);
            }
            let changed = if size > MAX_FILE_BYTES {
                report.skipped_files += 1;
                true
            } else {
                queue.submit(path, &mut report)?
            };
            if changed {
                progress(report_progress(&report, total));
            }
        }
        queue.drain(&mut report, |report| {
            progress(report_progress(report, total))
        })
    })?;
    Ok(report)
}

/// Проверяет системные корни по мере обхода. Нулевой total_files означает неизвестный объём.
pub fn sweep_computer(
    fs: &dyn FsPort,
    port: &dyn VirusScanPort,
    roots: &[String],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(ScanProgress),
) -> Result<ScanReport, CoreError> {
    sweep_computer_scoped(fs, port, roots, &|_| false, cancel, progress)
}

/// Вложенные точки монтирования исключены из обхода родительского тома.
pub fn sweep_computer_scoped(
    fs: &dyn FsPort,
    port: &dyn VirusScanPort,
    roots: &[String],
    excluded: &dyn Fn(&str) -> bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(ScanProgress),
) -> Result<ScanReport, CoreError> {
    let status = port.engine_status();
    if !status.available {
        return Err(CoreError::EngineMissing(status.reason));
    }
    if roots.is_empty() {
        return Err(CoreError::Probe("нет локальных дисков для проверки".into()));
    }

    let mut report = ScanReport {
        mode: port.mode(),
        ..ScanReport::default()
    };
    let mut stack = Vec::new();
    for root in roots.iter().rev() {
        validate_target(root)?;
        stack.push((root.clone(), 0u32));
    }
    progress(ScanProgress::default());

    with_scan_queue(port, cancel, |queue| {
        while let Some((dir, depth)) = stack.pop() {
            if is_cancelled(cancel) {
                return Err(CoreError::Cancelled);
            }
            if depth > MAX_COMPUTER_DEPTH
                || !matches!(fs.metadata(&dir), Ok(meta) if meta.is_dir && !meta.is_symlink)
            {
                report.incomplete = true;
                continue;
            }
            let Ok(entries) = fs.read_scan_dir(&dir, excluded) else {
                report.incomplete = true;
                continue;
            };

            let mut priority = Vec::new();
            for entry in entries {
                if is_cancelled(cancel) {
                    return Err(CoreError::Cancelled);
                }
                let changed = if entry.meta.is_symlink {
                    report.skipped_files = report.skipped_files.saturating_add(1);
                    true
                } else if entry.meta.is_dir {
                    let next = (entry.path, depth + 1);
                    if ["Users", "Downloads", "Desktop", "home"]
                        .iter()
                        .any(|name| entry.name.eq_ignore_ascii_case(name))
                    {
                        priority.push(next);
                    } else {
                        stack.push(next);
                    }
                    continue;
                } else if entry.meta.size > MAX_FILE_BYTES {
                    report.skipped_files = report.skipped_files.saturating_add(1);
                    true
                } else {
                    queue.submit(entry.path, &mut report)?
                };
                if changed {
                    progress(report_progress(&report, 0));
                }
            }
            stack.extend(priority.into_iter().rev());
        }
        queue.drain(&mut report, |report| progress(report_progress(report, 0)))
    })?;

    report.incomplete |= report.skipped_files > 0 || report.errors > 0;
    Ok(report)
}

fn report_progress(report: &ScanReport, total_files: u32) -> ScanProgress {
    ScanProgress {
        total_files,
        processed_files: report
            .scanned_files
            .saturating_add(report.errors)
            .saturating_add(report.skipped_files),
        scanned_files: report.scanned_files,
        infected_files: report.infected_files,
        errors: report.errors,
        skipped_files: report.skipped_files,
    }
}

type Verdict = Result<(FileVerdict, Option<FileIdentity>), CoreError>;

struct Workers {
    jobs: crossbeam_channel::Sender<String>,
    results: crossbeam_channel::Receiver<(String, Verdict)>,
    pending: usize,
    capacity: usize,
}

struct ScanQueue<'a> {
    port: &'a dyn VirusScanPort,
    cancel: &'a AtomicBool,
    workers: Option<Workers>,
}

fn with_scan_queue<T>(
    port: &dyn VirusScanPort,
    cancel: &AtomicBool,
    run: impl FnOnce(&mut ScanQueue<'_>) -> Result<T, CoreError>,
) -> Result<T, CoreError> {
    let workers = port.scan_workers().clamp(1, 8);
    if workers == 1 {
        return run(&mut ScanQueue {
            port,
            cancel,
            workers: None,
        });
    }
    std::thread::scope(|scope| {
        let capacity = workers * 2;
        let (jobs, tasks) = crossbeam_channel::bounded::<String>(capacity);
        let (finished, results) = crossbeam_channel::unbounded();
        for _ in 0..workers {
            let (tasks, finished) = (tasks.clone(), finished.clone());
            scope.spawn(move || {
                while let Ok(path) = tasks.recv() {
                    let verdict = if is_cancelled(cancel) {
                        Err(CoreError::Cancelled)
                    } else {
                        port.scan_file_with_identity(&path)
                    };
                    if finished.send((path, verdict)).is_err() {
                        break;
                    }
                }
            });
        }
        drop((tasks, finished));
        run(&mut ScanQueue {
            port,
            cancel,
            workers: Some(Workers {
                jobs,
                results,
                pending: 0,
                capacity,
            }),
        })
    })
}

impl ScanQueue<'_> {
    fn submit(&mut self, path: String, report: &mut ScanReport) -> Result<bool, CoreError> {
        let Some(workers) = &mut self.workers else {
            let verdict = self.port.scan_file_with_identity(&path);
            record_verdict(path, verdict, report)?;
            return Ok(true);
        };
        workers
            .jobs
            .send(path)
            .map_err(|_| workers_disconnected())?;
        workers.pending += 1;
        if workers.pending == workers.capacity {
            self.receive(report)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn receive(&mut self, report: &mut ScanReport) -> Result<(), CoreError> {
        if is_cancelled(self.cancel) {
            return Err(CoreError::Cancelled);
        }
        let workers = self.workers.as_mut().expect("queue has worker threads");
        let (path, verdict) = workers.results.recv().map_err(|_| workers_disconnected())?;
        workers.pending -= 1;
        record_verdict(path, verdict, report)
    }

    fn drain(
        &mut self,
        report: &mut ScanReport,
        mut progress: impl FnMut(&ScanReport),
    ) -> Result<(), CoreError> {
        while self
            .workers
            .as_ref()
            .is_some_and(|workers| workers.pending > 0)
        {
            self.receive(report)?;
            progress(report);
        }
        Ok(())
    }
}

fn workers_disconnected() -> CoreError {
    CoreError::Probe("очередь проверки остановилась".into())
}

fn record_verdict(
    path: String,
    verdict: Verdict,
    report: &mut ScanReport,
) -> Result<(), CoreError> {
    match verdict {
        Ok((verdict, identity)) => {
            report.scanned_files = report.scanned_files.saturating_add(1);
            if let FileVerdict::Found { signature } = verdict {
                report.infected_files = report.infected_files.saturating_add(1);
                report.findings.push(Finding {
                    path,
                    signature,
                    identity,
                });
            }
        }
        Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
        Err(_) => report.errors = report.errors.saturating_add(1),
    }
    Ok(())
}

fn is_cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(std::sync::atomic::Ordering::SeqCst)
}

fn into_core(err: CleanupError) -> CoreError {
    match err {
        CleanupError::Cancelled => CoreError::Cancelled,
        other => CoreError::Probe(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake_fs::FakeFs;
    use std::sync::atomic::{AtomicU32, Ordering};

    const ROOT: &str = "C:\\Users\\Slava\\Downloads";

    #[test]
    fn computer_scan_reports_unknown_total_and_scans_each_root() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\eicar.com"), b"test", 1);
        fs.file("D:\\Data\\clean.txt", b"clean", 1);
        let engine = FakeEngine::ready();
        let mut updates = Vec::new();
        let report = sweep_computer(
            &fs,
            &engine,
            &[ROOT.into(), "D:\\Data".into()],
            &AtomicBool::new(false),
            &mut |update| updates.push(update),
        )
        .unwrap();
        assert_eq!(report.scanned_files, 2);
        assert_eq!(report.infected_files, 1);
        assert!(!report.incomplete);
        assert!(updates.iter().all(|update| update.total_files == 0));
        assert_eq!(updates.last().unwrap().processed_files, 2);
    }

    #[test]
    fn computer_scan_continues_past_an_unavailable_root_and_skips_links() {
        let fs = FakeFs::new();
        fs.link(&format!("{ROOT}\\junction"));
        fs.file(&format!("{ROOT}\\clean.txt"), b"clean", 1);
        let report = sweep_computer(
            &fs,
            &FakeEngine::ready(),
            &["D:\\".into(), ROOT.into()],
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        assert!(report.incomplete);
        assert_eq!(report.skipped_files, 1);
        assert_eq!(report.scanned_files, 1);
    }

    #[test]
    fn computer_scan_only_enters_a_nested_volume_as_its_own_root() {
        let fs = FakeFs::new();
        let volume = format!("{ROOT}\\volume");
        let remote = format!("{ROOT}\\remote");
        fs.file(&format!("{ROOT}\\a.txt"), b"a", 1);
        fs.file(&format!("{volume}\\b.txt"), b"b", 1);
        fs.file(&format!("{remote}\\remote.txt"), b"c", 1);
        let engine = FakeEngine::ready();

        let report = sweep_computer_scoped(
            &fs,
            &engine,
            &[ROOT.into(), volume.clone()],
            &|path| path == volume || path == remote,
            &no_cancel(),
            &mut |_| {},
        )
        .unwrap();

        assert_eq!(report.scanned_files, 2);
        assert!(!report.incomplete);
    }

    #[test]
    fn computer_scan_uses_the_workers_offered_by_the_engine() {
        use std::sync::{Condvar, Mutex};

        struct ConcurrentEngine(Mutex<usize>, Condvar);
        impl VirusScanPort for ConcurrentEngine {
            fn engine_status(&self) -> EngineStatus {
                FakeEngine::ready().engine_status()
            }

            fn scan_workers(&self) -> usize {
                2
            }

            fn scan_file(&self, _: &str) -> Result<FileVerdict, CoreError> {
                let mut calls = self.0.lock().unwrap();
                *calls += 1;
                self.1.notify_all();
                let (_calls, deadline) = self
                    .1
                    .wait_timeout_while(calls, std::time::Duration::from_secs(2), |calls| {
                        *calls < 2
                    })
                    .unwrap();
                assert!(!deadline.timed_out(), "files were scanned serially");
                Ok(FileVerdict::Clean)
            }

            fn load_signatures(&self, _: &str) -> Result<(), CoreError> {
                Ok(())
            }
        }

        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\one.txt"), b"one", 1);
        fs.file(&format!("{ROOT}\\two.txt"), b"two", 1);
        let engine = ConcurrentEngine(Mutex::new(0), Condvar::new());
        let report =
            sweep_computer(&fs, &engine, &[ROOT.into()], &no_cancel(), &mut |_| {}).unwrap();

        assert_eq!(report.scanned_files, 2);
    }

    #[test]
    fn computer_scan_honors_cancellation_between_files() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\a.txt"), b"a", 1);
        fs.file(&format!("{ROOT}\\b.txt"), b"b", 1);
        let cancel = AtomicBool::new(false);
        let engine = FakeEngine::ready();
        let result = sweep_computer(&fs, &engine, &[ROOT.into()], &cancel, &mut |update| {
            if update.processed_files == 1 {
                cancel.store(true, Ordering::SeqCst);
            }
        });
        assert!(matches!(result, Err(CoreError::Cancelled)));
        assert_eq!(engine.calls.load(Ordering::SeqCst), 1);
    }

    /// Движок в памяти: заражён файл, если его имя в `infected`; в `broken` — ошибка чтения.
    struct FakeEngine {
        available: bool,
        infected: Vec<&'static str>,
        broken: Vec<&'static str>,
        calls: AtomicU32,
    }

    impl FakeEngine {
        fn ready() -> Self {
            Self {
                available: true,
                infected: vec!["eicar.com"],
                broken: vec!["broken.zip"],
                calls: AtomicU32::new(0),
            }
        }
    }

    impl VirusScanPort for FakeEngine {
        fn engine_status(&self) -> EngineStatus {
            EngineStatus {
                available: self.available,
                reason: if self.available {
                    String::new()
                } else {
                    "движок не собран: тест".into()
                },
            }
        }

        fn scan_file(&self, path: &str) -> Result<FileVerdict, CoreError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let name = path.rsplit('\\').next().unwrap_or("");
            if self.broken.contains(&name) {
                return Err(CoreError::Probe("файл повреждён".into()));
            }
            if self.infected.contains(&name) {
                return Ok(FileVerdict::Found {
                    signature: "Test.Signature".into(),
                });
            }
            Ok(FileVerdict::Clean)
        }

        fn load_signatures(&self, _dir: &str) -> Result<(), CoreError> {
            Ok(())
        }
    }

    fn no_cancel() -> AtomicBool {
        AtomicBool::new(false)
    }

    #[test]
    fn empty_folder_reports_nothing_scanned() {
        let fs = FakeFs::new();
        fs.dir(ROOT);
        let report = sweep(
            &fs,
            &FakeEngine::ready(),
            ROOT,
            &no_cancel(),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(report, ScanReport::default());
    }

    #[test]
    fn entry_limit_hit_marks_the_report_incomplete() {
        let fs = FakeFs::new();
        for name in ["a.txt", "b.txt", "c.txt", "d.txt", "e.txt"] {
            fs.file(&format!("{ROOT}\\{name}"), b"x", 0);
        }
        let limits = Limits {
            max_entries: 2,
            max_depth: 24,
        };
        let report = sweep(&fs, &FakeEngine::ready(), ROOT, &no_cancel(), limits).unwrap();
        assert!(report.incomplete);
        assert_eq!(report.scanned_files, 2);
    }

    #[test]
    fn depth_limit_marks_the_report_incomplete() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\nested\\inside.txt"), b"x", 0);
        let limits = Limits {
            max_entries: 100,
            max_depth: 0,
        };
        let report = sweep(&fs, &FakeEngine::ready(), ROOT, &no_cancel(), limits).unwrap();
        assert!(report.incomplete);
        assert_eq!(report.scanned_files, 0);
    }

    #[test]
    fn walk_that_fits_the_entry_limit_exactly_is_complete() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\a.txt"), b"x", 0);
        fs.file(&format!("{ROOT}\\b.txt"), b"x", 0);
        let limits = Limits {
            max_entries: 2,
            max_depth: 24,
        };
        let report = sweep(&fs, &FakeEngine::ready(), ROOT, &no_cancel(), limits).unwrap();
        assert!(!report.incomplete);
        assert_eq!(report.scanned_files, 2);
    }

    #[test]
    fn progress_counts_checked_and_failed_files() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\broken.zip"), b"x", 0);
        fs.file(&format!("{ROOT}\\eicar.com"), b"x", 0);
        let mut updates = Vec::new();
        let report = sweep_with_progress(
            &fs,
            &FakeEngine::ready(),
            ROOT,
            &no_cancel(),
            Limits::default(),
            &mut |p| updates.push(p),
        )
        .unwrap();
        assert_eq!(updates.first().unwrap().total_files, 2);
        let last = updates.last().unwrap();
        assert_eq!(last.processed_files, 2);
        assert_eq!(last.scanned_files, report.scanned_files);
        assert_eq!(last.errors, 1);
    }

    #[test]
    fn finding_is_reported_with_signature_and_counted() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\eicar.com"), b"x", 0);
        fs.file(&format!("{ROOT}\\notes.txt"), b"y", 0);
        let report = sweep(
            &fs,
            &FakeEngine::ready(),
            ROOT,
            &no_cancel(),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(report.scanned_files, 2);
        assert_eq!(report.infected_files, 1);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].signature, "Test.Signature");
    }

    #[test]
    fn damaged_file_is_counted_as_error_and_sweep_continues() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\broken.zip"), b"PK", 0);
        fs.file(&format!("{ROOT}\\eicar.com"), b"x", 0);
        let report = sweep(
            &fs,
            &FakeEngine::ready(),
            ROOT,
            &no_cancel(),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(report.errors, 1);
        assert_eq!(report.infected_files, 1);
    }

    #[test]
    fn zero_byte_file_is_scanned_as_clean() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\empty.bin"), b"", 0);
        let report = sweep(
            &fs,
            &FakeEngine::ready(),
            ROOT,
            &no_cancel(),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(report.scanned_files, 1);
        assert_eq!(report.errors, 0);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn engine_not_built_stops_before_walking() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\eicar.com"), b"x", 0);
        let engine = FakeEngine {
            available: false,
            ..FakeEngine::ready()
        };
        let err = sweep(&fs, &engine, ROOT, &no_cancel(), Limits::default()).unwrap_err();
        assert_eq!(
            err,
            CoreError::EngineMissing("движок не собран: тест".into())
        );
        assert_eq!(engine.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn invalid_target_is_refused_before_engine_is_used() {
        let fs = FakeFs::new();
        let engine = FakeEngine::ready();
        assert!(matches!(
            sweep(&fs, &engine, "Downloads", &no_cancel(), Limits::default()),
            Err(CoreError::InvalidTarget(_))
        ));
        assert_eq!(engine.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cancel_before_scan_returns_cancelled_without_engine_calls() {
        let fs = FakeFs::new();
        fs.file(&format!("{ROOT}\\eicar.com"), b"x", 0);
        let engine = FakeEngine::ready();
        let cancel = AtomicBool::new(true);
        let err = sweep(&fs, &engine, ROOT, &cancel, Limits::default()).unwrap_err();
        assert_eq!(err, CoreError::Cancelled);
        assert_eq!(engine.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn missing_folder_is_a_probe_error_with_reason() {
        let fs = FakeFs::new();
        let err = sweep(
            &fs,
            &FakeEngine::ready(),
            ROOT,
            &no_cancel(),
            Limits::default(),
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::Probe(ref m) if m.contains("недоступна")));
    }

    #[test]
    fn signatures_folder_missing_reports_reason() {
        let fs = FakeFs::new();
        let err = check_signatures_dir(&fs, "C:\\app\\signatures").unwrap_err();
        assert_eq!(
            err,
            CoreError::Probe("папка сигнатур не найдена: C:\\app\\signatures".into())
        );
    }

    #[test]
    fn signatures_folder_empty_reports_reason() {
        let fs = FakeFs::new();
        fs.dir("C:\\app\\signatures");
        let err = check_signatures_dir(&fs, "C:\\app\\signatures").unwrap_err();
        assert_eq!(
            err,
            CoreError::Probe("папка сигнатур пуста: C:\\app\\signatures".into())
        );
    }

    #[test]
    fn signatures_folder_with_files_is_accepted() {
        let fs = FakeFs::new();
        fs.file("C:\\app\\signatures\\main.cvd", b"x", 0);
        assert_eq!(check_signatures_dir(&fs, "C:\\app\\signatures"), Ok(1));
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
    fn engine_missing_message_is_russian() {
        let err = CoreError::EngineMissing("движок не собран".into());
        assert_eq!(err.to_string(), "движок недоступен: движок не собран");
    }
}
