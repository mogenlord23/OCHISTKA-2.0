//! Пробный запуск полного NetZapret Universal V13 Lite. Только три действия: статус, запуск и
//! остановка (плюс пометка наблюдения). Параметров движка из интерфейса нет: профиль встроен.
//! Запуск происходит только по явной команде с согласием, никогда при старте приложения.

use std::time::Instant;

use ochistka_core::desync::{Coverage, DesyncError, TrialStatus, TrialVerdict};
use serde::Serialize;
use tauri::State;

use crate::commands::CommandError;
use crate::vpn::DesyncState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialNoteDto {
    pub service: &'static str,
    pub verdict: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialDto {
    pub active: bool,
    pub message: String,
    /// Почему запуск сейчас невозможен; `None` — можно запускать.
    pub unavailable: Option<String>,
    /// Короткая причина, если обход не выключился до конца (например, драйвер не остановлен).
    pub error: Option<String>,
    /// Только наблюдения пользователя, не подтверждение работы сервиса.
    pub notes: Vec<TrialNoteDto>,
}

fn service_name(s: Coverage) -> &'static str {
    match s {
        Coverage::Sites => "sites",
        Coverage::Youtube => "youtube",
        Coverage::Discord => "discord",
        Coverage::Telegram => "telegram",
        Coverage::Games => "games",
    }
}

fn verdict_name(v: TrialVerdict) -> &'static str {
    match v {
        TrialVerdict::Works => "works",
        TrialVerdict::Fails => "fails",
        TrialVerdict::Unchecked => "unchecked",
    }
}

fn to_dto(st: TrialStatus) -> TrialDto {
    TrialDto {
        active: st.active,
        message: st.message,
        unavailable: st.unavailable,
        error: st.error,
        notes: st
            .notes
            .into_iter()
            .map(|(service, verdict)| TrialNoteDto {
                service: service_name(service),
                verdict: verdict_name(verdict),
            })
            .collect(),
    }
}

fn err(e: DesyncError) -> CommandError {
    CommandError {
        kind: "desync".into(),
        message: e.to_string(),
    }
}

fn parse_service(name: &str) -> Option<Coverage> {
    Some(match name {
        "sites" => Coverage::Sites,
        "youtube" => Coverage::Youtube,
        "discord" => Coverage::Discord,
        "telegram" => Coverage::Telegram,
        "games" => Coverage::Games,
        _ => return None,
    })
}

fn parse_verdict(name: &str) -> Option<TrialVerdict> {
    Some(match name {
        "works" => TrialVerdict::Works,
        "fails" => TrialVerdict::Fails,
        "unchecked" => TrialVerdict::Unchecked,
        _ => return None,
    })
}

#[tauri::command]
pub fn desync_trial_status(state: State<'_, DesyncState>) -> TrialDto {
    to_dto(state.0.trial_status(Instant::now()))
}

/// `consent` — явное согласие пользователя; Rust проверяет его ещё раз.
#[tauri::command]
pub async fn desync_trial_start(
    state: State<'_, DesyncState>,
    consent: bool,
) -> Result<TrialDto, CommandError> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.start_trial(consent, &mut |_| {}).map_err(err)?;
        Ok(to_dto(service.trial_status(Instant::now())))
    })
    .await
    .map_err(|_| CommandError {
        kind: "desync".into(),
        message: "Операция прервана".into(),
    })?
}

#[tauri::command]
pub async fn desync_trial_stop(state: State<'_, DesyncState>) -> Result<TrialDto, CommandError> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let stopped = service.stop_trial();
        let dto = to_dto(service.trial_status(Instant::now()));
        stopped.map(|()| dto).map_err(err)
    })
    .await
    .map_err(|_| CommandError {
        kind: "desync".into(),
        message: "Операция прервана".into(),
    })?
}

#[tauri::command]
pub fn desync_trial_note(
    state: State<'_, DesyncState>,
    service: String,
    verdict: String,
) -> Result<TrialDto, CommandError> {
    let (Some(service), Some(verdict)) = (parse_service(&service), parse_verdict(&verdict)) else {
        return Err(CommandError {
            kind: "invalidTarget".into(),
            message: "Неизвестный сервис или оценка".into(),
        });
    };
    state.0.note_trial(service, verdict).map_err(err)?;
    Ok(to_dto(state.0.trial_status(Instant::now())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_validated() {
        assert_eq!(parse_service("youtube"), Some(Coverage::Youtube));
        assert_eq!(parse_service("x"), None);
        assert_eq!(parse_verdict("works"), Some(TrialVerdict::Works));
        assert_eq!(parse_verdict("proof"), None);
    }
}
