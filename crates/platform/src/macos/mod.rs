//! Заглушка для macOS: адаптеры возвращают `Unsupported`, пока функции не реализованы.
//! Причина для интерфейса: «Недоступно на macOS в этой версии».

pub mod hardware;
pub mod network;

use ochistka_core::system::{CoreError, SystemInfoPort, SystemOverview};

pub mod cleanup;
pub mod drivers;
pub mod processes;
pub mod programs;

pub struct MacosSystemInfo;

impl SystemInfoPort for MacosSystemInfo {
    fn overview(&self) -> Result<SystemOverview, CoreError> {
        Err(CoreError::Unsupported {
            feature: "Сведения о системе".into(),
            os: "macOS".into(),
        })
    }
}
