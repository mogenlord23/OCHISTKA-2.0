//! Остатки прежней системы «Обход блокировок» (удалена, ADR 0019): проверка доступности сервисов
//! (`probe`, её использует VPN) и папка данных для списка сайтов. Движков, драйверов и запуска
//! с повышением прав здесь больше нет.

use std::path::PathBuf;

pub mod probe;

/// Папка данных обхода: `%LOCALAPPDATA%\\OCHISTKA\\desync`, иначе временная. Там хранится список
/// сайтов (`user-sites.json`).
pub fn desync_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OCHISTKA")
        .join("desync")
}
