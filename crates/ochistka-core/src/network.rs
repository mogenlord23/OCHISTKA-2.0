//! Сеть, только чтение: адаптеры (трафик, скорость, адреса) и список TCP-соединений.
//! DTO, чистые помощники и порт. Без кода ОС: адаптер платформы читает данные и передаёт сюда.

use serde::Serialize;
use ts_rs::TS;

use crate::system::CoreError;

/// Сетевой адаптер для вкладки «Сеть». Скорость — байты в секунду за последний интервал замера.
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "NetworkAdapter.ts")]
pub struct NetworkAdapter {
    pub name: String,
    /// MAC в виде `AA:BB:CC:DD:EE:FF`; `None`, если у адаптера нет аппаратного адреса.
    pub mac: Option<String>,
    #[ts(type = "number")]
    pub received_bytes: u64,
    #[ts(type = "number")]
    pub transmitted_bytes: u64,
    #[ts(type = "number")]
    pub receive_rate: u64,
    #[ts(type = "number")]
    pub transmit_rate: u64,
    /// Адреса вида `192.168.0.5/24`.
    pub addresses: Vec<String>,
}

/// TCP-соединение для вкладки «Соединения».
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "TcpConnection.ts")]
pub struct TcpConnection {
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    /// Состояние по-английски, как в `netstat`: ESTABLISHED, LISTEN, TIME_WAIT…
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

/// Порт чтения сети. Только чтение: ничего в системе не меняет.
pub trait NetworkPort: Send + Sync {
    fn adapters(&self) -> Result<Vec<NetworkAdapter>, CoreError>;
    fn connections(&self) -> Result<Vec<TcpConnection>, CoreError>;
}

/// MAC в читаемом виде. Нулевой адрес (петля, туннели) означает «нет значения».
pub fn format_mac(bytes: [u8; 6]) -> Option<String> {
    if bytes == [0; 6] {
        return None;
    }
    Some(
        bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":"),
    )
}

/// Байты в секунду из прироста за интервал. Нулевой интервал даёт 0, а не деление на ноль.
pub fn bytes_per_second(delta_bytes: u64, elapsed_ms: u64) -> u64 {
    if elapsed_ms == 0 {
        return 0;
    }
    u64::try_from(u128::from(delta_bytes) * 1000 / u128::from(elapsed_ms)).unwrap_or(u64::MAX)
}

/// Адрес с длиной префикса: `192.168.0.5/24`.
pub fn format_address(addr: &str, prefix: u8) -> String {
    format!("{addr}/{prefix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_mac_upper_case_with_colons() {
        assert_eq!(
            format_mac([0xaa, 0x0b, 0xcc, 0x0d, 0xee, 0xff]).as_deref(),
            Some("AA:0B:CC:0D:EE:FF")
        );
    }

    #[test]
    fn zero_mac_is_absent() {
        assert_eq!(format_mac([0; 6]), None);
    }

    #[test]
    fn rate_scales_to_one_second() {
        assert_eq!(bytes_per_second(500, 500), 1000);
        assert_eq!(bytes_per_second(1000, 2000), 500);
    }

    #[test]
    fn rate_with_zero_interval_is_zero() {
        assert_eq!(bytes_per_second(123, 0), 0);
    }

    #[test]
    fn rate_does_not_overflow() {
        assert_eq!(bytes_per_second(u64::MAX, 1), u64::MAX);
    }

    #[test]
    fn address_has_prefix() {
        assert_eq!(format_address("10.0.0.2", 8), "10.0.0.2/8");
    }
}
