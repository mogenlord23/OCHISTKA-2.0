//! Команды «Драйверы»: каталог по производителям, установка официального установщика,
//! открытие страницы производителя и обновлений Windows. Интерфейс передаёт только идентификатор
//! пункта; адреса и допустимые издатели подписи известны только ядру.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use ochistka_core::drivers::{self, DriverError, DriverHost, InstallProgress};
use serde::Serialize;
use tauri::ipc::Channel;

use crate::commands::CommandError;

fn host() -> &'static Arc<dyn DriverHost> {
    static HOST: OnceLock<Arc<dyn DriverHost>> = OnceLock::new();
    HOST.get_or_init(|| Arc::from(ochistka_platform::current_driver_host()))
}

/// Одновременно идёт не больше одной установки.
static INSTALLING: AtomicBool = AtomicBool::new(false);

impl From<DriverError> for CommandError {
    fn from(err: DriverError) -> Self {
        let kind = match err {
            DriverError::Unsupported(_) => "unsupported",
            DriverError::UnknownItem | DriverError::NoInstaller | DriverError::UrlRejected(_) => {
                "invalidTarget"
            }
            _ => "driver",
        };
        Self {
            kind: kind.into(),
            message: err.to_string(),
        }
    }
}

/// Пункт каталога для интерфейса. Адреса наружу не отдаются.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriverItem {
    pub id: String,
    pub vendor: String,
    pub title: String,
    pub description: String,
    /// `install` — есть прямой установщик, `openSite` — только официальная страница.
    pub action: &'static str,
    pub windows_update: bool,
}

pub fn catalog_items() -> Vec<DriverItem> {
    drivers::CATALOG
        .iter()
        .map(|item| DriverItem {
            id: item.id.into(),
            vendor: item.vendor.into(),
            title: item.title.into(),
            description: item.description.into(),
            action: if item.installer.is_some() {
                "install"
            } else {
                "openSite"
            },
            windows_update: item.windows_update,
        })
        .collect()
}

#[tauri::command]
pub fn drivers_catalog() -> Vec<DriverItem> {
    catalog_items()
}

/// Загрузка, проверка подписи и запуск идут в отдельном потоке (блокирующий HTTP-клиент).
/// Ход установки передаётся каналом `on_progress`.
#[tauri::command]
pub async fn driver_install(
    id: String,
    on_progress: Channel<InstallProgress>,
) -> Result<(), CommandError> {
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err(CommandError {
            kind: "busy".into(),
            message: "Уже идёт другая установка.".into(),
        });
    }
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        std::thread::spawn(move || {
            drivers::install(host().as_ref(), &id, &mut |event| {
                let _ = on_progress.send(event);
            })
        })
        .join()
    })
    .await;
    INSTALLING.store(false, Ordering::SeqCst);
    match outcome {
        Ok(Ok(result)) => result.map_err(CommandError::from),
        _ => Err(CommandError {
            kind: "driver".into(),
            message: "Установка прервалась неожиданно.".into(),
        }),
    }
}

#[tauri::command]
pub async fn driver_open_page(id: String) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        drivers::open_page(host().as_ref(), &id).map_err(CommandError::from)
    })
    .await
    .map_err(|err| CommandError {
        kind: "driver".into(),
        message: err.to_string(),
    })?
}

#[tauri::command]
pub async fn driver_open_windows_update() -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(|| {
        drivers::open_windows_update(host().as_ref()).map_err(CommandError::from)
    })
    .await
    .map_err(|err| CommandError {
        kind: "driver".into(),
        message: err.to_string(),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_dto_marks_install_and_open_site() {
        let items = catalog_items();
        let nvidia = items.iter().find(|i| i.id == "nvidia-app").unwrap();
        assert_eq!(nvidia.action, "install");
        let asus = items.iter().find(|i| i.id == "asus").unwrap();
        assert_eq!(asus.action, "openSite");
        assert!(asus.windows_update);
    }

    #[test]
    fn dto_serializes_camel_case_without_urls() {
        let json = serde_json::to_string(&catalog_items()[0]).unwrap();
        assert!(json.contains("\"windowsUpdate\""));
        assert!(!json.contains("http"));
    }
}
