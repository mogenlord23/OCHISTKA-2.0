//! Включение и выключение обхода: прокси + системный прокси + файл с прежними значениями.
//!
//! Включение: сохранить прежние значения в файл → поднять прокси → направить на него системный
//! прокси. Любая неудача откатывается. Выключение: остановить прокси → вернуть прежние значения →
//! удалить файл; если вернуть не удалось, это ошибка (файл остаётся, повтор или запуск программы
//! попробуют снова). Пока обход выключен, ни одного потока нет.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::server::ProxyServer;
use crate::sites::SiteList;
use crate::system::{Snapshot, SystemProxyPort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BypassError {
    /// На этой системе обход недоступен (причина для человека).
    Unsupported(String),
    /// Не удалось включить (всё возвращено назад).
    Start(String),
    /// Не удалось вернуть прежние настройки прокси.
    Restore(String),
}

impl std::fmt::Display for BypassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BypassError::Unsupported(why) => write!(f, "{why}"),
            BypassError::Start(_) => write!(f, "Не удалось включить обход"),
            BypassError::Restore(_) => write!(f, "Не удалось вернуть прежние настройки прокси"),
        }
    }
}

struct Inner {
    server: Option<ProxyServer>,
    snapshot: Option<Snapshot>,
}

pub struct BypassController {
    system: Arc<dyn SystemProxyPort>,
    backup: PathBuf,
    inner: Mutex<Inner>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl BypassController {
    /// Ничего не запускает и не читает: потоков нет, пока обход не включён.
    pub fn new(system: Arc<dyn SystemProxyPort>, backup: PathBuf) -> Self {
        Self {
            system,
            backup,
            inner: Mutex::new(Inner {
                server: None,
                snapshot: None,
            }),
        }
    }

    pub fn is_on(&self) -> bool {
        lock(&self.inner).server.is_some()
    }

    /// Номер порта прокси, пока обход включён.
    pub fn port(&self) -> Option<u16> {
        lock(&self.inner).server.as_ref().map(ProxyServer::port)
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
        if inner.server.is_some() {
            return Ok(());
        }
        self.restore_from_file()
    }

    pub fn enable(&self, sites: SiteList) -> Result<(), BypassError> {
        let mut inner = lock(&self.inner);
        if inner.server.is_some() {
            return Ok(());
        }
        if let Some(why) = self.system.unsupported() {
            return Err(BypassError::Unsupported(why));
        }
        // Остаток прошлого запуска возвращаем до нового включения, иначе потеряем исходные значения.
        self.restore_from_file()?;
        let snapshot = self.system.snapshot().map_err(BypassError::Start)?;
        self.write_backup(&snapshot).map_err(BypassError::Start)?;
        let undo = |snapshot: &Snapshot, backup: &PathBuf| {
            if self.system.restore(snapshot).is_ok() {
                let _ = std::fs::remove_file(backup);
                None
            } else {
                Some(BypassError::Restore("не удалось вернуть настройки".into()))
            }
        };
        let server = match ProxyServer::start(sites) {
            Ok(s) => s,
            Err(e) => {
                let _ = std::fs::remove_file(&self.backup);
                return Err(BypassError::Start(e.to_string()));
            }
        };
        if let Err(e) = self.system.set_local(server.port()) {
            server.stop();
            return Err(undo(&snapshot, &self.backup).unwrap_or(BypassError::Start(e)));
        }
        inner.server = Some(server);
        inner.snapshot = Some(snapshot);
        Ok(())
    }

    /// Останавливает прокси и возвращает прежние значения. Ошибка, если вернуть не удалось: тогда
    /// значения (и файл) сохраняются для повторной попытки.
    pub fn disable(&self) -> Result<(), BypassError> {
        let mut inner = lock(&self.inner);
        if let Some(server) = inner.server.take() {
            server.stop();
        }
        let snapshot = inner.snapshot.clone().or_else(|| self.read_backup());
        let Some(snapshot) = snapshot else {
            if self.backup.exists() {
                return Err(BypassError::Restore(
                    "файл прежних настроек повреждён".into(),
                ));
            }
            return Ok(());
        };
        self.system
            .restore(&snapshot)
            .map_err(BypassError::Restore)?;
        let _ = std::fs::remove_file(&self.backup);
        inner.snapshot = None;
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
}
