//! Заглушка для macOS: сведения о системе пока не реализованы.

use ochistka_core::system::{CoreError, SystemInfoPort, SystemOverview};

pub struct MacosSystemInfo;

impl SystemInfoPort for MacosSystemInfo {
    fn overview(&self) -> Result<SystemOverview, CoreError> {
        Err(CoreError::Unsupported {
            feature: "Сведения о системе".into(),
            os: "macOS".into(),
        })
    }
}
