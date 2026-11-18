//! Команда «Процессы»: один типизированный вход `processes_request` с полем `action`.
//! Всё обращение к ОС идёт через порт платформы, правила завершения — через ядро.

use std::sync::OnceLock;

use ochistka_core::processes::{self, ProcessError, ProcessInfo, ProcessPort, ServiceInfo};
use serde::{Deserialize, Serialize};

/// Ошибка для интерфейса: `kind` — тип для ветвления, `message` — текст на русском.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ProcessesError {
    pub kind: String,
    pub message: String,
}

impl From<ProcessError> for ProcessesError {
    fn from(err: ProcessError) -> Self {
        let kind = match err {
            ProcessError::NotConfirmed => "notConfirmed",
            ProcessError::IdleProcess | ProcessError::SystemProcess | ProcessError::OwnProcess => {
                "protected"
            }
            ProcessError::NotFound(_) => "notFound",
            ProcessError::Replaced(_) => "replaced",
            ProcessError::KillFailed(_) => "killFailed",
            ProcessError::NotConnected(_) => "notConnected",
            ProcessError::Unsupported { .. } => "unsupported",
            ProcessError::Probe(_) => "probe",
        };
        Self {
            kind: kind.to_string(),
            message: err.to_string(),
        }
    }
}

/// Запросы интерфейса. Завершение требует `confirmed: true`; без него ядро отказывает.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProcessesRequest {
    List,
    ExePath {
        pid: u32,
    },
    EndTask {
        pid: u32,
        start_time_unix: u64,
        confirmed: bool,
    },
    Services,
}

/// Ответы интерфейсу: тип ответа в поле `type`.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProcessesResponse {
    List { processes: Vec<ProcessInfo> },
    ExePath { path: Option<String> },
    Ended { pid: u32 },
    Services { services: Vec<ServiceInfo> },
}

/// Порт текущей ОС. Создаётся один раз: sysinfo хранит предыдущий замер для расчёта CPU.
fn port() -> &'static dyn ProcessPort {
    static PORT: OnceLock<Box<dyn ProcessPort>> = OnceLock::new();
    PORT.get_or_init(ochistka_platform::current_process_port)
        .as_ref()
}

/// Выполняет запрос. Обращение к ОС идёт только через порт.
pub fn run_request(
    port: &dyn ProcessPort,
    request: ProcessesRequest,
    own_pid: u32,
) -> Result<ProcessesResponse, ProcessesError> {
    Ok(match request {
        ProcessesRequest::List => ProcessesResponse::List {
            processes: port.list()?,
        },
        ProcessesRequest::ExePath { pid } => ProcessesResponse::ExePath {
            path: port.exe_path(pid)?,
        },
        ProcessesRequest::EndTask {
            pid,
            start_time_unix,
            confirmed,
        } => {
            processes::end_task(port, pid, start_time_unix, confirmed, own_pid)?;
            ProcessesResponse::Ended { pid }
        }
        ProcessesRequest::Services => ProcessesResponse::Services {
            services: port.services()?,
        },
    })
}

#[tauri::command]
pub async fn processes_request(
    request: ProcessesRequest,
) -> Result<ProcessesResponse, ProcessesError> {
    run_request(port(), request, std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_task_without_confirmation_is_refused_by_backend() {
        let json = r#"{"action":"endTask","pid":1234,"startTimeUnix":5,"confirmed":false}"#;
        let request: ProcessesRequest = serde_json::from_str(json).unwrap();
        let err = run_request(&NoOsPort, request, 99).unwrap_err();
        assert_eq!(err.kind, "notConfirmed");
    }

    #[test]
    fn protected_pid_maps_to_protected_kind() {
        let request = ProcessesRequest::EndTask {
            pid: 4,
            start_time_unix: 0,
            confirmed: true,
        };
        let err = run_request(&NoOsPort, request, 99).unwrap_err();
        assert_eq!(err.kind, "protected");
    }

    #[test]
    fn request_json_uses_camel_case_fields() {
        let json = r#"{"action":"exePath","pid":42}"#;
        assert_eq!(
            serde_json::from_str::<ProcessesRequest>(json).unwrap(),
            ProcessesRequest::ExePath { pid: 42 }
        );
    }

    /// Порт, который не должен вызываться в тестах отказа.
    struct NoOsPort;

    impl ProcessPort for NoOsPort {
        fn list(&self) -> Result<Vec<ProcessInfo>, ProcessError> {
            panic!("ОС не должна вызываться")
        }
        fn exe_path(&self, _pid: u32) -> Result<Option<String>, ProcessError> {
            panic!("ОС не должна вызываться")
        }
        fn start_time(&self, _pid: u32) -> Result<Option<u64>, ProcessError> {
            panic!("ОС не должна вызываться")
        }
        fn kill(&self, _pid: u32) -> Result<bool, ProcessError> {
            panic!("ОС не должна вызываться")
        }
        fn services(&self) -> Result<Vec<ServiceInfo>, ProcessError> {
            panic!("ОС не должна вызываться")
        }
    }
}
