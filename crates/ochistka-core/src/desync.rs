//! Единая система «Обход блокировок»: общее ядро для Windows, Linux и macOS.
//!
//! Общее (этот файл): состояния, профили и стратегии, автоматический выбор стратегии без
//! смешивания, управление процессом (запуск только по явному действию, проверка, остановка, защита
//! от второго экземпляра, очистка после аварии), конфликт с VPN, честная готовность адаптера.
//! Системное (адаптеры, `crates/platform`): Windows — `winws2` + WinDivert; Linux — `nfqws2` +
//! NFQUEUE; macOS — Zapret 2 не поддерживается, адаптер сообщает это и ничего не запускает.
//!
//! Пока ни один настоящий адаптер не готов, основной переключатель «Обход блокировок» не включается
//! (`Readiness`). Ничего не скачивается и не запускается без подтверждённых источников и хэшей.
//! Системный прокси, DNS, hosts и маршруты здесь не трогаются.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use thiserror::Error;

use crate::vpn::engine::{PinnedEngine, PrepareError, PrepareProgress};
use crate::vpn::mask::mask_secrets;
use crate::vpn::service::CoreProcess;

/// Официальные источники и лицензии (подтверждаются на Windows скриптом `tools/desync-core/verify-sources.ps1`).
pub const SOURCES: &[(&str, &str, &str)] = &[
    (
        "Zapret 2 (winws2)",
        "https://github.com/bol-van/zapret2/releases/",
        "MIT",
    ),
    (
        "WinDivert",
        "https://github.com/basil00/WinDivert/releases/",
        "LGPL v3 / GPL v2",
    ),
];

/// Операционная система адаптера.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    MacOs,
}

impl Platform {
    /// ОС, под которую собрано приложение.
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }
}

/// Движок десинка (одна реализация на ОС; на macOS его нет).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    /// Zapret 2 для Windows (перехват пакетов WinDivert).
    Winws2,
    /// Zapret 2 для Linux (перехват через NFQUEUE).
    Nfqws2,
}

impl Engine {
    pub fn for_platform(platform: Platform) -> Option<Self> {
        match platform {
            Platform::Windows => Some(Engine::Winws2),
            Platform::Linux => Some(Engine::Nfqws2),
            Platform::MacOs => None,
        }
    }
}

/// Что обход должен охватывать (одно понятие для всех ОС).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    Sites,
    Discord,
    Telegram,
    Games,
}

/// Готов ли системный адаптер. Функцию как работающую показываем только при `Ready`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Readiness {
    Ready,
    /// Адаптер для этой ОС ещё не написан или не проверен.
    NotImplemented(String),
    /// На этой ОС такого механизма нет.
    Unsupported(String),
}

/// Одна стратегия обхода: движок, охват и аргументы. Стратегии не складываются.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Strategy {
    pub id: &'static str,
    pub engine: Engine,
    pub covers: &'static [Coverage],
    pub args: Vec<String>,
}

/// Автоматический выбор: из стратегий движка берётся ОДНА, которая охватывает всё нужное целиком,
/// с наименьшим лишним охватом (меньше правил, меньше конфликтов). Объединять стратегии нельзя:
/// если одной подходящей нет, возвращается `None`.
pub fn select_strategy<'a>(
    candidates: &'a [Strategy],
    engine: Engine,
    wanted: &[Coverage],
) -> Option<&'a Strategy> {
    candidates
        .iter()
        .filter(|s| s.engine == engine && wanted.iter().all(|w| s.covers.contains(w)))
        .min_by_key(|s| s.covers.len())
}

/// Закреплённый выпуск компонентов. `None`, пока хэши официальных файлов не подтверждены:
/// в этом случае ничего не скачивается (неподтверждённые бинарные файлы не используем).
pub fn pinned() -> Option<&'static PinnedEngine> {
    None
}

/// Один проверенный профиль обхода (сайты и Discord): аргументы `winws2`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub id: &'static str,
    pub engine: Engine,
    pub title: &'static str,
    pub args: Vec<String>,
}

/// Профиль «Сайты и Discord» для ОС. `None`, пока набор аргументов не проверен на настоящей ОС:
/// придумывать параметры обхода нельзя.
pub fn site_discord_profile(_platform: Platform) -> Option<Profile> {
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DesyncError {
    #[error("Другая операция с обходом ещё выполняется")]
    Busy,
    #[error("{0}")]
    Unavailable(String),
    #[error("Сначала отключите VPN: пока обход и VPN вместе не работают")]
    VpnRunning,
    #[error("Профиль обхода пока не подтверждён, включение недоступно")]
    NoProfile,
    #[error("Компоненты обхода не подготовлены: {0}")]
    NotPrepared(String),
    #[error("Для обхода нужны права администратора")]
    NeedsAdmin,
    #[error("Не удалось подготовить компоненты: {0}")]
    Prepare(String),
    #[error("Не удалось запустить обход: {0}")]
    Start(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesyncState {
    Off,
    Running,
    Error,
}

/// Системный адаптер: всё, что зависит от ОС (реализации — в `crates/platform`, по одной на ОС).
pub trait DesyncAdapter: Send + Sync {
    fn platform(&self) -> Platform;
    /// Готов ли адаптер. Не `Ready` — переключатель не включается, запуск отказывает.
    fn readiness(&self) -> Readiness;
    /// Путь к движку (`winws2` или `nfqws2`), если все компоненты на месте и их хэши совпали.
    fn locate(&self) -> Result<PathBuf, String>;
    /// Скачивает и проверяет закреплённые компоненты (только при подтверждённых хэшах).
    fn install(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<(), PrepareError>;
    /// Есть ли права для перехвата пакетов (администратор в Windows, root или CAP_NET_ADMIN в Linux).
    fn has_privileges(&self) -> bool;
    fn spawn(
        &self,
        exe: &Path,
        args: &[String],
        log: &Path,
    ) -> Result<Box<dyn CoreProcess>, String>;
    /// Останавливает процесс от прошлого запуска, если это действительно наш движок.
    fn kill_stale(&self, pid: u32);
}

struct Inner {
    state: DesyncState,
    process: Option<Box<dyn CoreProcess>>,
    message: String,
}

pub struct DesyncService {
    dir: PathBuf,
    adapter: Box<dyn DesyncAdapter>,
    profile: Option<Profile>,
    /// Есть ли закреплённый выпуск (можно ли скачивать недостающее).
    installable: bool,
    /// Работает ли VPN (подключается или подключён): одновременно нельзя.
    vpn_active: Box<dyn Fn() -> bool + Send + Sync>,
    /// Сколько раз и как часто проверять, что процесс не завершился сразу после запуска.
    settle: (u32, Duration),
    inner: Mutex<Inner>,
    op: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl DesyncService {
    pub fn new(
        dir: PathBuf,
        adapter: Box<dyn DesyncAdapter>,
        vpn_active: Box<dyn Fn() -> bool + Send + Sync>,
    ) -> Self {
        let profile = site_discord_profile(adapter.platform());
        Self {
            dir,
            adapter,
            profile,
            installable: pinned().is_some(),
            vpn_active,
            settle: (5, Duration::from_millis(200)),
            inner: Mutex::new(Inner {
                state: DesyncState::Off,
                process: None,
                message: "Выключено".into(),
            }),
            op: Mutex::new(()),
        }
    }

    fn pid_file(&self) -> PathBuf {
        self.dir.join("desync.pid")
    }

    fn log_file(&self) -> PathBuf {
        self.dir.join("desync.log")
    }

    /// Текущее состояние. Неожиданно завершившийся процесс замечается здесь.
    pub fn status(&self) -> (DesyncState, String) {
        let mut inner = lock(&self.inner);
        if inner.state == DesyncState::Running
            && inner.process.as_mut().is_none_or(|p| p.has_exited())
        {
            inner.process = None;
            let _ = std::fs::remove_file(self.pid_file());
            inner.state = DesyncState::Error;
            inner.message = "Обход остановился".into();
        }
        (inner.state, inner.message.clone())
    }

    /// Запуск по явному действию пользователя. Повторный вызов второй экземпляр не создаёт.
    pub fn start(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<(), DesyncError> {
        let Ok(_op) = self.op.try_lock() else {
            return Err(DesyncError::Busy);
        };
        if self.status().0 == DesyncState::Running {
            return Ok(());
        }
        let result = self.start_locked(progress);
        let mut inner = lock(&self.inner);
        if let Err(err) = &result {
            inner.state = DesyncState::Error;
            inner.message = err.to_string();
        }
        result
    }

    fn start_locked(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<(), DesyncError> {
        match self.adapter.readiness() {
            Readiness::Ready => {}
            Readiness::NotImplemented(why) | Readiness::Unsupported(why) => {
                return Err(DesyncError::Unavailable(why));
            }
        }
        if (self.vpn_active)() {
            return Err(DesyncError::VpnRunning);
        }
        let profile = self.profile.as_ref().ok_or(DesyncError::NoProfile)?;
        let exe = match self.adapter.locate() {
            Ok(path) => path,
            Err(_) if self.installable => {
                self.adapter.install(progress).map_err(|e| {
                    let _ = std::fs::create_dir_all(&self.dir);
                    let _ = std::fs::write(
                        self.dir.join("desync-prepare.log"),
                        mask_secrets(&e.detail),
                    );
                    DesyncError::Prepare(e.reason)
                })?;
                self.adapter
                    .locate()
                    .map_err(|why| DesyncError::NotPrepared(mask_secrets(&why)))?
            }
            Err(why) => return Err(DesyncError::NotPrepared(mask_secrets(&why))),
        };
        if !self.adapter.has_privileges() {
            return Err(DesyncError::NeedsAdmin);
        }
        // Остаток собственного процесса после аварии: останавливаем до запуска нового.
        self.recover();
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| DesyncError::Start(format!("папка недоступна: {e}")))?;
        let mut process = self
            .adapter
            .spawn(&exe, &profile.args, &self.log_file())
            .map_err(|why| DesyncError::Start(mask_secrets(&why)))?;
        let _ = std::fs::write(self.pid_file(), process.pid().to_string());
        for _ in 0..self.settle.0 {
            if process.has_exited() {
                let _ = std::fs::remove_file(self.pid_file());
                return Err(DesyncError::Start(
                    "процесс завершился сразу после запуска".into(),
                ));
            }
            std::thread::sleep(self.settle.1);
        }
        let mut inner = lock(&self.inner);
        inner.process = Some(process);
        inner.state = DesyncState::Running;
        inner.message = format!("Включено · {}", profile.title);
        Ok(())
    }

    /// Останавливает движок и освобождает файл номера процесса. Без запущенного обхода безвредно.
    pub fn stop(&self) {
        let _op = lock(&self.op);
        let mut inner = lock(&self.inner);
        if let Some(mut p) = inner.process.take() {
            p.kill();
        }
        let _ = std::fs::remove_file(self.pid_file());
        inner.state = DesyncState::Off;
        inner.message = "Выключено".into();
    }

    /// Убирает остаток прошлого запуска (аварийное завершение приложения).
    fn recover(&self) {
        if let Some(pid) = std::fs::read_to_string(self.pid_file())
            .ok()
            .and_then(|t| t.trim().parse().ok())
        {
            self.adapter.kill_stale(pid);
        }
        let _ = std::fs::remove_file(self.pid_file());
    }

    /// Вызывается при запуске приложения.
    pub fn recover_on_start(&self) {
        let _op = lock(&self.op);
        self.recover();
    }

    /// Вызывается при выходе из приложения.
    pub fn shutdown(&self) {
        self.stop();
    }
}

/// Адаптер-заглушка для ОС, где системная часть ещё не написана или механизма нет. Ничего не
/// запускает и не скачивает; честно сообщает причину. Используется приложением, пока нет настоящего.
pub struct UnavailableAdapter {
    platform: Platform,
}

impl UnavailableAdapter {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }
}

impl DesyncAdapter for UnavailableAdapter {
    fn platform(&self) -> Platform {
        self.platform
    }

    fn readiness(&self) -> Readiness {
        match self.platform {
            Platform::Windows => Readiness::NotImplemented(
                "Обход блокировок для Windows ещё не готов: движок не подтверждён".into(),
            ),
            Platform::Linux => Readiness::NotImplemented(
                "Обход блокировок для Linux ещё не готов: системная часть не написана".into(),
            ),
            Platform::MacOs => {
                Readiness::Unsupported("На macOS движок обхода (Zapret 2) не поддерживается".into())
            }
        }
    }

    fn locate(&self) -> Result<PathBuf, String> {
        Err("компоненты обхода не подготовлены".into())
    }

    fn install(&self, _: &mut dyn FnMut(PrepareProgress)) -> Result<(), PrepareError> {
        Err(PrepareError::unsupported(
            "загрузка компонентов не разрешена",
        ))
    }

    fn has_privileges(&self) -> bool {
        false
    }

    fn spawn(&self, _: &Path, _: &[String], _: &Path) -> Result<Box<dyn CoreProcess>, String> {
        Err("запуск недоступен".into())
    }

    fn kill_stale(&self, _: u32) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    #[derive(Default)]
    struct Flags {
        spawned: AtomicU32,
        installed: AtomicBool,
        present: AtomicBool,
        exited: Arc<AtomicBool>,
        killed: Arc<AtomicBool>,
        stale: Mutex<Vec<u32>>,
    }

    struct FakeProcess(Arc<AtomicBool>, Arc<AtomicBool>);
    impl CoreProcess for FakeProcess {
        fn pid(&self) -> u32 {
            777
        }
        fn has_exited(&mut self) -> bool {
            self.0.load(Ordering::SeqCst)
        }
        fn kill(&mut self) {
            self.1.store(true, Ordering::SeqCst);
        }
    }

    struct Host {
        flags: Arc<Flags>,
        elevated: bool,
        readiness: Readiness,
        install_fails: bool,
        spawn_fails: bool,
    }

    impl DesyncAdapter for Host {
        fn platform(&self) -> Platform {
            Platform::Windows
        }
        fn readiness(&self) -> Readiness {
            self.readiness.clone()
        }
        fn locate(&self) -> Result<PathBuf, String> {
            if self.flags.present.load(Ordering::SeqCst) {
                Ok(PathBuf::from("winws2.exe"))
            } else {
                Err("компонентов нет".into())
            }
        }
        fn install(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<(), PrepareError> {
            if self.install_fails {
                return Err(PrepareError {
                    reason: "загруженный файл не прошёл проверку, попробуйте позже".into(),
                    detail: "SHA-256 не совпал https://x.example/path/TOKEN".into(),
                });
            }
            progress(PrepareProgress { percent: 100 });
            self.flags.installed.store(true, Ordering::SeqCst);
            self.flags.present.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn has_privileges(&self) -> bool {
            self.elevated
        }
        fn spawn(&self, _: &Path, _: &[String], _: &Path) -> Result<Box<dyn CoreProcess>, String> {
            if self.spawn_fails {
                return Err("не удалось запустить".into());
            }
            self.flags.spawned.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(FakeProcess(
                self.flags.exited.clone(),
                self.flags.killed.clone(),
            )))
        }
        fn kill_stale(&self, pid: u32) {
            self.flags.stale.lock().unwrap().push(pid);
        }
    }

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ochistka-desync-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn profile() -> Option<Profile> {
        Some(Profile {
            id: "sites-discord",
            engine: Engine::Winws2,
            title: "Сайты и Discord",
            args: vec!["--fake-arg".into()],
        })
    }

    fn make(
        flags: &Arc<Flags>,
        host: impl FnOnce(Arc<Flags>) -> Host,
        installable: bool,
        vpn: bool,
    ) -> DesyncService {
        let mut s = DesyncService::new(
            temp_dir(),
            Box::new(host(flags.clone())),
            Box::new(move || vpn),
        );
        s.profile = profile();
        s.installable = installable;
        s.settle = (2, Duration::from_millis(1));
        s
    }

    fn good(flags: Arc<Flags>) -> Host {
        Host {
            flags,
            elevated: true,
            readiness: Readiness::Ready,
            install_fails: false,
            spawn_fails: false,
        }
    }

    #[test]
    fn nothing_is_pinned_or_enabled_until_hashes_and_profile_are_verified() {
        assert!(pinned().is_none(), "неподтверждённые файлы не скачиваем");
        for platform in [Platform::Windows, Platform::Linux, Platform::MacOs] {
            assert!(
                site_discord_profile(platform).is_none(),
                "профиль не придуман"
            );
        }
        let flags = Arc::new(Flags::default());
        let mut s = DesyncService::new(
            temp_dir(),
            Box::new(good(flags.clone())),
            Box::new(|| false),
        );
        s.settle = (1, Duration::from_millis(1));
        assert_eq!(s.start(&mut |_| {}).unwrap_err(), DesyncError::NoProfile);
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        assert_eq!(s.status().0, DesyncState::Error);
    }

    #[test]
    fn prepares_missing_components_then_starts_and_stops() {
        let flags = Arc::new(Flags::default());
        let s = make(&flags, good, true, false);
        let mut percents = vec![];
        s.start(&mut |p| percents.push(p.percent)).unwrap();
        assert!(flags.installed.load(Ordering::SeqCst));
        assert_eq!(percents.last(), Some(&100));
        assert_eq!(s.status().0, DesyncState::Running);
        assert!(s.pid_file().exists());
        s.stop();
        assert!(flags.killed.load(Ordering::SeqCst), "процесс остановлен");
        assert_eq!(s.status().0, DesyncState::Off);
        assert!(!s.pid_file().exists(), "ресурсы освобождены");
    }

    #[test]
    fn failed_file_check_starts_nothing_and_hides_secrets() {
        let flags = Arc::new(Flags::default());
        let s = make(
            &flags,
            |f| Host {
                install_fails: true,
                ..good(f)
            },
            true,
            false,
        );
        let err = s.start(&mut |_| {}).unwrap_err();
        assert!(matches!(err, DesyncError::Prepare(_)), "{err:?}");
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        let log = std::fs::read_to_string(s.dir.join("desync-prepare.log")).unwrap();
        assert!(!log.contains("TOKEN"), "{log}");
    }

    #[test]
    fn without_a_pinned_release_missing_components_are_not_downloaded() {
        let flags = Arc::new(Flags::default());
        let s = make(&flags, good, false, false);
        assert!(matches!(
            s.start(&mut |_| {}).unwrap_err(),
            DesyncError::NotPrepared(_)
        ));
        assert!(!flags.installed.load(Ordering::SeqCst));
    }

    #[test]
    fn second_start_does_not_create_a_second_instance() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let s = make(&flags, good, true, false);
        s.start(&mut |_| {}).unwrap();
        s.start(&mut |_| {}).unwrap();
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn spawn_failure_and_instant_exit_leave_everything_clean() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let s = make(
            &flags,
            |f| Host {
                spawn_fails: true,
                ..good(f)
            },
            true,
            false,
        );
        assert!(matches!(
            s.start(&mut |_| {}).unwrap_err(),
            DesyncError::Start(_)
        ));
        assert!(!s.pid_file().exists());
        assert_eq!(s.status().0, DesyncState::Error);

        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        flags.exited.store(true, Ordering::SeqCst);
        let s = make(&flags, good, true, false);
        let err = s.start(&mut |_| {}).unwrap_err().to_string();
        assert!(err.contains("сразу после запуска"), "{err}");
        assert!(!s.pid_file().exists());
    }

    #[test]
    fn admin_rights_are_requested_only_when_needed_and_refusal_is_clear() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let s = make(
            &flags,
            |f| Host {
                elevated: false,
                ..good(f)
            },
            true,
            false,
        );
        assert_eq!(s.start(&mut |_| {}).unwrap_err(), DesyncError::NeedsAdmin);
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn running_vpn_blocks_the_start() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let s = make(&flags, good, true, true);
        assert_eq!(s.start(&mut |_| {}).unwrap_err(), DesyncError::VpnRunning);
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn leftover_process_after_a_crash_is_stopped_on_start_and_on_next_launch() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let s = make(&flags, good, true, false);
        s.start(&mut |_| {}).unwrap();
        // Авария: приложение закрыто без stop (файл номера остался).
        let dir = s.dir.clone();
        std::mem::forget(s);
        let restarted = make(&flags, good, true, false);
        let again = DesyncService {
            dir: dir.clone(),
            ..restarted
        };
        again.recover_on_start();
        assert_eq!(*flags.stale.lock().unwrap(), vec![777]);
        assert!(!dir.join("desync.pid").exists());
    }

    #[test]
    fn unexpected_exit_is_noticed() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let s = make(&flags, good, true, false);
        s.start(&mut |_| {}).unwrap();
        flags.exited.store(true, Ordering::SeqCst);
        let (state, message) = s.status();
        assert_eq!(state, DesyncState::Error);
        assert!(message.contains("остановился"));
        assert!(!s.pid_file().exists());
    }

    fn strategies() -> Vec<Strategy> {
        vec![
            Strategy {
                id: "win-sites",
                engine: Engine::Winws2,
                covers: &[Coverage::Sites],
                args: vec![],
            },
            Strategy {
                id: "win-sites-discord",
                engine: Engine::Winws2,
                covers: &[Coverage::Sites, Coverage::Discord],
                args: vec![],
            },
            Strategy {
                id: "win-all",
                engine: Engine::Winws2,
                covers: &[
                    Coverage::Sites,
                    Coverage::Discord,
                    Coverage::Telegram,
                    Coverage::Games,
                ],
                args: vec![],
            },
            Strategy {
                id: "linux-sites-discord",
                engine: Engine::Nfqws2,
                covers: &[Coverage::Sites, Coverage::Discord],
                args: vec![],
            },
        ]
    }

    #[test]
    fn one_compatible_strategy_is_chosen_and_strategies_are_never_merged() {
        let all = strategies();
        let pick =
            |engine, wanted: &[Coverage]| select_strategy(&all, engine, wanted).map(|s| s.id);
        assert_eq!(pick(Engine::Winws2, &[Coverage::Sites]), Some("win-sites"));
        assert_eq!(
            pick(Engine::Winws2, &[Coverage::Sites, Coverage::Discord]),
            Some("win-sites-discord"),
            "наименьший лишний охват"
        );
        assert_eq!(
            pick(Engine::Winws2, &[Coverage::Discord, Coverage::Games]),
            Some("win-all")
        );
        // Telegram есть только в «win-all»; для Linux его нет вовсе: стратегии не складываем.
        assert_eq!(pick(Engine::Nfqws2, &[Coverage::Telegram]), None);
        assert_eq!(
            pick(Engine::Nfqws2, &[Coverage::Sites, Coverage::Discord]),
            Some("linux-sites-discord"),
            "стратегия чужого движка не подходит"
        );
    }

    #[test]
    fn each_platform_has_its_own_engine_and_macos_has_none() {
        assert_eq!(
            Engine::for_platform(Platform::Windows),
            Some(Engine::Winws2)
        );
        assert_eq!(Engine::for_platform(Platform::Linux), Some(Engine::Nfqws2));
        assert_eq!(Engine::for_platform(Platform::MacOs), None);
    }

    #[test]
    fn a_not_ready_adapter_never_starts_anything() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        for readiness in [
            Readiness::NotImplemented("не готов".into()),
            Readiness::Unsupported("нет на этой ОС".into()),
        ] {
            let r = readiness.clone();
            let s = make(
                &flags,
                move |f| Host {
                    readiness: r,
                    ..good(f)
                },
                true,
                false,
            );
            assert!(matches!(
                s.start(&mut |_| {}).unwrap_err(),
                DesyncError::Unavailable(_)
            ));
            assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn the_placeholder_adapter_is_honest_on_every_platform() {
        for (platform, unsupported) in [
            (Platform::Windows, false),
            (Platform::Linux, false),
            (Platform::MacOs, true),
        ] {
            let adapter = UnavailableAdapter::new(platform);
            assert_eq!(adapter.platform(), platform);
            match adapter.readiness() {
                Readiness::Unsupported(_) => assert!(unsupported),
                Readiness::NotImplemented(_) => assert!(!unsupported),
                Readiness::Ready => panic!("заглушка не может быть готова"),
            }
            assert!(adapter.locate().is_err());
            assert!(!adapter.has_privileges());
            let s = DesyncService::new(temp_dir(), Box::new(adapter), Box::new(|| false));
            assert!(s.start(&mut |_| {}).is_err());
            assert_eq!(s.status().0, DesyncState::Error);
        }
    }
}
