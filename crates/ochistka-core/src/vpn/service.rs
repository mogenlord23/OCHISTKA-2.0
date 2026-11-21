//! Сервис VPN: список серверов, импорт, выбор, маршруты, задержка, подключение и отключение.
//! Процессы, порты и сеть берутся из порта `VpnHost`, реестр — из `RegistryPort`; в тестах всё
//! поддельное. Ядро sing-box — отдельная программа, которую приложение запускает как дочерний
//! процесс и всегда останавливает при отключении, выходе и (после аварии) при следующем запуске.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::config::build_config;
use super::engine::{PrepareError, PrepareProgress};
use super::fetch::{import_input, HttpFetch, Input};
use super::latency::{measure, Dialer, PARALLEL, TIMEOUT};
use super::mask::mask_secrets;
use super::parse::{ImportReport, Server};
use super::routes::{RoutingRules, PRESETS};
use super::sysproxy::SystemProxy;
use super::VpnError;

/// Дочерний процесс ядра.
pub trait CoreProcess: Send {
    fn pid(&self) -> u32;
    fn has_exited(&mut self) -> bool;
    fn kill(&mut self);
}

/// Всё, что требует операционной системы. Реализация — в `crates/platform`.
pub trait VpnHost: Send + Sync {
    /// Путь к исполняемому файлу ядра или причина, по которой его нет.
    fn locate_core(&self) -> Result<PathBuf, String>;
    /// Скачивает и проверяет движок (закреплённая версия) в папку приложения.
    fn install_core(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<(), PrepareError>;
    fn free_port(&self) -> Result<u16, String>;
    /// `sing-box check -c <config>`; текст ошибки ядра в `Err`.
    fn check_config(&self, core: &Path, config: &Path) -> Result<(), String>;
    fn spawn(&self, core: &Path, config: &Path, log: &Path)
        -> Result<Box<dyn CoreProcess>, String>;
    fn port_open(&self, port: u16) -> bool;
    /// Запрос через локальный прокси: доходит ли трафик до интернета.
    fn probe_via_proxy(&self, port: u16) -> bool;
    /// Останавливает процесс ядра, оставшийся от прошлого запуска (если это действительно ядро).
    fn kill_stale(&self, pid: u32);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnConnectionState.ts")]
pub enum ConnState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnConnection.ts")]
pub struct ConnectionStatus {
    pub state: ConnState,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    /// Начало подключения, секунды Unix.
    #[ts(type = "number | null")]
    pub since: Option<u64>,
    pub message: String,
    /// Трафик через сервер проверен запросом.
    pub verified: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnServer.ts")]
pub struct ServerView {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub country: Option<String>,
    /// Задержка TCP, мс. `None` — ещё не мерили или сервер не ответил.
    pub latency_ms: Option<u32>,
    /// `true`, если замер уже был (чтобы отличить «не ответил» от «не мерили»).
    pub measured: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnCoreStatus.ts")]
pub struct CoreAvailability {
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnPreset.ts")]
pub struct PresetInfo {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnSnapshot.ts")]
pub struct VpnSnapshot {
    pub servers: Vec<ServerView>,
    pub selected: Option<String>,
    pub subscription_url: String,
    pub rules: RoutingRules,
    pub presets: Vec<PresetInfo>,
    pub core: CoreAvailability,
    pub connection: ConnectionStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnImportSummary.ts")]
pub struct ImportSummary {
    pub count: u32,
    /// Сколько записей пропущено (не поддерживается или повреждено).
    pub skipped: u32,
    /// Первые причины пропуска.
    pub issues: Vec<String>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Data {
    subscription_url: String,
    servers: Vec<Server>,
    selected: Option<String>,
    rules: RoutingRules,
    #[serde(skip)]
    latencies: HashMap<String, Option<u32>>,
}

struct Conn {
    state: ConnState,
    server_id: Option<String>,
    server_name: Option<String>,
    since: Option<u64>,
    message: String,
    verified: bool,
    process: Option<Box<dyn CoreProcess>>,
}

impl Conn {
    fn idle() -> Self {
        Self {
            state: ConnState::Disconnected,
            server_id: None,
            server_name: None,
            since: None,
            message: "Не подключено".into(),
            verified: false,
            process: None,
        }
    }
}

pub struct VpnService {
    dir: PathBuf,
    host: Box<dyn VpnHost>,
    proxy: SystemProxy,
    fetcher: Box<dyn HttpFetch>,
    dialer: Box<dyn Dialer + Send>,
    data: Mutex<Data>,
    conn: Mutex<Conn>,
    /// Один подключающийся или отключающийся поток за раз.
    op: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl VpnService {
    /// `dir` — папка данных VPN (`%LOCALAPPDATA%\OCHISTKA\vpn`).
    pub fn new(
        dir: PathBuf,
        host: Box<dyn VpnHost>,
        proxy: SystemProxy,
        fetcher: Box<dyn HttpFetch>,
        dialer: Box<dyn Dialer + Send>,
    ) -> Self {
        let data = std::fs::read_to_string(dir.join("state.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Data>(&t).ok())
            .unwrap_or_default();
        Self {
            dir,
            host,
            proxy,
            fetcher,
            dialer,
            data: Mutex::new(data),
            conn: Mutex::new(Conn::idle()),
            op: Mutex::new(()),
        }
    }

    fn session_dir(&self) -> PathBuf {
        self.dir.join("session")
    }

    fn pid_file(&self) -> PathBuf {
        self.dir.join("core.pid")
    }

    fn save(&self, data: &Data) -> Result<(), VpnError> {
        let text =
            serde_json::to_string_pretty(data).map_err(|e| VpnError::Storage(e.to_string()))?;
        std::fs::create_dir_all(&self.dir)
            .and_then(|()| std::fs::write(self.dir.join("state.json"), text))
            .map_err(|e| VpnError::Storage(format!("Не удалось сохранить настройки VPN: {e}")))
    }

    // ---------- чтение ----------

    pub fn snapshot(&self) -> VpnSnapshot {
        let connection = self.status();
        let data = lock(&self.data);
        let servers = data
            .servers
            .iter()
            .map(|s| ServerView {
                id: s.id.clone(),
                name: s.name.clone(),
                protocol: s.protocol.clone(),
                host: s.host.clone(),
                port: s.port,
                country: s.country.clone(),
                latency_ms: data.latencies.get(&s.id).copied().flatten(),
                measured: data.latencies.contains_key(&s.id),
            })
            .collect();
        let core = CoreAvailability {
            available: self.host.locate_core().is_ok(),
        };
        VpnSnapshot {
            servers,
            selected: data.selected.clone(),
            subscription_url: data.subscription_url.clone(),
            rules: data.rules.clone(),
            presets: PRESETS
                .iter()
                .map(|p| PresetInfo {
                    id: p.id.into(),
                    title: p.title.into(),
                    description: p.description.into(),
                })
                .collect(),
            core,
            connection,
        }
    }

    pub fn status(&self) -> ConnectionStatus {
        let mut conn = lock(&self.conn);
        if conn.state == ConnState::Connected {
            let died = conn.process.as_mut().is_none_or(|p| p.has_exited());
            if died {
                let tail = self.log_tail();
                self.cleanup_locked(&mut conn);
                conn.state = ConnState::Error;
                conn.message = format!("VPN остановился.{tail}");
                conn.since = None;
            }
        }
        ConnectionStatus {
            state: conn.state,
            server_id: conn.server_id.clone(),
            server_name: conn.server_name.clone(),
            since: conn.since,
            message: conn.message.clone(),
            verified: conn.verified,
        }
    }

    fn log_tail(&self) -> String {
        let text =
            std::fs::read_to_string(self.session_dir().join("engine.log")).unwrap_or_default();
        let text = mask_secrets(&text);
        let chars: Vec<char> = text.trim().chars().collect();
        let start = chars.len().saturating_sub(600);
        let tail: String = chars[start..].iter().collect();
        if tail.is_empty() {
            String::new()
        } else {
            format!(" {tail}")
        }
    }

    fn is_busy(&self) -> bool {
        matches!(
            lock(&self.conn).state,
            ConnState::Connecting | ConnState::Connected
        )
    }

    // ---------- импорт и настройки ----------

    pub fn import(&self, input: &str) -> Result<ImportSummary, VpnError> {
        if self.is_busy() {
            return Err(VpnError::Busy);
        }
        let report: ImportReport = import_input(self.fetcher.as_ref(), input)?;
        let url = match super::fetch::classify_input(input) {
            Ok(Input::Url(u)) => Some(u),
            _ => None,
        };
        let skipped = report.issues.len();
        let mut data = lock(&self.data);
        data.servers = report.servers;
        if let Some(u) = url {
            data.subscription_url = u;
        }
        data.latencies.clear();
        if !data
            .selected
            .as_ref()
            .is_some_and(|id| data.servers.iter().any(|s| &s.id == id))
        {
            data.selected = data.servers.first().map(|s| s.id.clone());
        }
        self.save(&data)?;
        Ok(ImportSummary {
            count: data.servers.len() as u32,
            skipped: skipped as u32,
            issues: report.issues.into_iter().take(5).collect(),
        })
    }

    pub fn select(&self, id: &str) -> Result<(), VpnError> {
        let mut data = lock(&self.data);
        if !data.servers.iter().any(|s| s.id == id) {
            return Err(VpnError::NoServer);
        }
        data.selected = Some(id.to_string());
        self.save(&data)
    }

    pub fn set_rules(&self, rules: &RoutingRules) -> Result<RoutingRules, VpnError> {
        let clean = rules.sanitized()?;
        let mut data = lock(&self.data);
        data.rules = clean.clone();
        self.save(&data)?;
        Ok(clean)
    }

    /// Мерит задержку всех серверов (TCP). Подключению не мешает.
    pub fn measure_latency(&self) {
        let targets: Vec<(String, String, u16)> = lock(&self.data)
            .servers
            .iter()
            .map(|s| (s.id.clone(), s.host.clone(), s.port))
            .collect();
        let results = measure(self.dialer.as_ref(), &targets, PARALLEL, TIMEOUT);
        let mut data = lock(&self.data);
        for (id, ms) in results {
            if data.servers.iter().any(|s| s.id == id) {
                data.latencies.insert(id, ms);
            }
        }
    }

    // ---------- подключение ----------

    pub fn connect(&self) -> Result<ConnectionStatus, VpnError> {
        self.connect_with(&mut |_| {})
    }

    /// Подключение; при первом запуске сначала готовит движок и сообщает ход в `progress`.
    pub fn connect_with(
        &self,
        progress: &mut dyn FnMut(PrepareProgress),
    ) -> Result<ConnectionStatus, VpnError> {
        let Ok(_op) = self.op.try_lock() else {
            return Err(VpnError::Busy);
        };
        if lock(&self.conn).state == ConnState::Connected {
            return Ok(self.status());
        }
        let (server, rules) = {
            let data = lock(&self.data);
            let server = data
                .selected
                .as_ref()
                .and_then(|id| data.servers.iter().find(|s| &s.id == id))
                .cloned()
                .ok_or(VpnError::NoServer)?;
            (server, data.rules.clone())
        };
        {
            let mut conn = lock(&self.conn);
            *conn = Conn::idle();
            conn.state = ConnState::Connecting;
            conn.server_id = Some(server.id.clone());
            conn.server_name = Some(server.name.clone());
            conn.message = "Подключаемся…".into();
        }
        let core = match self.host.locate_core() {
            Ok(path) => path,
            Err(_) => match self.prepare_core(progress) {
                Ok(path) => path,
                Err(err) => {
                    let mut conn = lock(&self.conn);
                    *conn = Conn::idle();
                    conn.state = ConnState::Error;
                    conn.message = err.to_string();
                    return Err(err);
                }
            },
        };
        match self.start(&core, &server, &rules) {
            Ok((process, verified)) => {
                let mut conn = lock(&self.conn);
                conn.process = Some(process);
                conn.state = ConnState::Connected;
                conn.since = Some(now());
                conn.verified = verified;
                conn.message = if verified {
                    format!("Подключено · {} · передача данных проверена", server.name)
                } else {
                    "VPN запущен, но доступ через сервер не подтверждён. Попробуй другой сервер."
                        .into()
                };
                drop(conn);
                Ok(self.status())
            }
            Err(err) => {
                let mut conn = lock(&self.conn);
                self.cleanup_locked(&mut conn);
                *conn = Conn::idle();
                conn.state = ConnState::Error;
                conn.message = err.to_string();
                Err(err)
            }
        }
    }

    /// Скачивает движок и снова ищет его. Подробности неудачи пишутся в журнал, а не показываются.
    fn prepare_core(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<PathBuf, VpnError> {
        lock(&self.conn).message = "Подготовка VPN…".into();
        self.host.install_core(progress).map_err(|err| {
            let _ = std::fs::create_dir_all(&self.dir);
            let _ = std::fs::write(self.dir.join("prepare.log"), mask_secrets(&err.detail));
            VpnError::Prepare(err.reason)
        })?;
        self.host
            .locate_core()
            .map_err(|_| VpnError::Prepare("проверьте интернет".into()))
    }

    fn start(
        &self,
        core: &Path,
        server: &Server,
        rules: &RoutingRules,
    ) -> Result<(Box<dyn CoreProcess>, bool), VpnError> {
        let session = self.session_dir();
        std::fs::create_dir_all(&session)
            .map_err(|e| VpnError::Engine(format!("Папка сеанса недоступна: {e}")))?;
        let port = self.host.free_port().map_err(VpnError::Engine)?;
        let config = build_config(server, port, rules);
        let config_path = session.join("config.json");
        let text =
            serde_json::to_string_pretty(&config).map_err(|e| VpnError::Engine(e.to_string()))?;
        std::fs::write(&config_path, text)
            .map_err(|e| VpnError::Engine(format!("Не удалось записать конфигурацию: {e}")))?;
        self.host.check_config(core, &config_path).map_err(|e| {
            VpnError::Engine(format!(
                "VPN не принял параметры сервера: {}",
                mask_secrets(&e)
            ))
        })?;
        let log = session.join("engine.log");
        let mut process = self
            .host
            .spawn(core, &config_path, &log)
            .map_err(VpnError::Engine)?;
        let _ = std::fs::write(self.pid_file(), process.pid().to_string());
        let mut ready = false;
        for _ in 0..75 {
            if process.has_exited() {
                let tail = self.log_tail();
                return Err(VpnError::Engine(format!(
                    "VPN завершился при запуске.{tail}"
                )));
            }
            if self.host.port_open(port) {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        if !ready {
            process.kill();
            return Err(VpnError::Engine(
                "VPN не открыл локальный порт за 15 секунд".into(),
            ));
        }
        if let Err(err) = self.proxy.enable(port) {
            process.kill();
            return Err(err);
        }
        let verified = (0..3).any(|_| self.host.probe_via_proxy(port));
        Ok((process, verified))
    }

    /// Останавливает ядро, возвращает системный прокси, стирает конфигурацию с ключами.
    fn cleanup_locked(&self, conn: &mut Conn) {
        if let Some(mut p) = conn.process.take() {
            p.kill();
        }
        let _ = self.proxy.restore();
        let _ = std::fs::remove_file(self.pid_file());
        let _ = std::fs::remove_file(self.session_dir().join("config.json"));
    }

    pub fn disconnect(&self) -> ConnectionStatus {
        let _op = lock(&self.op);
        {
            let mut conn = lock(&self.conn);
            self.cleanup_locked(&mut conn);
            *conn = Conn::idle();
        }
        self.status()
    }

    /// Вызывается при запуске: убирает ядро и системный прокси после аварийного завершения.
    pub fn recover_on_start(&self) {
        if let Some(pid) = std::fs::read_to_string(self.pid_file())
            .ok()
            .and_then(|t| t.trim().parse().ok())
        {
            self.host.kill_stale(pid);
        }
        let _ = std::fs::remove_file(self.pid_file());
        let _ = std::fs::remove_file(self.session_dir().join("config.json"));
        let _ = self.proxy.restore();
    }

    /// Вызывается при выходе из приложения.
    pub fn shutdown(&self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vpn::fetch::FetchResponse;
    use crate::vpn::sysproxy::fake::{FakeRegistry, V};
    use crate::vpn::sysproxy::{NoRefresh, INTERNET_KEY};
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ochistka-vpn-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[derive(Clone, Default)]
    struct Flags {
        exited: Arc<AtomicBool>,
        killed: Arc<AtomicBool>,
        stale: Arc<Mutex<Vec<u32>>>,
    }

    struct FakeProcess(Flags);
    impl CoreProcess for FakeProcess {
        fn pid(&self) -> u32 {
            4242
        }
        fn has_exited(&mut self) -> bool {
            self.0.exited.load(Ordering::SeqCst)
        }
        fn kill(&mut self) {
            self.0.killed.store(true, Ordering::SeqCst);
        }
    }

    struct FakeHost {
        flags: Flags,
        core_missing: bool,
        install_fails: bool,
        installed: Arc<AtomicBool>,
        check_fails: bool,
        port_opens: bool,
        probe_ok: bool,
    }

    impl FakeHost {
        fn good(flags: &Flags) -> Self {
            Self {
                flags: flags.clone(),
                core_missing: false,
                install_fails: false,
                installed: Arc::new(AtomicBool::new(false)),
                check_fails: false,
                port_opens: true,
                probe_ok: true,
            }
        }
    }

    impl VpnHost for FakeHost {
        fn locate_core(&self) -> Result<PathBuf, String> {
            if self.core_missing && !self.installed.load(Ordering::SeqCst) {
                Err("файл sing-box.exe не найден".into())
            } else {
                Ok(PathBuf::from("/core/sing-box"))
            }
        }
        fn install_core(
            &self,
            progress: &mut dyn FnMut(PrepareProgress),
        ) -> Result<(), PrepareError> {
            if self.install_fails {
                return Err(PrepareError::network(
                    "offline: https://sub.example/api/SECRETTOKEN?k=1",
                ));
            }
            progress(PrepareProgress { percent: 50 });
            progress(PrepareProgress { percent: 100 });
            self.installed.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn free_port(&self) -> Result<u16, String> {
            Ok(23456)
        }
        fn check_config(&self, _: &Path, config: &Path) -> Result<(), String> {
            assert!(config.exists());
            if self.check_fails {
                Err("FATAL bad outbound".into())
            } else {
                Ok(())
            }
        }
        fn spawn(&self, _: &Path, _: &Path, log: &Path) -> Result<Box<dyn CoreProcess>, String> {
            std::fs::write(log, "INFO started\nERROR something broke").unwrap();
            Ok(Box::new(FakeProcess(self.flags.clone())))
        }
        fn port_open(&self, _: u16) -> bool {
            self.port_opens
        }
        fn probe_via_proxy(&self, _: u16) -> bool {
            self.probe_ok
        }
        fn kill_stale(&self, pid: u32) {
            self.flags.stale.lock().unwrap().push(pid);
        }
    }

    struct NoNet;
    impl HttpFetch for NoNet {
        fn get(&self, _: &str, _: &str, _: usize) -> Result<FetchResponse, String> {
            Err("сеть недоступна".into())
        }
    }
    struct Body(&'static str);
    impl HttpFetch for Body {
        fn get(&self, _: &str, _: &str, _: usize) -> Result<FetchResponse, String> {
            Ok(FetchResponse {
                status: 200,
                body: self.0.as_bytes().to_vec(),
            })
        }
    }
    struct OkDialer;
    impl Dialer for OkDialer {
        fn connect(&self, host: &str, _: u16, _: Duration) -> Result<(), String> {
            if host.contains("dead") {
                Err("нет".into())
            } else {
                Ok(())
            }
        }
    }

    const LIST: &str = "trojan://p@a.example.com:443#A\ntrojan://p@dead.example.com:443#B\n";

    fn make(
        host: FakeHost,
        fetcher: Box<dyn HttpFetch>,
        dir: &Path,
    ) -> (VpnService, Arc<FakeRegistry>) {
        let reg = Arc::new(FakeRegistry::default());
        let proxy = SystemProxy::new(reg.clone(), Arc::new(NoRefresh));
        (
            VpnService::new(
                dir.to_path_buf(),
                Box::new(host),
                proxy,
                fetcher,
                Box::new(OkDialer),
            ),
            reg,
        )
    }

    fn ready(flags: &Flags, dir: &Path) -> (VpnService, Arc<FakeRegistry>) {
        let (s, reg) = make(FakeHost::good(flags), Box::new(NoNet), dir);
        s.import(LIST).unwrap();
        (s, reg)
    }

    #[test]
    fn import_selects_first_and_persists() {
        let dir = temp_dir();
        let flags = Flags::default();
        let (s, _) = ready(&flags, &dir);
        let snap = s.snapshot();
        assert_eq!(snap.servers.len(), 2);
        assert_eq!(snap.selected.as_deref(), Some(snap.servers[0].id.as_str()));
        let (again, _) = make(FakeHost::good(&flags), Box::new(NoNet), &dir);
        assert_eq!(
            again.snapshot().servers.len(),
            2,
            "список переживает перезапуск"
        );
        assert_eq!(again.snapshot().selected, snap.selected);
    }

    #[test]
    fn url_import_stores_the_link_and_http_is_refused() {
        let dir = temp_dir();
        let flags = Flags::default();
        let (s, _) = make(FakeHost::good(&flags), Box::new(Body(LIST)), &dir);
        let r = s.import("https://sub.example/s?t=1").unwrap();
        assert_eq!(r.count, 2);
        assert_eq!(s.snapshot().subscription_url, "https://sub.example/s?t=1");
        assert_eq!(
            s.import("http://sub.example/s").unwrap_err(),
            VpnError::NotHttps
        );
        assert_eq!(
            s.snapshot().servers.len(),
            2,
            "список не потерян при ошибке"
        );
    }

    #[test]
    fn latency_is_measured_and_failures_marked() {
        let dir = temp_dir();
        let (s, _) = ready(&Flags::default(), &dir);
        s.measure_latency();
        let snap = s.snapshot();
        assert!(snap.servers[0].measured && snap.servers[0].latency_ms.is_some());
        assert!(snap.servers[1].measured && snap.servers[1].latency_ms.is_none());
    }

    #[test]
    fn connect_sets_proxy_and_disconnect_restores_it() {
        let dir = temp_dir();
        let flags = Flags::default();
        let (s, reg) = ready(&flags, &dir);
        reg.set_s(INTERNET_KEY, "ProxyServer", "orig:1");
        let st = s.connect().unwrap();
        assert_eq!(st.state, ConnState::Connected);
        assert!(st.verified && st.since.is_some());
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("127.0.0.1:23456".into()))
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("core.pid")).unwrap(),
            "4242"
        );
        assert_eq!(s.import(LIST).unwrap_err(), VpnError::Busy);
        let off = s.disconnect();
        assert_eq!(off.state, ConnState::Disconnected);
        assert!(flags.killed.load(Ordering::SeqCst));
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("orig:1".into()))
        );
        assert!(!dir.join("core.pid").exists());
        assert!(!dir.join("session/config.json").exists(), "ключи стёрты");
    }

    #[test]
    fn missing_engine_is_prepared_on_first_connect_with_progress() {
        let dir = temp_dir();
        let host = FakeHost {
            core_missing: true,
            ..FakeHost::good(&Flags::default())
        };
        let (s, _) = make(host, Box::new(NoNet), &dir);
        s.import(LIST).unwrap();
        assert!(!s.snapshot().core.available);
        let mut seen = Vec::new();
        let status = s.connect_with(&mut |p| seen.push(p.percent)).unwrap();
        assert_eq!(status.state, ConnState::Connected);
        assert_eq!(seen, vec![50, 100]);
        assert!(s.snapshot().core.available);
        s.disconnect();
    }

    #[test]
    fn failed_preparation_says_check_internet_and_touches_nothing() {
        let dir = temp_dir();
        let host = FakeHost {
            core_missing: true,
            install_fails: true,
            ..FakeHost::good(&Flags::default())
        };
        let (s, reg) = make(host, Box::new(NoNet), &dir);
        s.import(LIST).unwrap();
        let err = s.connect().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Не удалось подготовить VPN: проверьте интернет"
        );
        assert_eq!(s.status().state, ConnState::Error);
        assert!(reg.map.lock().unwrap().is_empty());
        // В журнал ссылки попадают без токена.
        assert_eq!(
            std::fs::read_to_string(dir.join("prepare.log")).unwrap(),
            "offline: https://sub.example/***"
        );
        // Повторная попытка («Повторить») снова пробует подготовку.
        assert!(matches!(s.connect(), Err(VpnError::Prepare(_))));
    }

    #[test]
    fn no_server_selected_is_an_error() {
        let dir = temp_dir();
        let (s, _) = make(FakeHost::good(&Flags::default()), Box::new(NoNet), &dir);
        assert_eq!(s.connect().unwrap_err(), VpnError::NoServer);
    }

    #[test]
    fn xray_subscription_imports_connects_and_restores_the_proxy() {
        let dir = temp_dir();
        let flags = Flags::default();
        let (s, reg) = make(FakeHost::good(&flags), Box::new(NoNet), &dir);
        let r = s
            .import(&crate::vpn::xray::tests::fictional_subscription())
            .unwrap();
        assert_eq!((r.count, r.skipped), (10, 0));
        // gRPC + REALITY (первый) и TCP + TLS (седьмой) подключаются через общий путь проверки.
        for index in [0usize, 6] {
            let id = s.snapshot().servers[index].id.clone();
            s.select(&id).unwrap();
            assert_eq!(s.connect().unwrap().state, ConnState::Connected);
            assert_eq!(reg.get(INTERNET_KEY, "ProxyEnable"), Some(V::D(1)));
            s.disconnect();
            assert_ne!(reg.get(INTERNET_KEY, "ProxyEnable"), Some(V::D(1)));
            assert!(
                !s.session_dir().join("config.json").exists(),
                "ключи стёрты после отключения"
            );
        }
    }

    #[test]
    fn core_check_output_with_ids_is_masked_in_the_message() {
        let secret = "b831381d-6324-4d53-ad4f-8cda48b30811";
        let masked = mask_secrets(&format!("FATAL decode uuid {secret}"));
        assert!(!masked.contains(secret) && masked.contains("FATAL"));
    }

    #[test]
    fn config_check_failure_rolls_back() {
        let dir = temp_dir();
        let flags = Flags::default();
        let host = FakeHost {
            check_fails: true,
            ..FakeHost::good(&flags)
        };
        let (s, reg) = make(host, Box::new(NoNet), &dir);
        s.import(LIST).unwrap();
        let err = s.connect().unwrap_err().to_string();
        assert!(err.contains("не принял") && err.contains("FATAL"));
        assert_eq!(s.status().state, ConnState::Error);
        assert!(reg.get(INTERNET_KEY, "ProxyEnable").is_none());
    }

    #[test]
    fn core_that_dies_at_start_returns_log_and_restores_proxy() {
        let dir = temp_dir();
        let flags = Flags::default();
        flags.exited.store(true, Ordering::SeqCst);
        let (s, reg) = ready(&flags, &dir);
        let err = s.connect().unwrap_err().to_string();
        assert!(err.contains("завершился при запуске") && err.contains("something broke"));
        assert!(reg.get(INTERNET_KEY, "ProxyServer").is_none());
    }

    #[test]
    fn unverified_connection_is_connected_with_warning() {
        let dir = temp_dir();
        let flags = Flags::default();
        let host = FakeHost {
            probe_ok: false,
            ..FakeHost::good(&flags)
        };
        let (s, _) = make(host, Box::new(NoNet), &dir);
        s.import(LIST).unwrap();
        let st = s.connect().unwrap();
        assert_eq!(st.state, ConnState::Connected);
        assert!(!st.verified && st.message.contains("не подтверждён"));
    }

    #[test]
    fn unexpected_exit_is_noticed_and_proxy_restored() {
        let dir = temp_dir();
        let flags = Flags::default();
        let (s, reg) = ready(&flags, &dir);
        s.connect().unwrap();
        flags.exited.store(true, Ordering::SeqCst);
        let st = s.status();
        assert_eq!(st.state, ConnState::Error);
        assert!(st.message.contains("остановился"));
        assert!(reg.get(INTERNET_KEY, "ProxyServer").is_none());
    }

    #[test]
    fn recovery_after_crash_kills_stale_core_and_restores_proxy() {
        let dir = temp_dir();
        let flags = Flags::default();
        let (s, reg) = ready(&flags, &dir);
        reg.set_s(INTERNET_KEY, "ProxyServer", "orig:1");
        s.connect().unwrap();
        // Авария: процесс приложения исчез, остались pid-файл и копия реестра.
        let (fresh, _) = {
            let proxy = SystemProxy::new(reg.clone(), Arc::new(NoRefresh));
            (
                VpnService::new(
                    dir.clone(),
                    Box::new(FakeHost::good(&flags)),
                    proxy,
                    Box::new(NoNet),
                    Box::new(OkDialer),
                ),
                (),
            )
        };
        fresh.recover_on_start();
        assert_eq!(flags.stale.lock().unwrap().as_slice(), [4242]);
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("orig:1".into()))
        );
        assert!(!dir.join("core.pid").exists());
    }

    #[test]
    fn rules_are_validated_and_saved() {
        let dir = temp_dir();
        let (s, _) = ready(&Flags::default(), &dir);
        let bad = RoutingRules {
            direct_domains: vec!["http://x".into()],
            ..Default::default()
        };
        assert!(matches!(s.set_rules(&bad), Err(VpnError::Rules(_))));
        let ok = RoutingRules {
            direct_domains: vec!["Example.com".into()],
            presets: vec!["ru-direct".into()],
            ..Default::default()
        };
        assert_eq!(
            s.set_rules(&ok).unwrap().direct_domains,
            vec!["example.com"]
        );
        assert_eq!(s.snapshot().rules.presets, vec!["ru-direct"]);
    }

    #[test]
    fn select_unknown_server_fails() {
        let dir = temp_dir();
        let (s, _) = ready(&Flags::default(), &dir);
        assert_eq!(s.select("nope"), Err(VpnError::NoServer));
        let second = s.snapshot().servers[1].id.clone();
        s.select(&second).unwrap();
        assert_eq!(s.snapshot().selected, Some(second));
    }
}
