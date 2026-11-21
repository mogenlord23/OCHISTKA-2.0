//! Явная проверка доступности сервисов. Запускается только командой из интерфейса:
//! при старте приложения никаких сетевых запросов нет.

use std::sync::Mutex;
use std::time::Instant;

use ochistka_core::desync::probe::{self, Evidence, ObservationStore, Target};
use ochistka_core::desync::routing::{AccessPath, Reachability, RoutePlan};
use ochistka_core::desync::{Coverage, DesyncService};
use serde::Serialize;
use tauri::State;

use crate::commands::CommandError;
use crate::vpn::DesyncState;

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
    // Номер берётся до запроса: запоздавшая старая проверка не затрёт более новую.
    let ticket = state
        .0
        .lock()
        .map(|mut s| s.begin())
        .map_err(|_| CommandError {
            kind: "probe".into(),
            message: "Состояние проверок недоступно".into(),
        })?;
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
        store.record(ticket, &results);
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccessDto {
    pub service: &'static str,
    /// `direct` / `vpn` / `desync` / `blocked`.
    pub route: &'static str,
    pub message: String,
    /// Что известно о проверке: не проверялось / устарело / подтверждения недостаточно / подтверждено.
    pub evidence: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanDto {
    pub services: Vec<ServiceAccessDto>,
    pub overall_route: &'static str,
    pub overall_message: String,
}

fn coverage(name: &str) -> Option<Coverage> {
    Some(match name {
        "sites" => Coverage::Sites,
        "youtube" => Coverage::Youtube,
        "discord" => Coverage::Discord,
        "telegram" => Coverage::Telegram,
        "games" => Coverage::Games,
        _ => return None,
    })
}

fn describe(plan: RoutePlan) -> (&'static str, String) {
    match plan {
        RoutePlan::Use(AccessPath::Direct) => ("direct", "Прямое соединение".into()),
        RoutePlan::Use(AccessPath::Vpn) => ("vpn", "Через VPN".into()),
        RoutePlan::Use(AccessPath::Desync(_)) => ("desync", "Обход блокировок".into()),
        RoutePlan::Block(reason) => ("blocked", reason.message().into()),
    }
}

fn evidence_text(e: &Evidence) -> String {
    match e {
        Evidence::NotChecked => "Не проверено".into(),
        Evidence::Stale => "Результат устарел: проверьте снова".into(),
        Evidence::Insufficient(_, note) => {
            format!("Подтверждения недостаточно: {note}")
        }
        Evidence::Confirmed(Reachability::Reachable) => "Работа сервиса подтверждена".into(),
        Evidence::Confirmed(_) => "Подтверждено: сервис недоступен".into(),
    }
}

/// Только чтение: берёт актуальные наблюдения и спрашивает существующий `DesyncService`.
/// Ничего не проверяет, не устанавливает и не запускает.
pub fn build_access_plan(
    desync: &DesyncService,
    store: &ObservationStore,
    services: &[String],
    site_url: Option<&str>,
    now: Instant,
) -> Result<AccessPlanDto, CommandError> {
    let wanted: Vec<Coverage> = services
        .iter()
        .map(|s| {
            coverage(s).ok_or_else(|| CommandError {
                kind: "invalidTarget".into(),
                message: "Неизвестный сервис".into(),
            })
        })
        .collect::<Result<_, _>>()?;
    let observed = store.fresh(now, site_url);
    let plan = desync.plan_access(&wanted, &observed);
    let services = plan
        .per_service
        .iter()
        .map(|r| {
            let (route, message) = describe(r.plan);
            ServiceAccessDto {
                service: name(r.service),
                route,
                message,
                evidence: evidence_text(&store.evidence(r.service, site_url, now)),
            }
        })
        .collect();
    let (overall_route, overall_message) = describe(plan.overall);
    Ok(AccessPlanDto {
        services,
        overall_route,
        overall_message,
    })
}

#[tauri::command]
pub fn service_access_plan(
    probe: State<'_, ProbeState>,
    desync: State<'_, DesyncState>,
    services: Vec<String>,
    site_url: Option<String>,
) -> Result<AccessPlanDto, CommandError> {
    let store = probe.0.lock().map_err(|_| CommandError {
        kind: "probe".into(),
        message: "Состояние проверок недоступно".into(),
    })?;
    build_access_plan(
        &desync.0,
        &store,
        &services,
        site_url.as_deref(),
        Instant::now(),
    )
}

#[cfg(test)]
mod tests {
    use ochistka_core::desync::probe::{run, ProbeTransport, Reply, CONTROL_URL, DISCORD_URL};
    use ochistka_core::desync::routing::VpnSignal;
    use ochistka_core::desync::{Platform, UnavailableAdapter};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;

    struct Counting(AtomicUsize);
    impl ProbeTransport for Counting {
        fn get(&self, url: &str) -> Reply {
            self.0.fetch_add(1, Ordering::SeqCst);
            if url == CONTROL_URL || url == DISCORD_URL {
                Reply::Answered(200)
            } else {
                Reply::Timeout
            }
        }
    }

    fn service(vpn: VpnSignal) -> DesyncService {
        let dir = std::env::temp_dir().join("ochistka-plan-test");
        DesyncService::new(
            dir,
            Box::new(UnavailableAdapter::new(Platform::Windows)),
            Box::new(move || vpn),
        )
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plan_is_read_only_and_never_enables_bypass_without_confirmation() {
        let net = Counting(AtomicUsize::new(0));
        let t0 = Instant::now();
        let mut store = ObservationStore::default();
        let ticket = store.begin();
        store.record(ticket, &run(&net, &[Target::Discord, Target::Telegram], t0));
        let calls_before = net.0.load(Ordering::SeqCst);
        let desync = service(VpnSignal::default());
        let plan = build_access_plan(
            &desync,
            &store,
            &names(&["discord", "telegram", "games", "sites"]),
            None,
            t0 + Duration::from_secs(5),
        )
        .unwrap();
        // Планирование не делало запросов и не запускало движок.
        assert_eq!(net.0.load(Ordering::SeqCst), calls_before);
        assert!(!desync.is_running());
        // Discord: веб-адрес отвечает, но этого мало; Telegram: не отвечает — но тоже не повод запускать.
        let by = |n: &str| plan.services.iter().find(|s| s.service == n).unwrap();
        assert!(by("discord")
            .evidence
            .starts_with("Подтверждения недостаточно"));
        assert_eq!(by("discord").route, "blocked");
        assert_eq!(by("telegram").route, "blocked");
        assert_eq!(by("games").evidence, "Не проверено");
        assert_eq!(by("sites").route, "blocked");
        assert_eq!(plan.overall_route, "blocked");
    }

    #[test]
    fn stale_results_are_reported_as_stale_and_unknown_service_is_rejected() {
        let net = Counting(AtomicUsize::new(0));
        let t0 = Instant::now();
        let mut store = ObservationStore::default();
        let ticket = store.begin();
        store.record(ticket, &run(&net, &[Target::Discord], t0));
        let desync = service(VpnSignal::default());
        let plan = build_access_plan(
            &desync,
            &store,
            &names(&["discord"]),
            None,
            t0 + Duration::from_secs(500),
        )
        .unwrap();
        assert!(plan.services[0].evidence.contains("устарел"));
        assert!(build_access_plan(&desync, &store, &names(&["x"]), None, t0).is_err());
    }

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
