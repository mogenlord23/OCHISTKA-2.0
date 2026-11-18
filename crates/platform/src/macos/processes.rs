//! Заглушка процессов для macOS: пока не реализовано, возвращает `Unsupported`.
//! Причина: «Недоступно на macOS в этой версии».

use ochistka_core::processes::{ProcessError, ProcessInfo, ProcessPort, ServiceInfo};

pub struct UnsupportedProcessPort;

fn unsupported() -> ProcessError {
    ProcessError::Unsupported {
        feature: "Процессы".into(),
        os: "macOS".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_process_stub_reports_unsupported() {
        let port = UnsupportedProcessPort;
        assert!(matches!(port.list(), Err(ProcessError::Unsupported { .. })));
        assert!(matches!(
            port.kill(1),
            Err(ProcessError::Unsupported { .. })
        ));
        assert!(matches!(
            port.services(),
            Err(ProcessError::Unsupported { .. })
        ));
    }
}
