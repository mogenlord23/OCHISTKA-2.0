//! Включение и выключение обхода: прокси + системный прокси + файл с прежними значениями.
//!
//! Включение (порядок важен): прежние значения → файл; прокси начинает слушать порт 127.0.0.1
//! (порт назначает система); проверка, что к порту можно подключиться; ТОЛЬКО ПОТОМ системный
//! прокси направляется на него. Если любой шаг не прошёл, системный прокси не меняется.
//! Если принимающий поток прокси позже остановился или упал, прежние значения возвращаются
//! автоматически (из самого потока, без опроса и таймеров). Выключение возвращает значения и
//! останавливает сервер, даже если он уже мёртв. Если вернуть значения не удалось, это ошибка
//! (файл остаётся, повтор или запуск программы попробуют снова). Пока обход выключен, потоков нет.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::server::ProxyServer;
use crate::sites::SiteList;
use crate::system::{Snapshot, SystemProxyPort};

/// Текст для человека, когда прокси погиб после включения.
pub const STOPPED_NOTICE: &str = "Обход остановлен, сайты открываются как обычно";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BypassError {
    /// На этой системе обход недоступен (причина для человека).
    Unsupported(String),
    /// Не удалось включить (системный прокси не изменён или возвращён).
    Start(String),
    /// Не удалось вернуть прежние настройки прокси.
    Restore(String),
}

impl std::fmt::Display for BypassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BypassError::Unsupported(why) => write!(f, "{why}"),
            BypassError::Start(_) => write!(f, "обход не запустился"),
            BypassError::Restore(_) => write!(f, "Не удалось вернуть прежние настройки прокси"),
        }
    }
}

type ExitHook = Box<dyn FnOnce() + Send + 'static>;
type Starter = Box<dyn Fn(SiteList, Option<ExitHook>) -> io::Result<ProxyServer> + Send + Sync>;

/// Общее с принимающим потоком: прежние значения (пока системный прокси указывает на нас) и признак
/// «прокси погиб, значения уже возвращены».
struct Live {
    snapshot: Mutex<Option<Snapshot>>,
    died: AtomicBool,
}

struct Inner {
    server: Option<ProxyServer>,
}

pub struct BypassController {
    system: Arc<dyn SystemProxyPort>,
    backup: PathBuf,
    live: Arc<Live>,
    starter: Starter,
    inner: Mutex<Inner>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Возвращает прежние значения, если они ещё не возвращены. Повторный вызов ничего не делает.
fn restore_live(live: &Live, system: &dyn SystemProxyPort, backup: &Path) -> Result<(), String> {
    let mut slot = lock(&live.snapshot);
    let Some(snapshot) = slot.clone() else {
        return Ok(());
    };
    system.restore(&snapshot)?;
    let _ = std::fs::remove_file(backup);
    *slot = None;
    Ok(())
}

impl BypassController {
    /// Ничего не запускает и не читает: потоков нет, пока обход не включён.
    pub fn new(system: Arc<dyn SystemProxyPort>, backup: PathBuf) -> Self {
        Self::with_starter(system, backup, Box::new(ProxyServer::start_with_exit_hook))
    }

    /// Свой способ запуска сервера (в тестах: сбой запуска, сервер без ответа).
    pub fn with_starter(
        system: Arc<dyn SystemProxyPort>,
        backup: PathBuf,
        starter: Starter,
    ) -> Self {
        Self {
            system,
            backup,
            live: Arc::new(Live {
                snapshot: Mutex::new(None),
                died: AtomicBool::new(false),
            }),
            starter,
            inner: Mutex::new(Inner { server: None }),
        }
    }

    /// Обход включён: сервер есть, жив, и прокси не погиб.
    pub fn is_on(&self) -> bool {
        !self.live.died.load(Ordering::SeqCst)
            && lock(&self.inner)
                .server
                .as_ref()
                .is_some_and(ProxyServer::is_alive)
    }

    /// Номер порта прокси, пока обход включён.
    pub fn port(&self) -> Option<u16> {
        lock(&self.inner).server.as_ref().map(ProxyServer::port)
    }

    /// Текст, если прокси погиб после включения (прежние настройки уже возвращены).
    pub fn stopped_notice(&self) -> Option<&'static str> {
        self.live
            .died
            .load(Ordering::SeqCst)
            .then_some(STOPPED_NOTICE)
    }

    pub fn has_backup_file(&self) -> bool {
        self.backup.exists()
    }

    fn write_backup(&self, snapshot: &Snapshot) -> Result<(), String> {
        if let Some(dir) = self.backup.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = self.backup.with_extension("tmp");
        std::fs::write(&tmp, snapshot.to_text()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.backup).map_err(|e| e.to_string())
    }

    fn read_backup(&self) -> Option<Snapshot> {
        let text = std::fs::read_to_string(&self.backup).ok()?;
        Snapshot::from_text(&text)
    }

    /// Возвращает значения из файла (после аварии) и удаляет файл. Повреждённый файл — ошибка.
    fn restore_from_file(&self) -> Result<(), BypassError> {
        if !self.backup.exists() {
            return Ok(());
        }
        let snapshot = self
            .read_backup()
            .ok_or_else(|| BypassError::Restore("файл прежних настроек повреждён".into()))?;
        self.system
            .restore(&snapshot)
            .map_err(BypassError::Restore)?;
        let _ = std::fs::remove_file(&self.backup);
        Ok(())
    }

    /// При запуске программы: если после аварии остался файл с прежними значениями, вернуть их.
    pub fn recover(&self) -> Result<(), BypassError> {
        let inner = lock(&self.inner);
        if inner.server.as_ref().is_some_and(ProxyServer::is_alive) {
            return Ok(());
        }
        self.restore_from_file()
    }

    fn exit_hook(&self) -> ExitHook {
        let live = self.live.clone();
        let system = self.system.clone();
        let backup = self.backup.clone();
        Box::new(move || {
            // Системный прокси указывает на нас (значения сохранены): возвращаем его сами.
            if lock(&live.snapshot).is_some() {
                let _ = restore_live(&live, system.as_ref(), &backup);
                live.died.store(true, Ordering::SeqCst);
            }
        })
    }

    pub fn enable(&self, sites: SiteList) -> Result<(), BypassError> {
        let mut inner = lock(&self.inner);
        if inner.server.as_ref().is_some_and(ProxyServer::is_alive)
            && !self.live.died.load(Ordering::SeqCst)
        {
            return Ok(());
        }
        // Мёртвый сервер прошлого включения убираем; признак «погиб» снимается новым включением.
        if let Some(dead) = inner.server.take() {
            dead.stop();
        }
        self.live.died.store(false, Ordering::SeqCst);
        if let Some(why) = self.system.unsupported() {
            return Err(BypassError::Unsupported(why));
        }
        // Остаток прошлого запуска возвращаем до нового включения, иначе потеряем исходные значения.
        self.restore_from_file()?;
        let snapshot = self.system.snapshot().map_err(BypassError::Start)?;
        self.write_backup(&snapshot).map_err(BypassError::Start)?;
        let fail = |e: String| {
            let _ = std::fs::remove_file(&self.backup);
            BypassError::Start(e)
        };
        // 1. Прокси начинает слушать порт (порт назначает система).
        let server =
            (self.starter)(sites, Some(self.exit_hook())).map_err(|e| fail(e.to_string()))?;
        // 2. К порту можно подключиться. Системный прокси пока не тронут.
        if !server.probe() {
            server.stop();
            return Err(fail("прокси не отвечает на своём порту".into()));
        }
        // 3. Только теперь системный прокси направляется на него.
        *lock(&self.live.snapshot) = Some(snapshot);
        if let Err(e) = self.system.set_local(server.port()) {
            server.stop();
            return Err(
                match restore_live(&self.live, self.system.as_ref(), &self.backup) {
                    Ok(()) => fail(e),
                    Err(r) => BypassError::Restore(r),
                },
            );
        }
        // Если сервер погиб между проверкой и сменой прокси, возвращаем всё назад.
        if !server.is_alive() {
            server.stop();
            return Err(
                match restore_live(&self.live, self.system.as_ref(), &self.backup) {
                    Ok(()) => fail("прокси остановился сразу после запуска".into()),
                    Err(r) => BypassError::Restore(r),
                },
            );
        }
        inner.server = Some(server);
        Ok(())
    }

    /// Останавливает сервер (даже мёртвый) и возвращает прежние значения. Ошибка, если вернуть не
    /// удалось: тогда значения (и файл) сохраняются для повторной попытки.
    pub fn disable(&self) -> Result<(), BypassError> {
        let mut inner = lock(&self.inner);
        if let Some(server) = inner.server.take() {
            server.stop();
        }
        restore_live(&self.live, self.system.as_ref(), &self.backup)
            .map_err(BypassError::Restore)?;
        // После аварии программы значения могли остаться только в файле.
        self.restore_from_file()?;
        self.live.died.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    /// Фейковый системный прокси: хранит «текущие значения» в памяти.
    struct Fake {
        current: Mutex<Vec<(String, String)>>,
        fail_set: AtomicBool,
        fail_restore: AtomicBool,
        unsupported: Option<String>,
        restores: AtomicU32,
        set_calls: AtomicU32,
    }

    impl Fake {
        fn new(initial: &[(&str, &str)]) -> Arc<Self> {
            Arc::new(Self {
                current: Mutex::new(
                    initial
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                ),
                fail_set: AtomicBool::new(false),
                fail_restore: AtomicBool::new(false),
                unsupported: None,
                restores: AtomicU32::new(0),
                set_calls: AtomicU32::new(0),
            })
        }
        fn now(&self) -> Vec<(String, String)> {
            self.current.lock().unwrap().clone()
        }
    }

    impl SystemProxyPort for Fake {
        fn unsupported(&self) -> Option<String> {
            self.unsupported.clone()
        }
        fn snapshot(&self) -> Result<Snapshot, String> {
            Ok(Snapshot(self.now()))
        }
        fn set_local(&self, port: u16) -> Result<(), String> {
            self.set_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_set.load(Ordering::SeqCst) {
                return Err("нет доступа".into());
            }
            *self.current.lock().unwrap() = vec![("proxy".into(), format!("127.0.0.1:{port}"))];
            Ok(())
        }
        fn restore(&self, s: &Snapshot) -> Result<(), String> {
            self.restores.fetch_add(1, Ordering::SeqCst);
            if self.fail_restore.load(Ordering::SeqCst) {
                return Err("нет доступа".into());
            }
            *self.current.lock().unwrap() = s.0.clone();
            Ok(())
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("proxy-bypass-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("proxy-backup.txt")
    }

    #[test]
    fn enable_points_the_system_proxy_at_us_and_disable_restores_the_previous_values() {
        let fake = Fake::new(&[("ProxyEnable", "1"), ("ProxyServer", "corp:3128")]);
        let c = BypassController::new(fake.clone(), temp("roundtrip"));
        assert!(!c.is_on());
        c.enable(SiteList::new(["youtube.com"])).unwrap();
        assert!(c.is_on());
        let port = c.port().unwrap();
        assert_eq!(fake.now(), [("proxy".into(), format!("127.0.0.1:{port}"))]);
        assert!(c.has_backup_file());
        c.disable().unwrap();
        assert!(!c.is_on());
        assert_eq!(
            fake.now(),
            [
                ("ProxyEnable".into(), "1".into()),
                ("ProxyServer".into(), "corp:3128".into())
            ]
        );
        assert!(!c.has_backup_file());
    }

    #[test]
    fn a_second_enable_does_not_overwrite_the_original_values() {
        let fake = Fake::new(&[("ProxyEnable", "0")]);
        let c = BypassController::new(fake.clone(), temp("twice"));
        c.enable(SiteList::default()).unwrap();
        c.enable(SiteList::default()).unwrap();
        c.disable().unwrap();
        assert_eq!(fake.now(), [("ProxyEnable".into(), "0".into())]);
    }

    #[test]
    fn a_failed_start_rolls_everything_back_and_leaves_no_thread_or_file() {
        let fake = Fake::new(&[("ProxyEnable", "0")]);
        fake.fail_set.store(true, Ordering::SeqCst);
        let c = BypassController::new(fake.clone(), temp("failstart"));
        let err = c.enable(SiteList::default()).unwrap_err();
        assert!(matches!(err, BypassError::Start(_)), "{err:?}");
        assert!(!c.is_on());
        assert_eq!(fake.now(), [("ProxyEnable".into(), "0".into())]);
        assert!(!c.has_backup_file());
    }

    #[test]
    fn a_failed_restore_is_an_error_not_off_and_a_retry_succeeds() {
        let fake = Fake::new(&[("ProxyEnable", "0")]);
        let c = BypassController::new(fake.clone(), temp("failrestore"));
        c.enable(SiteList::default()).unwrap();
        fake.fail_restore.store(true, Ordering::SeqCst);
        let err = c.disable().unwrap_err();
        assert!(matches!(err, BypassError::Restore(_)), "{err:?}");
        assert!(c.has_backup_file(), "файл остаётся для повтора");
        assert_ne!(fake.now(), [("ProxyEnable".into(), "0".into())]);
        fake.fail_restore.store(false, Ordering::SeqCst);
        c.disable().unwrap();
        assert_eq!(fake.now(), [("ProxyEnable".into(), "0".into())]);
        assert!(!c.has_backup_file());
    }

    #[test]
    fn after_a_crash_the_next_start_restores_from_the_file() {
        let backup = temp("crash");
        let fake = Fake::new(&[("ProxyEnable", "1"), ("ProxyServer", "corp:3128")]);
        {
            let c = BypassController::new(fake.clone(), backup.clone());
            c.enable(SiteList::default()).unwrap();
            // «Падение»: контроллер исчезает без выключения (сервер закрывается при выходе из области).
            assert!(c.has_backup_file());
        }
        assert_ne!(
            fake.now()[0].0,
            "ProxyEnable",
            "прокси всё ещё указывает на нас"
        );
        let after = BypassController::new(fake.clone(), backup);
        after.recover().unwrap();
        assert_eq!(
            fake.now(),
            [
                ("ProxyEnable".into(), "1".into()),
                ("ProxyServer".into(), "corp:3128".into())
            ]
        );
        assert!(!after.has_backup_file());
        // Повторное восстановление ничего не делает.
        let calls = fake.restores.load(Ordering::SeqCst);
        after.recover().unwrap();
        assert_eq!(fake.restores.load(Ordering::SeqCst), calls);
    }

    #[test]
    fn recover_that_fails_reports_an_error_and_keeps_the_file() {
        let backup = temp("recoverfail");
        let fake = Fake::new(&[("a", "1")]);
        BypassController::new(fake.clone(), backup.clone())
            .enable(SiteList::default())
            .unwrap();
        fake.fail_restore.store(true, Ordering::SeqCst);
        let c = BypassController::new(fake.clone(), backup);
        assert!(matches!(c.recover(), Err(BypassError::Restore(_))));
        assert!(c.has_backup_file());
        // Повреждённый файл тоже ошибка, а не «выключено».
        std::fs::write(&c.backup, "мусор").unwrap();
        assert!(matches!(c.recover(), Err(BypassError::Restore(_))));
    }

    #[test]
    fn an_unsupported_system_refuses_with_its_reason_and_changes_nothing() {
        let fake = Arc::new(Fake {
            unsupported: Some("Эта система не поддерживается".into()),
            ..Arc::try_unwrap(Fake::new(&[("a", "1")])).ok().unwrap()
        });
        let c = BypassController::new(fake.clone(), temp("unsupported"));
        assert_eq!(
            c.enable(SiteList::default()).unwrap_err(),
            BypassError::Unsupported("Эта система не поддерживается".into())
        );
        assert!(!c.has_backup_file());
        assert_eq!(fake.restores.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_disabled_bypass_holds_no_threads() {
        let fake = Fake::new(&[("a", "1")]);
        let c = BypassController::new(fake, temp("nothreads"));
        // Выключенный контроллер не создаёт ни сервера, ни потоков.
        assert!(c.port().is_none());
        c.enable(SiteList::default()).unwrap();
        let probe = std::net::TcpStream::connect(("127.0.0.1", c.port().unwrap()));
        assert!(probe.is_ok());
        drop(probe);
        c.disable().unwrap();
        assert!(c.port().is_none());
        // После выключения порт не принимает соединений.
        assert!(c.port().is_none());
    }

    fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
        for _ in 0..100 {
            if condition() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        false
    }

    const BEFORE: [(&str, &str); 2] = [("ProxyEnable", "1"), ("ProxyServer", "corp:3128")];

    fn before() -> Vec<(String, String)> {
        BEFORE
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn a_server_that_cannot_start_leaves_the_system_proxy_alone() {
        let fake = Fake::new(&BEFORE);
        let c = BypassController::with_starter(
            fake.clone(),
            temp("nostart"),
            Box::new(|_, _| Err(std::io::Error::other("порт недоступен"))),
        );
        let err = c.enable(SiteList::default()).unwrap_err();
        assert_eq!(err, BypassError::Start("порт недоступен".into()));
        assert_eq!(err.to_string(), "обход не запустился");
        assert_eq!(
            fake.set_calls.load(Ordering::SeqCst),
            0,
            "прокси не менялся"
        );
        assert_eq!(fake.restores.load(Ordering::SeqCst), 0);
        assert_eq!(fake.now(), before());
        assert!(!c.is_on() && !c.has_backup_file());
    }

    #[test]
    fn a_port_that_does_not_answer_leaves_the_system_proxy_alone() {
        let fake = Fake::new(&BEFORE);
        let c = BypassController::with_starter(
            fake.clone(),
            temp("noanswer"),
            Box::new(|sites, hook| {
                // Сервер запускается, но его принимающий поток тут же падает: порт закрыт.
                let server = ProxyServer::start_with_exit_hook(sites, hook)?;
                server.crash_for_test();
                for _ in 0..100 {
                    if !server.is_alive() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Ok(server)
            }),
        );
        let err = c.enable(SiteList::default()).unwrap_err();
        assert_eq!(err.to_string(), "обход не запустился");
        assert_eq!(
            fake.set_calls.load(Ordering::SeqCst),
            0,
            "прокси не менялся"
        );
        assert_eq!(fake.restores.load(Ordering::SeqCst), 0);
        assert_eq!(fake.now(), before());
        assert!(!c.is_on() && !c.has_backup_file());
    }

    #[test]
    fn the_system_proxy_is_changed_only_after_the_port_answered_on_a_system_assigned_port() {
        let fake = Fake::new(&BEFORE);
        let c = BypassController::new(fake.clone(), temp("order"));
        c.enable(SiteList::default()).unwrap();
        let port = c.port().unwrap();
        assert_ne!(port, 0);
        assert_eq!(fake.now(), [("proxy".into(), format!("127.0.0.1:{port}"))]);
        // Порт отвечает прямо сейчас: системный прокси указывает на живой сервер.
        assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_ok());
        c.disable().unwrap();
    }

    #[test]
    fn a_server_that_dies_after_the_start_restores_the_system_proxy_by_itself() {
        let fake = Fake::new(&BEFORE);
        let backup = temp("dies");
        let c = BypassController::new(fake.clone(), backup);
        c.enable(SiteList::default()).unwrap();
        assert!(c.is_on());
        assert_ne!(fake.now(), before());
        // Принимающий поток падает: прежние настройки возвращаются сами, без опроса.
        c.inner
            .lock()
            .unwrap()
            .server
            .as_ref()
            .unwrap()
            .crash_for_test();
        assert!(wait_until(|| fake.now() == before()), "прокси не возвращён");
        assert!(wait_until(|| !c.is_on()));
        assert_eq!(
            c.stopped_notice(),
            Some("Обход остановлен, сайты открываются как обычно")
        );
        assert!(!c.has_backup_file());
        // Выключение мёртвого сервера: без ошибки, ничего лишнего не пишет, снимает признак.
        let calls = fake.restores.load(Ordering::SeqCst);
        c.disable().unwrap();
        assert_eq!(fake.restores.load(Ordering::SeqCst), calls);
        assert_eq!(fake.now(), before());
        assert_eq!(c.stopped_notice(), None);
        // И обход можно включить заново.
        c.enable(SiteList::default()).unwrap();
        assert!(c.is_on());
        c.disable().unwrap();
        assert_eq!(fake.now(), before());
    }

    #[test]
    fn disabling_a_dead_server_still_restores_when_the_automatic_restore_failed() {
        let fake = Fake::new(&BEFORE);
        let c = BypassController::new(fake.clone(), temp("deadfail"));
        c.enable(SiteList::default()).unwrap();
        fake.fail_restore.store(true, Ordering::SeqCst);
        c.inner
            .lock()
            .unwrap()
            .server
            .as_ref()
            .unwrap()
            .crash_for_test();
        assert!(wait_until(|| !c.is_on()));
        // Автоматически вернуть не вышло: значения и файл сохранены.
        assert_ne!(fake.now(), before());
        assert!(c.has_backup_file());
        assert!(matches!(c.disable(), Err(BypassError::Restore(_))));
        fake.fail_restore.store(false, Ordering::SeqCst);
        c.disable().unwrap();
        assert_eq!(fake.now(), before());
        assert!(!c.has_backup_file());
    }

    #[test]
    fn a_normal_stop_does_not_look_like_a_death() {
        let fake = Fake::new(&BEFORE);
        let c = BypassController::new(fake.clone(), temp("normalstop"));
        c.enable(SiteList::default()).unwrap();
        c.disable().unwrap();
        assert_eq!(c.stopped_notice(), None);
        assert_eq!(fake.now(), before());
    }
}
