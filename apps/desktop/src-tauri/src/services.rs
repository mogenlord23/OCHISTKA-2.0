//! Команда «Службы»: один типизированный вход `services_request`. Правила (чёрный список,
//! подтверждения, проверка результата) живут в ядре, системные вызовы делает порт платформы.

use std::sync::OnceLock;

use ochistka_core::services::{
    self, RequestedStart, ServiceAction, ServiceBackend, ServiceError, ServiceInfo,
    ServicesSnapshot,
};
use serde::{Deserialize, Serialize};

/// Ошибка для интерфейса: `kind` — тип для ветвления, `message` — текст на русском.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ServicesError {
    pub kind: String,
    pub message: String,
}

impl From<ServiceError> for ServicesError {
    fn from(err: ServiceError) -> Self {
        let kind = match err {
            ServiceError::InvalidName => "invalidName",
            ServiceError::NotFound(_) => "notFound",
            ServiceError::Protected(_) => "protected",
            ServiceError::NotConfirmed(_) => "notConfirmed",
            ServiceError::NeedsAdmin => "needsAdmin",
            ServiceError::Refused(_) => "refused",
            ServiceError::NoSavedStart => "noSavedStart",
            ServiceError::Failed(_) => "failed",
            ServiceError::Probe(_) => "probe",
            ServiceError::Unsupported { .. } => "unsupported",
        };
        Self {
            kind: kind.to_string(),
            message: err.to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StartKind {
    Auto,
    Manual,
    Disabled,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ServicesRequest {
    List,
    Start {
        name: String,
    },
    Stop {
        name: String,
        confirmed: bool,
    },
    SetStart {
        name: String,
        start: StartKind,
        confirmed: bool,
    },
    RestoreStart {
        name: String,
    },
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServicesResponse {
    List { snapshot: ServicesSnapshot },
    Changed { service: ServiceInfo },
}

fn backend() -> &'static dyn ServiceBackend {
    static BACKEND: OnceLock<Box<dyn ServiceBackend>> = OnceLock::new();
    BACKEND
        .get_or_init(ochistka_platform::current_service_backend)
        .as_ref()
}

pub fn run_request(
    backend: &dyn ServiceBackend,
    request: ServicesRequest,
) -> Result<ServicesResponse, ServicesError> {
    let (name, action, confirmed) = match request {
        ServicesRequest::List => {
            return Ok(ServicesResponse::List {
                snapshot: services::list_services(backend)?,
            })
        }
        ServicesRequest::Start { name } => (name, ServiceAction::Start, false),
        ServicesRequest::Stop { name, confirmed } => (name, ServiceAction::Stop, confirmed),
        ServicesRequest::SetStart {
            name,
            start,
            confirmed,
        } => {
            let requested = match start {
                StartKind::Auto => RequestedStart::Auto,
                StartKind::Manual => RequestedStart::Manual,
                StartKind::Disabled => RequestedStart::Disabled,
            };
            (name, ServiceAction::SetStart(requested), confirmed)
        }
        ServicesRequest::RestoreStart { name } => (name, ServiceAction::RestoreStart, false),
    };
    Ok(ServicesResponse::Changed {
        service: services::apply(backend, &name, action, confirmed)?,
    })
}

/// Команды идут в фоновом потоке: запуск и остановка службы могут занять несколько секунд.
#[tauri::command]
pub async fn services_request(request: ServicesRequest) -> Result<ServicesResponse, ServicesError> {
    tauri::async_runtime::spawn_blocking(move || run_request(backend(), request))
        .await
        .map_err(|err| ServicesError {
            kind: "failed".into(),
            message: format!("Внутренняя ошибка: {err}"),
        })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::services::{RawService, ScOutput};

    /// Порт, который не должен вызываться при отказе по правилам ядра.
    struct Locked;

    impl ServiceBackend for Locked {
        fn registry_entries(&self) -> Result<Vec<RawService>, ServiceError> {
            Ok(vec![RawService {
                name: "RpcSs".into(),
                display_name: "RPC".into(),
                start: Some(2),
                delayed: false,
            }])
        }
        fn sc(&self, _args: &[&str]) -> Result<ScOutput, ServiceError> {
            panic!("sc не должен вызываться")
        }
        fn is_elevated(&self) -> bool {
            true
        }
        fn saved_start(&self, _name: &str) -> Option<u32> {
            None
        }
        fn save_start(&self, _name: &str, _code: u32) -> Result<(), ServiceError> {
            panic!("нельзя")
        }
        fn clear_saved_start(&self, _name: &str) {}
    }

    #[test]
    fn request_json_uses_camel_case() {
        let json = r#"{"action":"setStart","name":"Fax","start":"manual","confirmed":false}"#;
        assert_eq!(
            serde_json::from_str::<ServicesRequest>(json).unwrap(),
            ServicesRequest::SetStart {
                name: "Fax".into(),
                start: StartKind::Manual,
                confirmed: false
            }
        );
    }

    #[test]
    fn critical_service_is_refused_with_protected_kind_before_the_os() {
        let err = run_request(
            &Locked,
            ServicesRequest::Stop {
                name: "RpcSs".into(),
                confirmed: true,
            },
        )
        .unwrap_err();
        assert_eq!(err.kind, "protected");
        let err = run_request(&Locked, ServicesRequest::Start { name: "/?".into() }).unwrap_err();
        assert_eq!(err.kind, "invalidName");
    }
}
