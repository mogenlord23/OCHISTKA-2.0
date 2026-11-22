//! Кнопка «Обход сайтов»: локальный прокси + системный прокси (`proxy-bypass`), общее право на
//! сетевой режим с VPN и простые тексты. Включение выключает VPN, включение VPN выключает обход.
//! Фоновых потоков и таймеров нет: пока обход выключен, ничего не работает.

use std::sync::{Arc, Mutex, MutexGuard, Weak};

use proxy_bypass::{BypassController, BypassError, SiteList};

use crate::desync::coordinator::{NetCoordinator, Owner};
use crate::vpn::service::ConflictGuard;

pub const TEXT_ON: &str = "Сайты открываются через обход";
pub const TEXT_OFF: &str = "Выключено";
const RESTORE_FAILED: &str = "Не удалось вернуть прежние настройки прокси";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteBypassState {
    Off,
    On,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteBypassStatus {
    pub state: SiteBypassState,
    /// «Сайты открываются через обход» / «Выключено» / «Ошибка: причина».
    pub message: String,
}

/// Останавливает VPN перед включением обхода; `Err` — VPN остановить не удалось.
pub type VpnStopper = Box<dyn Fn() -> Result<(), String> + Send + Sync>;
/// Список сайтов пользователя (из панели «Сайты»).
pub type UserSitesLoader = Box<dyn Fn() -> Vec<String> + Send + Sync>;

pub struct SiteBypassService {
    controller: Arc<BypassController>,
    coord: Arc<NetCoordinator>,
    user_sites: UserSitesLoader,
    vpn_stopper: Mutex<Option<VpnStopper>>,
    status: Mutex<SiteBypassStatus>,
    op: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn error_status(reason: &str) -> SiteBypassStatus {
    SiteBypassStatus {
        state: SiteBypassState::Error,
        message: format!("Ошибка: {reason}"),
    }
}

impl SiteBypassService {
    pub fn new(
        controller: Arc<BypassController>,
        coord: Arc<NetCoordinator>,
        user_sites: UserSitesLoader,
    ) -> Self {
        Self {
            controller,
            coord,
            user_sites,
            vpn_stopper: Mutex::new(None),
            status: Mutex::new(SiteBypassStatus {
                state: SiteBypassState::Off,
                message: TEXT_OFF.into(),
            }),
            op: Mutex::new(()),
        }
    }

    pub fn set_vpn_stopper(&self, stopper: VpnStopper) {
        *lock(&self.vpn_stopper) = Some(stopper);
    }

    pub fn status(&self) -> SiteBypassStatus {
        self.sync_death()
            .unwrap_or_else(|| lock(&self.status).clone())
    }

    /// Если прокси погиб после включения (настройки уже возвращены самим прокси), освобождает общее
    /// право на сетевой режим и возвращает текст для человека. Иначе `None`.
    fn sync_death(&self) -> Option<SiteBypassStatus> {
        let notice = self.controller.stopped_notice()?;
        self.coord.release(Owner::Desync);
        Some(self.set(SiteBypassStatus {
            state: SiteBypassState::Error,
            message: notice.into(),
        }))
    }

    pub fn is_on(&self) -> bool {
        self.controller.is_on()
    }

    fn set(&self, status: SiteBypassStatus) -> SiteBypassStatus {
        *lock(&self.status) = status.clone();
        status
    }

    /// Включает обход. Сначала выключается работающий VPN (через переданную функцию).
    pub fn start(&self) -> SiteBypassStatus {
        let Ok(_op) = self.op.try_lock() else {
            return error_status("подождите, идёт переключение");
        };
        self.sync_death();
        if self.controller.is_on() {
            return self.set(SiteBypassStatus {
                state: SiteBypassState::On,
                message: TEXT_ON.into(),
            });
        }
        if let Some(stop) = lock(&self.vpn_stopper).as_ref() {
            if stop().is_err() {
                return self.set(error_status("не удалось выключить VPN"));
            }
        }
        if let Err(denied) = self.coord.begin(Owner::Desync) {
            return self.set(error_status(&denied.to_string()));
        }
        let list = SiteList::builtin_with(&(self.user_sites)());
        match self.controller.enable(list) {
            Ok(()) => {
                self.coord.activate(Owner::Desync);
                self.set(SiteBypassStatus {
                    state: SiteBypassState::On,
                    message: TEXT_ON.into(),
                })
            }
            Err(err) => {
                match err {
                    BypassError::Restore(_) => self.coord.mark_unclean(Owner::Desync),
                    _ => self.coord.release(Owner::Desync),
                }
                self.set(error_status(&err.to_string()))
            }
        }
    }

    /// Выключает обход и возвращает прежний системный прокси. Если вернуть не удалось —
    /// «Ошибка», а не «Выключено»; повторное нажатие повторяет попытку.
    pub fn stop(&self) -> SiteBypassStatus {
        let Ok(_op) = self.op.try_lock() else {
            return error_status("подождите, идёт переключение");
        };
        self.stop_locked()
    }

    fn stop_locked(&self) -> SiteBypassStatus {
        self.coord.begin_stop(Owner::Desync);
        match self.controller.disable() {
            Ok(()) => {
                self.coord.release(Owner::Desync);
                self.set(SiteBypassStatus {
                    state: SiteBypassState::Off,
                    message: TEXT_OFF.into(),
                })
            }
            Err(_) => {
                self.coord.mark_unclean(Owner::Desync);
                self.set(error_status(RESTORE_FAILED))
            }
        }
    }

    /// При запуске программы: вернуть прокси, если прошлый запуск закончился аварийно.
    pub fn recover_on_start(&self) -> SiteBypassStatus {
        match self.controller.recover() {
            Ok(()) => self.status(),
            Err(_) => {
                self.coord.mark_unclean(Owner::Desync);
                self.set(error_status(RESTORE_FAILED))
            }
        }
    }

    /// При выходе из программы: выключить обход и вернуть системный прокси.
    pub fn shutdown(&self) -> SiteBypassStatus {
        if self.controller.is_on() || self.controller.has_backup_file() {
            let _op = lock(&self.op);
            return self.stop_locked();
        }
        self.status()
    }
}

/// Охрана для VPN: перед включением VPN обход выключается; если не вышло — VPN не включается.
pub fn vpn_conflict_guard(service: Weak<SiteBypassService>) -> ConflictGuard {
    Box::new(move || {
        let service = service.upgrade()?;
        service.sync_death();
        if !service.is_on() {
            return None;
        }
        let status = service.stop();
        (status.state == SiteBypassState::Error).then_some(status.message)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxy_bypass::{Snapshot, SystemProxyPort};
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    struct Fake {
        current: Mutex<Vec<(String, String)>>,
        fail_restore: AtomicBool,
    }

    impl SystemProxyPort for Fake {
        fn unsupported(&self) -> Option<String> {
            None
        }
        fn snapshot(&self) -> Result<Snapshot, String> {
            Ok(Snapshot(self.current.lock().unwrap().clone()))
        }
        fn set_local(&self, port: u16) -> Result<(), String> {
            *self.current.lock().unwrap() = vec![("p".into(), format!("127.0.0.1:{port}"))];
            Ok(())
        }
        fn restore(&self, s: &Snapshot) -> Result<(), String> {
            if self.fail_restore.load(Ordering::SeqCst) {
                return Err("нет доступа".into());
            }
            *self.current.lock().unwrap() = s.0.clone();
            Ok(())
        }
    }

    fn service(name: &str) -> (Arc<SiteBypassService>, Arc<Fake>, Arc<NetCoordinator>) {
        let fake = Arc::new(Fake {
            current: Mutex::new(vec![("p".into(), "prev".into())]),
            fail_restore: AtomicBool::new(false),
        });
        let dir = std::env::temp_dir().join(format!("site-bypass-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let controller = Arc::new(BypassController::new(fake.clone(), dir.join("backup.txt")));
        let coord = Arc::new(NetCoordinator::new());
        let svc = Arc::new(SiteBypassService::new(
            controller,
            coord.clone(),
            Box::new(|| vec!["example.com".to_string()]),
        ));
        (svc, fake, coord)
    }

    #[test]
    fn on_off_with_plain_texts_and_the_previous_proxy_returned() {
        let (svc, fake, coord) = service("onoff");
        assert_eq!(svc.status().message, "Выключено");
        let on = svc.start();
        assert_eq!(on.state, SiteBypassState::On);
        assert_eq!(on.message, "Сайты открываются через обход");
        assert!(fake.current.lock().unwrap()[0].1.starts_with("127.0.0.1:"));
        let off = svc.stop();
        assert_eq!(
            (off.state, off.message.as_str()),
            (SiteBypassState::Off, "Выключено")
        );
        assert_eq!(fake.current.lock().unwrap()[0].1, "prev");
        assert_eq!(coord.mode(), Some(crate::desync::coordinator::Mode::Idle));
    }

    #[test]
    fn enabling_stops_the_vpn_first_and_a_vpn_that_will_not_stop_blocks_it() {
        let (svc, fake, _) = service("vpnstop");
        let calls = Arc::new(AtomicU32::new(0));
        let c = calls.clone();
        svc.set_vpn_stopper(Box::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        assert_eq!(svc.start().state, SiteBypassState::On);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        svc.stop();
        svc.set_vpn_stopper(Box::new(|| Err("занят".into())));
        let blocked = svc.start();
        assert_eq!(blocked.state, SiteBypassState::Error);
        assert_eq!(blocked.message, "Ошибка: не удалось выключить VPN");
        assert_eq!(
            fake.current.lock().unwrap()[0].1,
            "prev",
            "прокси не тронут"
        );
    }

    #[test]
    fn a_running_vpn_owning_the_mode_refuses_the_bypass_with_one_reason() {
        let (svc, _, coord) = service("vpnowns");
        coord.begin(Owner::Vpn).unwrap();
        coord.activate(Owner::Vpn);
        let status = svc.start();
        assert_eq!(status.state, SiteBypassState::Error);
        assert!(status.message.starts_with("Ошибка: "), "{}", status.message);
        assert!(!svc.is_on());
    }

    #[test]
    fn enabling_the_vpn_turns_the_bypass_off_through_the_guard() {
        let (svc, fake, _) = service("guard");
        let guard = vpn_conflict_guard(Arc::downgrade(&svc));
        assert!(guard().is_none(), "обход выключен: VPN можно включать");
        svc.start();
        assert!(guard().is_none(), "обход выключился");
        assert!(!svc.is_on());
        assert_eq!(fake.current.lock().unwrap()[0].1, "prev");
        // Не удалось вернуть прокси: VPN не включается, причина одна.
        svc.start();
        fake.fail_restore.store(true, Ordering::SeqCst);
        let refusal = guard().expect("отказ");
        assert!(refusal.starts_with("Ошибка: "), "{refusal}");
    }

    #[test]
    fn a_failed_restore_is_an_error_and_a_second_press_retries() {
        let (svc, fake, coord) = service("retry");
        svc.start();
        fake.fail_restore.store(true, Ordering::SeqCst);
        let err = svc.stop();
        assert_eq!(err.state, SiteBypassState::Error);
        assert_eq!(
            err.message,
            "Ошибка: Не удалось вернуть прежние настройки прокси"
        );
        assert_eq!(
            coord.mode(),
            Some(crate::desync::coordinator::Mode::Unclean(Owner::Desync))
        );
        fake.fail_restore.store(false, Ordering::SeqCst);
        assert_eq!(svc.stop().state, SiteBypassState::Off);
        assert_eq!(fake.current.lock().unwrap()[0].1, "prev");
    }

    #[test]
    fn recover_on_start_returns_the_proxy_left_after_a_crash_and_shutdown_stops_it() {
        let (svc, fake, _) = service("recover");
        svc.start();
        // «Падение»: новый сервис над тем же файлом.
        let dir = std::env::temp_dir().join(format!("site-bypass-{}-recover", std::process::id()));
        let again = SiteBypassService::new(
            Arc::new(BypassController::new(fake.clone(), dir.join("backup.txt"))),
            Arc::new(NetCoordinator::new()),
            Box::new(Vec::new),
        );
        assert_eq!(again.recover_on_start().state, SiteBypassState::Off);
        assert_eq!(fake.current.lock().unwrap()[0].1, "prev");
        drop(again);
        let (svc2, fake2, _) = service("shutdown");
        svc2.start();
        assert_eq!(svc2.shutdown().state, SiteBypassState::Off);
        assert_eq!(fake2.current.lock().unwrap()[0].1, "prev");
        drop(svc);
    }
}
