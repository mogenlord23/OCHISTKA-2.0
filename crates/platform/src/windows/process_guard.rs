//! Надёжное завершение движка обхода (`winws2.exe`): поиск процесса по PID с проверкой ИМЕНИ
//! (чужой процесс с тем же PID не трогается), завершение, проверка, что процесса больше нет, и
//! объект «задание» Windows: если программа закрыта принудительно или упала, система сама
//! завершает движок. Вызовы Win32 — только в `win32_ffi.rs`.

use std::path::Path;
use std::time::Duration;

use ochistka_core::vpn::service::CoreProcess;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::vpn::StdCoreProcess;

/// Имя совпадает без учёта регистра и расширения `.exe`.
pub fn name_matches(actual: &str, wanted: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().trim_end_matches(".exe").to_string();
    norm(actual) == norm(wanted)
}

fn find(system: &mut System, pid: u32) -> Option<(String, Option<std::path::PathBuf>)> {
    let target = [Pid::from_u32(pid)];
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&target),
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
    );
    system.process(target[0]).map(|p| {
        (
            p.name().to_string_lossy().into_owned(),
            p.exe().map(Path::to_path_buf),
        )
    })
}

/// Жив ли процесс с этим PID И этим именем (процесс с другим именем и тем же PID — не наш).
pub fn is_alive_named(pid: u32, name: &str) -> bool {
    let Ok(mut system) = System::new() else {
        // Не удалось спросить систему: считаем «жив» (безопасный отказ, а не ложное «выключено»).
        return true;
    };
    find(&mut system, pid).is_some_and(|(actual, _)| name_matches(&actual, name))
}

/// Завершает процесс с этим PID, только если имя совпадает (и путь, если он известен и задан).
/// Возвращает `true`, когда процесса с таким PID и именем больше нет.
pub fn terminate_named(pid: u32, name: &str, expected_exe: Option<&Path>) -> bool {
    let Ok(mut system) = System::new() else {
        return false;
    };
    let Some((actual, exe)) = find(&mut system, pid) else {
        return true; // уже нет
    };
    if !name_matches(&actual, name) {
        return true; // чужой процесс с тем же PID: не трогаем, нашего нет
    }
    if let (Some(want), Some(have)) = (expected_exe, exe.as_deref()) {
        if !same_file(have, want) {
            return true; // одноимённый, но из другой папки: не наш
        }
    }
    if let Some(process) = system.process(Pid::from_u32(pid)) {
        process.kill();
    }
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        if !is_alive_named(pid, name) {
            return true;
        }
    }
    false
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Процесс движка под защитой: «завершился» только если не завершился ни дескриптор, ни процесс с
/// этим PID и именем (ошибка опроса дескриптора больше не считается «завершился»).
pub struct GuardedProcess {
    inner: StdCoreProcess,
    name: &'static str,
}

impl GuardedProcess {
    pub fn new(inner: StdCoreProcess, name: &'static str) -> Self {
        Self { inner, name }
    }
}

impl CoreProcess for GuardedProcess {
    fn pid(&self) -> u32 {
        self.inner.pid()
    }

    fn has_exited(&mut self) -> bool {
        self.inner.has_exited() && !is_alive_named(self.inner.pid(), self.name)
    }

    /// Завершает процесс и проверяет результат, не блокируясь навсегда: сначала по дескриптору,
    /// затем по PID с проверкой имени, несколько попыток.
    fn kill(&mut self) {
        let pid = self.inner.pid();
        for _ in 0..3 {
            self.inner.terminate_nowait();
            for _ in 0..20 {
                if self.has_exited() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            if terminate_named(pid, self.name, None) && self.has_exited() {
                return;
            }
        }
    }
}

/// Привязывает процесс к заданию «завершить при закрытии»: при принудительном закрытии или падении
/// программы система завершает движок сама. Ошибка не мешает запуску (остаётся остановка по PID).
pub fn attach_kill_on_close(process: &StdCoreProcess) -> Result<(), String> {
    #[cfg(windows)]
    {
        match process.raw_handle() {
            Some(handle) => crate::windows::win32_ffi::assign_to_kill_on_close_job(handle),
            None => Err("нет дескриптора процесса".into()),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = process;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn sleeper() -> std::process::Child {
        Command::new("sleep")
            .arg("30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("sleep")
    }

    #[test]
    fn names_match_ignoring_case_and_exe() {
        assert!(name_matches("winws2.exe", "winws2.exe"));
        assert!(name_matches("WINWS2.EXE", "winws2"));
        assert!(!name_matches("winws2x.exe", "winws2.exe"));
        assert!(!name_matches("chrome.exe", "winws2.exe"));
    }

    #[test]
    fn a_foreign_process_with_another_name_is_never_touched() {
        let mut child = sleeper();
        let pid = child.id();
        assert!(is_alive_named(pid, "sleep"));
        assert!(
            !is_alive_named(pid, "winws2.exe"),
            "по имени winws2 такого процесса нет"
        );
        // Просим завершить «winws2» с PID чужого процесса: ничего не происходит.
        assert!(terminate_named(pid, "winws2.exe", None));
        assert!(child.try_wait().unwrap().is_none(), "чужой процесс жив");
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn a_process_with_the_right_name_is_terminated_and_gone_is_verified() {
        let mut child = sleeper();
        let pid = child.id();
        // На Linux завершённый, но не принятый родителем процесс виден как «зомби»: принимаем его
        // в отдельном потоке (в Windows такого состояния нет).
        let waiter = std::thread::spawn(move || {
            let _ = child.wait();
        });
        assert!(terminate_named(pid, "sleep", None));
        waiter.join().unwrap();
        assert!(!is_alive_named(pid, "sleep"));
        // Повторный вызов для уже завершённого — успех (процесса нет).
        assert!(terminate_named(pid, "sleep", None));
    }

    #[test]
    fn a_same_named_process_from_another_folder_is_not_ours() {
        let mut child = sleeper();
        let pid = child.id();
        let other = std::path::Path::new("/definitely/not/here/sleep");
        assert!(
            terminate_named(pid, "sleep", Some(other)),
            "другой путь: не наш, не трогаем"
        );
        assert!(child.try_wait().unwrap().is_none());
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn guarded_kill_ends_the_process_and_reports_exit_only_when_it_is_really_gone() {
        let child = sleeper();
        let std_proc = StdCoreProcess::from_child(child);
        let mut guarded = GuardedProcess::new(std_proc, "sleep");
        let pid = guarded.pid();
        assert!(!guarded.has_exited());
        guarded.kill();
        assert!(guarded.has_exited());
        assert!(!is_alive_named(pid, "sleep"));
    }
}
