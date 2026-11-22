//! Windows-адаптер «Обхода блокировок»: SonicDPI (`ochistkadpi.exe`) + WinDivert за общим
//! `DesyncAdapter` ядра.
//!
//! Что реализовано: поиск `ochistkadpi.exe` в папке компонентов, проверка прав администратора,
//! запуск без окна по абсолютному пути (через `SonicDpiEngine`), остановка только СВОЕГО
//! процесса (по номеру из файла, с проверкой имени и пути), состояние и остановка службы WinDivert.
//!
//! Чего нет: `ochistkadpi.exe` пока не входит в установщик и не скачивается, поэтому без файла в папке
//! компонентов кнопка сообщает, что не хватает файлов. Работа движка на Windows и у провайдера не
//! проверена.

use std::path::{Path, PathBuf};

use ochistka_core::desync::{self, DesyncAdapter, Platform, Readiness};
use ochistka_core::vpn::service::CoreProcess;

use crate::sonicdpi::{ElevationHost, SonicDpiEngine, SystemElevation, SONICDPI_EXE};

pub mod probe;

pub const ENGINE_EXE: &str = SONICDPI_EXE;

pub struct WindowsDesyncAdapter {
    host: std::sync::Arc<dyn ElevationHost>,
    /// Папки, где может лежать `ochistkadpi.exe` (ресурсы приложения; порядок = приоритет). Рабочая
    /// (текущая) папка не используется.
    dirs: Vec<PathBuf>,
}

impl WindowsDesyncAdapter {
    /// `dirs` — папки ресурсов приложения с компонентами SonicDPI.
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self::with_host(dirs, std::sync::Arc::new(SystemElevation))
    }

    /// Для тестов: подменяет запрос прав и журнал движка.
    pub fn with_host(dirs: Vec<PathBuf>, host: std::sync::Arc<dyn ElevationHost>) -> Self {
        Self { host, dirs }
    }

    /// Первая папка, где `ochistkadpi.exe` есть и это обычный файл (не ссылка и не папка).
    fn engine(&self) -> Option<SonicDpiEngine> {
        self.dirs.iter().find_map(|dir| {
            let engine = SonicDpiEngine::new(dir.clone());
            std::fs::symlink_metadata(engine.exe_path())
                .is_ok_and(|meta| meta.is_file())
                .then_some(engine)
        })
    }

    fn exe_path(&self) -> Option<PathBuf> {
        self.engine().map(|engine| engine.exe_path())
    }
}

#[cfg(windows)]
pub(crate) fn elevated() -> bool {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_WRITE};
    use winreg::RegKey;
    // Как у служб: открытие системного ключа на запись ничего не меняет, но без прав отклоняется.
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Services", KEY_WRITE)
        .is_ok()
}

#[cfg(not(windows))]
pub(crate) fn elevated() -> bool {
    false
}

impl DesyncAdapter for WindowsDesyncAdapter {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn readiness(&self) -> Readiness {
        self.trial_readiness()
    }

    fn trial_readiness(&self) -> Readiness {
        if cfg!(windows) {
            Readiness::Ready
        } else {
            Readiness::NotImplemented("Обход блокировок работает только в Windows".into())
        }
    }

    fn locate(&self) -> Result<PathBuf, String> {
        self.exe_path()
            .ok_or_else(|| "файл движка обхода отсутствует".into())
    }

    fn has_privileges(&self) -> bool {
        self.host.is_elevated()
    }

    /// Признак работы движка: либо его журнал появился или обновился после запуска, либо драйвер
    /// перехвата (WinDivert) загружен. Журнала движок может не писать (проверено на Windows:
    /// движок работает, журнала нет), поэтому одного журнала недостаточно как условие «ошибки».
    fn start_confirmed(&self, since: std::time::SystemTime) -> bool {
        let log_fresh = self
            .host
            .engine_log()
            .and_then(|log| std::fs::metadata(log).ok())
            .and_then(|meta| meta.modified().ok())
            .is_some_and(|modified| modified >= since - std::time::Duration::from_millis(200));
        log_fresh || matches!(self.driver_state(), desync::DriverState::Running)
    }

    fn spawn(
        &self,
        exe: &Path,
        args: &[String],
        log: &Path,
    ) -> Result<Box<dyn CoreProcess>, String> {
        // Только наш файл из папки компонентов и только известный набор аргументов.
        let Some(engine) = self.engine().filter(|e| e.exe_path() == exe) else {
            return Err("запуск чужого файла отклонён".into());
        };
        let profile = match args {
            [run, flag, profile] if run == "run" && flag == "--profile" => profile,
            _ => return Err("запуск с чужими аргументами отклонён".into()),
        };
        engine.spawn_auto(profile, log, &self.host)
    }

    fn driver_state(&self) -> desync::DriverState {
        crate::windows::driver_service::state()
    }

    fn stop_driver(&self) -> Result<(), String> {
        crate::windows::driver_service::stop()
    }

    /// Завершает движок от прошлого запуска по сохранённому PID: только процесс с именем
    /// `ochistkadpi.exe` из нашей папки; чужой процесс с тем же PID не трогается.
    fn kill_stale(&self, pid: u32) {
        for dir in &self.dirs {
            let _ = SonicDpiEngine::new(dir.clone()).stop_by_pid(pid);
        }
        // Процесс с повышенными правами обычным путём не завершить: отдельный запрос прав, но
        // только для процесса с нашим именем.
        if self.dirs.iter().any(|dir| {
            crate::windows::process_guard::is_alive_ours(pid, ENGINE_EXE, &dir.join(ENGINE_EXE))
        }) {
            let _ = self.host.kill_elevated(pid);
        }
    }

    /// Жив ли ещё наш движок с этим PID (после `kill_stale`).
    fn stale_alive(&self, pid: u32) -> bool {
        crate::windows::process_guard::is_alive_named(pid, ENGINE_EXE)
    }
}

/// Папка данных обхода: `%LOCALAPPDATA%\\OCHISTKA\\desync`, иначе временная.
pub fn desync_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OCHISTKA")
        .join("desync")
}

/// Адаптер для текущей ОС. Ядро (`ochistka-core`) об ОС ничего не знает.
pub fn adapter_for_current_os(engine_dirs: Vec<PathBuf>) -> Box<dyn DesyncAdapter> {
    if cfg!(windows) {
        Box::new(WindowsDesyncAdapter::new(engine_dirs))
    } else {
        Box::new(desync::UnavailableAdapter::new(Platform::current()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-wdesync-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn locate_needs_the_engine_file_and_refuses_a_folder_or_missing_file() {
        let dir = temp("locate");
        let adapter = WindowsDesyncAdapter::new(vec![dir.clone()]);
        assert!(adapter.locate().is_err(), "файла нет");
        std::fs::create_dir(dir.join(ENGINE_EXE)).unwrap();
        assert!(adapter.locate().is_err(), "папка вместо файла");
        std::fs::remove_dir(dir.join(ENGINE_EXE)).unwrap();
        std::fs::write(dir.join(ENGINE_EXE), b"x").unwrap();
        assert_eq!(adapter.locate().unwrap(), dir.join(ENGINE_EXE));
    }

    #[test]
    fn the_engine_is_taken_from_the_resource_folders_in_order_never_from_the_current_folder() {
        let missing = temp("res-missing");
        let second = temp("res-second");
        std::fs::write(second.join(ENGINE_EXE), b"x").unwrap();
        let adapter = WindowsDesyncAdapter::new(vec![missing.clone(), second.clone()]);
        assert_eq!(adapter.locate().unwrap(), second.join(ENGINE_EXE));
        // Первая папка получила файл: она важнее.
        std::fs::write(missing.join(ENGINE_EXE), b"y").unwrap();
        assert_eq!(adapter.locate().unwrap(), missing.join(ENGINE_EXE));
        // Без папок ресурсов (или с пустыми) файл не ищется ни в текущей папке, ни в PATH.
        let current = std::env::current_dir().unwrap();
        std::fs::write(current.join(ENGINE_EXE), b"z").unwrap();
        let none = WindowsDesyncAdapter::new(vec![]);
        assert!(none.locate().is_err());
        let _ = std::fs::remove_file(current.join(ENGINE_EXE));
    }

    #[test]
    fn spawn_refuses_a_foreign_file_and_foreign_arguments() {
        let dir = temp("spawn");
        let adapter = WindowsDesyncAdapter::new(vec![dir.clone()]);
        let log = dir.join("a.log");
        let run: Vec<String> = ["run", "--profile", "youtube-discord"]
            .iter()
            .map(|a| a.to_string())
            .collect();
        assert!(adapter
            .spawn(&dir.join("other.exe"), &run, &log)
            .err()
            .unwrap()
            .contains("чужого файла"));
        let evil: Vec<String> = ["run", "--profile", "x; calc"]
            .iter()
            .map(|a| a.to_string())
            .collect();
        // Файла движка в папке ресурсов ещё нет: тоже «чужой».
        assert!(adapter.spawn(&dir.join(ENGINE_EXE), &evil, &log).is_err());
        std::fs::write(dir.join(ENGINE_EXE), b"x").unwrap();
        assert!(adapter.spawn(&dir.join(ENGINE_EXE), &evil, &log).is_err());
        assert!(adapter
            .spawn(&dir.join(ENGINE_EXE), &["-vv".to_string()], &log)
            .err()
            .unwrap()
            .contains("чужими аргументами"));
    }

    #[test]
    fn stale_cleanup_stops_only_a_process_that_is_our_engine_file() {
        let sleeper = PathBuf::from("/bin/sleep");
        if !sleeper.exists() {
            return;
        }
        let dir = temp("stale");
        // Чужой процесс: имя не sonicdpi.exe.
        let mut foreign =
            crate::vpn::spawn_logged(&sleeper, &["30"], &dir, &dir.join("a.log")).unwrap();
        WindowsDesyncAdapter::new(vec![dir.clone()]).kill_stale(foreign.pid());
        assert!(!foreign.has_exited(), "чужой процесс остался");
        // «Наш» процесс: имя и папка совпадают.
        let ours_dir = temp("ours");
        std::fs::copy(&sleeper, ours_dir.join(ENGINE_EXE)).unwrap();
        let mut ours = crate::vpn::spawn_logged(
            &ours_dir.join(ENGINE_EXE),
            &["30"],
            &ours_dir,
            &ours_dir.join("b.log"),
        )
        .unwrap();
        WindowsDesyncAdapter::new(vec![ours_dir]).kill_stale(ours.pid());
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(ours.has_exited(), "свой процесс остановлен");
        foreign.kill();
    }

    #[test]
    fn readiness_is_honest_off_windows() {
        let adapter = WindowsDesyncAdapter::new(vec![temp("ready")]);
        if cfg!(windows) {
            assert_eq!(adapter.readiness(), Readiness::Ready);
        } else {
            assert!(matches!(adapter.readiness(), Readiness::NotImplemented(_)));
        }
    }

    #[test]
    fn the_start_is_confirmed_only_by_a_log_newer_than_the_launch() {
        use crate::sonicdpi::{ElevationError, ElevationHost};
        struct LogHost(Option<PathBuf>);
        impl ElevationHost for LogHost {
            fn is_elevated(&self) -> bool {
                true
            }
            fn run_elevated(
                &self,
                _: &Path,
                _: &[String],
                _: &Path,
            ) -> Result<u32, ElevationError> {
                Err(ElevationError::Failed("не нужен".into()))
            }
            fn kill_elevated(&self, _: u32) -> bool {
                false
            }
            fn engine_log(&self) -> Option<PathBuf> {
                self.0.clone()
            }
        }
        let dir = temp("log");
        let log = dir.join("sonicdpi.log");
        let make = |p: Option<PathBuf>| {
            WindowsDesyncAdapter::with_host(vec![dir.clone()], std::sync::Arc::new(LogHost(p)))
        };
        let before = std::time::SystemTime::now();
        // Журнала нет.
        assert!(!make(Some(log.clone())).start_confirmed(before));
        // Путь к журналу неизвестен.
        assert!(!make(None).start_confirmed(before));
        // Журнал появился после запуска.
        std::fs::write(&log, b"started").unwrap();
        assert!(make(Some(log.clone())).start_confirmed(before));
        // Старый журнал (от прошлого запуска) не считается.
        let after = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        assert!(!make(Some(log)).start_confirmed(after));
    }
}
