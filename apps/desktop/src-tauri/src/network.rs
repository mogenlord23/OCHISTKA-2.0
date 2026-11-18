//! Команды вкладок «Сеть» и «Соединения»: адаптеры и TCP-соединения. Только чтение, через порт платформы.

use std::sync::OnceLock;

use ochistka_core::network::{NetworkAdapter, NetworkPort, TcpConnection};

use crate::commands::CommandError;

/// Порт текущей ОС. Создаётся один раз: он хранит снимок для расчёта скорости.
fn port() -> &'static dyn NetworkPort {
    static PORT: OnceLock<Box<dyn NetworkPort>> = OnceLock::new();
    PORT.get_or_init(ochistka_platform::current_network_port)
        .as_ref()
}

fn join_error(err: tauri::Error) -> CommandError {
    CommandError {
        kind: "probe".into(),
        message: err.to_string(),
    }
}

#[tauri::command]
pub async fn get_network_adapters() -> Result<Vec<NetworkAdapter>, CommandError> {
    tauri::async_runtime::spawn_blocking(|| port().adapters().map_err(CommandError::from))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn get_tcp_connections() -> Result<Vec<TcpConnection>, CommandError> {
    tauri::async_runtime::spawn_blocking(|| port().connections().map_err(CommandError::from))
        .await
        .map_err(join_error)?
}
