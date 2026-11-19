//! Проверка задержки серверов: время TCP-подключения. Параллельно, но не больше `PARALLEL`
//! потоков, у каждого попытки свой таймаут.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Одновременных проверок не больше.
pub const PARALLEL: usize = 16;
pub const TIMEOUT: Duration = Duration::from_millis(3000);

/// Подключение для замера. Реализация (std::net) — в адаптере платформы.
pub trait Dialer: Sync {
    fn connect(&self, host: &str, port: u16, timeout: Duration) -> Result<(), String>;
}

/// Замер: (id, миллисекунды или `None`, если сервер не ответил).
pub fn measure(
    dialer: &dyn Dialer,
    targets: &[(String, String, u16)],
    parallel: usize,
    timeout: Duration,
) -> Vec<(String, Option<u32>)> {
    let results = Mutex::new(vec![None; targets.len()]);
    let next = AtomicUsize::new(0);
    let workers = parallel.clamp(1, PARALLEL).min(targets.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some((_, host, port)) = targets.get(i) else {
                    break;
                };
                let started = Instant::now();
                let ms = dialer
                    .connect(host, *port, timeout)
                    .ok()
                    .map(|()| (started.elapsed().as_millis().min(u32::MAX as u128) as u32).max(1));
                results.lock().unwrap()[i] = ms;
            });
        }
    });
    let results = results.into_inner().unwrap();
    targets
        .iter()
        .zip(results)
        .map(|((id, _, _), ms)| (id.clone(), ms))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct Fake {
        active: AtomicUsize,
        peak: AtomicUsize,
    }

    impl Dialer for Fake {
        fn connect(&self, host: &str, _: u16, _: Duration) -> Result<(), String> {
            let now = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(5));
            self.active.fetch_sub(1, Ordering::SeqCst);
            if host.starts_with("bad") {
                Err("нет ответа".into())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn results_keep_order_and_failures_are_none() {
        let fake = Fake {
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        };
        let targets: Vec<_> = (0..40)
            .map(|i| {
                (
                    format!("id{i}"),
                    if i % 7 == 0 {
                        format!("bad{i}")
                    } else {
                        format!("ok{i}")
                    },
                    443,
                )
            })
            .collect();
        let out = measure(&fake, &targets, 100, TIMEOUT);
        assert_eq!(out.len(), 40);
        assert_eq!(out[0], ("id0".to_string(), None));
        assert!(out[1].1.is_some());
        assert_eq!(out[39].0, "id39");
        assert!(
            fake.peak.load(Ordering::SeqCst) <= PARALLEL,
            "параллелизм ограничен"
        );
    }

    #[test]
    fn empty_list_is_fine() {
        let fake = Fake {
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        };
        assert!(measure(&fake, &[], 4, TIMEOUT).is_empty());
    }
}
