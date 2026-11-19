//! Запросы теста скорости. Только https к серверу измерения Cloudflare; личных данных нет:
//! в запросах ни идентификаторов, ни cookie, отправляются нули. Windows: reqwest (блокирующий
//! клиент, вызывается из рабочих потоков ядра); на других ОС теста нет.

use ochistka_core::speedtest::SpeedTransport;

/// Адреса измерения (Cloudflare Speed Test).
pub const DOWN_URL: &str = "https://speed.cloudflare.com/__down";
pub const UP_URL: &str = "https://speed.cloudflare.com/__up";

/// Транспорт текущей ОС.
pub fn current_speed_transport() -> Box<dyn SpeedTransport> {
    #[cfg(windows)]
    {
        Box::new(http::ReqwestSpeed::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(Unsupported)
    }
}

#[cfg(not(windows))]
struct Unsupported;

#[cfg(not(windows))]
impl SpeedTransport for Unsupported {
    fn ping(
        &self,
        _: &std::sync::atomic::AtomicBool,
    ) -> Result<std::time::Duration, ochistka_core::speedtest::TransportError> {
        Err(ochistka_core::speedtest::TransportError::Server)
    }
    fn download(
        &self,
        _: u64,
        _: &std::sync::Arc<std::sync::atomic::AtomicBool>,
        _: &std::sync::Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<(), ochistka_core::speedtest::TransportError> {
        Err(ochistka_core::speedtest::TransportError::Server)
    }
    fn upload(
        &self,
        _: u64,
        _: &std::sync::Arc<std::sync::atomic::AtomicBool>,
        _: &std::sync::Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<(), ochistka_core::speedtest::TransportError> {
        Err(ochistka_core::speedtest::TransportError::Server)
    }
}

#[cfg(windows)]
mod http {
    use std::io::{self, Read};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use ochistka_core::speedtest::{SpeedTransport, TransportError};
    use reqwest::blocking::{Body, Client};

    use super::{DOWN_URL, UP_URL};

    pub struct ReqwestSpeed {
        client: Option<Client>,
    }

    impl ReqwestSpeed {
        pub fn new() -> Self {
            let client = Client::builder()
                .user_agent("OCHISTKA")
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(6))
                .timeout(Duration::from_secs(30))
                .build()
                .ok();
            Self { client }
        }

        fn client(&self) -> Result<&Client, TransportError> {
            self.client.as_ref().ok_or(TransportError::Server)
        }
    }

    fn map_err(err: &reqwest::Error, abort: &AtomicBool) -> TransportError {
        if abort.load(Ordering::Relaxed) {
            TransportError::Aborted
        } else if err.is_connect() {
            TransportError::Offline
        } else {
            TransportError::Server
        }
    }

    /// Читает нули и считает отправленное; по флагу прерывает передачу.
    struct Zeros {
        left: u64,
        abort: Arc<AtomicBool>,
        counter: Arc<AtomicU64>,
    }

    impl Read for Zeros {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.abort.load(Ordering::Relaxed) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "stop"));
            }
            let n = (buf.len() as u64).min(self.left).min(32 * 1024) as usize;
            buf[..n].fill(0);
            self.left -= n as u64;
            self.counter.fetch_add(n as u64, Ordering::Relaxed);
            Ok(n)
        }
    }

    impl SpeedTransport for ReqwestSpeed {
        fn ping(&self, abort: &AtomicBool) -> Result<Duration, TransportError> {
            let started = Instant::now();
            let response = self
                .client()?
                .get(format!("{DOWN_URL}?bytes=0"))
                .send()
                .map_err(|e| map_err(&e, abort))?;
            if !response.status().is_success() {
                return Err(TransportError::Server);
            }
            let _ = response.bytes();
            Ok(started.elapsed())
        }

        fn download(
            &self,
            bytes: u64,
            abort: &Arc<AtomicBool>,
            counter: &Arc<AtomicU64>,
        ) -> Result<(), TransportError> {
            let mut response = self
                .client()?
                .get(format!("{DOWN_URL}?bytes={bytes}"))
                .send()
                .map_err(|e| map_err(&e, abort))?;
            if !response.status().is_success() {
                return Err(TransportError::Server);
            }
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                if abort.load(Ordering::Relaxed) {
                    return Err(TransportError::Aborted);
                }
                match response.read(&mut buf) {
                    Ok(0) => return Ok(()),
                    Ok(n) => {
                        counter.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    Err(_) if abort.load(Ordering::Relaxed) => return Err(TransportError::Aborted),
                    Err(_) => return Err(TransportError::Server),
                }
            }
        }

        fn upload(
            &self,
            bytes: u64,
            abort: &Arc<AtomicBool>,
            counter: &Arc<AtomicU64>,
        ) -> Result<(), TransportError> {
            let body = Body::sized(
                Zeros {
                    left: bytes,
                    abort: Arc::clone(abort),
                    counter: Arc::clone(counter),
                },
                bytes,
            );
            let response = self
                .client()?
                .post(UP_URL)
                .body(body)
                .send()
                .map_err(|e| map_err(&e, abort))?;
            if response.status().is_success() {
                Ok(())
            } else {
                Err(TransportError::Server)
            }
        }
    }
}
