//! Команды теста скорости. Тест идёт в рабочем потоке, ход приходит в интерфейс через Channel;
//! отмена взводит общий флаг. Запускается только по нажатию кнопки.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use ochistka_core::speedtest::{run_speedtest, SpeedConfig, SpeedEvent};
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::CommandError;

/// Флаг отмены текущего теста; `None`, когда тест не идёт.
#[derive(Default)]
pub struct SpeedState {
    running: Mutex<Option<Arc<AtomicBool>>>,
}

#[tauri::command]
pub async fn speedtest_start(
    state: State<'_, SpeedState>,
    on_event: Channel<SpeedEvent>,
) -> Result<(), CommandError> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = state.running.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_some() {
            return Err(CommandError {
                kind: "busy".into(),
                message: "Тест уже идёт".into(),
            });
        }
        *slot = Some(cancel.clone());
    }
    let flag = cancel.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let transport = ochistka_platform::speedtest::current_speed_transport();
        run_speedtest(
            transport.as_ref(),
            &SpeedConfig::default(),
            &flag,
            &mut |event| {
                let _ = on_event.send(event);
            },
        );
    })
    .await;
    *state.running.lock().unwrap_or_else(|e| e.into_inner()) = None;
    outcome.map_err(|err| CommandError {
        kind: "probe".into(),
        message: err.to_string(),
    })
}

#[tauri::command]
pub fn speedtest_cancel(state: State<'_, SpeedState>) {
    if let Some(flag) = state
        .running
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        flag.store(true, Ordering::Relaxed);
    }
}
