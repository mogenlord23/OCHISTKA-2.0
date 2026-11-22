//! VPN «Весь компьютер» в Windows: ядро держит служба `OchistkaVpn` (от имени системы), потому что
//! туннелю нужны права администратора. Службу и права на её запуск и остановку (для обычных
//! пользователей) один раз создаёт установщик. Приложение запускает и останавливает её через
//! диспетчер служб, без запросов прав и окон. Если службы нет — просит переустановить программу.

use std::sync::Arc;
use std::time::Duration;

use ochistka_core::vpn::service::CoreProcess;

pub const SERVICE_NAME: &str = "OchistkaVpn";

pub const REINSTALL: &str = "Для этого режима нужна переустановка программы";
pub const NOT_STARTED: &str = "VPN не запустился";
pub const STOPPED_AT_ONCE: &str = "VPN сразу остановился";
pub const NOT_STOPPED: &str = "VPN не выключился, перезагрузите компьютер";

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
    fn start(&self, name: &str, args: &[String]) -> Result<(), ServiceError>;
    /// Просит службу остановиться; «не запущена» — успех.
    fn stop(&self, name: &str) -> Result<(), ServiceError>;
}

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

pub struct TunnelService {
    control: Arc<dyn ServiceControl>,
    waits: Waits,
}

impl TunnelService {
    pub fn new(control: Arc<dyn ServiceControl>, waits: Waits) -> Self {
        Self { control, waits }
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

    /// Служба установлена и доступна пользователю; иначе — просьба переустановить программу.
    pub fn check_installed(&self) -> Result<(), String> {
        match self.state() {
            Ok(_) => Ok(()),
            Err(ServiceError::NotInstalled | ServiceError::AccessDenied) => Err(REINSTALL.into()),
            Err(ServiceError::Failed(_)) => Err(NOT_STARTED.into()),
        }
    }

    pub fn is_running(&self) -> bool {
        self.state() == Ok(ServiceState::Running)
    }

    /// Запускает службу с конфигурацией ядра и ждёт `SERVICE_RUNNING` (до 5 секунд). Если служба
    /// тут же остановилась — короткая ошибка.
    pub fn start(&self, config: &str) -> Result<(), String> {
        self.check_installed()?;
        if self.state() == Ok(ServiceState::Running) {
            // Служба от прошлого раза: перезапускаем, чтобы взяла новую конфигурацию.
            self.stop()?;
        }
        self.control
            .start(SERVICE_NAME, &[config.to_string()])
            .map_err(|e| match e {
                ServiceError::NotInstalled | ServiceError::AccessDenied => REINSTALL.to_string(),
                ServiceError::Failed(_) => NOT_STARTED.to_string(),
            })?;
        if !self.wait_for(self.waits.start_polls, ServiceState::Running) {
            let _ = self.control.stop(SERVICE_NAME);
            return Err(NOT_STARTED.into());
        }
        std::thread::sleep(self.waits.settle);
        if self.state() != Ok(ServiceState::Running) {
            return Err(STOPPED_AT_ONCE.into());
        }
        Ok(())
    }

    /// Останавливает службу и ждёт `SERVICE_STOPPED` (до 20 секунд). Нет службы или она уже
    /// остановлена — это успех.
    pub fn stop(&self) -> Result<(), String> {
        match self.state() {
            Ok(ServiceState::Stopped) | Err(ServiceError::NotInstalled) => return Ok(()),
            Ok(_) => {}
            Err(_) => return Err(NOT_STOPPED.into()),
        }
        self.control
            .stop(SERVICE_NAME)
            .map_err(|_| NOT_STOPPED.to_string())?;
        if self.wait_for(self.waits.stop_polls, ServiceState::Stopped) {
            Ok(())
        } else {
            Err(NOT_STOPPED.into())
        }
    }
}

/// Ядро, работающее в службе: «жив» — пока служба запускается или работает.
pub struct ServiceProcess(pub Arc<TunnelService>);

impl CoreProcess for ServiceProcess {
    /// Процесс принадлежит службе: номера у приложения нет (0).
    fn pid(&self) -> u32 {
        0
    }

    fn has_exited(&mut self) -> bool {
        !matches!(
            self.0.state(),
            Ok(ServiceState::Running | ServiceState::StartPending)
        )
    }

    fn kill(&mut self) {
        let _ = self.0.stop();
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

    fn start(&self, name: &str, args: &[String]) -> Result<(), ServiceError> {
        match crate::windows::win32_ffi::service_start(name, args) {
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

/// Служба VPN текущей системы.
#[cfg(windows)]
pub fn current_tunnel() -> Arc<TunnelService> {
    Arc::new(TunnelService::new(
        Arc::new(WindowsServiceControl),
        Waits::REAL,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct Fake {
        state: Mutex<ServiceState>,
        installed: bool,
        access_denied: bool,
        never_runs: bool,
        ignores_stop: bool,
        stops_at_once: bool,
        log: Mutex<Vec<String>>,
        last_args: Mutex<HashMap<String, Vec<String>>>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                state: Mutex::new(ServiceState::Stopped),
                installed: true,
                access_denied: false,
                never_runs: false,
                ignores_stop: false,
                stops_at_once: false,
                log: Mutex::new(vec![]),
                last_args: Mutex::new(HashMap::new()),
            }
        }
        fn calls(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
    }

    impl ServiceControl for Fake {
        fn state(&self, _name: &str) -> Result<ServiceState, ServiceError> {
            if !self.installed {
                return Err(ServiceError::NotInstalled);
            }
            if self.access_denied {
                return Err(ServiceError::AccessDenied);
            }
            let mut st = self.state.lock().unwrap();
            // Служба, которая сразу падает: первый опрос «работает», следующие — «остановлена».
            if self.stops_at_once && *st == ServiceState::Running {
                let seen = *st;
                *st = ServiceState::Stopped;
                return Ok(seen);
            }
            Ok(*st)
        }
        fn start(&self, name: &str, args: &[String]) -> Result<(), ServiceError> {
            self.log.lock().unwrap().push("start".into());
            self.last_args
                .lock()
                .unwrap()
                .insert(name.into(), args.to_vec());
            if !self.never_runs {
                *self.state.lock().unwrap() = ServiceState::Running;
            }
            Ok(())
        }
        fn stop(&self, _name: &str) -> Result<(), ServiceError> {
            self.log.lock().unwrap().push("stop".into());
            if !self.ignores_stop {
                *self.state.lock().unwrap() = ServiceState::Stopped;
            }
            Ok(())
        }
    }

    const FAST: Waits = Waits {
        poll: Duration::from_millis(1),
        start_polls: 5,
        settle: Duration::from_millis(1),
        stop_polls: 5,
    };

    fn svc(f: &Arc<Fake>) -> Arc<TunnelService> {
        Arc::new(TunnelService::new(f.clone(), FAST))
    }

    #[test]
    fn not_installed_asks_for_a_reinstall_and_starts_nothing() {
        let mut f = Fake::new();
        f.installed = false;
        let f = Arc::new(f);
        let s = svc(&f);
        assert_eq!(s.check_installed(), Err(REINSTALL.to_string()));
        assert_eq!(s.start("{}"), Err(REINSTALL.to_string()));
        assert!(f.calls().is_empty());
        // Выключать нечего: это не ошибка.
        assert_eq!(s.stop(), Ok(()));
    }

    #[test]
    fn no_access_to_the_service_is_the_same_plain_request() {
        let mut f = Fake::new();
        f.access_denied = true;
        let f = Arc::new(f);
        assert_eq!(svc(&f).start("{}"), Err(REINSTALL.to_string()));
    }

    #[test]
    fn start_passes_the_configuration_and_waits_for_running() {
        let f = Arc::new(Fake::new());
        let s = svc(&f);
        s.start(r#"{"a":1}"#).unwrap();
        assert!(s.is_running());
        assert_eq!(f.calls(), ["start"]);
        assert_eq!(
            f.last_args.lock().unwrap()[SERVICE_NAME],
            [r#"{"a":1}"#.to_string()]
        );
    }

    #[test]
    fn a_service_that_never_runs_is_an_error_and_is_stopped() {
        let mut f = Fake::new();
        f.never_runs = true;
        let f = Arc::new(f);
        assert_eq!(svc(&f).start("{}"), Err(NOT_STARTED.to_string()));
        assert!(f.calls().contains(&"stop".to_string()));
    }

    #[test]
    fn a_service_that_stops_at_once_is_a_short_error() {
        let mut f = Fake::new();
        f.stops_at_once = true;
        let f = Arc::new(f);
        assert_eq!(svc(&f).start("{}"), Err(STOPPED_AT_ONCE.to_string()));
        assert!(!svc(&f).is_running());
    }

    #[test]
    fn stop_waits_for_stopped_and_a_stuck_service_is_an_error() {
        let f = Arc::new(Fake::new());
        let s = svc(&f);
        s.start("{}").unwrap();
        s.stop().unwrap();
        assert!(!s.is_running());
        // Двойная остановка безопасна.
        s.stop().unwrap();
        let mut g = Fake::new();
        g.ignores_stop = true;
        let g = Arc::new(g);
        let s = svc(&g);
        s.start("{}").unwrap();
        assert_eq!(s.stop(), Err(NOT_STOPPED.to_string()));
        assert!(s.is_running(), "служба всё ещё работает");
    }

    #[test]
    fn a_running_service_from_before_is_restarted_with_the_new_configuration() {
        let f = Arc::new(Fake::new());
        let s = svc(&f);
        s.start(r#"{"v":1}"#).unwrap();
        s.start(r#"{"v":2}"#).unwrap();
        assert_eq!(f.calls(), ["start", "stop", "start"]);
        assert_eq!(
            f.last_args.lock().unwrap()[SERVICE_NAME],
            [r#"{"v":2}"#.to_string()]
        );
    }

    #[test]
    fn the_process_object_follows_the_service_state_and_kill_stops_it() {
        let f = Arc::new(Fake::new());
        let s = svc(&f);
        let mut p = ServiceProcess(s.clone());
        assert!(p.has_exited(), "службы нет в работе");
        s.start("{}").unwrap();
        assert!(!p.has_exited());
        assert_eq!(p.pid(), 0);
        p.kill();
        assert!(p.has_exited());
        // Служба остановилась сама: приложение это замечает.
        s.start("{}").unwrap();
        *f.state.lock().unwrap() = ServiceState::Stopped;
        assert!(p.has_exited());
    }
}
