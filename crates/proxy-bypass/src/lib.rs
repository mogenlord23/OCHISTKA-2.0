//! «Обход сайтов»: локальный прокси на 127.0.0.1 (HTTP CONNECT и SOCKS5), который для TLS-соединений
//! к сайтам из списка режет первый пакет ClientHello на две части так, чтобы имя сайта не
//! оказалось целиком в одном пакете. Остальной трафик проходит без изменений.
//!
//! Только стандартная библиотека: драйверов, перехвата пакетов и внешних программ нет. Настройки
//! системного прокси меняет не этот крейт, а адаптеры ОС (`SystemProxyPort`, `crates/platform`).

pub mod controller;
pub mod server;
pub mod sites;
pub mod system;
pub mod tls;

pub use controller::{BypassController, BypassError};
pub use server::ProxyServer;
pub use sites::{builtin_sites, SiteList};
pub use system::{Snapshot, SystemProxyPort};
