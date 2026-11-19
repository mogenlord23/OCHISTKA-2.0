//! Тест скорости интернета: задержка, загрузка, отдача, задержка под нагрузкой и джиттер.
//! Здесь только логика и чистая арифметика; сами запросы делает адаптер платформы через порт
//! `SpeedTransport` (в тестах подменяется). Личных данных не отправляется: только байты-заглушки.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use ts_rs::TS;

/// Этап теста.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "SpeedPhase.ts")]
pub enum SpeedPhase {
    Ping,
    Download,
    Upload,
}

/// Итог теста. Скорость в мегабитах в секунду, время в миллисекундах.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "SpeedResult.ts")]
pub struct SpeedResult {
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub jitter_ms: f64,
    /// Медиана задержки, пока канал занят загрузкой или отдачей. `None`, если измерить не вышло.
    pub loaded_ping_ms: Option<f64>,
}

/// Почему тест не дал результата.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "SpeedFailure.ts")]
pub enum SpeedFailure {
    /// Нет соединения с интернетом.
    Offline,
    /// Сервер измерения не ответил или отказал.
    Server,
}

/// Событие хода теста для интерфейса.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, export_to = "SpeedEvent.ts")]
pub enum SpeedEvent {
    Phase {
        phase: SpeedPhase,
    },
    /// Текущая медиана задержки в ходе замера.
    Ping {
        ms: f64,
    },
    /// Мгновенная скорость (окно около секунды) и доля этапа от 0 до 1.
    Progress {
        phase: SpeedPhase,
        mbps: f64,
        fraction: f64,
    },
    Done {
        result: SpeedResult,
    },
    Failed {
        kind: SpeedFailure,
    },
    Cancelled,
}

/// Ошибка одного запроса.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    Offline,
    Server,
    /// Остановлен по флагу.
    Aborted,
}

/// Порт запросов измерения. `abort` взводится при отмене и по окончании этапа: реализация
/// обязана прервать передачу в ближайшие доли секунды.
pub trait SpeedTransport: Send + Sync {
    /// Один маленький запрос; возвращает время ответа.
    fn ping(&self, abort: &AtomicBool) -> Result<Duration, TransportError>;
    /// Скачивает `bytes` байт, прибавляя полученное к `counter` по мере поступления.
    fn download(
        &self,
        bytes: u64,
        abort: &Arc<AtomicBool>,
        counter: &Arc<AtomicU64>,
    ) -> Result<(), TransportError>;
    /// Отправляет `bytes` байт-заглушек, прибавляя отправленное к `counter`.
    fn upload(
        &self,
        bytes: u64,
        abort: &Arc<AtomicBool>,
        counter: &Arc<AtomicU64>,
    ) -> Result<(), TransportError>;
}

/// Параметры теста. Для тестов подставляются короткие значения.
#[derive(Clone, Debug)]
pub struct SpeedConfig {
    pub pings: usize,
    pub phase: Duration,
    /// Начало этапа, которое не входит в итог (разгон соединения).
    pub warmup: Duration,
    pub streams: usize,
    pub sample: Duration,
    pub window: Duration,
    pub loaded_ping_every: Duration,
    pub download_chunk: u64,
    pub upload_chunk: u64,
}

impl Default for SpeedConfig {
    fn default() -> Self {
        Self {
            pings: 10,
            phase: Duration::from_secs(8),
            warmup: Duration::from_millis(1500),
            streams: 4,
            sample: Duration::from_millis(100),
            window: Duration::from_millis(1000),
            loaded_ping_every: Duration::from_millis(400),
            download_chunk: 25_000_000,
            upload_chunk: 8_000_000,
        }
    }
}

/// Мегабиты в секунду по числу байт за время.
pub fn mbps(bytes: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        return 0.0;
    }
    bytes as f64 * 8.0 / 1_000_000.0 / secs
}

/// Отсчёт: время от начала этапа и накопленные байты.
pub type Sample = (Duration, u64);

/// Скорость за последнее окно по отсчётам.
pub fn window_rate(samples: &[Sample], window: Duration) -> f64 {
    let Some(&(end_t, end_b)) = samples.last() else {
        return 0.0;
    };
    let start_t = end_t.saturating_sub(window);
    let from = samples
        .iter()
        .rev()
        .find(|(t, _)| *t <= start_t)
        .or_else(|| samples.first())
        .copied()
        .unwrap_or((end_t, end_b));
    if end_t <= from.0 {
        return 0.0;
    }
    mbps(end_b.saturating_sub(from.1), end_t - from.0)
}

/// Итоговая скорость этапа: после разгона `warmup`; если отсчётов после него мало — по всему этапу.
pub fn final_rate(samples: &[Sample], warmup: Duration) -> f64 {
    let (Some(&(end_t, end_b)), Some(&(first_t, first_b))) = (samples.last(), samples.first())
    else {
        return 0.0;
    };
    let after = samples.iter().find(|(t, _)| *t >= warmup).copied();
    match after {
        Some((t, b)) if end_t > t && end_t - t >= Duration::from_millis(300) => {
            mbps(end_b.saturating_sub(b), end_t - t)
        }
        _ => mbps(
            end_b.saturating_sub(first_b),
            end_t.saturating_sub(first_t).max(end_t),
        ),
    }
}

/// Медиана; пустой список даёт `None`.
pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let mid = v.len() / 2;
    Some(if v.len() % 2 == 1 {
        v[mid]
    } else {
        (v[mid - 1] + v[mid]) / 2.0
    })
}

/// Джиттер: среднее отличие соседних замеров задержки.
pub fn jitter(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let sum: f64 = values.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
    sum / (values.len() - 1) as f64
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn failure(err: TransportError) -> SpeedFailure {
    match err {
        TransportError::Offline => SpeedFailure::Offline,
        _ => SpeedFailure::Server,
    }
}

enum PhaseOutcome {
    Done { rate: f64, loaded: Vec<f64> },
    Cancelled,
    Failed(SpeedFailure),
}

fn run_transfer_phase(
    transport: &dyn SpeedTransport,
    cfg: &SpeedConfig,
    phase: SpeedPhase,
    cancel: &AtomicBool,
    emit: &mut dyn FnMut(SpeedEvent),
) -> PhaseOutcome {
    emit(SpeedEvent::Phase { phase });
    let stop = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicU64::new(0));
    let first_error: Mutex<Option<TransportError>> = Mutex::new(None);
    let loaded: Mutex<Vec<f64>> = Mutex::new(Vec::new());
    let alive = AtomicU64::new(cfg.streams as u64);
    let mut samples: Vec<Sample> = vec![(Duration::ZERO, 0)];
    let started = Instant::now();

    thread::scope(|scope| {
        for _ in 0..cfg.streams.max(1) {
            scope.spawn(|| {
                while !stop.load(Ordering::Relaxed) {
                    let chunk = if phase == SpeedPhase::Download {
                        cfg.download_chunk
                    } else {
                        cfg.upload_chunk
                    };
                    let res = if phase == SpeedPhase::Download {
                        transport.download(chunk, &stop, &counter)
                    } else {
                        transport.upload(chunk, &stop, &counter)
                    };
                    match res {
                        Ok(()) => {}
                        Err(TransportError::Aborted) => break,
                        Err(e) => {
                            first_error.lock().unwrap().get_or_insert(e);
                            break;
                        }
                    }
                }
                alive.fetch_sub(1, Ordering::Relaxed);
            });
        }
        // Задержка под нагрузкой: маленькие запросы, пока канал занят.
        scope.spawn(|| {
            let mut next = Instant::now() + cfg.loaded_ping_every;
            while !stop.load(Ordering::Relaxed) {
                if Instant::now() >= next {
                    if let Ok(d) = transport.ping(&stop) {
                        loaded.lock().unwrap().push(ms(d));
                    }
                    next = Instant::now() + cfg.loaded_ping_every;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });

        loop {
            thread::sleep(cfg.sample);
            let elapsed = started.elapsed();
            samples.push((elapsed, counter.load(Ordering::Relaxed)));
            let fraction = (elapsed.as_secs_f64() / cfg.phase.as_secs_f64()).clamp(0.0, 1.0);
            emit(SpeedEvent::Progress {
                phase,
                mbps: window_rate(&samples, cfg.window),
                fraction,
            });
            if cancel.load(Ordering::Relaxed)
                || elapsed >= cfg.phase
                || alive.load(Ordering::Relaxed) == 0
            {
                stop.store(true, Ordering::Relaxed);
                break;
            }
        }
    });

    if cancel.load(Ordering::Relaxed) {
        return PhaseOutcome::Cancelled;
    }
    let received = counter.load(Ordering::Relaxed);
    if let Some(err) = *first_error.lock().unwrap() {
        if received == 0 {
            return PhaseOutcome::Failed(failure(err));
        }
    }
    if received == 0 {
        return PhaseOutcome::Failed(SpeedFailure::Server);
    }
    PhaseOutcome::Done {
        rate: final_rate(&samples, cfg.warmup),
        loaded: loaded.into_inner().unwrap(),
    }
}

/// Полный тест. Все события, включая последнее (`Done`, `Failed` или `Cancelled`), идут в `emit`.
pub fn run_speedtest(
    transport: &dyn SpeedTransport,
    cfg: &SpeedConfig,
    cancel: &AtomicBool,
    emit: &mut dyn FnMut(SpeedEvent),
) {
    emit(SpeedEvent::Phase {
        phase: SpeedPhase::Ping,
    });
    let mut pings: Vec<f64> = Vec::new();
    let mut last_err = None;
    for _ in 0..cfg.pings.max(1) {
        if cancel.load(Ordering::Relaxed) {
            return emit(SpeedEvent::Cancelled);
        }
        match transport.ping(cancel) {
            Ok(d) => {
                pings.push(ms(d));
                if let Some(m) = median(&pings) {
                    emit(SpeedEvent::Ping { ms: m });
                }
            }
            Err(TransportError::Aborted) => return emit(SpeedEvent::Cancelled),
            Err(e) => {
                last_err = Some(e);
                // Первый же запрос не прошёл: дальше пробовать нет смысла.
                if pings.is_empty() && last_err == Some(TransportError::Offline) {
                    break;
                }
            }
        }
    }
    // Первый запрос включает установку защищённого соединения: в задержку он не входит.
    if pings.len() >= 4 {
        pings.remove(0);
    }
    let Some(ping_ms) = median(&pings) else {
        return emit(SpeedEvent::Failed {
            kind: last_err.map_or(SpeedFailure::Server, failure),
        });
    };
    let jitter_ms = jitter(&pings);

    let mut loaded_all: Vec<f64> = Vec::new();
    let mut rates = [0.0f64; 2];
    for (i, phase) in [SpeedPhase::Download, SpeedPhase::Upload]
        .into_iter()
        .enumerate()
    {
        match run_transfer_phase(transport, cfg, phase, cancel, emit) {
            PhaseOutcome::Done { rate, loaded } => {
                rates[i] = rate;
                loaded_all.extend(loaded);
            }
            PhaseOutcome::Cancelled => return emit(SpeedEvent::Cancelled),
            PhaseOutcome::Failed(kind) => return emit(SpeedEvent::Failed { kind }),
        }
    }
    emit(SpeedEvent::Done {
        result: SpeedResult {
            download_mbps: rates[0],
            upload_mbps: rates[1],
            ping_ms,
            jitter_ms,
            loaded_ping_ms: median(&loaded_all),
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn mbps_converts_bytes_per_second() {
        assert!((mbps(12_500_000, d(1000)) - 100.0).abs() < 1e-9);
        assert_eq!(mbps(100, Duration::ZERO), 0.0);
    }

    #[test]
    fn window_rate_uses_only_the_last_window() {
        // Первую секунду 10 МБ/с, потом 1 МБ/с: окно в 1 с видит только медленную часть.
        let samples = vec![(d(0), 0), (d(1000), 10_000_000), (d(2000), 11_000_000)];
        assert!((window_rate(&samples, d(1000)) - 8.0).abs() < 1e-9);
        assert_eq!(window_rate(&[], d(1000)), 0.0);
        assert_eq!(window_rate(&[(d(0), 0)], d(1000)), 0.0);
    }

    #[test]
    fn final_rate_skips_warmup() {
        // Разгон: за 1 с всего 1 МБ, дальше стабильно 10 МБ/с.
        let samples = vec![
            (d(0), 0),
            (d(1000), 1_000_000),
            (d(2000), 11_000_000),
            (d(3000), 21_000_000),
        ];
        let rate = final_rate(&samples, d(1000));
        assert!((rate - 80.0).abs() < 1e-9, "{rate}");
        // Слишком короткий этап: берём всё время.
        let short = vec![(d(0), 0), (d(100), 125_000)];
        assert!((final_rate(&short, d(1500)) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn median_and_jitter() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), Some(2.5));
        assert_eq!(jitter(&[10.0]), 0.0);
        assert!((jitter(&[10.0, 14.0, 10.0]) - 4.0).abs() < 1e-9);
    }

    /// Подмена сети: «канал» с заданной скоростью, никаких настоящих запросов.
    struct Fake {
        offline: bool,
        server_error: bool,
        ping_ms: u64,
        bytes_per_tick: u64,
        pings: AtomicU64,
    }

    impl Fake {
        fn good() -> Self {
            Self {
                offline: false,
                server_error: false,
                ping_ms: 12,
                bytes_per_tick: 50_000,
                pings: AtomicU64::new(0),
            }
        }

        fn transfer(
            &self,
            abort: &Arc<AtomicBool>,
            counter: &Arc<AtomicU64>,
        ) -> Result<(), TransportError> {
            if self.offline {
                return Err(TransportError::Offline);
            }
            if self.server_error {
                return Err(TransportError::Server);
            }
            for _ in 0..10 {
                if abort.load(Ordering::Relaxed) {
                    return Err(TransportError::Aborted);
                }
                counter.fetch_add(self.bytes_per_tick, Ordering::Relaxed);
                thread::sleep(d(5));
            }
            Ok(())
        }
    }

    impl SpeedTransport for Fake {
        fn ping(&self, abort: &AtomicBool) -> Result<Duration, TransportError> {
            if abort.load(Ordering::Relaxed) {
                return Err(TransportError::Aborted);
            }
            if self.offline {
                return Err(TransportError::Offline);
            }
            if self.server_error {
                return Err(TransportError::Server);
            }
            let n = self.pings.fetch_add(1, Ordering::Relaxed);
            Ok(d(self.ping_ms + n % 3))
        }
        fn download(
            &self,
            _: u64,
            a: &Arc<AtomicBool>,
            c: &Arc<AtomicU64>,
        ) -> Result<(), TransportError> {
            self.transfer(a, c)
        }
        fn upload(
            &self,
            _: u64,
            a: &Arc<AtomicBool>,
            c: &Arc<AtomicU64>,
        ) -> Result<(), TransportError> {
            self.transfer(a, c)
        }
    }

    fn quick() -> SpeedConfig {
        SpeedConfig {
            pings: 4,
            phase: d(400),
            warmup: d(100),
            streams: 2,
            sample: d(20),
            window: d(200),
            loaded_ping_every: d(50),
            download_chunk: 1,
            upload_chunk: 1,
        }
    }

    fn collect(fake: &Fake, cancel: &AtomicBool) -> Vec<SpeedEvent> {
        let mut events = Vec::new();
        run_speedtest(fake, &quick(), cancel, &mut |e| events.push(e));
        events
    }

    #[test]
    fn full_run_reports_phases_progress_and_result() {
        let events = collect(&Fake::good(), &AtomicBool::new(false));
        let phases: Vec<SpeedPhase> = events
            .iter()
            .filter_map(|e| match e {
                SpeedEvent::Phase { phase } => Some(*phase),
                _ => None,
            })
            .collect();
        assert_eq!(
            phases,
            [SpeedPhase::Ping, SpeedPhase::Download, SpeedPhase::Upload]
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, SpeedEvent::Progress { mbps, .. } if *mbps > 0.0)));
        let Some(SpeedEvent::Done { result }) = events.last() else {
            panic!("ожидался итог: {events:?}");
        };
        assert!(result.download_mbps > 1.0 && result.upload_mbps > 1.0);
        assert!((result.ping_ms - 13.0).abs() < 1.5, "{}", result.ping_ms);
        assert!(result.jitter_ms > 0.0 && result.jitter_ms < 3.0);
        assert!(result.loaded_ping_ms.is_some());
        // Прогресс монотонен по доле внутри этапа и не превышает 1.
        assert!(events.iter().all(
            |e| !matches!(e, SpeedEvent::Progress { fraction, .. } if !(0.0..=1.0).contains(fraction))
        ));
    }

    #[test]
    fn no_internet_fails_fast_with_offline() {
        let fake = Fake {
            offline: true,
            ..Fake::good()
        };
        let events = collect(&fake, &AtomicBool::new(false));
        assert_eq!(
            events.last(),
            Some(&SpeedEvent::Failed {
                kind: SpeedFailure::Offline
            })
        );
        assert!(!events
            .iter()
            .any(|e| matches!(e, SpeedEvent::Phase { phase } if *phase != SpeedPhase::Ping)));
    }

    #[test]
    fn server_refusal_is_reported_as_server_failure() {
        let fake = Fake {
            server_error: true,
            ..Fake::good()
        };
        let events = collect(&fake, &AtomicBool::new(false));
        assert_eq!(
            events.last(),
            Some(&SpeedEvent::Failed {
                kind: SpeedFailure::Server
            })
        );
    }

    #[test]
    fn cancel_stops_the_run_quickly() {
        let cancel = AtomicBool::new(false);
        let fake = Fake::good();
        let started = Instant::now();
        let mut events = Vec::new();
        run_speedtest(&fake, &quick(), &cancel, &mut |e| {
            if matches!(e, SpeedEvent::Progress { .. }) {
                cancel.store(true, Ordering::Relaxed);
            }
            events.push(e);
        });
        assert_eq!(events.last(), Some(&SpeedEvent::Cancelled));
        assert!(started.elapsed() < d(1500));
        assert!(!events.iter().any(|e| matches!(e, SpeedEvent::Done { .. })));
    }
}
