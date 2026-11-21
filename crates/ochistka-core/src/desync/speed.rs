//! Замер скорости открытия сайтов до и после включения обхода (во время пробного запуска).
//!
//! Только измерение и сравнение; сеть выполняет подставляемый `TimedFetch` (в `ochistka-platform`).
//! Три адреса запрашиваются ОДНОВРЕМЕННО, а общий лимит жёсткий: замер возвращается не позже
//! лимита, даже если запрос завис. Результат НЕ доказывает работу обхода у провайдера и
//! `functional_confirmed` не меняет.

use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use super::probe::{CONTROL_URL, DISCORD_URL, YOUTUBE_URL};

/// Пределы замера: на один запрос и на весь замер.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub per_request: Duration,
    pub total: Duration,
}

/// Замер без обхода идёт параллельно с подготовкой и не должен задерживать включение:
/// запрос и весь замер ≤ 3 с (закрытый сайт без обхода и так «не открылся»).
pub const BASELINE: Limits = Limits {
    per_request: Duration::from_secs(3),
    total: Duration::from_secs(3),
};
/// Замер с обходом (в фоне): запрос ≤ 5 с, весь замер ≤ 10 с.
pub const AFTER: Limits = Limits {
    per_request: Duration::from_secs(5),
    total: Duration::from_secs(10),
};

/// Обход «медленнее»: в 1,5 раза (и не меньше порога шума 1 с) либо на 5 с и больше — по любому
/// показателю любого адреса, который без обхода открывался. Адрес, переставший открываться, — тоже.
/// Откат делается только если это подтвердил повторный замер через 5 с на том же адресе.
pub const SLOWER_FACTOR: f64 = 1.5;
pub const SLOWER_BY: Duration = Duration::from_secs(5);
/// Разницу меньше этого порога не считаем замедлением по кратности (шум на миллисекундах).
pub const NOISE_FLOOR: Duration = Duration::from_secs(1);
/// Через сколько секунд подтверждается замедление повторным замером на том же адресе.
pub const RECHECK_AFTER: Duration = Duration::from_secs(5);

pub const URLS: [&str; 3] = [YOUTUBE_URL, DISCORD_URL, CONTROL_URL];

/// Сообщения пользователю: простые слова, без технических терминов.
pub const NOT_SLOWER_MESSAGE: &str =
    "Сайты отвечают быстрее или так же; это не доказывает работу у провайдера";
pub const NO_COMPARE_MESSAGE: &str = "Без обхода сайты не открываются, сравнить нельзя. Обход включён на 120 секунд — проверьте сайты сами";
pub const NO_INTERNET_MESSAGE: &str = "Нет подключения к интернету";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Timeout,
    Failed,
}

/// Результат одного запроса: время до первого ответа и до полной загрузки.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    pub outcome: Outcome,
    pub first_byte: Option<Duration>,
    pub total: Option<Duration>,
    /// Сколько времени запрос потратил.
    pub spent: Duration,
}

impl Timing {
    pub fn done(first_byte: Duration, total: Duration) -> Self {
        Self {
            outcome: Outcome::Done,
            first_byte: Some(first_byte),
            total: Some(total),
            spent: total,
        }
    }
    pub fn timeout(limit: Duration) -> Self {
        Self {
            outcome: Outcome::Timeout,
            first_byte: None,
            total: None,
            spent: limit,
        }
    }
    pub fn failed(spent: Duration) -> Self {
        Self {
            outcome: Outcome::Failed,
            first_byte: None,
            total: None,
            spent,
        }
    }
    pub fn opened(&self) -> bool {
        self.outcome == Outcome::Done
    }
}

/// Загрузка адреса с ограничением по времени (без системного прокси). Реализация — в платформе.
pub trait TimedFetch: Send + Sync {
    fn fetch(&self, url: &str, limit: Duration) -> Timing;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measurement {
    pub entries: Vec<(String, Timing)>,
}

impl Measurement {
    /// Открылся ли хоть один адрес.
    pub fn any_opened(&self) -> bool {
        self.entries.iter().any(|(_, t)| t.opened())
    }
}

/// Одновременный замер всех адресов. Возвращается не позже `limits.total`: адрес, не успевший
/// ответить, считается не открывшимся (зависший запрос завершится сам по своему лимиту).
pub fn measure(fetch: Arc<dyn TimedFetch>, urls: &[&str], limits: Limits) -> Measurement {
    let (tx, rx) = mpsc::channel();
    for (index, url) in urls.iter().enumerate() {
        let (fetch, tx, url) = (fetch.clone(), tx.clone(), (*url).to_string());
        std::thread::spawn(move || {
            let timing = fetch.fetch(&url, limits.per_request);
            let _ = tx.send((index, timing));
        });
    }
    drop(tx);
    let deadline = Instant::now() + limits.total;
    let mut got: Vec<Option<Timing>> = vec![None; urls.len()];
    let mut received = 0;
    while received < urls.len() {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok((index, timing)) => {
                got[index] = Some(timing);
                received += 1;
            }
            Err(_) => break,
        }
    }
    Measurement {
        entries: urls
            .iter()
            .zip(got)
            .map(|(url, t)| {
                (
                    (*url).to_string(),
                    t.unwrap_or(Timing::timeout(limits.total)),
                )
            })
            .collect(),
    }
}

/// Что именно не так с адресом при включённом обходе.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindingKind {
    /// Открывается, но медленнее: (было, стало), в миллисекундах.
    Slower { before: Duration, after: Duration },
    /// Перестал открываться.
    NotOpening,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub url: String,
    pub kind: FindingKind,
}

/// Понятное имя адреса для сообщения.
pub fn site_name(url: &str) -> &'static str {
    if url == YOUTUBE_URL {
        "YouTube"
    } else if url == DISCORD_URL {
        "Discord"
    } else {
        "проверочный сайт"
    }
}

fn seconds(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f64()).replace('.', ",")
}

/// Причина отката простыми словами: «Сайты с обходом открываются медленнее (YouTube: 3,2 с вместо
/// 0,8 с). Обход выключен».
pub fn rollback_message(findings: &[Finding]) -> String {
    let slower: Vec<String> = findings
        .iter()
        .filter_map(|f| match &f.kind {
            FindingKind::Slower { before, after } => Some(format!(
                "{}: {} с вместо {} с",
                site_name(&f.url),
                seconds(*after),
                seconds(*before)
            )),
            FindingKind::NotOpening => None,
        })
        .collect();
    let broken: Vec<&str> = findings
        .iter()
        .filter(|f| f.kind == FindingKind::NotOpening)
        .map(|f| site_name(&f.url))
        .collect();
    let mut parts = Vec::new();
    if !slower.is_empty() {
        parts.push(format!(
            "Сайты с обходом открываются медленнее ({})",
            slower.join("; ")
        ));
    }
    if !broken.is_empty() {
        parts.push(format!("С обходом не открывается: {}", broken.join(", ")));
    }
    format!("{}. Обход выключен", parts.join(". "))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Не медленнее (быстрее, так же).
    NotSlower,
    /// Медленнее или перестал открываться; находки по адресам.
    Slower(Vec<Finding>),
    /// Без обхода не открылся ни один адрес: сравнить нечем, откат не нужен.
    NothingToCompare { opened_with_bypass: bool },
}

fn slower(base: Duration, after: Duration) -> bool {
    if after <= base {
        return false;
    }
    let diff = after - base;
    diff >= SLOWER_BY
        || (diff >= NOISE_FLOOR && after.as_secs_f64() > base.as_secs_f64() * SLOWER_FACTOR)
}

/// Сравнивает замер с обходом с замером без него — только по адресам, которые без обхода открылись.
pub fn compare(baseline: &Measurement, after: &Measurement) -> Verdict {
    compare_urls(baseline, after, None)
}

/// То же, но только по перечисленным адресам (повторный замер подозрительных).
pub fn compare_urls(
    baseline: &Measurement,
    after: &Measurement,
    only: Option<&[String]>,
) -> Verdict {
    if !baseline.any_opened() {
        return Verdict::NothingToCompare {
            opened_with_bypass: after.any_opened(),
        };
    }
    let mut found = Vec::new();
    for (url, base) in &baseline.entries {
        if !base.opened() || only.is_some_and(|list| !list.contains(url)) {
            continue; // без обхода не открывался: это и есть цель обхода, не сравниваем
        }
        let Some((_, now)) = after.entries.iter().find(|(u, _)| u == url) else {
            continue;
        };
        if !now.opened() {
            found.push(Finding {
                url: url.clone(),
                kind: FindingKind::NotOpening,
            });
            continue;
        }
        let (bf, bt) = (
            base.first_byte.unwrap_or_default(),
            base.total.unwrap_or_default(),
        );
        let (nf, nt) = (
            now.first_byte.unwrap_or_default(),
            now.total.unwrap_or_default(),
        );
        // В сообщении — загрузка целиком, если она медленнее, иначе первый ответ.
        if slower(bt, nt) {
            found.push(Finding {
                url: url.clone(),
                kind: FindingKind::Slower {
                    before: bt,
                    after: nt,
                },
            });
        } else if slower(bf, nf) {
            found.push(Finding {
                url: url.clone(),
                kind: FindingKind::Slower {
                    before: bf,
                    after: nf,
                },
            });
        }
    }
    if found.is_empty() {
        Verdict::NotSlower
    } else {
        Verdict::Slower(found)
    }
}

/// Адреса, которые нужно перемерить.
pub fn flagged_urls(findings: &[Finding]) -> Vec<String> {
    findings.iter().map(|f| f.url.clone()).collect()
}

/// Подставной транспорт для тестов: на каждый адрес — очередь ответов (последний повторяется).
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    pub struct Fake {
        pub map: Mutex<HashMap<String, Vec<Timing>>>,
        pub calls: Mutex<Vec<(String, Duration)>>,
    }

    impl Fake {
        pub fn new(items: &[(&str, Vec<Timing>)]) -> Self {
            Self {
                map: Mutex::new(
                    items
                        .iter()
                        .map(|(u, t)| ((*u).to_string(), t.clone()))
                        .collect(),
                ),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl TimedFetch for Fake {
        fn fetch(&self, url: &str, limit: Duration) -> Timing {
            self.calls.lock().unwrap().push((url.to_string(), limit));
            let mut map = self.map.lock().unwrap();
            let list = map.entry(url.to_string()).or_default();
            if list.len() > 1 {
                list.remove(0)
            } else {
                list.first()
                    .copied()
                    .unwrap_or(Timing::failed(Duration::ZERO))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::Fake;
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    fn ok(first: u64, total: u64) -> Timing {
        Timing::done(ms(first), ms(total))
    }
    fn all(t: Timing) -> Vec<(&'static str, Vec<Timing>)> {
        URLS.iter().map(|u| (*u, vec![t])).collect()
    }
    fn m(fake: Fake) -> Measurement {
        measure(Arc::new(fake), &URLS, BASELINE)
    }

    #[test]
    fn limits_are_three_seconds_without_the_bypass_and_five_or_ten_with_it() {
        assert_eq!(
            (BASELINE.per_request, BASELINE.total),
            (Duration::from_secs(3), Duration::from_secs(3))
        );
        assert_eq!(
            (AFTER.per_request, AFTER.total),
            (Duration::from_secs(5), Duration::from_secs(10))
        );
    }

    #[test]
    fn slower_by_factor_by_five_seconds_but_not_by_noise() {
        assert!(slower(ms(1000), ms(2100)));
        assert!(!slower(ms(1000), ms(1500)));
        assert!(!slower(ms(400), ms(650)), "разброс 400 → 650 мс не откат");
        assert!(!slower(ms(20), ms(40)));
        assert!(slower(ms(10_000), ms(15_000)));
        assert!(!slower(ms(10_000), ms(14_900)));
        assert!(!slower(ms(2000), ms(1000)));
    }

    /// Транспорт с настоящей задержкой: проверяет параллельность и жёсткий лимит.
    struct Sleepy {
        delay: Duration,
        started: AtomicU32,
        limit_seen: std::sync::Mutex<Vec<Duration>>,
    }
    impl TimedFetch for Sleepy {
        fn fetch(&self, _url: &str, limit: Duration) -> Timing {
            self.started.fetch_add(1, Ordering::SeqCst);
            self.limit_seen.lock().unwrap().push(limit);
            std::thread::sleep(self.delay);
            Timing::done(self.delay, self.delay)
        }
    }

    #[test]
    fn the_three_addresses_are_requested_at_the_same_time() {
        let f = Arc::new(Sleepy {
            delay: ms(300),
            started: AtomicU32::new(0),
            limit_seen: Default::default(),
        });
        let started = Instant::now();
        let result = measure(f.clone(), &URLS, BASELINE);
        // Три запроса по 300 мс одновременно ≈ 300 мс, а не 900.
        assert!(started.elapsed() < ms(800), "{:?}", started.elapsed());
        assert!(result.entries.iter().all(|(_, t)| t.opened()));
        assert_eq!(f.started.load(Ordering::SeqCst), 3);
        assert!(f
            .limit_seen
            .lock()
            .unwrap()
            .iter()
            .all(|l| *l == BASELINE.per_request));
    }

    #[test]
    fn the_total_limit_is_hard_even_when_a_request_hangs() {
        let f = Arc::new(Sleepy {
            delay: Duration::from_secs(4),
            started: AtomicU32::new(0),
            limit_seen: Default::default(),
        });
        let limits = Limits {
            per_request: ms(150),
            total: ms(200),
        };
        let started = Instant::now();
        let result = measure(f, &URLS, limits);
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "замер не должен ждать зависший запрос"
        );
        assert!(result
            .entries
            .iter()
            .all(|(_, t)| t.outcome == Outcome::Timeout));
    }

    #[test]
    fn slower_with_the_bypass_is_reported_with_reasons() {
        let base = m(Fake::new(&all(ok(300, 900))));
        let after = m(Fake::new(&all(ok(300, 2500))));
        match compare(&base, &after) {
            Verdict::Slower(r) => assert!(matches!(r[0].kind, FindingKind::Slower { .. }), "{r:?}"),
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn same_or_faster_is_not_slower() {
        let base = m(Fake::new(&all(ok(300, 900))));
        assert_eq!(
            compare(&base, &m(Fake::new(&all(ok(300, 900))))),
            Verdict::NotSlower
        );
        assert_eq!(
            compare(&base, &m(Fake::new(&all(ok(100, 400))))),
            Verdict::NotSlower
        );
    }

    #[test]
    fn an_address_that_stops_opening_with_the_bypass_counts_as_slower() {
        let base = m(Fake::new(&all(ok(300, 900))));
        let after = m(Fake::new(&all(Timing::timeout(BASELINE.per_request))));
        assert!(matches!(compare(&base, &after), Verdict::Slower(_)));
    }

    #[test]
    fn only_addresses_that_opened_without_the_bypass_are_compared() {
        // Discord без обхода не открывался (это и есть цель обхода): с обходом открылся — не замедление,
        // и даже медленно открывшийся не откатывает.
        let mut items = all(ok(300, 900));
        items[1].1 = vec![Timing::timeout(BASELINE.per_request)];
        let base = m(Fake::new(&items));
        let mut after_items = all(ok(300, 900));
        after_items[1].1 = vec![ok(4000, 4900)];
        assert_eq!(
            compare(&base, &m(Fake::new(&after_items))),
            Verdict::NotSlower
        );
    }

    #[test]
    fn when_nothing_opened_without_the_bypass_there_is_nothing_to_compare() {
        let dead = m(Fake::new(&all(Timing::failed(ms(10)))));
        let alive = m(Fake::new(&all(ok(300, 900))));
        assert_eq!(
            compare(&dead, &alive),
            Verdict::NothingToCompare {
                opened_with_bypass: true
            }
        );
        assert_eq!(
            compare(&dead, &dead),
            Verdict::NothingToCompare {
                opened_with_bypass: false
            }
        );
    }

    #[test]
    fn the_rollback_reason_names_the_site_and_the_times_in_plain_words() {
        let f = vec![
            Finding {
                url: YOUTUBE_URL.into(),
                kind: FindingKind::Slower {
                    before: ms(800),
                    after: ms(3200),
                },
            },
            Finding {
                url: DISCORD_URL.into(),
                kind: FindingKind::NotOpening,
            },
        ];
        let text = rollback_message(&f);
        assert!(
            text.starts_with("Сайты с обходом открываются медленнее (YouTube: 3,2 с вместо 0,8 с)"),
            "{text}"
        );
        assert!(text.contains("С обходом не открывается: Discord"), "{text}");
        assert!(text.ends_with("Обход выключен"));
        let t = text.to_lowercase();
        for bad in [
            "замер",
            "базов",
            "служб",
            "порт",
            "драйвер",
            "пресет",
            "движок",
            "sha",
        ] {
            assert!(!t.contains(bad), "{text}: {bad}");
        }
    }

    #[test]
    fn user_messages_contain_no_technical_words() {
        for text in [NOT_SLOWER_MESSAGE, NO_COMPARE_MESSAGE, NO_INTERNET_MESSAGE] {
            let t = text.to_lowercase();
            for bad in [
                "замер",
                "базов",
                "служб",
                "порт",
                "драйвер",
                "таймаут",
                "timeout",
                "dns",
                "tls",
            ] {
                assert!(!t.contains(bad), "{text}: {bad}");
            }
        }
        assert!(!NO_INTERNET_MESSAGE.contains("проверьте интернет"));
    }
}
