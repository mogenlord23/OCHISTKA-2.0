//! VPN: разбор подписок, конфигурация ядра sing-box, маршруты, системный прокси, подключение.
//! Здесь только логика и порты; сеть, процессы и реестр делает адаптер в `crates/platform`.

pub mod config;
pub mod country;
pub mod engine;
pub mod fetch;
pub mod latency;
pub mod mask;
pub mod parse;
pub mod routes;
pub mod service;
pub mod sysproxy;
pub mod xray;

use thiserror::Error;

/// Ошибки VPN. Тексты на русском: интерфейс показывает их как есть.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VpnError {
    #[error("Разрешены только ссылки https://")]
    NotHttps,
    #[error("Вставь ссылку подписки или список серверов")]
    EmptyInput,
    #[error("{0}")]
    Fetch(String),
    #[error("{0}")]
    Parse(String),
    #[error("VPN не готов: {0}")]
    CoreMissing(String),
    #[error("Не удалось подготовить VPN: {0}")]
    Prepare(String),
    #[error("{0}")]
    Engine(String),
    #[error("Не удалось изменить настройки сети: {0}")]
    Proxy(String),
    #[error("{0}")]
    ProxyConflict(String),
    #[error("Сначала выбери сервер")]
    NoServer,
    #[error("Переустановите программу, чтобы включить режим «Весь компьютер»")]
    NeedsReinstall,
    #[error("Сначала отключи VPN")]
    Busy,
    #[error("{0}")]
    Storage(String),
    #[error("{0}")]
    Rules(String),
}
