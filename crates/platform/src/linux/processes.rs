//! Заглушка процессов для Linux: пока не реализовано, возвращает `Unsupported`.

use ochistka_core::processes::{ProcessError, ProcessInfo, ProcessPort, ServiceInfo};

pub struct UnsupportedProcessPort;

fn unsupported() -> ProcessError {
    ProcessError::Unsupported {
        feature: "Процессы".into(),
        os: "Linux".into(),
    }
}

impl ProcessPort for UnsupportedProcessPort {
    fn list(&self) -> Result<Vec<ProcessInfo>, ProcessError> {
        Err(unsupported())
    }
    fn exe_path(&self, _pid: u32) -> Result<Option<String>, ProcessError> {
        Err(unsupported())
    }
    fn start_time(&self, _pid: u32) -> Result<Option<u64>, ProcessError> {
        Err(unsupported())
    }
    fn kill(&self, _pid: u32) -> Result<bool, ProcessError> {
        Err(unsupported())
    }
    fn services(&self) -> Result<Vec<ServiceInfo>, ProcessError> {
        Err(unsupported())
    }
}
