//! Независимая проверка доступности веб-адресов сервисов: даёт маршрутизатору наблюдения.
//!
//! Ограничения (важно): ответ веб-адреса — НЕ доказательство того, что сервис работает целиком
//! (звонки Discord, MTProto Telegram, видеопоток YouTube). Поэтому `functional_confirmed`
//! всегда `false`. Блокировкой считается только повторяемая обрыв/таймаут при доказанно
//! работающем интернете; ошибки DNS, TLS и неопределённые ответы остаются `Unchecked`.
//! Запросы выполняет подставляемый транспорт; сам модуль сеть не использует.

use std::time::{Duration, Instant};

use super::routing::{Reachability, ServiceObservations};
use super::Coverage;

/// Сколько результат считается актуальным.
pub const FRESH_FOR: Duration = Duration::from_secs(120);

/// Нейтральный адрес проверки самого интернета (тот же, что использует Windows).
pub const CONTROL_URL: &str = "https://www.msftconnecttest.com/connecttest.txt";
pub const YOUTUBE_URL: &str = "https://www.youtube.com/generate_204";
pub const DISCORD_URL: &str = "https://discord.com/api/v10/gateway";
pub const TELEGRAM_URL: &str = "https://web.telegram.org/";

/// Что проверять. Для сайта нужен конкретный адрес, для игры — конкретная игра.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Youtube,
    Discord,
    Telegram,
    Site(String),
    Game(String),
}

/// Итог одного запроса, как его видит транспорт.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// Сервер ответил по HTTPS (сертификат проверен).
    Answered(u16),
    Timeout,
    /// Соединение отклонено или оборвано.
    Refused,
    Dns,
    Tls,
    Other(String),
}

/// Транспорт: HTTPS без системного и переменного прокси, с проверкой TLS, ограничением времени
/// и размера ответа. Реализация — в `ochistka-platform`.
pub trait ProbeTransport: Send + Sync {
    fn get(&self, url: &str) -> Reply;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeResult {
    pub service: Coverage,
    pub reachability: Reachability,
    /// Короткое пояснение для пользователя.
    pub note: String,
    /// Всегда `false`: проверен только веб-адрес.
    pub functional_confirmed: bool,
    /// Для `Sites` — проверенный адрес: результат одного сайта не относится к другому.
    pub site: Option<String>,
    pub at: Instant,
}

fn unchecked(service: Coverage, note: &str, at: Instant) -> ProbeResult {
    ProbeResult {
        service,
        reachability: Reachability::Unchecked,
        note: note.into(),
        functional_confirmed: false,
        site: None,
        at,
    }
}

fn valid_site(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    url.len() <= 2048
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
        && !host.is_empty()
        && !host.contains('@')
        && host.contains('.')
}

fn target_info(target: &Target) -> Option<(Coverage, String)> {
    match target {
        Target::Youtube => Some((Coverage::Youtube, YOUTUBE_URL.into())),
        Target::Discord => Some((Coverage::Discord, DISCORD_URL.into())),
        Target::Telegram => Some((Coverage::Telegram, TELEGRAM_URL.into())),
        Target::Site(url) if valid_site(url) => Some((Coverage::Sites, url.clone())),
        _ => None,
    }
}

fn answered(reply: &Reply) -> bool {
    matches!(reply, Reply::Answered(code) if *code < 500)
}

/// Проверяет цели. Сначала один раз проверяется сам интернет: если он не отвечает, ничего не
/// утверждаем (всё `Unchecked`).
pub fn run(transport: &dyn ProbeTransport, targets: &[Target], now: Instant) -> Vec<ProbeResult> {
    let mut control: Option<bool> = None;
    targets
        .iter()
        .map(|target| {
            let Some((service, url)) = target_info(target) else {
                return match target {
                    Target::Game(_) => unchecked(
                        Coverage::Games,
                        "Для игр нет надёжной проверки: состояние не определено",
                        now,
                    ),
                    _ => unchecked(
                        Coverage::Sites,
                        "Нужен конкретный адрес сайта (https://…)",
                        now,
                    ),
                };
            };
            let internet = *control.get_or_insert_with(|| answered(&transport.get(CONTROL_URL)));
            if !internet {
                return unchecked(
                    service,
                    "Нет общего доступа в интернет: не проверялось",
                    now,
                );
            }
            let first = transport.get(&url);
            let (reachability, note) = if answered(&first) {
                (
                    Reachability::Reachable,
                    "Веб-адрес отвечает (работа сервиса целиком не проверена)",
                )
            } else {
                match first {
                    Reply::Timeout | Reply::Refused => {
                        let second = transport.get(&url);
                        if matches!(second, Reply::Timeout | Reply::Refused) {
                            (
                                Reachability::Unreachable,
                                "Веб-адрес не отвечает при рабочем интернете (повторно)",
                            )
                        } else if answered(&second) {
                            (
                                Reachability::Reachable,
                                "Веб-адрес отвечает (работа сервиса целиком не проверена)",
                            )
                        } else {
                            (Reachability::Unchecked, "Результат неоднозначный")
                        }
                    }
                    Reply::Dns => (Reachability::Unchecked, "Ошибка DNS: причина не доказана"),
                    Reply::Tls => (Reachability::Unchecked, "Ошибка TLS: причина не доказана"),
                    _ => (Reachability::Unchecked, "Ответ неопределённый"),
                }
            };
            ProbeResult {
                service,
                reachability,
                note: note.into(),
                functional_confirmed: false,
                site: (service == Coverage::Sites).then(|| url.clone()),
                at: now,
            }
        })
        .collect()
}

/// Что известно о сервисе на данный момент (для понятного ответа пользователю).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evidence {
    NotChecked,
    Stale,
    /// Свежий результат есть, но проверен только веб-адрес: подтверждения недостаточно.
    Insufficient(Reachability, String),
    /// Работа сервиса подтверждена функциональной проверкой (пока таких проверок нет).
    Confirmed(Reachability),
}

struct Entry {
    ticket: u64,
    result: ProbeResult,
}

/// Последние результаты. Для маршрутизатора годятся только актуальные результаты с
/// подтверждённой работой сервиса; диагностика веб-адреса маршрут не определяет.
#[derive(Default)]
pub struct ObservationStore {
    next: u64,
    entries: Vec<Entry>,
}

impl ObservationStore {
    /// Номер начатой проверки. Берётся ДО запроса: более поздний номер — более свежая проверка.
    pub fn begin(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// Записывает результаты проверки с номером `ticket`. Запоздавшая старая проверка не
    /// перезаписывает результат более новой.
    pub fn record(&mut self, ticket: u64, results: &[ProbeResult]) {
        for r in results {
            match self
                .entries
                .iter_mut()
                .find(|e| e.result.service == r.service && e.result.site == r.site)
            {
                Some(entry) if entry.ticket > ticket => {}
                Some(entry) => {
                    entry.ticket = ticket;
                    entry.result = r.clone();
                }
                None => self.entries.push(Entry {
                    ticket,
                    result: r.clone(),
                }),
            }
        }
    }

    fn find(&self, service: Coverage, site: Option<&str>) -> Option<&ProbeResult> {
        self.entries.iter().map(|e| &e.result).find(|r| {
            r.service == service && (service != Coverage::Sites || r.site.as_deref() == site)
        })
    }

    fn is_fresh(r: &ProbeResult, now: Instant) -> bool {
        now.checked_duration_since(r.at)
            .is_some_and(|age| age <= FRESH_FOR)
    }

    pub fn evidence(&self, service: Coverage, site: Option<&str>, now: Instant) -> Evidence {
        let Some(r) = self.find(service, site) else {
            return Evidence::NotChecked;
        };
        if !Self::is_fresh(r, now) {
            Evidence::Stale
        } else if r.functional_confirmed {
            Evidence::Confirmed(r.reachability)
        } else if r.reachability == Reachability::Unchecked {
            Evidence::NotChecked
        } else {
            Evidence::Insufficient(r.reachability, r.note.clone())
        }
    }

    /// Наблюдения для маршрутизатора: только актуальные И функционально подтверждённые.
    /// Диагностика HTTPS сюда не попадает; всё остальное — `Unchecked`.
    pub fn fresh(&self, now: Instant, site: Option<&str>) -> ServiceObservations {
        let mut out = ServiceObservations::new();
        for service in [
            Coverage::Sites,
            Coverage::Youtube,
            Coverage::Discord,
            Coverage::Telegram,
            Coverage::Games,
        ] {
            if let Evidence::Confirmed(r) = self.evidence(service, site, now) {
                out.set(service, r);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fake {
        replies: HashMap<String, Vec<Reply>>,
        calls: Mutex<Vec<String>>,
    }
    impl Fake {
        fn with(mut self, url: &str, replies: Vec<Reply>) -> Self {
            self.replies.insert(url.into(), replies);
            self
        }
        fn online() -> Self {
            Fake::default().with(CONTROL_URL, vec![Reply::Answered(200)])
        }
    }
    impl ProbeTransport for Fake {
        fn get(&self, url: &str) -> Reply {
            let mut calls = self.calls.lock().unwrap();
            let n = calls.iter().filter(|c| *c == url).count();
            calls.push(url.into());
            let list = self.replies.get(url).cloned().unwrap_or_default();
            list.get(n)
                .or(list.last())
                .cloned()
                .unwrap_or(Reply::Other("нет".into()))
        }
    }

    fn one(f: &Fake, t: Target) -> ProbeResult {
        run(f, &[t], Instant::now()).remove(0)
    }

    #[test]
    fn answering_address_is_reachable_but_never_functionally_confirmed() {
        let f = Fake::online().with(YOUTUBE_URL, vec![Reply::Answered(204)]);
        let r = one(&f, Target::Youtube);
        assert_eq!(
            (r.service, r.reachability),
            (Coverage::Youtube, Reachability::Reachable)
        );
        assert!(!r.functional_confirmed);
    }

    #[test]
    fn services_are_independent() {
        let f = Fake::online()
            .with(YOUTUBE_URL, vec![Reply::Answered(204)])
            .with(DISCORD_URL, vec![Reply::Timeout])
            .with(TELEGRAM_URL, vec![Reply::Tls]);
        let all = run(
            &f,
            &[Target::Youtube, Target::Discord, Target::Telegram],
            Instant::now(),
        );
        let got: Vec<_> = all.iter().map(|r| (r.service, r.reachability)).collect();
        assert_eq!(
            got,
            vec![
                (Coverage::Youtube, Reachability::Reachable),
                (Coverage::Discord, Reachability::Unreachable),
                (Coverage::Telegram, Reachability::Unchecked),
            ]
        );
        // Интернет проверен один раз на всю серию.
        let calls = f.calls.lock().unwrap();
        assert_eq!(calls.iter().filter(|c| *c == CONTROL_URL).count(), 1);
    }

    #[test]
    fn one_timeout_is_not_a_proven_block() {
        let f = Fake::online().with(DISCORD_URL, vec![Reply::Timeout, Reply::Answered(200)]);
        assert_eq!(
            one(&f, Target::Discord).reachability,
            Reachability::Reachable
        );
        let f = Fake::online().with(DISCORD_URL, vec![Reply::Timeout, Reply::Dns]);
        assert_eq!(
            one(&f, Target::Discord).reachability,
            Reachability::Unchecked
        );
    }

    #[test]
    fn dns_tls_and_server_errors_are_never_a_block() {
        for reply in [
            Reply::Dns,
            Reply::Tls,
            Reply::Answered(503),
            Reply::Other("x".into()),
        ] {
            let f = Fake::online().with(TELEGRAM_URL, vec![reply.clone()]);
            assert_eq!(
                one(&f, Target::Telegram).reachability,
                Reachability::Unchecked,
                "{reply:?}"
            );
        }
    }

    #[test]
    fn no_internet_means_no_claims() {
        let f = Fake::default()
            .with(CONTROL_URL, vec![Reply::Timeout])
            .with(DISCORD_URL, vec![Reply::Timeout]);
        assert_eq!(
            one(&f, Target::Discord).reachability,
            Reachability::Unchecked
        );
        assert_eq!(f.calls.lock().unwrap().len(), 1, "сервис не опрашивался");
    }

    #[test]
    fn games_and_bad_sites_stay_unchecked_without_any_request() {
        let f = Fake::online();
        let g = one(&f, Target::Game("Dota 2".into()));
        assert_eq!(
            (g.service, g.reachability),
            (Coverage::Games, Reachability::Unchecked)
        );
        for bad in [
            "http://example.com",
            "example.com",
            "https://",
            "https://a b.com",
            "https://u@x.com",
        ] {
            let r = one(&f, Target::Site(bad.into()));
            assert_eq!(r.reachability, Reachability::Unchecked, "{bad}");
        }
        assert!(f.calls.lock().unwrap().is_empty());
        let f = Fake::online().with("https://example.com/", vec![Reply::Answered(200)]);
        let r = one(&f, Target::Site("https://example.com/".into()));
        assert_eq!(
            (r.service, r.reachability),
            (Coverage::Sites, Reachability::Reachable)
        );
    }

    fn stored(f: &Fake, targets: &[Target], t: Instant) -> ObservationStore {
        let mut store = ObservationStore::default();
        let ticket = store.begin();
        store.record(ticket, &run(f, targets, t));
        store
    }

    #[test]
    fn https_reply_alone_never_becomes_a_routing_observation() {
        let t0 = Instant::now();
        let f = Fake::online().with(YOUTUBE_URL, vec![Reply::Answered(204)]);
        let store = stored(&f, &[Target::Youtube], t0);
        // Для диагностики результат есть…
        assert!(matches!(
            store.evidence(Coverage::Youtube, None, t0),
            Evidence::Insufficient(Reachability::Reachable, _)
        ));
        // …но маршрутизатор получает «не проверено».
        assert_eq!(
            store.fresh(t0, None).get(Coverage::Youtube),
            Reachability::Unchecked
        );
    }

    #[test]
    fn functionally_confirmed_fresh_result_reaches_the_router_and_goes_stale() {
        let t0 = Instant::now();
        let f = Fake::online().with(YOUTUBE_URL, vec![Reply::Answered(204)]);
        let mut results = run(&f, &[Target::Youtube], t0);
        results[0].functional_confirmed = true; // только в тесте: искусственный источник
        let mut store = ObservationStore::default();
        let ticket = store.begin();
        store.record(ticket, &results);
        let soon = t0 + Duration::from_secs(60);
        assert_eq!(
            store.fresh(soon, None).get(Coverage::Youtube),
            Reachability::Reachable
        );
        assert_eq!(
            store.fresh(soon, None).get(Coverage::Discord),
            Reachability::Unchecked
        );
        let late = t0 + FRESH_FOR + Duration::from_secs(1);
        assert_eq!(
            store.fresh(late, None).get(Coverage::Youtube),
            Reachability::Unchecked
        );
        assert_eq!(
            store.evidence(Coverage::Youtube, None, late),
            Evidence::Stale
        );
    }

    #[test]
    fn stale_and_missing_results_are_not_checked() {
        let t0 = Instant::now();
        let f = Fake::online().with(YOUTUBE_URL, vec![Reply::Answered(204)]);
        let store = stored(&f, &[Target::Youtube], t0);
        let late = t0 + FRESH_FOR + Duration::from_secs(1);
        assert_eq!(
            store.evidence(Coverage::Youtube, None, late),
            Evidence::Stale
        );
        assert_eq!(
            store.evidence(Coverage::Discord, None, t0),
            Evidence::NotChecked
        );
        // Время «до» записи тоже не считается актуальным.
        if let Some(before) = t0.checked_sub(Duration::from_secs(1)) {
            assert_eq!(
                store.evidence(Coverage::Youtube, None, before),
                Evidence::Stale
            );
        }
    }

    #[test]
    fn result_for_site_a_does_not_describe_site_b() {
        let t0 = Instant::now();
        let a = "https://a.example.com/";
        let f = Fake::online().with(a, vec![Reply::Answered(200)]);
        let store = stored(&f, &[Target::Site(a.into())], t0);
        assert!(matches!(
            store.evidence(Coverage::Sites, Some(a), t0),
            Evidence::Insufficient(..)
        ));
        assert_eq!(
            store.evidence(Coverage::Sites, Some("https://b.example.com/"), t0),
            Evidence::NotChecked
        );
        assert_eq!(
            store.evidence(Coverage::Sites, None, t0),
            Evidence::NotChecked
        );
    }

    #[test]
    fn a_late_older_check_does_not_overwrite_a_newer_one() {
        let t0 = Instant::now();
        let ok = Fake::online().with(DISCORD_URL, vec![Reply::Answered(200)]);
        let bad = Fake::online().with(DISCORD_URL, vec![Reply::Timeout]);
        let mut store = ObservationStore::default();
        let old = store.begin();
        let new = store.begin();
        store.record(
            new,
            &run(&bad, &[Target::Discord], t0 + Duration::from_secs(5)),
        );
        // Старая проверка закончилась позже и пытается записаться.
        store.record(old, &run(&ok, &[Target::Discord], t0));
        match store.evidence(Coverage::Discord, None, t0 + Duration::from_secs(6)) {
            Evidence::Insufficient(r, _) => assert_eq!(r, Reachability::Unreachable),
            other => panic!("{other:?}"),
        }
    }
}
