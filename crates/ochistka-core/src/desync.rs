//! Остатки прежней системы «Обход блокировок» (удалена, ADR 0019), без которых работает VPN:
//! - `coordinator` — одно общее право на сетевой режим (взаимное исключение VPN и обхода);
//! - `probe` и `routing` — проверка доступности сервисов, её результат использует VPN;
//! - `user_sites` — хранение списка сайтов для панели «Сайты».
//!
//! Движков обхода, драйверов и кнопки здесь больше нет: новый обход будет написан заново.

pub mod coordinator;
pub mod probe;
pub mod routing;
pub mod user_sites;

/// Что проверяется или должно открываться (одно понятие для всех ОС).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    Sites,
    Youtube,
    Discord,
    Telegram,
    Games,
}
