//! Кнопка «Обход сайтов» в Windows: запуск и остановка службы `OchistkaBypass` через диспетчер служб
//! (OpenService / StartService / ControlService), без повышения прав и окон. Службу и права на неё
//! (запуск и остановка для обычных пользователей) один раз создаёт установщик. Если службы нет —
//! «обход не установлен, переустановите программу». Список сайтов службе не передаётся.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ochistka_core::site_bypass::{BypassEngine, EngineError};

pub const SERVICE_NAME: &str = "OchistkaBypass";
pub const DRIVER_SERVICE: &str = "WinDivert";

pub const NOT_INSTALLED: &str = "обход не установлен, переустановите программу";
pub const NO_ACCESS: &str = "нет доступа к службе обхода, переустановите программу";
pub const NOT_STARTED: &str = "обход не запустился";
pub const STOPPED_AT_ONCE: &str = "служба обхода сразу остановилась";
pub const NOT_STOPPED: &str = "обход не выключился, перезагрузите компьютер";
pub const DRIVER_NOT_STOPPED: &str = "драйвер не остановлен, перезагрузите компьютер";
pub const STOPPED_NOTICE: &str = "Обход остановлен, сайты открываются как обычно";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    StartPending,
    StopPending,
    Running,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceError {
    NotInstalled,
    AccessDenied,
    Failed(u32),
}

/// Диспетчер служб (в тестах подменяется).
pub trait ServiceControl: Send + Sync {
    fn state(&self, name: &str) -> Result<ServiceState, ServiceError>;
    /// Запускает службу; «уже запущена» — успех.
    fn start(&self, name: &str) -> Result<(), ServiceError>;
    /// Просит службу остановиться; «не запущена» — успех.
    fn stop(&self, name: &str) -> Result<(), ServiceError>;
}

/// Сколько и как долго ждать.
#[derive(Clone, Copy)]
pub struct Waits {
    pub poll: Duration,
    /// Ожидание «работает» после запуска (в опросах).
    pub start_polls: u32,
    /// Пауза перед повторной проверкой, что служба не остановилась сразу.
    pub settle: Duration,
    /// Ожидание «остановлена» после остановки (в опросах).
    pub stop_polls: u32,
}

impl Waits {
    /// До 5 секунд на запуск, до 20 на остановку.
    pub const REAL: Waits = Waits {
        poll: Duration::from_millis(100),
        start_polls: 50,
        settle: Duration::from_millis(700),
        stop_polls: 200,
    };
}

pub struct ServiceBypassEngine {
    control: Arc<dyn ServiceControl>,
    waits: Waits,
    /// Состояние драйвера до нашего запуска: останавливаем его, только если он не работал раньше.
    driver_before: Mutex<Option<ServiceState>>,
    /// Службу запустили мы и ещё не выключали (чтобы заметить, что она остановилась сама).
    started: AtomicBool,
}

fn map_error(err: ServiceError, fallback: &str) -> EngineError {
    EngineError::Failed(match err {
        ServiceError::NotInstalled => NOT_INSTALLED.into(),
        ServiceError::AccessDenied => NO_ACCESS.into(),
        ServiceError::Failed(_) => fallback.into(),
    })
}

impl ServiceBypassEngine {
    pub fn new(control: Arc<dyn ServiceControl>, waits: Waits) -> Self {
        Self {
            control,
            waits,
            driver_before: Mutex::new(None),
            started: AtomicBool::new(false),
        }
    }

    fn state(&self) -> Result<ServiceState, ServiceError> {
        self.control.state(SERVICE_NAME)
    }

    fn wait_for(&self, polls: u32, wanted: ServiceState) -> bool {
        for _ in 0..polls {
            if self.state() == Ok(wanted) {
                return true;
            }
            std::thread::sleep(self.waits.poll);
        }
        self.state() == Ok(wanted)
    }

    /// Драйвер WinDivert: если он запустился из-за нас, останавливаем и убеждаемся.
    fn driver_stopped_if_ours(&self) -> Result<(), EngineError> {
        let before = *self.driver_before.lock().unwrap_or_else(|e| e.into_inner());
        if before == Some(ServiceState::Running) {
            return Ok(()); // работал до нас: не трогаем
        }
        let running = |s: &Self| s.control.state(DRIVER_SERVICE) == Ok(ServiceState::Running);
        if !running(self) {
            return Ok(());
        }
        let _ = self.control.stop(DRIVER_SERVICE);
        for _ in 0..self.waits.start_polls.min(20) {
            if !running(self) {
                return Ok(());
            }
            std::thread::sleep(self.waits.poll);
        }
        Err(EngineError::Unclean(DRIVER_NOT_STOPPED.into()))
    }
}

impl BypassEngine for ServiceBypassEngine {
    /// «Включено» только при SERVICE_RUNNING.
    fn is_on(&self) -> bool {
        self.state() == Ok(ServiceState::Running)
    }

    fn enable(&self, _user_sites: &[String]) -> Result<(), EngineError> {
        match self.state() {
            Ok(ServiceState::Running) => {
                self.started.store(true, Ordering::SeqCst);
                return Ok(());
            }
            Ok(_) => {}
            Err(e) => return Err(map_error(e, NOT_STARTED)),
        }
        *self.driver_before.lock().unwrap_or_else(|e| e.into_inner()) =
            self.control.state(DRIVER_SERVICE).ok();
        self.control
            .start(SERVICE_NAME)
            .map_err(|e| map_error(e, NOT_STARTED))?;
        if !self.wait_for(self.waits.start_polls, ServiceState::Running) {
            let _ = self.control.stop(SERVICE_NAME);
            return Err(EngineError::Failed(NOT_STARTED.into()));
        }
        // Служба могла запуститься и тут же остановиться (движок не стартовал): проверяем ещё раз.
        std::thread::sleep(self.waits.settle);
        if self.state() != Ok(ServiceState::Running) {
            return Err(EngineError::Failed(STOPPED_AT_ONCE.into()));
        }
        self.started.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Останавливает службу, ждёт SERVICE_STOPPED, затем проверяет драйвер.
    fn disable(&self) -> Result<(), EngineError> {
        match self.state() {
            Ok(ServiceState::Stopped) | Err(ServiceError::NotInstalled) => {}
            Ok(_) => {
                self.control
                    .stop(SERVICE_NAME)
                    .map_err(|_| EngineError::Unclean(NOT_STOPPED.into()))?;
                if !self.wait_for(self.waits.stop_polls, ServiceState::Stopped) {
                    return Err(EngineError::Unclean(NOT_STOPPED.into()));
                }
            }
            Err(e) => {
                return Err(EngineError::Unclean(match e {
                    ServiceError::AccessDenied => NO_ACCESS.into(),
                    _ => NOT_STOPPED.into(),
                }))
            }
        }
        self.driver_stopped_if_ours()?;
        self.started.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Служба — состояние самой системы: после аварии программы ничего восстанавливать не надо.
    fn recover(&self) -> Result<(), EngineError> {
        Ok(())
    }

    fn stopped_notice(&self) -> Option<String> {
        (self.started.load(Ordering::SeqCst) && self.state() == Ok(ServiceState::Stopped))
            .then(|| STOPPED_NOTICE.to_string())
    }

    fn has_leftovers(&self) -> bool {
        false
    }
}

#[cfg(windows)]
pub struct WindowsServiceControl;

#[cfg(windows)]
impl WindowsServiceControl {
    fn map(code: u32) -> ServiceError {
        match code {
            1060 => ServiceError::NotInstalled, // ERROR_SERVICE_DOES_NOT_EXIST
            5 => ServiceError::AccessDenied,    // ERROR_ACCESS_DENIED
            other => ServiceError::Failed(other),
        }
    }
}

#[cfg(windows)]
impl ServiceControl for WindowsServiceControl {
    fn state(&self, name: &str) -> Result<ServiceState, ServiceError> {
        crate::windows::win32_ffi::service_state(name)
            .map(|s| match s {
                1 => ServiceState::Stopped,
                2 => ServiceState::StartPending,
                3 => ServiceState::StopPending,
                4 => ServiceState::Running,
                _ => ServiceState::Other,
            })
            .map_err(Self::map)
    }

    fn start(&self, name: &str) -> Result<(), ServiceError> {
        match crate::windows::win32_ffi::service_start(name) {
            Ok(()) | Err(1056) => Ok(()), // ERROR_SERVICE_ALREADY_RUNNING
            Err(code) => Err(Self::map(code)),
        }
    }

    fn stop(&self, name: &str) -> Result<(), ServiceError> {
        match crate::windows::win32_ffi::service_stop(name) {
            Ok(()) | Err(1062) => Ok(()), // ERROR_SERVICE_NOT_ACTIVE
            Err(code) => Err(Self::map(code)),
        }
    }
}

/// Движок кнопки на Windows: служба `OchistkaBypass`.
#[cfg(windows)]
pub fn current_engine() -> Arc<dyn BypassEngine> {
    Arc::new(ServiceBypassEngine::new(
        Arc::new(WindowsServiceControl),
        Waits::REAL,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Фейковый диспетчер: хранит состояния и «поведение» службы.
    struct Fake {
        states: Mutex<HashMap<String, ServiceState>>,
        installed: bool,
        access_denied: bool,
        /// Запуск не приводит к «работает».
        never_runs: bool,
        /// Остановка не срабатывает.
        ignores_stop: bool,
        /// Запуск службы «запускает» драйвер.
        starts_driver: bool,
        driver_ignores_stop: bool,
        log: Mutex<Vec<String>>,
    }

    impl Fake {
        fn new() -> Self {
            let mut states = HashMap::new();
            states.insert(SERVICE_NAME.to_string(), ServiceState::Stopped);
            states.insert(DRIVER_SERVICE.to_string(), ServiceState::Stopped);
            Self {
                states: Mutex::new(states),
                installed: true,
                access_denied: false,
                never_runs: false,
                ignores_stop: false,
                starts_driver: true,
                driver_ignores_stop: false,
                log: Mutex::new(vec![]),
            }
        }
        fn set(&self, name: &str, s: ServiceState) {
            self.states.lock().unwrap().insert(name.into(), s);
        }
        fn get(&self, name: &str) -> ServiceState {
            self.states.lock().unwrap()[name]
        }
        fn calls(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
    }

    impl ServiceControl for Fake {
        fn state(&self, name: &str) -> Result<ServiceState, ServiceError> {
            if name == SERVICE_NAME && !self.installed {
                return Err(ServiceError::NotInstalled);
            }
            if self.access_denied && name == SERVICE_NAME {
                return Err(ServiceError::AccessDenied);
            }
            Ok(self.get(name))
        }
        fn start(&self, name: &str) -> Result<(), ServiceError> {
            self.log.lock().unwrap().push(format!("start {name}"));
            if self.never_runs {
                return Ok(());
            }
            self.set(name, ServiceState::Running);
            if self.starts_driver {
                self.set(DRIVER_SERVICE, ServiceState::Running);
            }
            Ok(())
        }
        fn stop(&self, name: &str) -> Result<(), ServiceError> {
            self.log.lock().unwrap().push(format!("stop {name}"));
            if name == SERVICE_NAME && self.ignores_stop {
                return Ok(());
            }
            if name == DRIVER_SERVICE && self.driver_ignores_stop {
                return Ok(());
            }
            self.set(name, ServiceState::Stopped);
            Ok(())
        }
    }

    const FAST: Waits = Waits {
        poll: Duration::from_millis(1),
        start_polls: 5,
        settle: Duration::from_millis(1),
        stop_polls: 5,
    };

    fn engine(fake: &Arc<Fake>) -> ServiceBypassEngine {
        ServiceBypassEngine::new(fake.clone(), FAST)
    }

    #[test]
    fn not_installed_gives_the_reinstall_message_and_starts_nothing() {
        let mut f = Fake::new();
        f.installed = false;
        let f = Arc::new(f);
        let e = engine(&f);
        assert_eq!(
            e.enable(&[]),
            Err(EngineError::Failed(
                "обход не установлен, переустановите программу".into()
            ))
        );
        assert!(f.calls().is_empty());
        assert!(!e.is_on());
    }

    #[test]
    fn a_service_without_access_rights_is_a_short_error_not_a_prompt() {
        let mut f = Fake::new();
        f.access_denied = true;
        let f = Arc::new(f);
        assert_eq!(
            engine(&f).enable(&[]),
            Err(EngineError::Failed(NO_ACCESS.into()))
        );
    }

    #[test]
    fn start_waits_for_running_and_only_then_is_on() {
        let f = Arc::new(Fake::new());
        let e = engine(&f);
        assert!(!e.is_on());
        e.enable(&["example.com".into()]).unwrap();
        assert!(e.is_on());
        assert_eq!(f.calls(), ["start OchistkaBypass"]);
        // Повторное включение работающей службы ничего не запускает.
        e.enable(&[]).unwrap();
        assert_eq!(f.calls().len(), 1);
    }

    #[test]
    fn a_service_that_never_reaches_running_is_an_error_and_is_stopped() {
        let mut f = Fake::new();
        f.never_runs = true;
        let f = Arc::new(f);
        let e = engine(&f);
        assert_eq!(e.enable(&[]), Err(EngineError::Failed(NOT_STARTED.into())));
        assert!(f.calls().contains(&"stop OchistkaBypass".to_string()));
        assert!(!e.is_on());
    }

    /// Служба запускается, но к повторной проверке уже остановлена.
    struct DiesAfterStart(Fake, Mutex<u32>);

    impl ServiceControl for DiesAfterStart {
        fn state(&self, name: &str) -> Result<ServiceState, ServiceError> {
            let mut n = self.1.lock().unwrap();
            if name == SERVICE_NAME && self.0.get(name) == ServiceState::Running {
                *n += 1;
                // Первый опрос видит «работает», следующий — уже «остановлена».
                if *n >= 2 {
                    self.0.set(name, ServiceState::Stopped);
                }
            }
            self.0.state(name)
        }
        fn start(&self, name: &str) -> Result<(), ServiceError> {
            self.0.start(name)
        }
        fn stop(&self, name: &str) -> Result<(), ServiceError> {
            self.0.stop(name)
        }
    }

    #[test]
    fn a_service_that_stops_at_once_is_an_error_with_a_short_text() {
        let f = Arc::new(DiesAfterStart(Fake::new(), Mutex::new(0)));
        let e = ServiceBypassEngine::new(f, FAST);
        assert_eq!(
            e.enable(&[]),
            Err(EngineError::Failed(STOPPED_AT_ONCE.into()))
        );
        assert!(!e.is_on());
    }

    #[test]
    fn stop_waits_for_stopped_and_stops_the_driver_started_by_us() {
        let f = Arc::new(Fake::new());
        let e = engine(&f);
        e.enable(&[]).unwrap();
        assert_eq!(f.get(DRIVER_SERVICE), ServiceState::Running);
        e.disable().unwrap();
        assert_eq!(f.get(SERVICE_NAME), ServiceState::Stopped);
        assert_eq!(f.get(DRIVER_SERVICE), ServiceState::Stopped);
        assert!(f.calls().contains(&"stop WinDivert".to_string()));
        assert_eq!(e.stopped_notice(), None);
    }

    #[test]
    fn a_driver_that_was_running_before_us_is_left_alone() {
        let f = Arc::new(Fake::new());
        f.set(DRIVER_SERVICE, ServiceState::Running);
        let e = engine(&f);
        e.enable(&[]).unwrap();
        e.disable().unwrap();
        assert_eq!(f.get(DRIVER_SERVICE), ServiceState::Running);
        assert!(!f.calls().contains(&"stop WinDivert".to_string()));
    }

    #[test]
    fn a_service_that_will_not_stop_and_a_driver_that_will_not_stop_are_errors() {
        let mut f = Fake::new();
        f.ignores_stop = true;
        let f = Arc::new(f);
        let e = engine(&f);
        e.enable(&[]).unwrap();
        assert_eq!(e.disable(), Err(EngineError::Unclean(NOT_STOPPED.into())));
        assert!(e.is_on(), "служба всё ещё работает: не «выключено»");

        let mut g = Fake::new();
        g.driver_ignores_stop = true;
        let g = Arc::new(g);
        let e = engine(&g);
        e.enable(&[]).unwrap();
        assert_eq!(
            e.disable(),
            Err(EngineError::Unclean(DRIVER_NOT_STOPPED.into()))
        );
    }

    #[test]
    fn stopping_when_already_stopped_or_not_installed_is_fine() {
        let f = Arc::new(Fake::new());
        engine(&f).disable().unwrap();
        let mut g = Fake::new();
        g.installed = false;
        engine(&Arc::new(g)).disable().unwrap();
    }

    #[test]
    fn a_service_that_stopped_by_itself_after_our_start_gives_the_notice() {
        let f = Arc::new(Fake::new());
        let e = engine(&f);
        assert_eq!(e.stopped_notice(), None);
        e.enable(&[]).unwrap();
        assert_eq!(e.stopped_notice(), None);
        f.set(SERVICE_NAME, ServiceState::Stopped);
        assert_eq!(
            e.stopped_notice().as_deref(),
            Some("Обход остановлен, сайты открываются как обычно")
        );
        assert!(!e.is_on());
    }
}
