//! Адаптер процессов на sysinfo (чтение и завершение по PID). Службы читаются из реестра
//! (см. `services.rs`).

use std::sync::Mutex;

use ochistka_core::processes::{ProcessError, ProcessInfo, ProcessPort, ServiceInfo};
use ochistka_core::system::clamp_percent;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub struct WindowsProcessPort {
    system: Mutex<Option<System>>,
}

impl WindowsProcessPort {
    /// Ничего не читает: снимок `System` создаётся при первом вызове.
    pub fn new() -> Self {
        Self {
            system: Mutex::new(None),
        }
    }

    /// Выполняет действие над общим снимком. Отравление мьютекса не фатально:
    /// каждый вызов обновляет нужные данные сам.
    fn with_system<T>(&self, action: impl FnOnce(&mut System) -> T) -> Result<T, ProcessError> {
        let mut guard = self
            .system
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_none() {
            let system = System::new().map_err(|err| ProcessError::Probe(err.to_string()))?;
            *guard = Some(system);
        }
        match guard.as_mut() {
            Some(system) => Ok(action(system)),
            None => Err(ProcessError::Probe("снимок процессов не создан".into())),
        }
    }
}

impl Default for WindowsProcessPort {
    fn default() -> Self {
        Self::new()
    }
}

/// Обновляет один процесс по PID, не трогая остальные.
fn refresh_one(system: &mut System, pid: u32, kind: ProcessRefreshKind) {
    let pids = [Pid::from_u32(pid)];
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, kind);
}

impl ProcessPort for WindowsProcessPort {
    fn list(&self) -> Result<Vec<ProcessInfo>, ProcessError> {
        self.with_system(|system| {
            system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_cpu().with_memory(),
            );
            // sysinfo отдаёт загрузку одного ядра (может быть больше 100%). Делим на число ядер,
            // чтобы проценты совпадали с общей загрузкой машины.
            let cores = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1) as f32;
            let mut list: Vec<ProcessInfo> = system
                .processes()
                .iter()
                .map(|(pid, process)| ProcessInfo {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    cpu_percent: clamp_percent(process.cpu_usage() / cores),
                    memory_bytes: process.memory(),
                    start_time_unix: process.start_time(),
                })
                .collect();
            list.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes).then(a.pid.cmp(&b.pid)));
            list
        })
    }

    fn exe_path(&self, pid: u32) -> Result<Option<String>, ProcessError> {
        self.with_system(|system| {
            refresh_one(
                system,
                pid,
                ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
            );
            system
                .process(Pid::from_u32(pid))
                .and_then(|process| process.exe())
                .map(|path| path.display().to_string())
        })
    }

    fn start_time(&self, pid: u32) -> Result<Option<u64>, ProcessError> {
        self.with_system(|system| {
            refresh_one(system, pid, ProcessRefreshKind::nothing());
            system
                .process(Pid::from_u32(pid))
                .map(|process| process.start_time())
        })
    }

    fn kill(&self, pid: u32) -> Result<bool, ProcessError> {
        self.with_system(|system| {
            refresh_one(system, pid, ProcessRefreshKind::nothing());
            system
                .process(Pid::from_u32(pid))
                .is_some_and(|process| process.kill())
        })
    }

    fn services(&self) -> Result<Vec<ServiceInfo>, ProcessError> {
        super::services::list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_contains_this_process_with_memory() {
        let port = WindowsProcessPort::new();
        let list = port.list().expect("sysinfo should list processes");
        let own = std::process::id();
        let me = list.iter().find(|p| p.pid == own).expect("own pid listed");
        assert!(me.memory_bytes > 0);
        assert!((0.0..=100.0).contains(&me.cpu_percent));
    }

    #[test]
    fn start_time_known_for_own_and_missing_for_absurd_pid() {
        let port = WindowsProcessPort::new();
        assert!(port.start_time(std::process::id()).unwrap().is_some());
        assert_eq!(port.start_time(u32::MAX).unwrap(), None);
    }

    #[test]
    fn exe_path_is_reported_for_own_process() {
        let port = WindowsProcessPort::new();
        let path = port.exe_path(std::process::id()).unwrap();
        assert!(path.is_some_and(|p| !p.is_empty()));
    }

    #[test]
    fn kill_of_missing_pid_reports_false() {
        let port = WindowsProcessPort::new();
        assert_eq!(port.kill(u32::MAX), Ok(false));
    }

    #[cfg(not(windows))]
    #[test]
    fn services_report_unsupported_outside_windows() {
        let port = WindowsProcessPort::new();
        assert!(matches!(
            port.services(),
            Err(ProcessError::Unsupported { .. })
        ));
    }
}
