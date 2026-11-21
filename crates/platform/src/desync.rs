//! Windows-адаптер «Обхода блокировок»: `winws2` + WinDivert за общим `DesyncAdapter` ядра.
//!
//! Что реализовано: проверка состава и SHA-256 файлов закреплённого выпуска, загрузка с проверкой
//! (общий механизм sing-box), запуск без окна через общий `spawn_logged`, проверка прав
//! администратора, остановка только СВОЕГО процесса (по номеру из файла и по пути к exe, а не по
//! имени), понятная реакция на блокировку файла защитой Windows (защиту не отключаем и
//! исключений не добавляем).
//!
//! Чего нет и почему: подтверждённых файлов и профиля. `winws2.exe` и подписанный драйвер WinDivert
//! не лежат ни в одном проверяемом исходном репозитории (только в выпусках, откуда их не получить
//! и не проверить), а переименованный драйвер из NetZapret («Monkey64.sys») не берём: это обход
//! распознавания защитой. Поэтому `pinned()` пока `None`, адаптер не `Ready`, ничего не запускается.

use std::path::{Path, PathBuf};

use ochistka_core::desync::{self, DesyncAdapter, Platform, Readiness};
use ochistka_core::vpn::engine::{self, PinnedEngine, PrepareError, PrepareProgress};
use ochistka_core::vpn::service::CoreProcess;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::vpn::spawn_logged;

pub const ENGINE_EXE: &str = "winws2.exe";

pub struct WindowsDesyncAdapter {
    dir: PathBuf,
    spec: Option<&'static PinnedEngine>,
}

impl WindowsDesyncAdapter {
    /// `dir` — папка компонентов в данных приложения.
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            spec: desync::pinned(),
        }
    }

    #[cfg(test)]
    fn with_spec(dir: PathBuf, spec: Option<&'static PinnedEngine>) -> Self {
        Self { dir, spec }
    }

    fn exe_path(&self) -> PathBuf {
        self.dir.join(ENGINE_EXE)
    }
}

/// Какие сообщения системы значат «файл заблокирован защитой Windows».
fn blocked_by_protection(detail: &str) -> bool {
    let d = detail.to_lowercase();
    d.contains("virus")
        || d.contains("вирус")
        || d.contains("potentially unwanted")
        || d.contains("нежелательн")
        || d.contains("os error 225")
}

fn protection_error(detail: String) -> PrepareError {
    PrepareError {
        reason: "защита Windows заблокировала файл обхода. Защиту OCHISTKA не отключает и исключения не добавляет".into(),
        detail,
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
        if !cfg!(windows) {
            return Readiness::NotImplemented("Эта сборка не для Windows".into());
        }
        match self.spec {
            None => Readiness::NotImplemented(
                "Обход блокировок пока недоступен: подтверждённые файлы движка отсутствуют".into(),
            ),
            Some(_) => Readiness::Ready,
        }
    }

    fn locate(&self) -> Result<PathBuf, String> {
        let spec = self
            .spec
            .ok_or("файлы движка не закреплены: нет подтверждённых версий и хэшей")?;
        if engine::is_installed(spec, &self.dir) {
            Ok(self.exe_path())
        } else {
            Err("компоненты обхода не найдены или изменены (хэши не совпали)".into())
        }
    }

    fn install(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<(), PrepareError> {
        let spec = self.spec.ok_or_else(|| {
            PrepareError::unsupported("загрузка запрещена: файлы движка не подтверждены")
        })?;
        #[cfg(windows)]
        {
            engine::install(
                spec,
                &self.dir,
                &crate::vpn::http::ReqwestDownloader,
                progress,
            )
            .map_err(|e| {
                if blocked_by_protection(&e.detail) {
                    protection_error(e.detail)
                } else {
                    e
                }
            })?;
            // Защита могла убрать файл сразу после записи.
            if engine::is_installed(spec, &self.dir) {
                Ok(())
            } else {
                Err(protection_error(
                    "после загрузки файлы отсутствуют или изменены".into(),
                ))
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (spec, progress);
            Err(PrepareError::unsupported("эта сборка не для Windows"))
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
        // Только наш файл из папки компонентов: чужой путь не запускаем.
        if exe != self.exe_path() {
            return Err("запуск чужого файла отклонён".into());
        }
        let workdir = exe.parent().unwrap_or(Path::new("."));
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let process = spawn_logged(exe, &arg_refs, workdir, log)?;
        Ok(Box::new(process))
    }

    /// Останавливает процесс из прошлого запуска, только если номер принадлежит именно нашему
    /// `winws2.exe` (путь к файлу совпадает). По имени чужие процессы не трогаем.
    fn kill_stale(&self, pid: u32) {
        let target = [Pid::from_u32(pid)];
        let Ok(mut system) = System::new() else {
            return;
        };
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&target),
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
        );
        let ours = self.exe_path();
        if let Some(process) = system.process(target[0]) {
            if process.exe().is_some_and(|path| same_file(path, &ours)) {
                process.kill();
            }
        }
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
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
    use ochistka_core::vpn::engine::sha256_hex;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-wdesync-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn spec_for(files: &[(&str, &[u8])]) -> &'static PinnedEngine {
        let list: Vec<(&'static str, &'static str)> = files
            .iter()
            .map(|(n, d)| {
                (
                    &*Box::leak(n.to_string().into_boxed_str()),
                    &*Box::leak(sha256_hex(d).into_boxed_str()),
                )
            })
            .collect();
        Box::leak(Box::new(PinnedEngine {
            version: "t",
            url: "https://github.com/x/y/releases/download/t/a.zip",
            url_prefix: "/x/y/releases/download/",
            archive_sha256: "00",
            files: Box::leak(list.into_boxed_slice()),
        }))
    }

    #[test]
    fn nothing_is_ready_while_no_verified_files_are_pinned() {
        let adapter = WindowsDesyncAdapter::new(temp("none"));
        assert!(matches!(adapter.readiness(), Readiness::NotImplemented(_)));
        assert!(adapter.locate().is_err());
        assert!(adapter.install(&mut |_| {}).is_err());
    }

    #[test]
    fn components_are_found_only_when_every_hash_matches() {
        let dir = temp("hash");
        let spec = spec_for(&[(ENGINE_EXE, b"engine"), ("WinDivert.dll", b"dll")]);
        let adapter = WindowsDesyncAdapter::with_spec(dir.clone(), Some(spec));
        assert!(adapter.locate().is_err(), "файлов нет");
        std::fs::write(dir.join(ENGINE_EXE), b"engine").unwrap();
        std::fs::write(dir.join("WinDivert.dll"), b"dll").unwrap();
        assert_eq!(adapter.locate().unwrap(), dir.join(ENGINE_EXE));
        std::fs::write(dir.join("WinDivert.dll"), b"tampered").unwrap();
        assert!(adapter.locate().unwrap_err().contains("хэши не совпали"));
    }

    #[test]
    fn only_our_own_engine_file_can_be_started() {
        let dir = temp("spawn");
        let adapter = WindowsDesyncAdapter::with_spec(dir.clone(), None);
        let err = adapter
            .spawn(Path::new("/bin/sh"), &[], &dir.join("l"))
            .map(|_| ())
            .unwrap_err();
        assert!(err.contains("чужого файла"));
    }

    #[test]
    fn stale_cleanup_stops_only_a_process_that_is_our_engine_file() {
        let dir = temp("stale");
        let sleeper = PathBuf::from("/bin/sleep");
        if !sleeper.exists() {
            return;
        }
        // Чужой процесс: путь не совпадает с нашим winws2.exe, имя ни при чём.
        let mut foreign = spawn_logged(&sleeper, &["30"], &dir, &dir.join("a.log")).unwrap();
        WindowsDesyncAdapter::with_spec(dir.clone(), None).kill_stale(foreign.pid());
        assert!(!foreign.has_exited(), "чужой процесс остался");
        // «Наш» процесс: экземпляр, чей exe равен пути адаптера.
        let ours_dir = temp("ours");
        std::fs::copy(&sleeper, ours_dir.join(ENGINE_EXE)).unwrap();
        let mut ours = spawn_logged(
            &ours_dir.join(ENGINE_EXE),
            &["30"],
            &ours_dir,
            &ours_dir.join("b.log"),
        )
        .unwrap();
        WindowsDesyncAdapter::with_spec(ours_dir.clone(), None).kill_stale(ours.pid());
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(ours.has_exited(), "свой процесс остановлен");
        foreign.kill();
    }

    #[test]
    fn protection_blocks_are_recognised_and_explained() {
        assert!(blocked_by_protection(
            "Operation did not complete successfully because the file contains a virus or potentially unwanted software."
        ));
        assert!(blocked_by_protection("os error 225"));
        assert!(!blocked_by_protection("timeout"));
        let e = protection_error("x".into());
        assert!(e.reason.contains("не отключает"));
    }

    #[test]
    fn privileges_and_os_selection_are_honest_off_windows() {
        let adapter = WindowsDesyncAdapter::new(temp("priv"));
        assert!(!adapter.has_privileges() || cfg!(windows));
        let chosen = adapter_for_current_os(temp("pick"));
        assert!(matches!(
            chosen.readiness(),
            Readiness::NotImplemented(_) | Readiness::Unsupported(_)
        ));
    }
}
