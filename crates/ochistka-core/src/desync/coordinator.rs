//! Единое право на сетевой режим: VPN и перехват трафика (Desync) не работают одновременно.
//!
//! Резервирование — один атомарный переход под короткой блокировкой, до любых побочных
//! эффектов запуска. Блокировка не удерживается во время загрузки, запуска процесса,
//! сетевых проверок и остановки: сервисы вызывают координатор только для смены состояния.

use std::sync::{Mutex, MutexGuard};

use thiserror::Error;

/// Кто владеет сетевым режимом.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Vpn,
    Desync,
}

impl Owner {
    fn title(self) -> &'static str {
        match self {
            Owner::Vpn => "VPN",
            Owner::Desync => "Обход сайтов",
        }
    }
}

/// Состояние владения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    /// Идёт запуск: право уже занято, но механизм ещё не работает.
    Starting(Owner),
    Active(Owner),
    /// Идёт остановка и очистка.
    Stopping(Owner),
    /// Откат не завершён (например, не удалось вернуть прокси): право не освобождено.
    Unclean(Owner),
}

impl Mode {
    fn owner(self) -> Option<Owner> {
        match self {
            Mode::Idle => None,
            Mode::Starting(o) | Mode::Active(o) | Mode::Stopping(o) | Mode::Unclean(o) => Some(o),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Denied {
    #[error("Сейчас запускается или работает другой режим ({0}). Сначала остановите его")]
    Busy(&'static str),
    #[error("Предыдущий режим ({0}) не завершил очистку. Повторите остановку или перезапустите OCHISTKA")]
    NotCleaned(&'static str),
    #[error("Состояние сетевого режима неизвестно. Запуск отклонён")]
    Undetermined,
}

#[derive(Default)]
pub struct NetCoordinator {
    mode: Mutex<Option<Mode>>,
}

impl NetCoordinator {
    pub fn new() -> Self {
        Self {
            mode: Mutex::new(Some(Mode::Idle)),
        }
    }

    /// Отравленная блокировка или потерянное состояние — «неизвестно».
    fn state(&self) -> Result<MutexGuard<'_, Option<Mode>>, Denied> {
        self.mode.lock().map_err(|_| Denied::Undetermined)
    }

    pub fn mode(&self) -> Option<Mode> {
        self.state().ok().and_then(|g| *g)
    }

    /// Атомарно занимает право запуска. Повторный запуск тем же владельцем (повторное
    /// подключение или повтор после неудачного отката) разрешён; чужому — отказ.
    pub fn begin(&self, who: Owner) -> Result<(), Denied> {
        let mut g = self.state()?;
        let cur = (*g).ok_or(Denied::Undetermined)?;
        match cur {
            Mode::Idle => {}
            Mode::Stopping(o) | Mode::Starting(o) | Mode::Active(o) if o != who => {
                return Err(Denied::Busy(o.title()))
            }
            Mode::Unclean(o) if o != who => return Err(Denied::NotCleaned(o.title())),
            Mode::Starting(_) | Mode::Stopping(_) => {
                return Err(Denied::Busy(who.title()));
            }
            Mode::Active(_) | Mode::Unclean(_) => {}
        }
        *g = Some(Mode::Starting(who));
        Ok(())
    }

    /// Запуск завершён успешно.
    pub fn activate(&self, who: Owner) {
        self.transition(who, |m| match m {
            Mode::Starting(_) | Mode::Active(_) => Some(Mode::Active(who)),
            _ => None,
        });
    }

    /// Начало остановки и очистки (право остаётся за владельцем).
    pub fn begin_stop(&self, who: Owner) {
        self.transition(who, |m| match m {
            Mode::Idle => None,
            _ => Some(Mode::Stopping(who)),
        });
    }

    /// Очистка завершена: право свободно. Чужое владение не трогает.
    pub fn release(&self, who: Owner) {
        self.transition(who, |_| Some(Mode::Idle));
    }

    /// Откат не завершён: право не освобождается, другой механизм получает отказ.
    pub fn mark_unclean(&self, who: Owner) {
        if let Ok(mut g) = self.state() {
            if matches!(*g, Some(Mode::Idle) | None) || (*g).and_then(Mode::owner) == Some(who) {
                *g = Some(Mode::Unclean(who));
            }
        }
    }

    fn transition(&self, who: Owner, f: impl FnOnce(Mode) -> Option<Mode>) {
        if let Ok(mut g) = self.state() {
            if let Some(cur) = *g {
                if cur.owner() == Some(who) {
                    if let Some(next) = f(cur) {
                        *g = Some(next);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[test]
    fn only_one_of_two_simultaneous_starts_wins() {
        for _ in 0..50 {
            let c = Arc::new(NetCoordinator::new());
            let gate = Arc::new(Barrier::new(2));
            let spawn = |who| {
                let (c, gate) = (c.clone(), gate.clone());
                std::thread::spawn(move || {
                    gate.wait();
                    c.begin(who).is_ok()
                })
            };
            let (a, b) = (spawn(Owner::Vpn), spawn(Owner::Desync));
            let wins = [a.join().unwrap(), b.join().unwrap()];
            assert_eq!(wins.iter().filter(|w| **w).count(), 1, "{wins:?}");
        }
    }

    #[test]
    fn starting_or_active_mode_denies_the_other_side() {
        let c = NetCoordinator::new();
        c.begin(Owner::Vpn).unwrap();
        assert!(matches!(c.begin(Owner::Desync), Err(Denied::Busy(_))));
        c.activate(Owner::Vpn);
        assert!(matches!(c.begin(Owner::Desync), Err(Denied::Busy(_))));
        let c = NetCoordinator::new();
        c.begin(Owner::Desync).unwrap();
        assert!(matches!(c.begin(Owner::Vpn), Err(Denied::Busy(_))));
    }

    #[test]
    fn unfinished_rollback_never_gives_a_false_permission() {
        let c = NetCoordinator::new();
        c.begin(Owner::Vpn).unwrap();
        c.mark_unclean(Owner::Vpn);
        assert!(matches!(c.begin(Owner::Desync), Err(Denied::NotCleaned(_))));
        // Чужой release ничего не освобождает.
        c.release(Owner::Desync);
        assert_eq!(c.mode(), Some(Mode::Unclean(Owner::Vpn)));
        // Тот же владелец может повторить и после чистого завершения освободить право.
        c.begin(Owner::Vpn).unwrap();
        c.release(Owner::Vpn);
        c.begin(Owner::Desync).unwrap();
    }

    #[test]
    fn stopping_keeps_ownership_until_release() {
        let c = NetCoordinator::new();
        c.begin(Owner::Desync).unwrap();
        c.activate(Owner::Desync);
        c.begin_stop(Owner::Desync);
        assert!(c.begin(Owner::Vpn).is_err());
        assert!(c.begin(Owner::Desync).is_err());
        c.release(Owner::Desync);
        assert_eq!(c.mode(), Some(Mode::Idle));
        c.begin(Owner::Vpn).unwrap();
    }

    #[test]
    fn poisoned_state_is_undetermined_and_denies() {
        let c = Arc::new(NetCoordinator::new());
        let c2 = c.clone();
        let _ = std::thread::spawn(move || {
            let _g = c2.mode.lock().unwrap();
            panic!("poison");
        })
        .join();
        assert_eq!(c.begin(Owner::Vpn), Err(Denied::Undetermined));
        assert_eq!(c.mode(), None);
    }

    #[test]
    fn same_owner_may_restart_and_release_is_idempotent() {
        let c = NetCoordinator::new();
        c.begin(Owner::Vpn).unwrap();
        c.activate(Owner::Vpn);
        c.begin(Owner::Vpn).unwrap(); // повторное подключение
        c.release(Owner::Vpn);
        c.release(Owner::Vpn);
        assert_eq!(c.mode(), Some(Mode::Idle));
    }
}
