//! Заглушка установки драйверов для macOS: все действия возвращают `Unsupported`.

use std::path::{Path, PathBuf};

use ochistka_core::drivers::{DriverError, DriverHost};
use url::Url;

pub struct UnsupportedDriverHost;

fn unsupported() -> DriverError {
    DriverError::Unsupported("macOS".into())
}

impl DriverHost for UnsupportedDriverHost {
    fn downloads_dir(&self) -> Result<PathBuf, DriverError> {
        Err(unsupported())
    }
    fn download(
        &self,
        _url: &Url,
        _dest: &Path,
        _max_bytes: u64,
        _on_progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), DriverError> {
        Err(unsupported())
    }
    fn signature_output(&self, _file: &Path) -> Result<String, DriverError> {
        Err(unsupported())
    }
    fn rename(&self, _from: &Path, _to: &Path) -> Result<(), DriverError> {
        Err(unsupported())
    }
    fn remove(&self, _file: &Path) {}
    fn launch(&self, _file: &Path) -> Result<(), DriverError> {
        Err(unsupported())
    }
    fn open_url(&self, _url: &str) -> Result<(), DriverError> {
        Err(unsupported())
    }
}
