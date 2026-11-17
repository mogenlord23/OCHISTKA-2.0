//! Типизированные команды Tauri. Интерфейс получает данные только отсюда.

use std::sync::Arc;

use ochistka_core::system::{CoreError, SystemInfoPort, SystemOverview};
use serde::Serialize;
use tauri::State;

/// Ошибка для интерфейса: `kind` — "unsupported" (ОС не поддерживается) или "probe" (сбой чтения).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct CommandError {
    pub kind: String,
    pub message: String,
}

impl From<CoreError> for CommandError {
    fn from(err: CoreError) -> Self {
        let kind = match err {
            CoreError::Unsupported { .. } => "unsupported",
            CoreError::Probe(_) => "probe",
        };
        Self {
            kind: kind.to_string(),
            message: err.to_string(),
        }
    }
}

pub fn load_overview(port: &dyn SystemInfoPort) -> Result<SystemOverview, CommandError> {
    port.overview().map_err(CommandError::from)
}

/// Состояние приложения: адаптер системных сведений текущей ОС.
pub struct AppState {
    pub system: Arc<dyn SystemInfoPort>,
}

/// Чтение выполняется в фоновом потоке, чтобы не блокировать главный поток окна.
#[tauri::command]
pub async fn get_system_overview(
    state: State<'_, AppState>,
) -> Result<SystemOverview, CommandError> {
    let port = Arc::clone(&state.system);
    tauri::async_runtime::spawn_blocking(move || load_overview(port.as_ref()))
        .await
        .map_err(|err| CommandError {
            kind: "probe".into(),
            message: err.to_string(),
        })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::system::{
        CoreError, CpuStatus, MemoryStatus, SystemInfoPort, SystemOverview,
    };

    enum Outcome {
        Ok,
        Unsupported,
        Probe,
    }

    struct FakePort {
        outcome: Outcome,
    }

    impl SystemInfoPort for FakePort {
        fn overview(&self) -> Result<SystemOverview, CoreError> {
            match self.outcome {
                Outcome::Ok => Ok(sample()),
                Outcome::Unsupported => Err(CoreError::Unsupported {
                    feature: "Сведения о системе".into(),
                    os: "macOS".into(),
                }),
                Outcome::Probe => Err(CoreError::Probe("x".into())),
            }
        }
    }

    fn sample() -> SystemOverview {
        SystemOverview {
            cpu: CpuStatus::new(12.0, 8),
            memory: MemoryStatus::new(100, 50),
            disks: vec![],
        }
    }

    #[test]
    fn load_overview_passes_through_snapshot() {
        let port = FakePort {
            outcome: Outcome::Ok,
        };
        assert_eq!(load_overview(&port), Ok(sample()));
    }

    #[test]
    fn load_overview_maps_unsupported_to_kind_unsupported() {
        let port = FakePort {
            outcome: Outcome::Unsupported,
        };
        let err = load_overview(&port).unwrap_err();
        assert_eq!(err.kind, "unsupported");
        assert_eq!(err.message, "Сведения о системе недоступно на macOS");
    }

    #[test]
    fn load_overview_maps_probe_error_to_kind_probe() {
        let port = FakePort {
            outcome: Outcome::Probe,
        };
        let err = load_overview(&port).unwrap_err();
        assert_eq!(err.kind, "probe");
        assert_eq!(err.message, "ошибка чтения системных данных: x");
    }
}
