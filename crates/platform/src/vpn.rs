//! Адаптеры VPN: запуск ядра sing-box как дочернего процесса, проверка порта, замер TCP,
//! загрузка подписки (HTTPS, только Windows), обновление настроек прокси. Всё, что зависит от
//! ОС, — здесь; логика подключения живёт в `ochistka_core::vpn`.

use std::fs::File;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ochistka_core::tweaks::RegistryPort;
#[cfg(windows)]
use ochistka_core::vpn::engine;
use ochistka_core::vpn::engine::{PrepareError, PrepareProgress};
use ochistka_core::vpn::fetch::{FetchResponse, HttpFetch};
use ochistka_core::vpn::latency::Dialer;
use ochistka_core::vpn::service::{CoreProcess, VpnHost};
use ochistka_core::vpn::sysproxy::{NoRefresh, ProxyRefresh};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// Имя исполняемого файла ядра.
#[cfg(windows)]
pub const CORE_EXE: &str = "sing-box.exe";
#[cfg(not(windows))]
pub const CORE_EXE: &str = "sing-box";

/// Дочерний процесс ядра.
pub struct StdCoreProcess {
    child: Child,
}

impl CoreProcess for StdCoreProcess {
    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn has_exited(&mut self) -> bool {
        !matches!(self.child.try_wait(), Ok(None))
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for StdCoreProcess {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            self.kill();
        }
    }
}

/// Запускает программу без окна консоли, вывод пишет в `log`.
pub fn spawn_logged(
    exe: &Path,
    args: &[&str],
    workdir: &Path,
    log: &Path,
) -> Result<StdCoreProcess, String> {
    let out = File::create(log).map_err(|e| format!("Не удалось создать журнал ядра: {e}"))?;
    let err = out.try_clone().map_err(|e| e.to_string())?;
    let mut command = Command::new(exe);
    command
        .args(args)
        .current_dir(workdir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err));
    hide_window(&mut command);
    let child = command
        .spawn()
        .map_err(|e| format!("Не удалось запустить VPN-ядро: {e}"))?;
    Ok(StdCoreProcess { child })
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_: &mut Command) {}

/// Запускает программу и ждёт до `limit`; возвращает код успеха и вывод (хвост).
pub fn run_with_timeout(exe: &Path, args: &[&str], limit: Duration) -> Result<(), String> {
    let mut command = Command::new(exe);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("не удалось запустить: {e}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut text = String::new();
                if let Some(mut s) = child.stderr.take() {
                    let _ = s.read_to_string(&mut text);
                }
                if let Some(mut s) = child.stdout.take() {
                    let _ = s.read_to_string(&mut text);
                }
                if status.success() {
                    return Ok(());
                }
                let chars: Vec<char> = text.trim().chars().collect();
                let tail: String = chars[chars.len().saturating_sub(400)..].iter().collect();
                return Err(tail);
            }
            Ok(None) if started.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("проверка не завершилась за отведённое время".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    }
}

/// Хозяин ядра для текущей ОС. Ядро ищется в папках `core_dirs` по порядку.
pub struct SingBoxHost {
    core_dirs: Vec<PathBuf>,
    /// Куда приложение само кладёт скачанный движок (`%LOCALAPPDATA%\OCHISTKA\vpn-core`).
    install_dir: Option<PathBuf>,
}

impl SingBoxHost {
    pub fn new(core_dirs: Vec<PathBuf>) -> Self {
        Self {
            core_dirs,
            install_dir: None,
        }
    }

    pub fn with_install_dir(mut self, dir: PathBuf) -> Self {
        self.core_dirs.push(dir.clone());
        self.install_dir = Some(dir);
        self
    }
}

impl VpnHost for SingBoxHost {
    fn locate_core(&self) -> Result<PathBuf, String> {
        if !cfg!(windows) {
            return Err("VPN пока работает только на Windows 10/11".into());
        }
        self.core_dirs
            .iter()
            .map(|d| d.join(CORE_EXE))
            .find(|p| p.is_file())
            .ok_or_else(|| {
                format!(
                    "файл {CORE_EXE} не найден (искали в: {})",
                    self.core_dirs
                        .iter()
                        .map(|d| d.display().to_string())
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            })
    }

    fn install_core(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<(), PrepareError> {
        if !cfg!(windows) {
            return Err(PrepareError::unsupported(
                "VPN пока работает только на Windows 10/11",
            ));
        }
        let Some(dir) = &self.install_dir else {
            return Err(PrepareError::unsupported("папка для VPN не задана"));
        };
        #[cfg(windows)]
        {
            engine::install(&engine::PINNED, dir, &http::ReqwestDownloader, progress)
        }
        #[cfg(not(windows))]
        {
            let _ = (dir, progress);
            Err(PrepareError::unsupported(
                "VPN пока работает только на Windows 10/11",
            ))
        }
    }

    fn free_port(&self) -> Result<u16, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|e| format!("Нет свободного порта: {e}"))?;
        listener
            .local_addr()
            .map(|a| a.port())
            .map_err(|e| e.to_string())
    }

    fn check_config(&self, core: &Path, config: &Path) -> Result<(), String> {
        run_with_timeout(
            core,
            &["check", "-c", &config.to_string_lossy()],
            Duration::from_secs(15),
        )
    }

    fn spawn(
        &self,
        core: &Path,
        config: &Path,
        log: &Path,
    ) -> Result<Box<dyn CoreProcess>, String> {
        let workdir = core.parent().unwrap_or(Path::new("."));
        let process = spawn_logged(
            core,
            &["run", "-c", &config.to_string_lossy()],
            workdir,
            log,
        )?;
        Ok(Box::new(process))
    }

    fn port_open(&self, port: u16) -> bool {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
    }

    fn probe_via_proxy(&self, port: u16) -> bool {
        probe_via_proxy(port)
    }

    fn kill_stale(&self, pid: u32) {
        let target = [Pid::from_u32(pid)];
        let Ok(mut system) = System::new() else {
            return;
        };
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&target),
            true,
            ProcessRefreshKind::nothing(),
        );
        if let Some(process) = system.process(target[0]) {
            // Только настоящее ядро: номер процесса мог достаться другой программе.
            let name = process.name().to_string_lossy().to_lowercase();
            if name == CORE_EXE || name.trim_end_matches(".exe") == "sing-box" {
                process.kill();
            }
        }
    }
}

/// Замер задержки: время TCP-подключения (первый адрес из DNS-ответа).
pub struct TcpDialer;

impl Dialer for TcpDialer {
    fn connect(&self, host: &str, port: u16, timeout: Duration) -> Result<(), String> {
        let addrs = (host, port)
            .to_socket_addrs()
            .map_err(|e| format!("DNS: {e}"))?;
        let mut last = "нет адреса".to_string();
        for addr in addrs.take(2) {
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(_) => return Ok(()),
                Err(e) => last = e.to_string(),
            }
        }
        Err(last)
    }
}

// ---------- HTTPS: только Windows (SChannel), на других ОС честная заглушка ----------

#[cfg(windows)]
pub(crate) mod http {
    use super::*;
    use ochistka_core::vpn::engine::DownloadFailure;
    use reqwest::blocking::Client;
    use reqwest::redirect::Policy;

    /// Загрузка подписки: HTTPS, до 5 переходов (только на https), таймаут, лимит размера.
    pub struct ReqwestFetch;

    impl HttpFetch for ReqwestFetch {
        fn get(
            &self,
            url: &str,
            user_agent: &str,
            max_bytes: usize,
        ) -> Result<FetchResponse, String> {
            let policy = Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 {
                    attempt.error("слишком много переходов")
                } else if attempt.url().scheme() != "https" {
                    attempt.error("переход на незащищённый адрес отклонён")
                } else {
                    attempt.follow()
                }
            });
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("text/plain, application/json, */*"),
            );
            let client = Client::builder()
                .user_agent(user_agent)
                .default_headers(headers)
                .redirect(policy)
                .timeout(Duration::from_secs(25))
                .https_only(true)
                .build()
                .map_err(|e| e.to_string())?;
            let response = client.get(url).send().map_err(|e| describe(&e))?;
            let status = response.status().as_u16();
            if response
                .content_length()
                .is_some_and(|n| n as usize > max_bytes)
            {
                return Err("ответ слишком большой".into());
            }
            let mut body = Vec::new();
            response
                .take(max_bytes as u64 + 1)
                .read_to_end(&mut body)
                .map_err(|e| format!("обрыв при чтении: {e}"))?;
            if body.len() > max_bytes {
                return Err("ответ слишком большой".into());
            }
            Ok(FetchResponse { status, body })
        }
    }

    /// Вид неудачи загрузки по ошибке reqwest. Адрес из текста ошибки убирается.
    fn download_error(err: reqwest::Error) -> PrepareError {
        use ochistka_core::desync::probe::Reply;
        use std::error::Error;
        let err = err.without_url();
        let mut chain = err.to_string();
        let mut source = err.source();
        while let Some(s) = source {
            chain.push(' ');
            chain.push_str(&s.to_string());
            source = s.source();
        }
        let kind = match crate::desync::probe::classify(err.is_timeout(), err.is_connect(), &chain)
        {
            Reply::Timeout => DownloadFailure::Timeout,
            Reply::Tls => DownloadFailure::Tls,
            Reply::Dns => DownloadFailure::Dns,
            Reply::Refused => DownloadFailure::Connect,
            _ if err.is_redirect() => DownloadFailure::Rejected,
            _ => DownloadFailure::Other,
        };
        PrepareError::download(kind, chain)
    }

    fn describe(err: &reqwest::Error) -> String {
        if err.is_timeout() {
            "время ожидания истекло".into()
        } else if err.is_connect() {
            "нет соединения с сервисом".into()
        } else if err.is_redirect() {
            "некорректный переход сервиса".into()
        } else {
            err.to_string()
        }
    }

    /// Загрузка движка: https только с github.com, переходы только на адреса GitHub.
    pub struct ReqwestDownloader;

    impl engine::Downloader for ReqwestDownloader {
        fn download(
            &self,
            url: &str,
            url_prefix: &str,
            max_bytes: usize,
            on_progress: &mut dyn FnMut(u64, Option<u64>),
        ) -> Result<Vec<u8>, PrepareError> {
            // Адрес проверяется по префиксу выпуска, который загружается (sing-box или обход).
            if !engine::url_allowed_under(url, url_prefix) {
                return Err(PrepareError::download(
                    DownloadFailure::Rejected,
                    "начальный адрес не относится к закреплённому выпуску",
                ));
            }
            let policy = Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 {
                    attempt.error("слишком много переходов")
                } else if !engine::redirect_allowed(attempt.url().as_str()) {
                    attempt.error("переход на чужой адрес отклонён")
                } else {
                    attempt.follow()
                }
            });
            let client = Client::builder()
                .user_agent("OCHISTKA")
                .redirect(policy)
                .connect_timeout(Duration::from_secs(20))
                .timeout(Duration::from_secs(600))
                .https_only(true)
                .build()
                .map_err(|e| {
                    PrepareError::download(DownloadFailure::Other, e.without_url().to_string())
                })?;
            let mut response = client.get(url).send().map_err(download_error)?;
            if !response.status().is_success() {
                return Err(PrepareError::download(
                    DownloadFailure::Http(response.status().as_u16()),
                    format!("ответ сервера {}", response.status().as_u16()),
                ));
            }
            let total = response.content_length();
            if total.is_some_and(|n| n as usize > max_bytes) {
                return Err(PrepareError::download(
                    DownloadFailure::TooLarge,
                    "размер больше лимита",
                ));
            }
            let mut body = Vec::with_capacity(total.unwrap_or(0).min(max_bytes as u64) as usize);
            let mut chunk = [0u8; 64 * 1024];
            loop {
                let n = response.read(&mut chunk).map_err(|e| {
                    PrepareError::download(
                        DownloadFailure::Connect,
                        format!("обрыв при чтении: {e}"),
                    )
                })?;
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..n]);
                if body.len() > max_bytes {
                    return Err(PrepareError::download(
                        DownloadFailure::TooLarge,
                        "размер больше лимита",
                    ));
                }
                on_progress(body.len() as u64, total);
            }
            Ok(body)
        }
    }

    pub fn probe(port: u16) -> bool {
        let Ok(proxy) = reqwest::Proxy::all(format!("http://127.0.0.1:{port}")) else {
            return false;
        };
        let Ok(client) = Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(6))
            .build()
        else {
            return false;
        };
        [
            "https://www.gstatic.com/generate_204",
            "https://www.msftconnecttest.com/connecttest.txt",
        ]
        .iter()
        .any(|u| client.get(*u).send().is_ok_and(|r| r.status().is_success()))
    }
}

#[cfg(windows)]
fn probe_via_proxy(port: u16) -> bool {
    http::probe(port)
}

#[cfg(not(windows))]
fn probe_via_proxy(_: u16) -> bool {
    false
}

#[cfg(not(windows))]
struct UnsupportedFetch;

#[cfg(not(windows))]
impl HttpFetch for UnsupportedFetch {
    fn get(&self, _: &str, _: &str, _: usize) -> Result<FetchResponse, String> {
        Err("загрузка подписки пока работает только на Windows 10/11".into())
    }
}

pub fn current_http_fetch() -> Box<dyn HttpFetch> {
    #[cfg(windows)]
    {
        Box::new(http::ReqwestFetch)
    }
    #[cfg(not(windows))]
    {
        Box::new(UnsupportedFetch)
    }
}

pub fn current_dialer() -> Box<dyn Dialer + Send> {
    Box::new(TcpDialer)
}

pub fn current_vpn_host(core_dirs: Vec<PathBuf>) -> Box<dyn VpnHost> {
    Box::new(SingBoxHost::new(core_dirs).with_install_dir(vpn_engine_dir()))
}

struct WindowsProxyRefresh;

impl ProxyRefresh for WindowsProxyRefresh {
    fn refresh(&self) {
        #[cfg(windows)]
        crate::windows::win32_ffi::notify_proxy_settings_changed();
    }
}

pub fn current_proxy_refresh() -> Arc<dyn ProxyRefresh> {
    if cfg!(windows) {
        Arc::new(WindowsProxyRefresh)
    } else {
        Arc::new(NoRefresh)
    }
}

/// Реестр HKCU (вне Windows методы возвращают `Unsupported`).
pub fn current_registry_port() -> Arc<dyn RegistryPort> {
    Arc::new(crate::windows::tweaks::WindowsTweaks::new())
}

/// Папка данных VPN: `%LOCALAPPDATA%\OCHISTKA\vpn`, иначе временная.
pub fn vpn_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OCHISTKA")
        .join("vpn")
}

/// Папка скачанного движка: `%LOCALAPPDATA%\OCHISTKA\vpn-core`, иначе временная.
pub fn vpn_engine_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OCHISTKA")
        .join("vpn-core")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-vpn-plat-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn spawn_logs_output_and_kill_stops_the_process() {
        let dir = temp("spawn");
        let log = dir.join("engine.log");
        let mut p = spawn_logged(
            Path::new("/bin/sh"),
            &["-c", "echo started; sleep 30"],
            &dir,
            &log,
        )
        .unwrap();
        assert!(!p.has_exited());
        std::thread::sleep(Duration::from_millis(300));
        assert!(std::fs::read_to_string(&log).unwrap().contains("started"));
        p.kill();
        assert!(p.has_exited());
    }

    #[test]
    fn run_with_timeout_reports_failure_text_and_timeouts() {
        assert!(run_with_timeout(
            Path::new("/bin/sh"),
            &["-c", "exit 0"],
            Duration::from_secs(5)
        )
        .is_ok());
        let err = run_with_timeout(
            Path::new("/bin/sh"),
            &["-c", "echo FATAL bad >&2; exit 3"],
            Duration::from_secs(5),
        )
        .unwrap_err();
        assert!(err.contains("FATAL"));
        let err = run_with_timeout(
            Path::new("/bin/sh"),
            &["-c", "sleep 5"],
            Duration::from_millis(200),
        )
        .unwrap_err();
        assert!(err.contains("не завершилась"));
    }

    #[test]
    fn port_and_dialer_work_on_loopback() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let host = SingBoxHost::new(vec![]);
        assert!(host.port_open(port));
        assert!(TcpDialer
            .connect("127.0.0.1", port, Duration::from_secs(1))
            .is_ok());
        drop(listener);
        assert!(!host.port_open(port));
        assert!(TcpDialer
            .connect("127.0.0.1", port, Duration::from_millis(500))
            .is_err());
        assert!(host.free_port().unwrap() > 0);
    }

    #[test]
    fn install_dir_is_searched_and_install_is_unsupported_off_windows() {
        let dir = temp("engine");
        std::fs::create_dir_all(&dir).unwrap();
        let host = SingBoxHost::new(vec![]).with_install_dir(dir.clone());
        // Вне Windows движок не ищется и не скачивается.
        assert!(host.locate_core().is_err());
        let err = host.install_core(&mut |_| {}).unwrap_err();
        assert!(err.reason.contains("Windows"));
        assert!(vpn_engine_dir().ends_with("vpn-core"));
    }

    #[test]
    fn missing_core_gives_a_reason() {
        let host = SingBoxHost::new(vec![PathBuf::from("/nonexistent")]);
        assert!(host.locate_core().is_err());
    }

    #[test]
    fn kill_stale_ignores_foreign_processes() {
        let dir = temp("stale");
        let mut p = spawn_logged(
            Path::new("/bin/sh"),
            &["-c", "sleep 30"],
            &dir,
            &dir.join("l"),
        )
        .unwrap();
        SingBoxHost::new(vec![]).kill_stale(p.pid());
        std::thread::sleep(Duration::from_millis(200));
        assert!(!p.has_exited(), "чужой процесс не должен быть убит");
        p.kill();
    }
}
