//! Заглушка сети для macOS: адаптеры и соединения пока не читаются.

use ochistka_core::network::{NetworkAdapter, NetworkPort, TcpConnection};
use ochistka_core::system::CoreError;

pub struct UnsupportedNetwork;

fn unsupported() -> CoreError {
    CoreError::Unsupported {
        feature: "Чтение сети".into(),
        os: "macOS".into(),
    }
}

impl NetworkPort for UnsupportedNetwork {
    fn adapters(&self) -> Result<Vec<NetworkAdapter>, CoreError> {
        Err(unsupported())
    }
    fn connections(&self) -> Result<Vec<TcpConnection>, CoreError> {
        Err(unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_reports_unsupported() {
        assert!(matches!(
            UnsupportedNetwork.adapters(),
            Err(CoreError::Unsupported { .. })
        ));
        assert!(matches!(
            UnsupportedNetwork.connections(),
            Err(CoreError::Unsupported { .. })
        ));
    }
}
