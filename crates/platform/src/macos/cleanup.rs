//! Заглушка для macOS: очистка дисков пока не реализована. Причина: «Недоступно на macOS в этой версии».

use ochistka_core::fs::{CleanupError, FsPort};
use ochistka_core::layout::{CleanupEnvPort, Layout};

pub struct UnsupportedCleanupEnv;

impl CleanupEnvPort for UnsupportedCleanupEnv {
    fn layout(&self) -> Result<Layout, CleanupError> {
        Err(CleanupError::Unsupported(
            "Недоступно на macOS в этой версии: очистка дисков".into(),
        ))
    }

    fn fs(&self) -> Result<&dyn FsPort, CleanupError> {
        Err(CleanupError::Unsupported(
            "Недоступно на macOS в этой версии: файловые операции очистки".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_cleanup_stub_reports_unsupported() {
        let env = UnsupportedCleanupEnv;
        assert!(matches!(env.layout(), Err(CleanupError::Unsupported(_))));
        assert!(matches!(env.fs(), Err(CleanupError::Unsupported(_))));
    }
}
