//! Единая система «Обход блокировок»: общее ядро для Windows, Linux и macOS.
//!
//! Общее (этот файл): состояния, профили и стратегии, автоматический выбор стратегии без
//! смешивания, управление процессом (запуск только по явному действию, проверка, остановка, защита
//! от второго экземпляра, очистка после аварии), конфликт с VPN, честная готовность адаптера.
//! Системное (адаптеры, `crates/platform`): Windows — `winws2` + WinDivert; Linux — `nfqws2` +
//! NFQUEUE; macOS — Zapret 2 не поддерживается, адаптер сообщает это и ничего не запускает.
//!
//! Windows-движок из проверенного Release закреплён по SHA-256, но основной переключатель
//! «Обход блокировок» остаётся недоступным, пока рабочий профиль не проверен (`Readiness`).
//! Ничего не скачивается и не запускается без подтверждённых источников и хэшей.
//! Системный прокси, DNS, hosts и маршруты здесь не трогаются.

pub mod preset;
pub mod routing;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, Weak};
use std::time::Duration;

use thiserror::Error;

use crate::vpn::engine::{PinnedEngine, PrepareError, PrepareProgress};
use crate::vpn::mask::mask_secrets;
use crate::vpn::service::{ConflictGuard, CoreProcess, VpnService};

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

/// Проверенный Windows x64 ZIP из успешного запуска build-zapret2-windows №44885741341.
/// ZIP опубликован отдельным prerelease: содержимое и SHA-256 не менялись.
/// GitHub Release: https://github.com/mogenlord23/OCHISTKA-2.0/releases/tag/zapret2-winws2-2026-11-21
pub static PINNED_WINDOWS: PinnedEngine = PinnedEngine {
    version: "zapret2-winws2-2026-11-21",
    url: "https://github.com/mogenlord23/OCHISTKA-2.0/releases/download/zapret2-winws2-2026-11-21/zapret2-winws2-windows-x64.zip",
    url_prefix: "/mogenlord23/OCHISTKA-2.0/releases/download/",
    archive_sha256: "15bb46aca6904c674cb018250855398c60d7676079fd534832f1e4d87f07e04a",
    files: &[
        ("winws2.exe", "5390cb6cec8cdb39bda657f83e8b87ce8dc96d074bf6a71e22c03c6896bcaf42"),
        ("cygwin1.dll", "ed9d5379061e5de79c3375cc62246c2142ea1c97d94f4d16bfbea6fb1b7fc889"),
        ("cygz.dll", "02561b4b4b473c40e9bed9c5df59334b5d79efae568b6e212a7c9d775ba8e647"),
        ("WinDivert.dll", "c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2"),
        ("WinDivert64.sys", "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2"),
        ("LICENSE-zapret2.txt", "d089978dd77d53cb6aa5dff51cfdbff617e52dd44af1ac44a6df02c6644f17d5"),
        ("LICENSE-WinDivert.txt", "542e02ca56c33aea103ce84a8b874b0b15e25b9f559d9bf578a10652f824815d"),
        ("LICENSE-luajit2.txt", "a9eddee2daeff45faaa62fce2d31f63a43a241b530519c305169f3d544249c31"),
        ("manifest.json", "60505764e6ef5aac937e3259eb443069d433a944dfb0d05b218160f541f3c3d5"),
    ],
};

/// Выпуск доступен только Windows; Linux и macOS пока без установленного движка.
pub fn pinned() -> Option<&'static PinnedEngine> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some(&PINNED_WINDOWS)
    } else {
        None
    }
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

/// Кандидат TCP-профиля для Windows, НЕ подтверждённый на реальной сети.
/// Аргументы повторяют пример из bol-van/zapret2/docs/readme.md на коммите
/// 1eb4fd887b6091ccaf59feea7486162ac64c9319 (с WinDivert --wf-tcp-out).
/// Это только HTTP/TLS (80/443): не заявляем поддержку голосового UDP Discord,
/// Telegram, игр, QUIC или всех сайтов. Пока нет проверки, пользовательский
/// профиль выше остаётся None, и включение обхода недоступно.
pub fn windows_tcp_candidate_profile() -> Profile {
    Profile {
        id: "windows-tcp-http-tls-candidate",
        engine: Engine::Winws2,
        title: "Пробный TCP-профиль для HTTP/HTTPS",
        args: [
            "--wf-tcp-out=80,443",
            "--lua-init=@zapret-lib.lua",
            "--lua-init=@zapret-antidpi.lua",
            "--filter-tcp=80,443",
            "--filter-l7=tls,http",
            "--payload=tls_client_hello",
            "--lua-desync=fake:blob=fake_default_tls:tcp_md5:tls_mod=rnd,rndsni,dupsid",
            "--payload=http_req",
            "--lua-desync=fake:blob=fake_default_http:tcp_md5",
            "--payload=tls_client_hello,http_req",
            "--lua-desync=multisplit:pos=1:seqovl=5:seqovl_pattern=0x1603030000",
        ]
        .iter()
        .map(|arg| (*arg).to_string())
        .collect(),
    }
}

/// Реальный Universal V13 Lite из NetZapret, ещё НЕ подтверждённый внешней сетью.
/// Возвращает целый пресет со всеми 73 секциями без объединения чужих рецептов.
/// Это кандидат для будущей установки через WindowsDesyncAdapter; само наличие
/// функции НЕ меняет site_discord_profile() и не разблокирует переключатель.
pub fn windows_netzapret_v13_lite_candidate_profile() -> Result<Profile, preset::PresetError> {
    let original = include_str!("../../../assets/desync/presets/Universal V13 Lite.txt");
    let parsed = preset::ZapretPreset::parse(original)?;
    Ok(Profile {
        id: "windows-netzapret-v13-lite-unverified",
        engine: Engine::Winws2,
        title: "Universal V13 Lite (требуется подтверждение сети)",
        args: parsed.command_arguments(),
    })
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
    /// План доступа не разрешает перехват (причина показана человеку).
    #[error("{0}")]
    NotAllowed(String),
    /// Сервисы и так доступны напрямую или через VPN: перехват не нужен.
    #[error("{0}")]
    NotNeeded(String),
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
    #[cfg(test)]
    profile: Option<Profile>,
    /// Есть ли закреплённый выпуск (можно ли скачивать недостающее).
    installable: bool,
    /// Реальное состояние VPN (из `VpnService::status`): одновременно перехват и VPN нельзя.
    vpn: Box<dyn Fn() -> routing::VpnSignal + Send + Sync>,
    /// Целый проверяемый пресет для плана (только Windows); не выбирается без подтверждения.
    candidate: Option<Profile>,
    /// Подтверждённое реальными опытами покрытие (пока пусто).
    confirmed: &'static [routing::ConfirmedCoverage],
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
        vpn: Box<dyn Fn() -> routing::VpnSignal + Send + Sync>,
    ) -> Self {
        #[cfg(test)]
        let profile = site_discord_profile(adapter.platform());
        let candidate = if adapter.platform() == Platform::Windows {
            windows_netzapret_v13_lite_candidate_profile().ok()
        } else {
            None
        };
        Self {
            dir,
            adapter,
            #[cfg(test)]
            profile,
            installable: pinned().is_some(),
            vpn,
            candidate,
            confirmed: routing::CONFIRMED_EXTERNAL_PROFILES,
            settle: (5, Duration::from_millis(200)),
            inner: Mutex::new(Inner {
                state: DesyncState::Off,
                process: None,
                message: "Выключено".into(),
            }),
            op: Mutex::new(()),
        }
    }

    /// Пассивный план единой кнопки: для КАЖДОГО сервиса отдельно (по его собственному наблюдению)
    /// и один общий итог. Ничего не запускает и не скачивает. Состояние VPN берётся из реального
    /// сервиса, а не из предположений; неподтверждённый пресет не выбирается.
    pub fn plan_access(
        &self,
        wanted: &[Coverage],
        observed: &routing::ServiceObservations,
    ) -> routing::AccessPlan {
        routing::plan_with_approved(
            wanted,
            observed,
            (self.vpn)(),
            &self.adapter.readiness(),
            self.adapter.platform(),
            self.candidate.as_ref(),
            self.confirmed,
        )
    }

    /// Работает ли обход сейчас (для охраны VPN от одновременного запуска).
    pub fn is_running(&self) -> bool {
        self.status().0 == DesyncState::Running
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

    /// Единственный путь запуска для приложения: движок стартует только если общий план доступа
    /// разрешает ЦЕЛЫЙ подтверждённый профиль для всех выбранных сервисов. Прямой доступ, VPN и
    /// блокировки плана запуск не выполняют. Повторный вызов второй экземпляр не создаёт.
    pub fn start_planned(
        &self,
        wanted: &[Coverage],
        observed: &routing::ServiceObservations,
        progress: &mut dyn FnMut(PrepareProgress),
    ) -> Result<(), DesyncError> {
        let Ok(_op) = self.op.try_lock() else {
            return Err(DesyncError::Busy);
        };
        if self.status().0 == DesyncState::Running {
            return Ok(());
        }
        let plan = self.plan_access(wanted, observed);
        let profile = match plan.overall {
            routing::RoutePlan::Block(reason) => {
                return self.fail(DesyncError::NotAllowed(reason.message().into()));
            }
            routing::RoutePlan::Use(routing::AccessPath::Direct) => {
                return self.fail(DesyncError::NotNeeded(
                    "Выбранные сервисы доступны напрямую: обход не нужен".into(),
                ));
            }
            routing::RoutePlan::Use(routing::AccessPath::Vpn) => {
                return self.fail(DesyncError::NotNeeded(
                    "Для выбранных сервисов используется VPN: обход не включается".into(),
                ));
            }
            routing::RoutePlan::Use(routing::AccessPath::Desync(id)) => {
                match self.candidate.as_ref().filter(|p| p.id == id) {
                    Some(profile) => profile.clone(),
                    None => return self.fail(DesyncError::NoProfile),
                }
            }
        };
        let result = self.start_locked(progress, &profile);
        self.record(&result);
        result
    }

    /// Запись ошибки в состояние сервиса и возврат её же.
    fn fail(&self, err: DesyncError) -> Result<(), DesyncError> {
        let result = Err(err);
        self.record(&result);
        result
    }

    fn record(&self, result: &Result<(), DesyncError>) {
        let mut inner = lock(&self.inner);
        if let Err(err) = result {
            inner.state = DesyncState::Error;
            inner.message = err.to_string();
        }
    }

    /// Запуск профиля `self.profile` БЕЗ плана доступа: только для внутренних проверок движка.
    /// Приложение пользуется `start_planned`.
    #[cfg(test)]
    pub(crate) fn start(
        &self,
        progress: &mut dyn FnMut(PrepareProgress),
    ) -> Result<(), DesyncError> {
        let Ok(_op) = self.op.try_lock() else {
            return Err(DesyncError::Busy);
        };
        if self.status().0 == DesyncState::Running {
            return Ok(());
        }
        let Some(profile) = self.profile.clone() else {
            return self.fail(DesyncError::NoProfile);
        };
        let result = self.start_locked(progress, &profile);
        self.record(&result);
        result
    }

    fn start_locked(
        &self,
        progress: &mut dyn FnMut(PrepareProgress),
        profile: &Profile,
    ) -> Result<(), DesyncError> {
        match self.adapter.readiness() {
            Readiness::Ready => {}
            Readiness::NotImplemented(why) | Readiness::Unsupported(why) => {
                return Err(DesyncError::Unavailable(why));
            }
        }
        if (self.vpn)().active {
            return Err(DesyncError::VpnRunning);
        }
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

/// Источник состояния VPN для `DesyncService::new`: читает настоящий `VpnService::status()`.
/// Держит только `Weak`, поэтому два сервиса не владеют друг другом. Если VPN недоступен —
/// считаем его активным и непроверенным (безопасный отказ, а не «можно запускать»).
pub fn vpn_signal_source(
    vpn: Weak<VpnService>,
) -> Box<dyn Fn() -> routing::VpnSignal + Send + Sync> {
    Box::new(move || match vpn.upgrade() {
        Some(vpn) => routing::VpnSignal::from_status(&vpn.status()),
        None => routing::VpnSignal {
            active: true,
            verified: false,
        },
    })
}

/// Защита для `VpnService::set_conflict_guard`: VPN не подключается, пока работает перехват.
/// Если сервис обхода недоступен или состояние неизвестно — отказ.
/// Внимание: проверка и последующий запуск не атомарны; до включения функции нужен общий
/// атомарный механизм (одна блокировка на оба сервиса).
pub fn desync_conflict_guard(desync: Weak<DesyncService>) -> ConflictGuard {
    Box::new(move || match desync.upgrade() {
        Some(d) if !d.is_running() => None,
        Some(_) => Some("Работает обход блокировок. Сначала остановите его".into()),
        None => Some("Состояние обхода блокировок недоступно".into()),
    })
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
            Box::new(move || routing::VpnSignal {
                active: vpn,
                verified: vpn,
            }),
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
    fn candidate_profile_follows_upstream_tcp_example_but_is_not_activated() {
        let candidate = windows_tcp_candidate_profile();
        assert_eq!(candidate.engine, Engine::Winws2);
        assert_eq!(candidate.args[0], "--wf-tcp-out=80,443");
        assert_eq!(candidate.args[1], "--lua-init=@zapret-lib.lua");
        assert_eq!(candidate.args[2], "--lua-init=@zapret-antidpi.lua");
        assert!(candidate
            .args
            .iter()
            .any(|arg| arg.starts_with("--lua-desync=fake:")));
        assert!(candidate
            .args
            .iter()
            .any(|arg| arg.starts_with("--lua-desync=multisplit:")));
        assert!(!candidate.args.iter().any(|arg| arg.starts_with("--wf-udp")));
        assert!(site_discord_profile(Platform::Windows).is_none());
    }

    #[test]
    fn whole_netzapret_profile_is_in_core_but_not_user_enabled() {
        let candidate = windows_netzapret_v13_lite_candidate_profile().unwrap();
        assert_eq!(candidate.engine, Engine::Winws2);
        assert_eq!(candidate.id, "windows-netzapret-v13-lite-unverified");
        assert_eq!(candidate.args.len(), 564);
        assert_eq!(
            candidate
                .args
                .iter()
                .filter(|x| x.as_str() == "--new")
                .count(),
            73
        );
        assert_eq!(
            candidate.args.first().map(String::as_str),
            Some("--lua-init=@lua/zapret-lib.lua")
        );
        assert!(site_discord_profile(Platform::Windows).is_none());
    }

    #[test]
    fn verified_windows_engine_is_pinned_but_profile_remains_disabled() {
        let spec = &PINNED_WINDOWS;
        assert!(crate::vpn::engine::url_allowed_under(
            spec.url,
            spec.url_prefix
        ));
        assert_eq!(spec.archive_sha256.len(), 64);
        assert_eq!(
            spec.files.len(),
            9,
            "пять бинарных файлов, лицензии и manifest.json"
        );
        for required_name in [
            "winws2.exe",
            "cygwin1.dll",
            "cygz.dll",
            "WinDivert.dll",
            "WinDivert64.sys",
        ] {
            assert!(spec.files.iter().any(|(name, _)| *name == required_name));
        }
        assert!(spec.files.iter().all(|(_, hash)| hash.len() == 64));
        assert_eq!(
            pinned().is_some(),
            cfg!(all(target_os = "windows", target_arch = "x86_64"))
        );

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
            Box::new(routing::VpnSignal::default),
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

    use routing::{
        AccessPath, ConfirmedCoverage, Reachability, RouteBlock, RoutePlan, ServiceObservations,
    };

    static TEST_CONFIRMED: [ConfirmedCoverage; 1] = [ConfirmedCoverage {
        profile_id: "sites-discord",
        engine: Engine::Winws2,
        covers: &[Coverage::Sites, Coverage::Discord],
    }];

    /// Сервис с подтверждённым (ТОЛЬКО в тесте) покрытием: проверка пути запуска по плану.
    fn make_confirmed(flags: &Arc<Flags>, vpn: bool) -> DesyncService {
        flags.present.store(true, Ordering::SeqCst);
        let mut s = make(flags, good, true, vpn);
        s.candidate = profile();
        s.confirmed = &TEST_CONFIRMED;
        s
    }

    fn blocked(wanted: &[Coverage]) -> ServiceObservations {
        let mut o = ServiceObservations::new();
        for w in wanted {
            o.set(*w, Reachability::Unreachable);
        }
        o
    }

    #[test]
    fn link_helpers_fail_safe_and_follow_real_state() {
        // VPN недоступен → считается активным и непроверенным.
        let sig = vpn_signal_source(Weak::new())();
        assert!(sig.active && !sig.verified);
        // Сервис обхода недоступен → VPN запускать нельзя.
        assert!(desync_conflict_guard(Weak::new())().is_some());
        // Реальный сервис: пока не работает — разрешено, работает — отказ.
        let flags = Arc::new(Flags::default());
        let s = Arc::new(make_confirmed(&flags, false));
        let guard = desync_conflict_guard(Arc::downgrade(&s));
        assert!(guard().is_none());
        s.start_planned(
            &[Coverage::Sites],
            &blocked(&[Coverage::Sites]),
            &mut |_| {},
        )
        .unwrap();
        assert!(guard().is_some());
        s.shutdown();
        assert!(guard().is_none());
        // Нет циклов владения: после удаления сервиса защита отказывает.
        drop(s);
        assert!(guard().is_some());
    }

    #[test]
    fn live_service_plan_stays_off_for_unverified_profile() {
        let flags = Arc::new(Flags::default());
        let service = make(&flags, good, true, false);
        // Адаптер в тесте заявляет Ready, но реальный пресет NetZapret не подтверждён опытом.
        let plan = service.plan_access(&[Coverage::Sites], &blocked(&[Coverage::Sites]));
        assert_eq!(
            plan.overall,
            RoutePlan::Block(RouteBlock::UnverifiedCoverage)
        );
        let reachable = ServiceObservations::new().with(Coverage::Sites, Reachability::Reachable);
        assert_eq!(
            service.plan_access(&[Coverage::Sites], &reachable).overall,
            RoutePlan::Use(AccessPath::Direct)
        );
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        assert!(!flags.installed.load(Ordering::SeqCst));
    }

    #[test]
    fn start_planned_never_starts_for_blocked_direct_or_vpn_plans() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        // 1. Неподтверждённый пресет: перехват запрещён.
        let s = make(&flags, good, true, false);
        let err = s
            .start_planned(
                &[Coverage::Sites],
                &blocked(&[Coverage::Sites]),
                &mut |_| {},
            )
            .unwrap_err();
        assert!(matches!(err, DesyncError::NotAllowed(_)), "{err:?}");
        assert_eq!(s.status().0, DesyncState::Error);
        // 2. Неизвестное состояние сервиса — не успех.
        let s = make_confirmed(&flags, false);
        let err = s
            .start_planned(&[Coverage::Sites], &ServiceObservations::new(), &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, DesyncError::NotAllowed(_)), "{err:?}");
        // 3. Всё доступно напрямую: запускать нечего.
        let reachable = ServiceObservations::new().with(Coverage::Sites, Reachability::Reachable);
        let err = s
            .start_planned(&[Coverage::Sites], &reachable, &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, DesyncError::NotNeeded(_)), "{err:?}");
        // 4. Работает подтверждённый VPN: перехват не накладывается.
        let s = make_confirmed(&flags, true);
        let err = s
            .start_planned(
                &[Coverage::Sites],
                &blocked(&[Coverage::Sites]),
                &mut |_| {},
            )
            .unwrap_err();
        assert!(matches!(err, DesyncError::NotNeeded(_)), "{err:?}");
        let err = s
            .start_planned(
                &[Coverage::Discord],
                &blocked(&[Coverage::Discord]),
                &mut |_| {},
            )
            .unwrap_err();
        assert!(matches!(err, DesyncError::NotAllowed(_)), "{err:?}");
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn start_planned_runs_one_confirmed_profile_once_and_stops_cleanly() {
        let flags = Arc::new(Flags::default());
        let s = make_confirmed(&flags, false);
        let wanted = [Coverage::Sites, Coverage::Discord];
        let mixed = ServiceObservations::new()
            .with(Coverage::Sites, Reachability::Reachable)
            .with(Coverage::Discord, Reachability::Unreachable);
        s.start_planned(&wanted, &mixed, &mut |_| {}).unwrap();
        assert!(s.is_running());
        // Второй запуск (в том числе «из другого места») второй экземпляр не создаёт.
        s.start_planned(&wanted, &mixed, &mut |_| {}).unwrap();
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 1);
        s.stop();
        assert!(!s.is_running());
        assert!(flags.killed.load(Ordering::SeqCst));
        assert!(!s.pid_file().exists());
    }

    #[test]
    fn start_planned_does_not_cover_a_service_the_profile_was_not_confirmed_for() {
        let flags = Arc::new(Flags::default());
        let s = make_confirmed(&flags, false);
        let wanted = [Coverage::Sites, Coverage::Telegram];
        let err = s
            .start_planned(&wanted, &blocked(&wanted), &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, DesyncError::NotAllowed(_)), "{err:?}");
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn start_planned_reports_spawn_and_privilege_errors_without_claiming_success() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let mut s = make(
            &flags,
            |f| Host {
                spawn_fails: true,
                ..good(f)
            },
            true,
            false,
        );
        s.candidate = profile();
        s.confirmed = &TEST_CONFIRMED;
        let err = s
            .start_planned(
                &[Coverage::Sites],
                &blocked(&[Coverage::Sites]),
                &mut |_| {},
            )
            .unwrap_err();
        assert!(matches!(err, DesyncError::Start(_)), "{err:?}");
        assert!(!s.is_running());
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let mut s = make(
            &flags,
            |f| Host {
                elevated: false,
                ..good(f)
            },
            true,
            false,
        );
        s.candidate = profile();
        s.confirmed = &TEST_CONFIRMED;
        let err = s
            .start_planned(
                &[Coverage::Sites],
                &blocked(&[Coverage::Sites]),
                &mut |_| {},
            )
            .unwrap_err();
        assert_eq!(err, DesyncError::NeedsAdmin);
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn vpn_starting_after_desync_is_seen_and_unverified_vpn_blocks_everything() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let signal = Arc::new(Mutex::new(routing::VpnSignal::default()));
        let shared = signal.clone();
        let mut s = DesyncService::new(
            temp_dir(),
            Box::new(good(flags.clone())),
            Box::new(move || *shared.lock().unwrap()),
        );
        s.candidate = profile();
        s.confirmed = &TEST_CONFIRMED;
        s.settle = (1, Duration::from_millis(1));
        s.start_planned(
            &[Coverage::Sites],
            &blocked(&[Coverage::Sites]),
            &mut |_| {},
        )
        .unwrap();
        assert!(s.is_running());
        // VPN подключается: план больше не разрешает перехват, даже если «напрямую недоступно».
        *signal.lock().unwrap() = routing::VpnSignal {
            active: true,
            verified: false,
        };
        let plan = s.plan_access(&[Coverage::Sites], &blocked(&[Coverage::Sites]));
        assert_eq!(plan.overall, RoutePlan::Block(RouteBlock::VpnNotVerified));
        s.stop();
        assert!(!s.is_running());
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
            let s = DesyncService::new(
                temp_dir(),
                Box::new(adapter),
                Box::new(routing::VpnSignal::default),
            );
            assert!(s.start(&mut |_| {}).is_err());
            assert_eq!(s.status().0, DesyncState::Error);
        }
    }
}
