//! Команда «Компьютер»: видеокарты и материнская плата. Только чтение, через порт платформы.

use std::sync::OnceLock;

use ochistka_core::hardware::{HardwareOverview, HardwarePort};

use crate::commands::CommandError;

/// Порт текущей ОС. Создаётся один раз.
fn port() -> &'static dyn HardwarePort {
    static PORT: OnceLock<Box<dyn HardwarePort>> = OnceLock::new();
    PORT.get_or_init(ochistka_platform::current_hardware_port)
        .as_ref()
}

/// Чтение идёт в фоне: драйверы и WMI могут отвечать заметно дольше обычного вызова.
#[tauri::command]
pub async fn get_hardware() -> Result<HardwareOverview, CommandError> {
    tauri::async_runtime::spawn_blocking(|| port().overview().map_err(CommandError::from))
        .await
        .map_err(|err| CommandError {
            kind: "probe".into(),
            message: err.to_string(),
        })?
}
