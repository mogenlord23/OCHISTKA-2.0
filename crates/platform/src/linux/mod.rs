//! Заглушка для Linux: сведения о системе пока не реализованы.

use ochistka_core::system::{CoreError, SystemInfoPort, SystemOverview};

pub struct LinuxSystemInfo;

impl SystemInfoPort for LinuxSystemInfo {
    fn overview(&self) -> Result<SystemOverview, CoreError> {
        Err(CoreError::Unsupported {
            feature: "Сведения о системе".into(),
            os: "Linux".into(),
        })
    }
}

pub mod cleanup;
pub mod hardware;
pub mod processes;
pub mod programs;
