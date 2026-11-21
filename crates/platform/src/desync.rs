//! Windows-адаптер «Обхода блокировок»: SonicDPI (`sonicdpi.exe`) + WinDivert за общим
//! `DesyncAdapter` ядра.
//!
//! Что реализовано: поиск `sonicdpi.exe` в папке компонентов, проверка прав администратора,
//! запуск без окна по абсолютному пути (через `SonicDpiEngine`), остановка только СВОЕГО
//! процесса (по номеру из файла, с проверкой имени и пути), состояние и остановка службы WinDivert.
//!
//! Чего нет: `sonicdpi.exe` пока не входит в установщик и не скачивается, поэтому без файла в папке
//! компонентов кнопка сообщает, что не хватает файлов. Работа движка на Windows и у провайдера не
//! проверена.

use std::path::{Path, PathBuf};

use ochistka_core::desync::{self, DesyncAdapter, Platform, Readiness};
use ochistka_core::vpn::service::CoreProcess;

use crate::sonicdpi::{SonicDpiEngine, SONICDPI_EXE};

pub mod probe;

pub const ENGINE_EXE: &str = SONICDPI_EXE;

pub struct WindowsDesyncAdapter {
    dir: PathBuf,
}

impl WindowsDesyncAdapter {
    /// `dir` — папка компонентов в данных приложения.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn engine(&self) -> SonicDpiEngine {
        SonicDpiEngine::new(self.dir.clone())
    }

    fn exe_path(&self) -> PathBuf {
        self.engine().exe_path()
    }
}

#[cfg(windows)]
fn elevated() -> bool {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_WRITE};
    use winreg::RegKey;
    // Как у служб: открытие системного ключа на запись ничего не меняет, но без прав отклоняется.
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Services", KEY_WRITE)
        .is_ok()
}

#[cfg(not(windows))]
fn elevated() -> bool {
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
        let exe = self.exe_path();
        match std::fs::symlink_metadata(&exe) {
            Ok(meta) if meta.is_file() => Ok(exe),
            _ => Err("файл движка обхода отсутствует".into()),
        }
    }

    fn has_privileges(&self) -> bool {
        elevated()
    }

    fn spawn(
        &self,
        exe: &Path,
        args: &[String],
        log: &Path,
    ) -> Result<Box<dyn CoreProcess>, String> {
        // Только наш файл из папки компонентов и только известный набор аргументов.
        if exe != self.exe_path() {
            return Err("запуск чужого файла отклонён".into());
        }
        let profile = match args {
            [run, flag, profile] if run == "run" && flag == "--profile" => profile,
            _ => return Err("запуск с чужими аргументами отклонён".into()),
        };
        Ok(Box::new(self.engine().spawn(profile, log)?))
    }

    fn driver_state(&self) -> desync::DriverState {
        crate::windows::driver_service::state()
    }

    fn stop_driver(&self) -> Result<(), String> {
        crate::windows::driver_service::stop()
    }

    /// Завершает движок от прошлого запуска по сохранённому PID: только процесс с именем
    /// `sonicdpi.exe` из нашей папки; чужой процесс с тем же PID не трогается.
    fn kill_stale(&self, pid: u32) {
        let _ = self.engine().stop_by_pid(pid);
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
pub fn adapter_for_current_os(dir: PathBuf) -> Box<dyn DesyncAdapter> {
    if cfg!(windows) {
        Box::new(WindowsDesyncAdapter::new(dir))
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
        let adapter = WindowsDesyncAdapter::new(dir.clone());
        assert!(adapter.locate().is_err(), "файла нет");
        std::fs::create_dir(dir.join(ENGINE_EXE)).unwrap();
        assert!(adapter.locate().is_err(), "папка вместо файла");
        std::fs::remove_dir(dir.join(ENGINE_EXE)).unwrap();
        std::fs::write(dir.join(ENGINE_EXE), b"x").unwrap();
        assert_eq!(adapter.locate().unwrap(), dir.join(ENGINE_EXE));
    }

    #[test]
    fn spawn_refuses_a_foreign_file_and_foreign_arguments() {
        let dir = temp("spawn");
        let adapter = WindowsDesyncAdapter::new(dir.clone());
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
        WindowsDesyncAdapter::new(dir.clone()).kill_stale(foreign.pid());
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
        WindowsDesyncAdapter::new(ours_dir).kill_stale(ours.pid());
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(ours.has_exited(), "свой процесс остановлен");
        foreign.kill();
    }

    #[test]
    fn readiness_is_honest_off_windows() {
        let adapter = WindowsDesyncAdapter::new(temp("ready"));
        if cfg!(windows) {
            assert_eq!(adapter.readiness(), Readiness::Ready);
        } else {
            assert!(matches!(adapter.readiness(), Readiness::NotImplemented(_)));
        }
    }
}
