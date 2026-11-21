//! Служба драйвера перехвата WinDivert (Windows): запрос состояния и остановка через `sc.exe`
//! по абсолютному пути `C:\Windows\System32\sc.exe` (без поиска по PATH).
//! Правила (кого и когда останавливать) живут в `ochistka_core::desync`; здесь только вызовы ОС.
//! Права администратора отдельно не запрашиваются: обход и так запускается с ними.

use ochistka_core::desync::DriverState;

/// Имя службы драйвера (создаётся самим WinDivert при открытии).
pub const WINDIVERT_SERVICE: &str = "WinDivert";

/// Разбор вывода `sc query WinDivert`. `code` — код выхода sc.exe.
pub fn parse_query(code: i32, output: &str) -> DriverState {
    // 1060: служба не установлена.
    if code == 1060 || output.contains("1060") {
        return DriverState::Absent;
    }
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("STATE") || line.starts_with("СОСТОЯНИЕ") {
            let upper = line.to_uppercase();
            if upper.contains("RUNNING")
                || upper.contains("START_PENDING")
                || upper.contains("STOP_PENDING")
            {
                return DriverState::Running;
            }
            if upper.contains("STOPPED") {
                return DriverState::Stopped;
            }
            return DriverState::Unknown(line.to_string());
        }
    }
    DriverState::Unknown(format!("sc.exe вернул код {code} без состояния службы"))
}

/// Аргументы остановки: только наша служба.
pub fn stop_args() -> [&'static str; 2] {
    ["stop", WINDIVERT_SERVICE]
}

pub fn query_args() -> [&'static str; 2] {
    ["query", WINDIVERT_SERVICE]
}

#[cfg(windows)]
mod imp {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use super::super::services::sc_path;
    use super::*;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const TIMEOUT: Duration = Duration::from_secs(10);

    fn run_sc(args: &[&str]) -> Result<(i32, String), String> {
        // Фиксированный абсолютный путь: переменная окружения не может подменить sc.exe,
        // а команда выполняется с правами администратора.
        let mut child = Command::new(sc_path(None))
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("не удалось запустить sc.exe: {e}"))?;
        let mut stdout = child.stdout.take();
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(pipe) = stdout.as_mut() {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        });
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() > TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("sc.exe не ответил вовремя".into());
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                Err(e) => return Err(format!("ошибка sc.exe: {e}")),
            }
        };
        let bytes = reader.join().unwrap_or_default();
        Ok((
            status.code().unwrap_or(-1),
            String::from_utf8_lossy(&bytes).into_owned(),
        ))
    }

    pub fn state() -> DriverState {
        match run_sc(&query_args()) {
            Ok((code, out)) => parse_query(code, &out),
            Err(why) => DriverState::Unknown(why),
        }
    }

    /// `sc.exe stop WinDivert`, затем ждём (до ~5 с), пока служба остановится.
    pub fn stop() -> Result<(), String> {
        let (code, out) = run_sc(&stop_args())?;
        let after = parse_query(code, &out);
        if matches!(after, DriverState::Stopped | DriverState::Absent) {
            return Ok(());
        }
        // 1062 «служба не запущена» — тоже конечное состояние.
        if code != 0 && code != 1062 && !out.contains("STOP_PENDING") {
            return Err(format!("sc.exe stop вернул код {code}"));
        }
        for _ in 0..25 {
            std::thread::sleep(Duration::from_millis(200));
            if matches!(state(), DriverState::Stopped | DriverState::Absent) {
                return Ok(());
            }
        }
        Err("служба не остановилась за 5 секунд".into())
    }
}

/// Текущее состояние службы WinDivert. Вне Windows драйвера нет.
pub fn state() -> DriverState {
    #[cfg(windows)]
    {
        imp::state()
    }
    #[cfg(not(windows))]
    {
        DriverState::NotApplicable
    }
}

/// Остановка службы WinDivert (только Windows).
pub fn stop() -> Result<(), String> {
    #[cfg(windows)]
    {
        imp::stop()
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::services::sc_path;
    use super::*;

    const RUNNING: &str = "SERVICE_NAME: WinDivert\n        TYPE               : 1  KERNEL_DRIVER  \n        STATE              : 4  RUNNING \n                                (STOPPABLE, NOT_PAUSABLE, IGNORES_SHUTDOWN)\n";
    const STOPPED: &str = "SERVICE_NAME: WinDivert\n        STATE              : 1  STOPPED \n";

    #[test]
    fn query_output_is_parsed_into_driver_states() {
        assert_eq!(parse_query(0, RUNNING), DriverState::Running);
        assert_eq!(parse_query(0, STOPPED), DriverState::Stopped);
        assert_eq!(
            parse_query(0, "        STATE              : 3  STOP_PENDING \n"),
            DriverState::Running,
            "пока служба останавливается, она ещё не остановлена"
        );
        assert_eq!(
            parse_query(
                1060,
                "[SC] EnumQueryServicesStatus:OpenService: ошибка 1060"
            ),
            DriverState::Absent
        );
        assert!(matches!(
            parse_query(5, "Отказано в доступе"),
            DriverState::Unknown(_)
        ));
        assert!(matches!(parse_query(0, ""), DriverState::Unknown(_)));
    }

    #[test]
    fn only_our_service_is_touched_through_the_absolute_sc_path() {
        assert_eq!(stop_args(), ["stop", "WinDivert"]);
        assert_eq!(query_args(), ["query", "WinDivert"]);
        assert_eq!(sc_path(None), r"C:\Windows\System32\sc.exe");
    }

    #[test]
    fn other_os_have_no_driver() {
        #[cfg(not(windows))]
        {
            assert_eq!(state(), DriverState::NotApplicable);
            assert!(stop().is_ok());
        }
    }
}
