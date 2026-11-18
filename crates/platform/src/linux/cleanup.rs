//! Заглушка для Linux и macOS: очистка дисков пока не реализована.

use ochistka_core::fs::{CleanupError, FsPort};
use ochistka_core::layout::{CleanupEnvPort, Layout};

pub struct UnsupportedCleanupEnv;

impl CleanupEnvPort for UnsupportedCleanupEnv {
    fn layout(&self) -> Result<Layout, CleanupError> {
        Err(CleanupError::Unsupported(
            "очистка дисков: раскладка папок Windows".into(),
        ))
    }

    fn fs(&self) -> Result<&dyn FsPort, CleanupError> {
        Err(CleanupError::Unsupported(
            "очистка дисков: файловые операции Windows".into(),
        ))
    }
}
