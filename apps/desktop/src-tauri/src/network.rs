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

// ---------- Провайдер и расположение ----------

use ochistka_core::netinfo::{self, ConsentStore, GeoConsent, GeoError, GeoInfo};

impl From<GeoError> for CommandError {
    fn from(err: GeoError) -> Self {
        Self {
            kind: err.kind().into(),
            message: err.message().into(),
        }
    }
}

fn consent_store() -> ochistka_platform::geo::FileConsent {
    ochistka_platform::geo::FileConsent::new(&ochistka_platform::geo::settings_path())
}

/// Дано ли согласие на внешний запрос. Ничего никуда не отправляет.
#[tauri::command]
pub fn geo_consent() -> GeoConsent {
    GeoConsent {
        granted: consent_store().granted(),
    }
}

/// Выдать (кнопка «Определить») или отозвать согласие.
#[tauri::command]
pub fn geo_set_consent(granted: bool) -> Result<GeoConsent, CommandError> {
    netinfo::set_consent(&consent_store(), granted).map_err(CommandError::from)
}

/// Определить провайдера и расположение. Без согласия запрос не уходит.
#[tauri::command]
pub async fn geo_lookup() -> Result<GeoInfo, CommandError> {
    tauri::async_runtime::spawn_blocking(|| {
        let transport = ochistka_platform::geo::current_geo_transport();
        netinfo::lookup(&consent_store(), transport.as_ref()).map_err(CommandError::from)
    })
    .await
    .map_err(join_error)?
}
