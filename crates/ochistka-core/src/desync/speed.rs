//! Замер скорости открытия сайтов до и после включения обхода (во время пробного запуска).
//!
//! Только измерение и сравнение; сеть выполняет подставляемый `TimedFetch` (в `ochistka-platform`).
//! Результат НЕ доказывает работу обхода у провайдера и `functional_confirmed` не меняет.

use std::time::Duration;

use super::probe::{CONTROL_URL, DISCORD_URL, YOUTUBE_URL};

/// Предел одного запроса и всего замера.
pub const REQUEST_LIMIT: Duration = Duration::from_secs(15);
pub const TOTAL_LIMIT: Duration = Duration::from_secs(45);
/// Обход «медленнее»: во столько раз или на столько секунд и больше (по любому адресу и показателю).
pub const SLOWER_FACTOR: f64 = 1.5;
pub const SLOWER_BY: Duration = Duration::from_secs(5);
/// Разницу меньше этого порога не считаем замедлением по кратности (шум на миллисекундах).
pub const NOISE_FLOOR: Duration = Duration::from_millis(250);

pub const URLS: [&str; 3] = [YOUTUBE_URL, DISCORD_URL, CONTROL_URL];

pub const SLOWER_MESSAGE: &str = "С обходом сайты грузятся медленнее — обход отключён";
pub const NOT_SLOWER_MESSAGE: &str =
    "Сайты отвечают быстрее или так же; это не доказывает работу у провайдера";
pub const NO_BASELINE_MESSAGE: &str =
    "Не удалось сделать базовый замер скорости: проверьте интернет. Обход не включён";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Timeout,
    Failed,
}

/// Результат одного запроса: время до первого ответа и до полной загрузки, и сколько запрос занял.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    pub outcome: Outcome,
    pub first_byte: Option<Duration>,
    pub total: Option<Duration>,
    /// Сколько времени запрос потратил (учитывается в общем лимите замера).
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
}

/// Загрузка адреса с ограничением по времени (без системного прокси). Реализация — в платформе.
pub trait TimedFetch: Send + Sync {
    fn fetch(&self, url: &str, limit: Duration) -> Timing;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measurement {
    pub entries: Vec<(String, Timing)>,
}

/// Замер всех адресов. Каждый запрос ≤ 15 с, весь замер ≤ 45 с: когда время вышло, оставшиеся
/// адреса считаются не проверенными (таймаут без запроса).
pub fn measure(fetch: &dyn TimedFetch, urls: &[&str]) -> Measurement {
    let mut left = TOTAL_LIMIT;
    let mut entries = Vec::new();
    for url in urls {
        let limit = REQUEST_LIMIT.min(left);
        let timing = if limit.is_zero() {
            Timing::timeout(Duration::ZERO)
        } else {
            let t = fetch.fetch(url, limit);
            left = left.saturating_sub(t.spent);
            t
        };
        entries.push(((*url).to_string(), timing));
    }
    Measurement { entries }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Не медленнее (быстрее, так же или сравнить нечем).
    NotSlower,
    /// Медленнее; причины по адресам.
    Slower(Vec<String>),
    /// Базового замера нет ни по одному адресу.
    NoBaseline,
}

fn slower(base: Duration, after: Duration) -> bool {
    if after <= base {
        return false;
    }
    let diff = after - base;
    diff >= SLOWER_BY
        || (diff >= NOISE_FLOOR && after.as_secs_f64() > base.as_secs_f64() * SLOWER_FACTOR)
}

pub fn has_baseline(baseline: &Measurement) -> bool {
    baseline
        .entries
        .iter()
        .any(|(_, t)| t.outcome == Outcome::Done)
}

/// Сравнивает замер с обходом с базовым. Адрес без успешного базового замера не сравнивается
/// (обход его не ухудшил); адрес, который раньше открывался, а с обходом нет, — это замедление.
pub fn compare(baseline: &Measurement, after: &Measurement) -> Verdict {
    if !has_baseline(baseline) {
        return Verdict::NoBaseline;
    }
    let mut reasons = Vec::new();
    for (url, base) in &baseline.entries {
        if base.outcome != Outcome::Done {
            continue;
        }
        let Some((_, now)) = after.entries.iter().find(|(u, _)| u == url) else {
            continue;
        };
        if now.outcome != Outcome::Done {
            reasons.push(format!("{url}: с обходом не открылся"));
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
        if slower(bf, nf) {
            reasons.push(format!(
                "{url}: первый ответ {} мс вместо {} мс",
                nf.as_millis(),
                bf.as_millis()
            ));
        }
        if slower(bt, nt) {
            reasons.push(format!(
                "{url}: загрузка {} мс вместо {} мс",
                nt.as_millis(),
                bt.as_millis()
            ));
        }
    }
    if reasons.is_empty() {
        Verdict::NotSlower
    } else {
        Verdict::Slower(reasons)
    }
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

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    fn ok(first: u64, total: u64) -> Timing {
        Timing::done(ms(first), ms(total))
    }

    fn all(t: Timing) -> Vec<(&'static str, Vec<Timing>)> {
        URLS.iter().map(|u| (*u, vec![t])).collect()
    }

    #[test]
    fn slower_by_factor_by_five_seconds_but_not_by_noise() {
        assert!(slower(ms(1000), ms(1600))); // >1,5 раза и разница ≥ 250 мс
        assert!(!slower(ms(1000), ms(1500))); // ровно 1,5 раза — не медленнее
        assert!(!slower(ms(20), ms(40))); // шум на миллисекундах
        assert!(slower(ms(10_000), ms(15_000))); // +5 с и больше
        assert!(!slower(ms(10_000), ms(14_900)));
        assert!(!slower(ms(2000), ms(1000))); // быстрее
    }

    #[test]
    fn measure_respects_the_per_request_and_total_limits() {
        let f = Fake::new(&all(Timing::timeout(REQUEST_LIMIT)));
        let m = measure(&f, &URLS);
        assert_eq!(m.entries.len(), 3);
        let calls = f.calls.lock().unwrap();
        assert!(calls.iter().all(|(_, l)| *l <= REQUEST_LIMIT));
        // Три запроса по 15 с = 45 с: больше времени нет.
        let f = Fake::new(&all(Timing::timeout(REQUEST_LIMIT)));
        let m = measure(&f, &[URLS[0], URLS[1], URLS[2], "https://extra.example/"]);
        assert_eq!(
            f.calls.lock().unwrap().len(),
            3,
            "четвёртый адрес не запрашивался: общий лимит 45 с"
        );
        assert_eq!(m.entries[3].1.outcome, Outcome::Timeout);
    }

    #[test]
    fn slower_with_the_bypass_is_reported_with_reasons() {
        let base = measure(&Fake::new(&all(ok(300, 900))), &URLS);
        let after = measure(&Fake::new(&all(ok(300, 2500))), &URLS);
        match compare(&base, &after) {
            Verdict::Slower(r) => assert!(r.iter().any(|x| x.contains("загрузка")), "{r:?}"),
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn same_or_faster_is_not_slower() {
        let base = measure(&Fake::new(&all(ok(300, 900))), &URLS);
        assert_eq!(
            compare(&base, &measure(&Fake::new(&all(ok(300, 900))), &URLS)),
            Verdict::NotSlower
        );
        assert_eq!(
            compare(&base, &measure(&Fake::new(&all(ok(100, 400))), &URLS)),
            Verdict::NotSlower
        );
    }

    #[test]
    fn a_timeout_with_the_bypass_counts_as_slower_when_the_baseline_worked() {
        let base = measure(&Fake::new(&all(ok(300, 900))), &URLS);
        let after = measure(&Fake::new(&all(Timing::timeout(REQUEST_LIMIT))), &URLS);
        assert!(matches!(compare(&base, &after), Verdict::Slower(_)));
    }

    #[test]
    fn no_baseline_at_all_is_reported_and_a_blocked_address_is_not_compared() {
        let dead = measure(&Fake::new(&all(Timing::failed(ms(10)))), &URLS);
        assert_eq!(compare(&dead, &dead), Verdict::NoBaseline);
        // Discord не открывался напрямую: с обходом открылся — это не замедление.
        let mut items = all(ok(300, 900));
        items[1].1 = vec![Timing::timeout(REQUEST_LIMIT)];
        let base = measure(&Fake::new(&items), &URLS);
        let after = measure(&Fake::new(&all(ok(300, 900))), &URLS);
        assert_eq!(compare(&base, &after), Verdict::NotSlower);
    }
}
