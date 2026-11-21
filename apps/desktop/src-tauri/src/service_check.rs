//! Явная проверка доступности сервисов. Запускается только командой из интерфейса:
//! при старте приложения никаких сетевых запросов нет.

use std::sync::Mutex;
use std::time::Instant;

use ochistka_core::desync::probe::{self, ObservationStore, Target};
use ochistka_core::desync::routing::Reachability;
use ochistka_core::desync::Coverage;
use serde::Serialize;
use tauri::State;

use crate::commands::CommandError;

/// Последние результаты проверок; маршрутизатор берёт из них только актуальные.
#[derive(Default)]
pub struct ProbeState(pub Mutex<ObservationStore>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceCheckDto {
    pub service: &'static str,
    /// `reachable` / `unreachable` / `unchecked`.
    pub state: &'static str,
    pub note: String,
    /// Всегда `false`: проверен веб-адрес, а не работа сервиса целиком.
    pub functional_confirmed: bool,
}

fn name(service: Coverage) -> &'static str {
    match service {
        Coverage::Sites => "sites",
        Coverage::Youtube => "youtube",
        Coverage::Discord => "discord",
        Coverage::Telegram => "telegram",
        Coverage::Games => "games",
    }
}

fn parse_targets(
    services: &[String],
    site_url: Option<String>,
    game: Option<String>,
) -> Result<Vec<Target>, CommandError> {
    services
        .iter()
        .map(|s| match s.as_str() {
            "youtube" => Ok(Target::Youtube),
            "discord" => Ok(Target::Discord),
            "telegram" => Ok(Target::Telegram),
            "sites" => Ok(Target::Site(site_url.clone().unwrap_or_default())),
            "games" => Ok(Target::Game(game.clone().unwrap_or_default())),
            _ => Err(CommandError {
                kind: "invalidTarget".into(),
                message: "Неизвестный сервис".into(),
            }),
        })
        .collect()
}

#[tauri::command]
pub async fn service_check(
    state: State<'_, ProbeState>,
    services: Vec<String>,
    site_url: Option<String>,
    game: Option<String>,
) -> Result<Vec<ServiceCheckDto>, CommandError> {
    let targets = parse_targets(&services, site_url, game)?;
    let results = tauri::async_runtime::spawn_blocking(move || {
        probe::run(
            &ochistka_platform::desync::probe::HttpsProbe,
            &targets,
            Instant::now(),
        )
    })
    .await
    .map_err(|_| CommandError {
        kind: "probe".into(),
        message: "Проверка прервана".into(),
    })?;
    if let Ok(mut store) = state.0.lock() {
        store.record(&results);
    }
    Ok(results
        .into_iter()
        .map(|r| ServiceCheckDto {
            service: name(r.service),
            state: match r.reachability {
                Reachability::Reachable => "reachable",
                Reachability::Unreachable => "unreachable",
                Reachability::Unchecked => "unchecked",
            },
            note: r.note,
            functional_confirmed: r.functional_confirmed,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_are_parsed_and_unknown_names_rejected() {
        let t = parse_targets(
            &["youtube".into(), "games".into(), "sites".into()],
            Some("https://example.com/".into()),
            None,
        )
        .unwrap();
        assert_eq!(t[0], Target::Youtube);
        assert_eq!(t[1], Target::Game(String::new()));
        assert_eq!(t[2], Target::Site("https://example.com/".into()));
        assert!(parse_targets(&["x".into()], None, None).is_err());
    }
}
