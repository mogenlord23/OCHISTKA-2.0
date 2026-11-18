//! Адаптер ClamAV: вызывает clamscan.exe из папки установки по абсолютному пути.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use ochistka_core::scan::{parse_clamscan_output, EngineStatus, ScanReport, VirusScanPort};
use ochistka_core::system::CoreError;

/// Путь к clamscan внутри папки установки. Поиска по PATH нет.
pub fn clamscan_path(install_dir: &Path) -> PathBuf {
    install_dir.join("clamav").join("clamscan.exe")
}

/// Предел времени одного запуска clamscan: 10 минут (в миллисекундах).
pub const MAX_SCAN_TIME_MS: u64 = 10 * 60 * 1000;

/// Предел размера одного файла: 200 МБ (в байтах). Более крупные файлы clamscan пропускает.
pub const MAX_FILE_SIZE_BYTES: u64 = 200 * 1024 * 1024;

/// Как часто проверяется состояние дочернего процесса.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Аргументы: только заражённые файлы, рекурсивно, с пределами времени и размера.
/// Путь — последним отдельным аргументом.
pub fn scan_args(target: &str) -> Vec<String> {
    vec![
        "--stdout".to_string(),
        "--infected".to_string(),
        "--recursive".to_string(),
        format!("--max-scantime={MAX_SCAN_TIME_MS}"),
        format!("--max-filesize={MAX_FILE_SIZE_BYTES}"),
        target.to_string(),
    ]
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Запущенный clamscan и признак отмены. Одновременно идёт не больше одной проверки
/// (это гарантирует защита в `scan_start`), поэтому состояние общее на процесс.
struct RunState {
    child: Option<Child>,
    cancelled: bool,
}

static RUNNING: Mutex<RunState> = Mutex::new(RunState {
    child: None,
    cancelled: false,
});

fn lock_running() -> std::sync::MutexGuard<'static, RunState> {
    RUNNING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Останавливает идущий clamscan. Возвращает `true`, если было что останавливать.
pub fn cancel_running_scan() -> bool {
    let mut state = lock_running();
    let Some(child) = state.child.as_mut() else {
        return false;
    };
    let _ = child.kill();
    state.cancelled = true;
    true
}

fn drain<R: Read + Send + 'static>(stream: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stream) = stream {
            let _ = stream.read_to_end(&mut buf);
        }
        buf
    })
}

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
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|err| CoreError::Probe(format!("не удалось запустить clamscan: {err}")))?;
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());
        {
            let mut state = lock_running();
            state.cancelled = false;
            state.child = Some(child);
        }
        let deadline = Instant::now() + Duration::from_millis(MAX_SCAN_TIME_MS);
        let mut timed_out = false;
        let (status, cancelled) = loop {
            {
                let mut state = lock_running();
                let waited = match state.child.as_mut() {
                    Some(child) => child.try_wait(),
                    None => break (None, state.cancelled),
                };
                match waited {
                    Ok(Some(status)) => {
                        state.child = None;
                        break (Some(status), state.cancelled);
                    }
                    Ok(None) => {
                        if !timed_out && Instant::now() >= deadline {
                            timed_out = true;
                            if let Some(child) = state.child.as_mut() {
                                let _ = child.kill();
                            }
                        }
                    }
                    Err(err) => {
                        state.child = None;
                        return Err(CoreError::Probe(format!(
                            "не удалось дождаться clamscan: {err}"
                        )));
                    }
                }
            }
            std::thread::sleep(POLL_INTERVAL);
        };
        let stdout = stdout.join().unwrap_or_default();
        let stderr = stderr.join().unwrap_or_default();
        if cancelled {
            return Err(CoreError::Probe("проверка отменена".to_string()));
        }
        if timed_out {
            return Err(CoreError::Probe("превышено время проверки".to_string()));
        }
        // Коды clamscan: 0 — заражений нет, 1 — найдены, 2 — ошибка.
        match status.and_then(|s| s.code()) {
            Some(0) | Some(1) => Ok(parse_clamscan_output(&String::from_utf8_lossy(&stdout))),
            _ => Err(CoreError::Probe(format!(
                "clamscan завершился с ошибкой: {}",
                String::from_utf8_lossy(&stderr).trim()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Состояние запущенного процесса общее, поэтому тесты с запуском идут по очереди.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

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
    fn scan_args_set_time_and_file_size_limits_before_target() {
        let args = scan_args("C:\\Users");
        assert!(args.contains(&"--max-scantime=600000".to_string()));
        assert!(args.contains(&"--max-filesize=209715200".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("C:\\Users"));
    }

    #[test]
    fn cancel_without_running_scan_returns_false() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        assert!(!cancel_running_scan());
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
        let _serial = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());

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

    #[cfg(unix)]
    #[test]
    fn cancel_kills_running_engine_and_scan_reports_cancelled() {
        use std::os::unix::fs::PermissionsExt;
        let _serial = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());

        let root =
            std::env::temp_dir().join(format!("ochistka-clamav-cancel-{}", std::process::id()));
        let engine_dir = root.join("clamav");
        std::fs::create_dir_all(&engine_dir).unwrap();
        let engine = engine_dir.join("clamscan.exe");
        std::fs::write(&engine, "#!/bin/sh\nexec sleep 30\n").unwrap();
        std::fs::set_permissions(&engine, std::fs::Permissions::from_mode(0o755)).unwrap();

        let scanner = ClamAvScanner::from_install_dir(&root);
        let worker = std::thread::spawn(move || scanner.scan("/data"));
        let started = Instant::now();
        while !cancel_running_scan() {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "scan never started"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let result = worker.join().unwrap();
        std::fs::remove_dir_all(&root).ok();

        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(matches!(result, Err(CoreError::Probe(ref m)) if m == "проверка отменена"));
        assert!(!cancel_running_scan());
    }
}
