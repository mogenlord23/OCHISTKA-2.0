//! Порт раскладки: где на этом компьютере лежат категории очистки, карантин и места приватности.
//! Реализация для Windows читает переменные окружения в `crates/platform`.

use crate::cleanup::CleanupLocation;
use crate::fs::{CleanupError, FsPort};
use crate::readonly::PrivacyCandidate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// Категории очистки, которые на этом компьютере существуют.
    pub cleanup: Vec<CleanupLocation>,
    /// Папка «Загрузки» пользователя.
    pub downloads: String,
    pub privacy: Vec<PrivacyCandidate>,
    /// Папка карантина внутри профиля пользователя (ADR 0005).
    pub quarantine_root: String,
    /// Буквы дисков, которые отвечают, например `C:\`.
    pub drives: Vec<String>,
}

pub trait CleanupEnvPort: Send + Sync {
    fn layout(&self) -> Result<Layout, CleanupError>;
    /// Файловая операция для этой ОС. На неподдерживаемых ОС возвращает `Unsupported`.
    fn fs(&self) -> Result<&dyn FsPort, CleanupError>;
}
