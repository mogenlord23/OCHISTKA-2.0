//! Процессы и службы: DTO, правила завершения задачи и порт. Без кода ОС.
//!
//! Все правила безопасности завершения живут здесь, как чистые функции: порт платформы
//! только выполняет системный вызов после того, как ядро разрешило действие.

use crate::reveal::{no_path_reason, RevealError};
use serde::Serialize;
use thiserror::Error;

/// PID простоя системы. Завершать нельзя.
pub const PID_IDLE: u32 = 0;
/// PID системного процесса Windows (System). Завершать нельзя.
pub const PID_SYSTEM: u32 = 4;

/// Процесс для списка. Проценты CPU приведены к одному ядру: 0..=100 на всю машину.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    /// Время запуска в секундах Unix. Нужно, чтобы не завершить новый процесс с тем же PID.
    pub start_time_unix: u64,
}

/// Ошибки раздела «Процессы». Текст сообщения уже на русском.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    #[error("завершение задачи не подтверждено пользователем")]
    NotConfirmed,
    #[error("PID 0 — служебный простой системы, его нельзя завершить")]
    IdleProcess,
    #[error("PID 4 — системный процесс Windows, его нельзя завершить")]
    SystemProcess,
    #[error("нельзя завершить сам OCHISTKA")]
    OwnProcess,
    #[error("процесс {0} не найден")]
    NotFound(u32),
    #[error("процесс {0} уже завершился, PID занят другим процессом")]
    Replaced(u32),
    #[error("не удалось завершить процесс {0}: нет доступа или он уже завершился")]
    KillFailed(u32),
    #[error("{0}")]
    NotConnected(String),
    #[error("{feature} недоступно на {os}")]
    Unsupported { feature: String, os: String },
    #[error("ошибка чтения процессов: {0}")]
    Probe(String),
    /// У процесса нет пути к файлу; внутри объяснение для человека.
    #[error("{0}")]
    NoPath(String),
    #[error(transparent)]
    Reveal(#[from] RevealError),
}

/// Порт процессов. Реализации живут в `crates/platform`.
pub trait ProcessPort: Send + Sync {
    /// Все процессы с нагрузкой и памятью.
    fn list(&self) -> Result<Vec<ProcessInfo>, ProcessError>;
    /// Путь к исполняемому файлу; `None`, если ОС не отдала путь (обычно нет доступа).
    fn exe_path(&self, pid: u32) -> Result<Option<String>, ProcessError>;
    /// Время запуска процесса, `None`, если процесса с таким PID нет.
    fn start_time(&self, pid: u32) -> Result<Option<u64>, ProcessError>;
    /// Завершает процесс. `Ok(false)`, если ОС отказала.
    fn kill(&self, pid: u32) -> Result<bool, ProcessError>;
    /// Показывает файл в Проводнике (путь уже проверен ядром). Без ожидания закрытия окна.
    fn reveal(&self, path: &str) -> Result<(), RevealError>;
}

/// «Расположение файла»: путь процесса из системы, проверка пути, открытие Проводника.
/// Возвращает путь, который показали. Нет пути или файла — понятная причина.
pub fn reveal_exe(port: &dyn ProcessPort, pid: u32) -> Result<String, ProcessError> {
    let raw = port
        .exe_path(pid)?
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| ProcessError::NoPath(no_path_reason(pid)))?;
    let path = crate::reveal::normalize_path(&raw)?;
    port.reveal(&path)?;
    Ok(path)
}

/// Правила завершения без обращения к ОС. Порядок важен: сначала подтверждение,
/// затем защищённые PID, затем собственный процесс.
pub fn check_end_task(pid: u32, own_pid: u32, confirmed: bool) -> Result<(), ProcessError> {
    if !confirmed {
        return Err(ProcessError::NotConfirmed);
    }
    if pid == PID_IDLE {
        return Err(ProcessError::IdleProcess);
    }
    if pid == PID_SYSTEM {
        return Err(ProcessError::SystemProcess);
    }
    if pid == own_pid {
        return Err(ProcessError::OwnProcess);
    }
    Ok(())
}

/// Сверяет время запуска процесса с тем, что показывал интерфейс.
/// Если PID уже занят другим процессом, завершать его нельзя.
pub fn verify_identity(
    pid: u32,
    expected_start_unix: u64,
    actual_start_unix: Option<u64>,
) -> Result<(), ProcessError> {
    match actual_start_unix {
        None => Err(ProcessError::NotFound(pid)),
        Some(actual) if actual != expected_start_unix => Err(ProcessError::Replaced(pid)),
        Some(_) => Ok(()),
    }
}

/// Полный путь завершения: проверки правил, сверка личности процесса, системный вызов.
pub fn end_task(
    port: &dyn ProcessPort,
    pid: u32,
    expected_start_unix: u64,
    confirmed: bool,
    own_pid: u32,
) -> Result<(), ProcessError> {
    check_end_task(pid, own_pid, confirmed)?;
    let actual = port.start_time(pid)?;
    verify_identity(pid, expected_start_unix, actual)?;
    if port.kill(pid)? {
        Ok(())
    } else {
        Err(ProcessError::KillFailed(pid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn end_task_refuses_without_confirmation() {
        assert_eq!(
            check_end_task(1234, 99, false),
            Err(ProcessError::NotConfirmed)
        );
    }

    #[test]
    fn end_task_refuses_pid_zero_even_when_confirmed() {
        assert_eq!(
            check_end_task(PID_IDLE, 99, true),
            Err(ProcessError::IdleProcess)
        );
    }

    #[test]
    fn end_task_refuses_pid_four_even_when_confirmed() {
        assert_eq!(
            check_end_task(PID_SYSTEM, 99, true),
            Err(ProcessError::SystemProcess)
        );
    }

    #[test]
    fn end_task_refuses_own_process() {
        assert_eq!(
            check_end_task(4321, 4321, true),
            Err(ProcessError::OwnProcess)
        );
    }

    #[test]
    fn end_task_allows_confirmed_ordinary_process() {
        assert_eq!(check_end_task(1234, 99, true), Ok(()));
    }

    #[test]
    fn confirmation_is_checked_before_protected_pids() {
        assert_eq!(
            check_end_task(PID_SYSTEM, 99, false),
            Err(ProcessError::NotConfirmed)
        );
    }

    #[test]
    fn identity_mismatch_means_pid_was_reused() {
        assert_eq!(
            verify_identity(7, 100, Some(200)),
            Err(ProcessError::Replaced(7))
        );
        assert_eq!(verify_identity(7, 100, Some(100)), Ok(()));
        assert_eq!(
            verify_identity(7, 100, None),
            Err(ProcessError::NotFound(7))
        );
    }

    /// Порт-двойник: считает вызовы и отвечает заданными значениями.
    struct FakePort {
        start: Option<u64>,
        kill_ok: bool,
        kills: Mutex<Vec<u32>>,
        revealed: Mutex<Vec<String>>,
        exe: Option<String>,
    }

    impl ProcessPort for FakePort {
        fn list(&self) -> Result<Vec<ProcessInfo>, ProcessError> {
            Ok(Vec::new())
        }
        fn exe_path(&self, _pid: u32) -> Result<Option<String>, ProcessError> {
            Ok(self.exe.clone())
        }
        fn start_time(&self, _pid: u32) -> Result<Option<u64>, ProcessError> {
            Ok(self.start)
        }
        fn kill(&self, pid: u32) -> Result<bool, ProcessError> {
            self.kills.lock().unwrap().push(pid);
            Ok(self.kill_ok)
        }
        fn reveal(&self, path: &str) -> Result<(), RevealError> {
            self.revealed.lock().unwrap().push(path.to_string());
            Ok(())
        }
    }

    fn port(start: Option<u64>, kill_ok: bool) -> FakePort {
        FakePort {
            start,
            kill_ok,
            kills: Mutex::new(Vec::new()),
            revealed: Mutex::new(Vec::new()),
            exe: None,
        }
    }

    #[test]
    fn reveal_shows_the_normalized_exe_path() {
        let mut p = port(None, true);
        p.exe = Some(r#""C:\Program Files\App\app.exe""#.into());
        assert_eq!(reveal_exe(&p, 10).unwrap(), r"C:\Program Files\App\app.exe");
        assert_eq!(
            *p.revealed.lock().unwrap(),
            vec![r"C:\Program Files\App\app.exe"]
        );
    }

    #[test]
    fn reveal_without_path_gives_a_human_reason_and_never_opens_explorer() {
        let p = port(None, true);
        let err = reveal_exe(&p, 4).unwrap_err();
        assert!(matches!(&err, ProcessError::NoPath(m) if m.contains("системный")));
        let err = reveal_exe(&p, 1234).unwrap_err();
        assert!(matches!(&err, ProcessError::NoPath(m) if m.contains("защищён")));
        assert!(p.revealed.lock().unwrap().is_empty());
    }

    #[test]
    fn reveal_refuses_a_relative_path_from_the_system() {
        let mut p = port(None, true);
        p.exe = Some("app.exe".into());
        assert!(matches!(
            reveal_exe(&p, 10),
            Err(ProcessError::Reveal(RevealError::NotAbsolute))
        ));
        assert!(p.revealed.lock().unwrap().is_empty());
    }

    #[test]
    fn refused_request_never_reaches_the_os() {
        let p = port(Some(5), true);
        assert_eq!(
            end_task(&p, PID_SYSTEM, 5, true, 99),
            Err(ProcessError::SystemProcess)
        );
        assert_eq!(
            end_task(&p, 1234, 5, false, 99),
            Err(ProcessError::NotConfirmed)
        );
        assert!(p.kills.lock().unwrap().is_empty());
    }

    #[test]
    fn reused_pid_is_not_killed() {
        let p = port(Some(200), true);
        assert_eq!(
            end_task(&p, 1234, 100, true, 99),
            Err(ProcessError::Replaced(1234))
        );
        assert!(p.kills.lock().unwrap().is_empty());
    }

    #[test]
    fn confirmed_matching_process_is_killed_once() {
        let p = port(Some(100), true);
        assert_eq!(end_task(&p, 1234, 100, true, 99), Ok(()));
        assert_eq!(*p.kills.lock().unwrap(), vec![1234]);
    }

    #[test]
    fn os_refusal_is_reported_as_kill_failed() {
        let p = port(Some(100), false);
        assert_eq!(
            end_task(&p, 1234, 100, true, 99),
            Err(ProcessError::KillFailed(1234))
        );
    }
}
