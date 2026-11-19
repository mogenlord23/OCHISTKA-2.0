//! Заглушка служб для macOS: пока не реализовано, возвращает `Unsupported`.

use ochistka_core::services::{RawService, ScOutput, ServiceBackend, ServiceError};

pub struct UnsupportedServiceBackend;

fn unsupported() -> ServiceError {
    ServiceError::Unsupported {
        feature: "Службы".into(),
        os: "macOS".into(),
    }
}

impl ServiceBackend for UnsupportedServiceBackend {
    fn registry_entries(&self) -> Result<Vec<RawService>, ServiceError> {
        Err(unsupported())
    }
    fn sc(&self, _args: &[&str]) -> Result<ScOutput, ServiceError> {
        Err(unsupported())
    }
    fn is_elevated(&self) -> bool {
        false
    }
    fn saved_start(&self, _name: &str) -> Option<u32> {
        None
    }
    fn save_start(&self, _name: &str, _code: u32) -> Result<(), ServiceError> {
        Err(unsupported())
    }
    fn clear_saved_start(&self, _name: &str) {}
}
