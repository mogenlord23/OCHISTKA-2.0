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

pub mod coordinator;
pub mod preset;
pub mod probe;
pub mod routing;
pub mod speed;

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, Weak};
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
    Youtube,
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

/// Предел длины командной строки CreateProcess в Windows (символов).
pub const WINDOWS_COMMAND_LINE_LIMIT: usize = 32_767;

/// Длина командной строки Windows для запуска `exe` с `args` (как её собирает Rust: путь в
/// кавычках, аргументы через пробел, аргументы с пробелом — в кавычках, `"` экранируется).
pub fn windows_command_line_len(exe: &str, args: &[String]) -> usize {
    let mut len = exe.chars().count() + 2;
    for arg in args {
        let quoted = arg.is_empty() || arg.contains([' ', '\t']);
        len += 1 + arg.chars().count() + arg.matches('"').count();
        if quoted {
            len += 2;
        }
    }
    len
}

/// Заранее отказывает, если командная строка не поместится, вместо загадочной ошибки ОС 206.
pub fn check_command_line(exe: &str, args: &[String]) -> Result<usize, String> {
    let len = windows_command_line_len(exe, args);
    if len > WINDOWS_COMMAND_LINE_LIMIT {
        return Err(format!(
            "набор правил слишком длинный для запуска: {len} символов, Windows принимает не больше {WINDOWS_COMMAND_LINE_LIMIT}"
        ));
    }
    Ok(len)
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

/// Единый объединённый пресет «Обход блокировок» (основа Universal V13 Lite + секции из других
/// пресетов RixyPow/netzapret без повторов; см. `docs/status/bypass-merge-*.md`). НЕ подтверждён
/// на реальной сети: годится только для явного пробного запуска, обычный профиль не разблокирует.
pub fn windows_merged_candidate_profile() -> Result<Profile, preset::PresetError> {
    let merged = include_str!("../../../assets/desync/presets/OCHISTKA Merged.txt");
    let parsed = preset::ZapretPreset::parse(merged)?;
    Ok(Profile {
        id: "windows-ochistka-merged-unverified",
        engine: Engine::Winws2,
        title: "Обход блокировок (объединённый пресет, не проверен)",
        args: parsed.launch_arguments(),
    })
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
    #[error("Не удалось надёжно остановить обход: {0}")]
    Stop(String),
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

/// Состояние службы драйвера перехвата.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriverState {
    /// На этой ОС драйвера нет.
    NotApplicable,
    /// Службы нет.
    Absent,
    Stopped,
    Running,
    /// Состояние выяснить не удалось.
    Unknown(String),
}

/// Короткая причина для интерфейса, если драйвер не остановился.
pub const DRIVER_NOT_STOPPED: &str = "Драйвер не остановлен, перезагрузите компьютер";

/// Файл состояния обхода в папке данных: помнит, что службу драйвера запустила программа.
const STATE_FILE: &str = "desync-state.json";

/// Системный адаптер: всё, что зависит от ОС (реализации — в `crates/platform`, по одной на ОС).
pub trait DesyncAdapter: Send + Sync {
    fn platform(&self) -> Platform;
    /// Готов ли адаптер. Не `Ready` — переключатель не включается, запуск отказывает.
    fn readiness(&self) -> Readiness;
    /// Готовность ТОЛЬКО для явного пробного запуска. Обычную `readiness()` не меняет и не
    /// разблокирует переключатель. По умолчанию пробный режим недоступен.
    fn trial_readiness(&self) -> Readiness {
        Readiness::NotImplemented("Пробный режим недоступен на этой системе".into())
    }
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
    /// Заранее (в фоне, при старте программы) проверяет и запоминает целостность уже установленных
    /// файлов, чтобы нажатие кнопки не тратило на это время. Ничего не скачивает и не запускает.
    fn warm_up(&self) {}
    /// Состояние службы драйвера перехвата (Windows: WinDivert). Где драйвера нет — `NotApplicable`.
    fn driver_state(&self) -> DriverState {
        DriverState::NotApplicable
    }
    /// Останавливает службу драйвера. Вызывается только для службы, которую запустила программа.
    fn stop_driver(&self) -> Result<(), String> {
        Ok(())
    }
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
    /// Общее право на сетевой режим (одно на VPN и обход); по умолчанию собственное.
    coord: Arc<coordinator::NetCoordinator>,
    /// Отключает работающий VPN перед включением обхода (включение обхода выключает VPN).
    vpn_stopper: Mutex<Option<VpnStopper>>,
    /// Замер скорости до и после включения (если задан транспорт).
    speed: Mutex<Option<Arc<dyn speed::TimedFetch>>>,
    /// Проверка скорости после запуска идёт в фоновом потоке (в тестах — вызывается вручную).
    speed_async: bool,
    /// Пробный режим: срок, номер запуска (старый таймер сам завершается) и пометки пользователя.
    trial: Mutex<TrialInner>,
    trial_limit: Duration,
    /// Сколько раз и как часто проверять, что процесс действительно завершился после остановки.
    stop_wait: (u32, Duration),
    /// Сколько раз подряд повторять `kill()` за одну остановку.
    kill_attempts: u32,
    inner: Mutex<Inner>,
    op: Mutex<()>,
}

/// Сколько неудачных остановок подряд таймер повторяет, прежде чем перестать (без бесконечных потоков).
const MAX_FAILED_STOPS: u32 = 5;

/// Отключение VPN: `Err(причина)`, если отключить не удалось.
pub type VpnStopper = Box<dyn Fn() -> Result<(), String> + Send + Sync>;

/// Максимальная длительность пробного запуска полного профиля.
pub const TRIAL_LIMIT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialVerdict {
    Works,
    Fails,
    Unchecked,
}

/// Сколько заняли этапы последнего запуска (для журнала и тестов).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartTimings {
    pub vpn_stop: Duration,
    /// Проверка файлов, подготовка, права, остаток прошлого запуска, служба драйвера.
    pub prepare: Duration,
    /// Сколько ещё пришлось ждать замер, который идёт параллельно с подготовкой.
    pub baseline_wait: Duration,
    /// Запуск процесса и проверка, что он не завершился сразу.
    pub spawn_settle: Duration,
    pub total: Duration,
}

/// Сколько результат замера без обхода считается годным для следующего включения.
const BASELINE_CACHE: Duration = Duration::from_secs(300);

/// Что сейчас делает пробный запуск (для надписи «Проверяем…»).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrialPhase {
    #[default]
    Idle,
    MeasuringBaseline,
    MeasuringAfter,
}

#[derive(Default)]
struct TrialInner {
    deadline: Option<std::time::Instant>,
    generation: u64,
    /// Сколько раз подряд таймер не смог остановить процесс (ограничивает повторы).
    failed_stops: u32,
    phase: TrialPhase,
    /// Только для тестов: базовый замер, ожидающий ручного вызова проверки.
    pending_baseline: Option<speed::Measurement>,
    /// Последний успешный замер без обхода и когда он сделан (повторное включение без ожидания).
    baseline_cache: Option<(std::time::Instant, speed::Measurement)>,
    timings: Option<StartTimings>,
    /// Итог замера скорости для интерфейса (откат или «не медленнее»).
    speed_note: Option<String>,
    notes: Vec<(Coverage, TrialVerdict)>,
}

/// Состояние пробного запуска для интерфейса. Пометки пользователя — только его наблюдения,
/// они не являются подтверждением работы сервиса.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrialStatus {
    pub active: bool,
    pub remaining_secs: u64,
    pub message: String,
    /// Причина, по которой пробный запуск сейчас невозможен (если невозможен).
    pub unavailable: Option<String>,
    pub notes: Vec<(Coverage, TrialVerdict)>,
    /// Короткая причина, если обход не выключился до конца (например, драйвер не остановлен).
    pub error: Option<String>,
    pub phase: TrialPhase,
    /// Итог замера скорости: «медленнее — обход отключён» или «быстрее или так же».
    pub speed_note: Option<String>,
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
            windows_merged_candidate_profile().ok()
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
            coord: Arc::new(coordinator::NetCoordinator::new()),
            vpn_stopper: Mutex::new(None),
            speed: Mutex::new(None),
            speed_async: true,
            trial: Mutex::new(TrialInner::default()),
            trial_limit: TRIAL_LIMIT,
            stop_wait: (25, Duration::from_millis(40)),
            kill_attempts: 3,
            candidate,
            confirmed: routing::CONFIRMED_EXTERNAL_PROFILES,
            // Проверка, что процесс не завершился сразу: 8 × 50 мс = 400 мс (раньше 1 с ожидания).
            settle: (8, Duration::from_millis(50)),
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
        if lock(&self.inner).process.is_some() {
            // Прошлый процесс не остановлен: нового запуска нет, пока это не устранено.
            return Err(DesyncError::NotAllowed(
                "Прошлый запуск не остановлен. Повторите остановку или перезапустите OCHISTKA"
                    .into(),
            ));
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
        // Право занимается атомарно ДО побочных эффектов запуска.
        if let Err(denied) = self.coord.begin(coordinator::Owner::Desync) {
            return self.fail(DesyncError::NotAllowed(denied.to_string()));
        }
        let result = self.start_locked(
            progress,
            &profile,
            false,
            None,
            &mut StartTimings::default(),
        );
        match &result {
            Ok(()) => self.coord.activate(coordinator::Owner::Desync),
            Err(_) => self.coord.release(coordinator::Owner::Desync),
        }
        self.record(&result);
        result
    }

    /// Задаёт действие «отключить VPN» для автоотключения при включении обхода.
    pub fn set_vpn_stopper(&self, stopper: VpnStopper) {
        *lock(&self.vpn_stopper) = Some(stopper);
    }

    /// Задаёт транспорт замера скорости: включается замер до и после запуска обхода.
    pub fn set_speed_probe(&self, probe: Arc<dyn speed::TimedFetch>) {
        *lock(&self.speed) = Some(probe);
    }

    fn set_phase(&self, phase: TrialPhase) {
        lock(&self.trial).phase = phase;
    }

    /// Замер БЕЗ обхода стартует сразу и идёт параллельно с подготовкой файлов; к запуску движка
    /// его результат нужен готовым. Свежий прошлый результат (≤ 5 минут) берётся без нового
    /// замера. Без транспорта — `None`. «Ничего не открылось» обход не блокирует: сайт, закрытый
    /// без обхода, и есть цель обхода.
    fn begin_baseline(&self) -> Result<Option<mpsc::Receiver<speed::Measurement>>, DesyncError> {
        let Some(probe) = lock(&self.speed).clone() else {
            return Ok(None);
        };
        let (tx, rx) = mpsc::channel();
        {
            let mut t = lock(&self.trial);
            if t.phase != TrialPhase::Idle {
                return Err(DesyncError::Busy);
            }
            if let Some((at, m)) = t.baseline_cache.clone() {
                if at.elapsed() < BASELINE_CACHE {
                    let _ = tx.send(m);
                    return Ok(Some(rx));
                }
            }
            t.phase = TrialPhase::MeasuringBaseline;
        }
        std::thread::spawn(move || {
            let _ = tx.send(speed::measure(probe, &speed::URLS, speed::BASELINE));
        });
        Ok(Some(rx))
    }

    /// Ждёт результат замера без обхода (ограничен пределом замера) и запоминает его.
    fn finish_baseline(&self, rx: &mpsc::Receiver<speed::Measurement>) {
        let measurement = rx
            .recv_timeout(speed::BASELINE.total + Duration::from_secs(1))
            .ok();
        let mut t = lock(&self.trial);
        t.phase = TrialPhase::Idle;
        if let Some(m) = measurement {
            if m.any_opened() {
                t.baseline_cache = Some((std::time::Instant::now(), m.clone()));
            }
            t.pending_baseline = Some(m);
        } else {
            t.pending_baseline = None;
        }
    }

    /// Одна строка в журнал запуска (папка данных обхода): чтобы по файлу было видно, где ушло время.
    fn log_timings(&self, t: &StartTimings) {
        use std::io::Write;
        let _ = std::fs::create_dir_all(&self.dir);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("desync-start.log"))
        {
            let _ = writeln!(
                f,
                "запуск: всего {} мс (VPN {} мс, подготовка {} мс, ожидание проверки {} мс, процесс {} мс)",
                t.total.as_millis(),
                t.vpn_stop.as_millis(),
                t.prepare.as_millis(),
                t.baseline_wait.as_millis(),
                t.spawn_settle.as_millis()
            );
        }
    }

    /// Подготовка в фоне при старте программы (проверка установленных файлов).
    pub fn warm_up(&self) {
        self.adapter.warm_up();
    }

    /// Этапы последнего запуска.
    pub fn last_start_timings(&self) -> Option<StartTimings> {
        lock(&self.trial).timings
    }

    /// Замер с обходом (в фоне, ≤ 10 с) и сравнение только по адресам, которые открылись без обхода.
    /// Медленнее или перестал открываться — обход выключается (тем же надёжным путём остановки,
    /// включая драйвер); иначе остаётся до конца 120 с. Если без обхода не открылось ничего,
    /// откат не делается. Если пробный запуск уже остановлен (кнопка, таймер), результат не применяется.
    pub(crate) fn run_speed_check(&self, generation: u64, baseline: &speed::Measurement) {
        let Some(probe) = lock(&self.speed).clone() else {
            return;
        };
        {
            let mut t = lock(&self.trial);
            if t.generation != generation || t.deadline.is_none() {
                return;
            }
            t.phase = TrialPhase::MeasuringAfter;
        }
        let after = speed::measure(probe, &speed::URLS, speed::AFTER);
        let (note, rollback) = match speed::compare(baseline, &after) {
            speed::Verdict::Slower(_) => (speed::SLOWER_MESSAGE, true),
            speed::Verdict::NotSlower => (speed::NOT_SLOWER_MESSAGE, false),
            speed::Verdict::NothingToCompare {
                opened_with_bypass: true,
            } => (speed::NO_COMPARE_MESSAGE, false),
            speed::Verdict::NothingToCompare {
                opened_with_bypass: false,
            } => (speed::NO_INTERNET_MESSAGE, false),
        };
        {
            let mut t = lock(&self.trial);
            t.phase = TrialPhase::Idle;
            if t.generation != generation {
                return;
            }
            t.speed_note = Some(note.into());
        }
        if rollback {
            let _ = self.stop_trial();
        }
    }

    /// Если VPN работает — отключает его и убеждается, что он действительно не активен.
    /// Без заданного действия или при неудаче — отказ (запуск обхода не происходит).
    fn stop_vpn_if_active(&self) -> Result<(), DesyncError> {
        if !(self.vpn)().active {
            return Ok(());
        }
        let result = match lock(&self.vpn_stopper).as_ref() {
            Some(stop) => stop(),
            None => return Err(DesyncError::VpnRunning),
        };
        result.map_err(|why| {
            DesyncError::NotAllowed(format!(
                "Не удалось отключить VPN перед включением обхода: {why}"
            ))
        })?;
        if (self.vpn)().active {
            return Err(DesyncError::VpnRunning);
        }
        Ok(())
    }

    /// Подключает общий координатор (один экземпляр на VPN и обход).
    pub fn with_coordinator(mut self, coord: Arc<coordinator::NetCoordinator>) -> Self {
        self.coord = coord;
        self
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
        let result = self.start_locked(
            progress,
            &profile,
            false,
            None,
            &mut StartTimings::default(),
        );
        self.record(&result);
        result
    }

    fn start_locked(
        &self,
        progress: &mut dyn FnMut(PrepareProgress),
        profile: &Profile,
        trial: bool,
        baseline: Option<mpsc::Receiver<speed::Measurement>>,
        timings: &mut StartTimings,
    ) -> Result<(), DesyncError> {
        let prepare_started = std::time::Instant::now();
        let readiness = if trial {
            self.adapter.trial_readiness()
        } else {
            self.adapter.readiness()
        };
        match readiness {
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
        // Служба драйвера от прошлого запуска должна быть остановлена до нового.
        self.release_driver()?;
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| DesyncError::Start(format!("папка недоступна: {e}")))?;
        let driver_before = self.adapter.driver_state();
        timings.prepare = prepare_started.elapsed();
        // Замер без обхода шёл параллельно подготовке; движок стартует только после него.
        if let Some(rx) = &baseline {
            let waited = std::time::Instant::now();
            self.finish_baseline(rx);
            timings.baseline_wait = waited.elapsed();
        }
        let spawn_started = std::time::Instant::now();
        let mut process = self
            .adapter
            .spawn(&exe, &profile.args, &self.log_file())
            .map_err(|why| DesyncError::Start(mask_secrets(&why)))?;
        let _ = std::fs::write(self.pid_file(), process.pid().to_string());
        for _ in 0..self.settle.0 {
            if process.has_exited() {
                let _ = std::fs::remove_file(self.pid_file());
                self.note_driver_started(&driver_before);
                let _ = self.release_driver();
                return Err(DesyncError::Start(
                    "процесс завершился сразу после запуска".into(),
                ));
            }
            std::thread::sleep(self.settle.1);
        }
        self.note_driver_started(&driver_before);
        timings.spawn_settle = spawn_started.elapsed();
        let mut inner = lock(&self.inner);
        inner.process = Some(process);
        inner.state = DesyncState::Running;
        inner.message = format!("Включено · {}", profile.title);
        Ok(())
    }

    /// Пробный запуск ПОЛНОГО профиля NetZapret Universal V13 Lite. Только по явному согласию
    /// (проверяется здесь, а не только в интерфейсе), только Windows, одним экземпляром, под
    /// общим координатором (при VPN или его неочищенных настройках — отказ). Останавливается
    /// автоматически через `TRIAL_LIMIT`. Обычную готовность адаптера и реестр подтверждённых
    /// профилей не меняет.
    pub fn start_trial(
        self: &Arc<Self>,
        consent: bool,
        progress: &mut dyn FnMut(PrepareProgress),
    ) -> Result<(), DesyncError> {
        if !consent {
            return Err(DesyncError::NotAllowed(
                "Для пробного запуска нужно ваше явное согласие".into(),
            ));
        }
        if self.adapter.platform() != Platform::Windows {
            return Err(DesyncError::Unavailable(
                "Пробный запуск доступен только в Windows".into(),
            ));
        }
        let Some(profile) = self.candidate.clone() else {
            return Err(DesyncError::NoProfile);
        };
        // Включение обхода выключает VPN. Делается ДО блокировки `op`, чтобы два сервиса
        // не ждали друг друга; и только если обход вообще может стартовать (готовность, права).
        if let Readiness::NotImplemented(why) | Readiness::Unsupported(why) =
            self.adapter.trial_readiness()
        {
            return self.fail(DesyncError::Unavailable(why));
        }
        if !self.adapter.has_privileges() {
            return Err(DesyncError::NeedsAdmin);
        }
        let started = std::time::Instant::now();
        let mut timings = StartTimings::default();
        let baseline_rx = if self.status().0 != DesyncState::Running {
            self.stop_vpn_if_active()?;
            timings.vpn_stop = started.elapsed();
            {
                let mut t = lock(&self.trial);
                t.speed_note = None;
                t.pending_baseline = None;
            }
            self.begin_baseline()?
        } else {
            None
        };
        let Ok(_op) = self.op.try_lock() else {
            return Err(DesyncError::Busy);
        };
        if self.status().0 == DesyncState::Running {
            return Err(DesyncError::NotAllowed(
                "Пробный запуск уже идёт: второй экземпляр не создаётся".into(),
            ));
        }
        if lock(&self.inner).process.is_some() {
            // Прошлый процесс не остановлен: нового запуска нет, пока это не устранено.
            return Err(DesyncError::NotAllowed(
                "Прошлый запуск не остановлен. Повторите остановку или перезапустите OCHISTKA"
                    .into(),
            ));
        }
        match self.adapter.trial_readiness() {
            Readiness::Ready => {}
            Readiness::NotImplemented(why) | Readiness::Unsupported(why) => {
                return self.fail(DesyncError::Unavailable(why));
            }
        }
        if let Err(denied) = self.coord.begin(coordinator::Owner::Desync) {
            return self.fail(DesyncError::NotAllowed(denied.to_string()));
        }
        let result = self.start_locked(progress, &profile, true, baseline_rx, &mut timings);
        if result.is_err() {
            // Незавершённый замер без обхода больше не нужен.
            self.set_phase(TrialPhase::Idle);
        }
        match &result {
            Ok(()) => {
                timings.total = started.elapsed();
                lock(&self.trial).timings = Some(timings);
                self.log_timings(&timings);
                self.coord.activate(coordinator::Owner::Desync);
                let generation = {
                    let mut t = lock(&self.trial);
                    t.generation += 1;
                    t.notes.clear();
                    t.deadline = Some(std::time::Instant::now() + self.trial_limit);
                    t.generation
                };
                lock(&self.inner).message = format!(
                    "Пробный запуск · {} · не более {} с",
                    profile.title,
                    self.trial_limit.as_secs()
                );
                self.spawn_trial_timer(generation);
                let baseline = lock(&self.trial).pending_baseline.clone();
                if let Some(baseline) = baseline {
                    self.spawn_speed_check(generation, baseline);
                }
            }
            Err(_) => self.coord.release(coordinator::Owner::Desync),
        }
        self.record(&result);
        result
    }

    /// Проверка скорости с обходом — в фоне, интерфейс не блокируется.
    fn spawn_speed_check(self: &Arc<Self>, generation: u64, baseline: speed::Measurement) {
        if !self.speed_async {
            lock(&self.trial).pending_baseline = Some(baseline);
            return;
        }
        let weak = Arc::downgrade(self);
        std::thread::spawn(move || {
            if let Some(service) = weak.upgrade() {
                service.run_speed_check(generation, &baseline);
            }
        });
    }

    /// Таймер в Rust: следит за сроком и за самим процессом. Держит только `Weak`.
    fn spawn_trial_timer(self: &Arc<Self>, generation: u64) {
        let weak = Arc::downgrade(self);
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(250));
            let Some(service) = weak.upgrade() else {
                break;
            };
            if lock(&service.trial).generation != generation {
                break;
            }
            if service.tick(std::time::Instant::now()) {
                break;
            }
        });
    }

    /// Один шаг контроля. Останавливает при истёкшем сроке, при самостоятельном завершении
    /// процесса и при предыдущей неудачной остановке (процесс не остаётся без наблюдения).
    /// Возвращает `true`, когда контроль больше не нужен: остановка подтверждена либо исчерпан
    /// лимит повторов (тогда процесс остаётся учтённым, состояние — ошибка, запуск заблокирован).
    pub(crate) fn tick(&self, now: std::time::Instant) -> bool {
        let Some(deadline) = lock(&self.trial).deadline else {
            return true;
        };
        let (failed_before, died) = {
            let mut inner = lock(&self.inner);
            let died = inner.process.as_mut().is_some_and(|p| p.has_exited());
            (
                inner.state == DesyncState::Error && inner.process.is_some(),
                died,
            )
        };
        let expired = now >= deadline;
        if !expired && !died && !failed_before {
            return false;
        }
        let why = if expired {
            "Пробный запуск остановлен: истекло время"
        } else {
            "Пробный запуск остановлен: процесс завершился сам"
        };
        match self.stop_trial() {
            Ok(()) => {
                lock(&self.inner).message = why.into();
                true
            }
            Err(_) => {
                let mut t = lock(&self.trial);
                t.failed_stops += 1;
                t.failed_stops >= MAX_FAILED_STOPS
            }
        }
    }

    /// Единый надёжный путь остановки (ручной, по таймеру, при выходе). Вызывается под `op`.
    /// Право освобождается и состояние «Выключено» ставится ТОЛЬКО после подтверждения, что
    /// процесс завершился. Иначе: процесс остаётся учтённым, состояние — ошибка, право `Unclean`.
    fn stop_process(&self) -> Result<(), DesyncError> {
        self.coord.begin_stop(coordinator::Owner::Desync);
        let mut inner = lock(&self.inner);
        if let Some(mut p) = inner.process.take() {
            let mut gone = p.has_exited();
            for _ in 0..self.kill_attempts {
                if gone {
                    break;
                }
                p.kill();
                for _ in 0..self.stop_wait.0 {
                    gone = p.has_exited();
                    if gone {
                        break;
                    }
                    std::thread::sleep(self.stop_wait.1);
                }
                gone = gone || p.has_exited();
            }
            if !gone {
                inner.process = Some(p);
                inner.state = DesyncState::Error;
                inner.message =
                    "Не удалось остановить обход. Повторите остановку или перезапустите OCHISTKA"
                        .into();
                drop(inner);
                self.coord.mark_unclean(coordinator::Owner::Desync);
                return Err(DesyncError::Stop("процесс не завершился".into()));
            }
        }
        let _ = std::fs::remove_file(self.pid_file());
        // Процесс завершён: если службу драйвера запустила программа, останавливаем и её.
        // Пока служба работает, обход НЕ считается выключенным.
        if let Err(err) = self.release_driver() {
            inner.state = DesyncState::Error;
            inner.message = DRIVER_NOT_STOPPED.into();
            drop(inner);
            self.coord.mark_unclean(coordinator::Owner::Desync);
            return Err(err);
        }
        inner.state = DesyncState::Off;
        inner.message = "Выключено".into();
        drop(inner);
        self.coord.release(coordinator::Owner::Desync);
        Ok(())
    }

    /// Ручная или автоматическая остановка пробного запуска. Срок и контроль таймера снимаются
    /// только после подтверждённой остановки.
    pub fn stop_trial(&self) -> Result<(), DesyncError> {
        let _op = lock(&self.op);
        self.stop_process()?;
        let mut t = lock(&self.trial);
        t.deadline = None;
        t.generation += 1;
        t.failed_stops = 0;
        Ok(())
    }

    /// Пометка пользователя («работает / не работает / не проверено») во время испытания.
    /// Это только его наблюдение: подтверждением работы сервиса оно не становится.
    pub fn note_trial(&self, service: Coverage, verdict: TrialVerdict) -> Result<(), DesyncError> {
        let mut t = lock(&self.trial);
        if t.deadline.is_none() {
            return Err(DesyncError::NotAllowed(
                "Пометки доступны только во время пробного запуска".into(),
            ));
        }
        match t.notes.iter_mut().find(|(s, _)| *s == service) {
            Some(entry) => entry.1 = verdict,
            None => t.notes.push((service, verdict)),
        }
        Ok(())
    }

    pub fn trial_status(&self, now: std::time::Instant) -> TrialStatus {
        let t = lock(&self.trial);
        // Пока остановка не подтверждена, срок сохраняется: испытание считается идущим.
        let active = t.deadline.is_some();
        let remaining_secs = t
            .deadline
            .map_or(0, |d| d.saturating_duration_since(now).as_secs());
        let notes = t.notes.clone();
        let (phase, speed_note) = (t.phase, t.speed_note.clone());
        drop(t);
        let unavailable = if active {
            None
        } else if lock(&self.inner).process.is_some() {
            Some(
                "Прошлый запуск не остановлен. Повторите остановку или перезапустите OCHISTKA"
                    .into(),
            )
        } else if self.adapter.platform() != Platform::Windows {
            Some("Пробный запуск доступен только в Windows".into())
        } else if self.candidate.is_none() {
            Some("Профиль NetZapret недоступен в этой сборке".into())
        } else {
            match self.adapter.trial_readiness() {
                Readiness::Ready if !self.adapter.has_privileges() => {
                    Some("Нужны права администратора".into())
                }
                Readiness::Ready => None,
                Readiness::NotImplemented(why) | Readiness::Unsupported(why) => Some(why),
            }
        };
        TrialStatus {
            active,
            remaining_secs,
            message: {
                let inner = lock(&self.inner);
                inner.message.clone()
            },
            unavailable,
            notes,
            error: {
                let inner = lock(&self.inner);
                (inner.state == DesyncState::Error).then(|| inner.message.clone())
            },
            phase,
            speed_note,
        }
    }

    /// Останавливает движок и освобождает файл номера процесса. Без запущенного обхода безвредно.
    pub fn stop(&self) {
        let _op = lock(&self.op);
        let _ = self.stop_process();
        let mut t = lock(&self.trial);
        if lock(&self.inner).process.is_none() {
            t.deadline = None;
            t.generation += 1;
        }
    }

    fn state_file(&self) -> PathBuf {
        self.dir.join(STATE_FILE)
    }

    /// Запустила ли службу драйвера сама программа (отметка в файле состояния).
    pub fn driver_marked(&self) -> bool {
        std::fs::read_to_string(self.state_file())
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|v| v.get("driver_started_by_app").and_then(|x| x.as_bool()))
            .unwrap_or(false)
    }

    fn set_driver_mark(&self, on: bool) {
        if on {
            let _ = std::fs::create_dir_all(&self.dir);
            let _ = std::fs::write(self.state_file(), "{\"driver_started_by_app\":true}\n");
        } else {
            let _ = std::fs::remove_file(self.state_file());
        }
    }

    /// Если до запуска движка служба не работала, а теперь работает — её запустила программа:
    /// запоминаем это в файле состояния. Служба, уже работавшая до нас, не помечается.
    fn note_driver_started(&self, before: &DriverState) {
        if *before != DriverState::Running && self.adapter.driver_state() == DriverState::Running {
            self.set_driver_mark(true);
        }
    }

    /// Останавливает службу драйвера, если её запустила программа. Ничего не трогает, если службу
    /// запустил кто-то другой (например, другое VPN-приложение). Вызывается под `op` после
    /// остановки движка. Успех — только если служба действительно не работает.
    fn release_driver(&self) -> Result<(), DesyncError> {
        if !self.driver_marked() {
            return Ok(());
        }
        let fail = |why: String| DesyncError::Stop(format!("{DRIVER_NOT_STOPPED} ({why})"));
        match self.adapter.driver_state() {
            DriverState::NotApplicable | DriverState::Absent | DriverState::Stopped => {
                self.set_driver_mark(false);
                return Ok(());
            }
            DriverState::Running | DriverState::Unknown(_) => {}
        }
        if let Err(why) = self.adapter.stop_driver() {
            return Err(fail(why));
        }
        match self.adapter.driver_state() {
            DriverState::Running => Err(fail("служба всё ещё работает".into())),
            DriverState::Unknown(why) => Err(fail(why)),
            _ => {
                self.set_driver_mark(false);
                Ok(())
            }
        }
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
        // Прошлый запуск мог закончиться принудительно: служба драйвера осталась работать.
        if let Err(err) = self.release_driver() {
            let mut inner = lock(&self.inner);
            inner.state = DesyncState::Error;
            inner.message = err.to_string();
            drop(inner);
            self.coord.mark_unclean(coordinator::Owner::Desync);
        }
    }

    /// Вызывается при выходе из приложения.
    /// Возвращает ошибку, если процесс не удалось остановить (ложного успеха нет).
    pub fn shutdown(&self) -> Result<(), DesyncError> {
        let _op = lock(&self.op);
        let result = self.stop_process();
        if result.is_ok() {
            let mut t = lock(&self.trial);
            t.deadline = None;
            t.generation += 1;
        }
        result
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

/// Для `VpnService::set_conflict_guard`: включение VPN выключает «Обход блокировок».
/// Останавливает работающий обход (с подтверждением завершения процесса) и возвращает `None`.
/// `Some(причина)` — обход остановить не удалось или сервис недоступен: VPN не включается.
pub fn desync_conflict_guard(desync: Weak<DesyncService>) -> ConflictGuard {
    Box::new(move || match desync.upgrade() {
        Some(d) => {
            if d.is_running() || lock(&d.inner).process.is_some() {
                d.stop_trial().err().map(|e| {
                    format!("Не удалось выключить обход блокировок перед включением VPN: {e}")
                })
            } else {
                None
            }
        }
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
        /// Если `true`, процесс игнорирует `kill()` и продолжает работать.
        ignore_kill: Arc<AtomicBool>,
        stale: Mutex<Vec<u32>>,
        /// Аргументы последнего запуска движка.
        spawn_args: Mutex<Vec<String>>,
        /// Состояние службы драйвера в фейке и число её остановок.
        driver: Mutex<Option<DriverState>>,
        driver_stops: AtomicU32,
        /// Запуск движка запускает службу драйвера (как настоящий winws2).
        spawn_starts_driver: AtomicBool,
        /// Остановка службы не удаётся.
        driver_stop_fails: AtomicBool,
    }

    struct FakeProcess(Arc<AtomicBool>, Arc<AtomicBool>, Arc<AtomicBool>, bool);
    impl CoreProcess for FakeProcess {
        fn pid(&self) -> u32 {
            777
        }
        fn has_exited(&mut self) -> bool {
            self.3 || self.0.load(Ordering::SeqCst)
        }
        fn kill(&mut self) {
            self.1.store(true, Ordering::SeqCst);
            // Как настоящий процесс: после `kill()` завершается, если не игнорирует его.
            if !self.2.load(Ordering::SeqCst) {
                self.3 = true;
            }
        }
    }

    struct Host {
        flags: Arc<Flags>,
        elevated: bool,
        readiness: Readiness,
        trial: Readiness,
        install_fails: bool,
        spawn_fails: bool,
        /// Задержка `locate` (проверка файлов), чтобы проверить параллельность с замером.
        locate_delay: Duration,
    }

    impl DesyncAdapter for Host {
        fn platform(&self) -> Platform {
            Platform::Windows
        }
        fn readiness(&self) -> Readiness {
            self.readiness.clone()
        }
        fn trial_readiness(&self) -> Readiness {
            self.trial.clone()
        }
        fn locate(&self) -> Result<PathBuf, String> {
            std::thread::sleep(self.locate_delay);
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
        fn spawn(
            &self,
            _: &Path,
            args: &[String],
            _: &Path,
        ) -> Result<Box<dyn CoreProcess>, String> {
            *self.flags.spawn_args.lock().unwrap() = args.to_vec();
            if self.spawn_fails {
                return Err("не удалось запустить".into());
            }
            self.flags.spawned.fetch_add(1, Ordering::SeqCst);
            if self.flags.spawn_starts_driver.load(Ordering::SeqCst) {
                *self.flags.driver.lock().unwrap() = Some(DriverState::Running);
            }
            Ok(Box::new(FakeProcess(
                self.flags.exited.clone(),
                self.flags.killed.clone(),
                self.flags.ignore_kill.clone(),
                false,
            )))
        }
        fn kill_stale(&self, pid: u32) {
            self.flags.stale.lock().unwrap().push(pid);
        }
        fn driver_state(&self) -> DriverState {
            self.flags
                .driver
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(DriverState::NotApplicable)
        }
        fn stop_driver(&self) -> Result<(), String> {
            self.flags.driver_stops.fetch_add(1, Ordering::SeqCst);
            if self.flags.driver_stop_fails.load(Ordering::SeqCst) {
                return Err("отказано в доступе".into());
            }
            *self.flags.driver.lock().unwrap() = Some(DriverState::Stopped);
            Ok(())
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
            trial: Readiness::Ready,
            install_fails: false,
            spawn_fails: false,
            locate_delay: Duration::ZERO,
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
        // Включение VPN останавливает работающий обход и освобождает право запуска.
        assert!(guard().is_none());
        assert!(!s.is_running());
        assert!(s.coord.begin(coordinator::Owner::Vpn).is_ok());
        s.coord.release(coordinator::Owner::Vpn);
        s.shutdown().unwrap();
        assert!(guard().is_none());
        // Нет циклов владения: после удаления сервиса защита отказывает.
        drop(s);
        assert!(guard().is_some());
    }

    #[test]
    fn desync_is_refused_while_vpn_owns_the_mode_and_reservation_is_released_on_errors() {
        use coordinator::{Mode, NetCoordinator, Owner};
        let flags = Arc::new(Flags::default());
        let coord = Arc::new(NetCoordinator::new());
        let s = make_confirmed(&flags, false).with_coordinator(coord.clone());
        let want = [Coverage::Sites];
        // VPN уже запускается: обход отклонён без запуска движка.
        coord.begin(Owner::Vpn).unwrap();
        let err = s
            .start_planned(&want, &blocked(&want), &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, DesyncError::NotAllowed(_)), "{err:?}");
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        assert_eq!(coord.mode(), Some(Mode::Starting(Owner::Vpn)));
        s.stop(); // чужое право не сбрасывается
        assert_eq!(coord.mode(), Some(Mode::Starting(Owner::Vpn)));
        // VPN освободил право: запуск проходит, и теперь VPN отклоняется.
        coord.release(Owner::Vpn);
        s.start_planned(&want, &blocked(&want), &mut |_| {})
            .unwrap();
        assert_eq!(coord.mode(), Some(Mode::Active(Owner::Desync)));
        assert!(coord.begin(Owner::Vpn).is_err());
        // Повторный запуск не создаёт второй процесс; остановка освобождает право.
        s.start_planned(&want, &blocked(&want), &mut |_| {})
            .unwrap();
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 1);
        s.shutdown().unwrap();
        s.shutdown().unwrap();
        assert_eq!(coord.mode(), Some(Mode::Idle));
        coord.begin(Owner::Vpn).unwrap();
    }

    #[test]
    fn failed_desync_start_releases_the_reservation() {
        use coordinator::{Mode, NetCoordinator};
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let coord = Arc::new(NetCoordinator::new());
        let mut s = make(
            &flags,
            |f| Host {
                spawn_fails: true,
                ..good(f)
            },
            true,
            false,
        )
        .with_coordinator(coord.clone());
        s.candidate = profile();
        s.confirmed = &TEST_CONFIRMED;
        let want = [Coverage::Sites];
        assert!(s
            .start_planned(&want, &blocked(&want), &mut |_| {})
            .is_err());
        assert_eq!(coord.mode(), Some(Mode::Idle));
    }

    #[test]
    fn simultaneous_vpn_and_desync_starts_give_the_right_to_at_most_one() {
        use coordinator::{NetCoordinator, Owner};
        use std::sync::Barrier;
        let flags = Arc::new(Flags::default());
        let coord = Arc::new(NetCoordinator::new());
        let s = Arc::new(make_confirmed(&flags, false).with_coordinator(coord.clone()));
        let gate = Arc::new(Barrier::new(2));
        let (s2, g2) = (s.clone(), gate.clone());
        let desync = std::thread::spawn(move || {
            g2.wait();
            let want = [Coverage::Sites];
            s2.start_planned(&want, &blocked(&want), &mut |_| {})
                .is_ok()
        });
        let (c2, g3) = (coord.clone(), gate.clone());
        let vpn = std::thread::spawn(move || {
            g3.wait();
            c2.begin(Owner::Vpn).is_ok()
        });
        let (d, v) = (desync.join().unwrap(), vpn.join().unwrap());
        assert!(d ^ v, "desync={d} vpn={v}");
        assert!(flags.spawned.load(Ordering::SeqCst) <= 1);
        s.shutdown().unwrap();
    }

    // ----- пробный запуск полного профиля -----

    fn trial_service(flags: &Arc<Flags>, vpn: bool) -> Arc<DesyncService> {
        flags.present.store(true, Ordering::SeqCst);
        // Обычная готовность адаптера закрыта, пробная — открыта.
        let mut s = make(
            flags,
            |f| Host {
                readiness: Readiness::NotImplemented("профиль не проверен".into()),
                ..good(f)
            },
            true,
            vpn,
        );
        s.candidate = profile();
        s.stop_wait = (2, Duration::from_millis(1));
        Arc::new(s)
    }

    #[test]
    fn trial_needs_explicit_consent_and_leaves_normal_state_closed() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        let err = s.start_trial(false, &mut |_| {}).unwrap_err();
        assert!(matches!(err, DesyncError::NotAllowed(_)), "{err:?}");
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        // Обычная готовность и реестр подтверждений не изменились.
        assert!(matches!(
            s.adapter.readiness(),
            Readiness::NotImplemented(_)
        ));
        assert!(routing::CONFIRMED_EXTERNAL_PROFILES.is_empty());
        let plan = s.plan_access(&[Coverage::Sites], &blocked(&[Coverage::Sites]));
        assert!(matches!(plan.overall, RoutePlan::Block(_)));
        s.start_trial(true, &mut |_| {}).unwrap();
        assert!(matches!(
            s.adapter.readiness(),
            Readiness::NotImplemented(_)
        ));
        assert!(matches!(
            s.plan_access(&[Coverage::Sites], &blocked(&[Coverage::Sites]))
                .overall,
            RoutePlan::Block(_)
        ));
        s.stop_trial().unwrap();
    }

    #[test]
    fn trial_is_refused_with_vpn_unprepared_engine_or_no_privileges() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, true);
        assert!(s.start_trial(true, &mut |_| {}).is_err());
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        // Координатор: VPN уже владеет режимом (или не очистил настройки) — отказ.
        let s = trial_service(&flags, false);
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
        s.coord.mark_unclean(coordinator::Owner::Vpn);
        assert!(s.start_trial(true, &mut |_| {}).is_err());
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        // Движок не подготовлен и скачать нельзя.
        let flags2 = Arc::new(Flags::default());
        let mut raw = make(&flags2, good, false, false);
        raw.candidate = profile();
        let raw = Arc::new(raw);
        assert!(matches!(
            raw.start_trial(true, &mut |_| {}).unwrap_err(),
            DesyncError::NotPrepared(_)
        ));
        assert_eq!(flags2.spawned.load(Ordering::SeqCst), 0);
        // Нет готовности пробного режима.
        let mut notready = make(
            &flags2,
            |f| Host {
                trial: Readiness::NotImplemented("нет".into()),
                ..good(f)
            },
            true,
            false,
        );
        notready.candidate = profile();
        assert!(matches!(
            Arc::new(notready)
                .start_trial(true, &mut |_| {})
                .unwrap_err(),
            DesyncError::Unavailable(_)
        ));
        // Нет прав администратора.
        let flags3 = Arc::new(Flags::default());
        flags3.present.store(true, Ordering::SeqCst);
        let mut noadmin = make(
            &flags3,
            |f| Host {
                elevated: false,
                ..good(f)
            },
            true,
            false,
        );
        noadmin.candidate = profile();
        assert_eq!(
            Arc::new(noadmin)
                .start_trial(true, &mut |_| {})
                .unwrap_err(),
            DesyncError::NeedsAdmin
        );
        assert_eq!(flags3.spawned.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn trial_runs_one_instance_and_stops_manually_and_twice_safely() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        assert!(s.is_running());
        let st = s.trial_status(std::time::Instant::now());
        assert!(st.active && st.remaining_secs > 0 && st.remaining_secs <= 120);
        // Повторный вызов второй экземпляр не создаёт.
        assert!(s.start_trial(true, &mut |_| {}).is_err());
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 1);
        // VPN не получает право, пока идёт испытание.
        assert!(s.coord.begin(coordinator::Owner::Vpn).is_err());
        s.stop_trial().unwrap();
        s.stop_trial().unwrap();
        assert!(!s.is_running() && flags.killed.load(Ordering::SeqCst));
        assert!(!s.trial_status(std::time::Instant::now()).active);
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn trial_stops_by_itself_after_the_time_limit() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        let soon = std::time::Instant::now() + Duration::from_secs(60);
        assert!(!s.tick(soon));
        assert!(s.is_running());
        let late = std::time::Instant::now() + Duration::from_secs(121);
        assert!(s.tick(late));
        assert!(!s.is_running() && flags.killed.load(Ordering::SeqCst));
        assert!(s.trial_status(late).message.contains("истекло время"));
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn real_rust_timer_stops_a_short_trial_without_the_ui() {
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let mut s = make(&flags, good, true, false);
        s.candidate = profile();
        s.trial_limit = Duration::from_millis(20);
        let s = Arc::new(s);
        s.start_trial(true, &mut |_| {}).unwrap();
        for _ in 0..100 {
            if !s.is_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(!s.is_running(), "таймер Rust должен остановить испытание");
    }

    #[test]
    fn failed_stop_is_not_reported_as_success_and_keeps_the_right() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        flags.ignore_kill.store(true, Ordering::SeqCst); // процесс не умирает
        let err = s.stop_trial().unwrap_err();
        assert!(matches!(err, DesyncError::Stop(_)), "{err:?}");
        assert_eq!(s.status().0, DesyncState::Error);
        assert!(s.coord.begin(coordinator::Owner::Vpn).is_err());
        // Повтор после того, как процесс всё-таки завершился.
        flags.exited.store(true, Ordering::SeqCst);
        s.stop_trial().unwrap();
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn failed_trial_start_cleans_up_and_shutdown_stops_the_trial() {
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
        let s = Arc::new(s);
        assert!(matches!(
            s.start_trial(true, &mut |_| {}).unwrap_err(),
            DesyncError::Start(_)
        ));
        assert!(!s.is_running());
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
        s.coord.release(coordinator::Owner::Vpn);
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        s.shutdown().unwrap();
        assert!(!s.is_running() && flags.killed.load(Ordering::SeqCst));
        assert!(!s.trial_status(std::time::Instant::now()).active);
    }

    #[test]
    fn process_that_ignores_kill_keeps_the_right_and_blocks_any_new_start() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        flags.ignore_kill.store(true, Ordering::SeqCst);
        assert!(matches!(s.stop_trial().unwrap_err(), DesyncError::Stop(_)));
        // Не «Выключено», право не освобождено, испытание всё ещё считается идущим.
        assert_eq!(s.status().0, DesyncState::Error);
        assert_eq!(
            s.coord.mode(),
            Some(coordinator::Mode::Unclean(coordinator::Owner::Desync))
        );
        let st = s.trial_status(std::time::Instant::now());
        assert!(st.active && !st.message.contains("Выключено"));
        // Ни пробный, ни обычный запуск, ни VPN не получают разрешения; второго процесса нет.
        assert!(s.start_trial(true, &mut |_| {}).is_err());
        let want = [Coverage::Sites];
        assert!(s
            .start_planned(&want, &blocked(&want), &mut |_| {})
            .is_err());
        assert!(s.coord.begin(coordinator::Owner::Vpn).is_err());
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 1);
        // Обычная готовность и реестр по-прежнему закрыты.
        assert!(matches!(
            s.adapter.readiness(),
            Readiness::NotImplemented(_)
        ));
        assert!(routing::CONFIRMED_EXTERNAL_PROFILES.is_empty());
        // Процесс наконец завершился: подтверждённая остановка освобождает право.
        flags.ignore_kill.store(false, Ordering::SeqCst);
        s.stop_trial().unwrap();
        assert_eq!(s.status().0, DesyncState::Off);
        assert!(!s.trial_status(std::time::Instant::now()).active);
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn timer_keeps_watching_after_a_failed_stop_with_bounded_retries() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        flags.ignore_kill.store(true, Ordering::SeqCst);
        let late = std::time::Instant::now() + Duration::from_secs(121);
        // Неудачи не прекращают контроль, пока не исчерпан лимит повторов.
        for attempt in 1..MAX_FAILED_STOPS {
            assert!(!s.tick(late), "попытка {attempt}");
            assert!(
                s.trial_status(late).active,
                "процесс не должен остаться без наблюдения"
            );
        }
        // Лимит исчерпан: поток прекращает повторы, но процесс учтён, запуск заблокирован.
        assert!(s.tick(late));
        assert_eq!(s.status().0, DesyncState::Error);
        assert!(s.trial_status(late).active);
        assert!(s.start_trial(true, &mut |_| {}).is_err());
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 1);
        // Ручная остановка после исправления причины работает.
        flags.ignore_kill.store(false, Ordering::SeqCst);
        s.stop_trial().unwrap();
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn timer_retry_succeeds_when_the_process_finally_exits() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        flags.ignore_kill.store(true, Ordering::SeqCst);
        let late = std::time::Instant::now() + Duration::from_secs(121);
        assert!(!s.tick(late));
        flags.exited.store(true, Ordering::SeqCst);
        assert!(s.tick(late));
        assert_eq!(s.status().0, DesyncState::Off);
        assert!(!s.trial_status(late).active);
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn shutdown_and_general_stop_do_not_claim_success_for_a_live_process() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        flags.ignore_kill.store(true, Ordering::SeqCst);
        assert!(matches!(s.shutdown().unwrap_err(), DesyncError::Stop(_)));
        assert_eq!(s.status().0, DesyncState::Error);
        s.stop(); // общий путь тоже ничего не освобождает
        assert_eq!(s.status().0, DesyncState::Error);
        assert!(s.coord.begin(coordinator::Owner::Vpn).is_err());
        assert!(
            s.pid_file().exists(),
            "файл номера процесса не удалён, пока он жив"
        );
        flags.ignore_kill.store(false, Ordering::SeqCst);
        s.shutdown().unwrap();
        assert_eq!(s.status().0, DesyncState::Off);
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn restart_after_confirmed_stop_never_runs_two_processes_and_foreign_right_stays() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        assert!(s.start_trial(true, &mut |_| {}).is_err());
        s.stop_trial().unwrap();
        s.start_trial(true, &mut |_| {}).unwrap();
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 2);
        s.stop_trial().unwrap();
        // Чужое право (VPN) остановкой обхода не освобождается.
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
        s.stop_trial().unwrap();
        s.stop();
        assert_eq!(
            s.coord.mode(),
            Some(coordinator::Mode::Starting(coordinator::Owner::Vpn))
        );
    }

    #[test]
    fn user_notes_are_observations_not_proof() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        assert!(s
            .note_trial(Coverage::Youtube, TrialVerdict::Works)
            .is_err());
        s.start_trial(true, &mut |_| {}).unwrap();
        s.note_trial(Coverage::Youtube, TrialVerdict::Works)
            .unwrap();
        s.note_trial(Coverage::Youtube, TrialVerdict::Fails)
            .unwrap();
        s.note_trial(Coverage::Discord, TrialVerdict::Unchecked)
            .unwrap();
        let st = s.trial_status(std::time::Instant::now());
        assert_eq!(
            st.notes,
            vec![
                (Coverage::Youtube, TrialVerdict::Fails),
                (Coverage::Discord, TrialVerdict::Unchecked)
            ]
        );
        // Пометки не меняют маршрут и не разрешают обычный запуск.
        assert!(matches!(
            s.plan_access(&[Coverage::Youtube], &ServiceObservations::new())
                .overall,
            RoutePlan::Block(_)
        ));
        s.stop_trial().unwrap();
    }

    #[test]
    fn merged_profile_is_base_plus_sections_without_duplicates_and_resources_exist() {
        let merged = windows_merged_candidate_profile().unwrap();
        assert_eq!(merged.id, "windows-ochistka-merged-unverified");
        let m = preset::ZapretPreset::parse(include_str!(
            "../../../assets/desync/presets/OCHISTKA Merged.txt"
        ))
        .unwrap();
        let base = preset::ZapretPreset::parse(include_str!(
            "../../../assets/desync/presets/Universal V13 Lite.txt"
        ))
        .unwrap();
        // Основа первой и без изменений, кроме секций удалённых по решению владельца
        // (Telegram, Steam, Riot / Valorant); общие аргументы основы на месте.
        let removed = |text: &str| {
            let t = text.to_lowercase();
            ["telegram", "steam", "riot", "valorant"]
                .iter()
                .any(|w| t.contains(w))
        };
        let kept: Vec<_> = base
            .sections
            .iter()
            .filter(|sec| !removed(&sec.arguments.join(" ")))
            .cloned()
            .collect();
        assert_eq!(
            base.sections.len() - kept.len(),
            3,
            "удалены ровно секции Telegram, Steam, Riot"
        );
        assert_eq!(m.sections[..kept.len()], kept[..]);
        for g in &base.global_arguments {
            assert!(removed(g) || m.global_arguments.contains(g), "{g}");
        }
        assert!(m.sections.len() > kept.len());
        let everything = format!(
            "{}\n{}",
            m.global_arguments.join("\n"),
            m.sections
                .iter()
                .map(|sec| sec.arguments.join(" "))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(
            !removed(&everything),
            "в объединённом пресете не должно быть Telegram, Steam, Riot"
        );
        // Нет повторов: ни секций (имя не учитывается), ни общих аргументов.
        let mut seen = std::collections::BTreeSet::new();
        for s in &m.sections {
            let body: Vec<&String> = s
                .arguments
                .iter()
                .filter(|a| !a.starts_with("--name="))
                .collect();
            assert!(seen.insert(body.clone()), "повтор секции: {body:?}");
        }
        let mut globals = std::collections::BTreeSet::new();
        for g in &m.global_arguments {
            assert!(globals.insert(g), "повтор аргумента: {g}");
        }
        // Все ресурсы есть в дереве закреплённых 135 файлов или в дополнительных данных.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/desync");
        for rel in m.required_resources().unwrap() {
            assert!(
                root.join("runtime").join(&rel).is_file()
                    || root.join("runtime-extra").join(&rel).is_file(),
                "{rel}"
            );
        }
    }

    #[test]
    fn trial_runs_the_merged_profile_not_the_old_single_preset() {
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        let args = flags.spawn_args.lock().unwrap().clone();
        let merged = windows_merged_candidate_profile().unwrap();
        // В тесте кандидат подставлен тестовым профилем, но настоящий конструктор берёт объединённый.
        assert!(!args.is_empty());
        let real = DesyncService::new(
            temp_dir(),
            Box::new(good(flags.clone())),
            Box::new(routing::VpnSignal::default),
        );
        assert_eq!(
            real.candidate.as_ref().map(|p| p.args.clone()),
            Some(merged.args)
        );
        s.stop_trial().unwrap();
    }

    #[test]
    fn merged_profile_does_not_unlock_normal_activation() {
        assert!(routing::CONFIRMED_EXTERNAL_PROFILES.is_empty());
        assert!(site_discord_profile(Platform::Windows).is_none());
        let flags = Arc::new(Flags::default());
        let s = trial_service(&flags, false);
        let plan = s.plan_access(&[Coverage::Sites], &blocked(&[Coverage::Sites]));
        assert!(matches!(plan.overall, RoutePlan::Block(_)));
    }

    // ----- автоотключение VPN при включении обхода и обратно -----

    fn trial_with_switchable_vpn(
        flags: &Arc<Flags>,
        vpn_active: &Arc<std::sync::atomic::AtomicBool>,
        elevated: bool,
    ) -> Arc<DesyncService> {
        flags.present.store(true, Ordering::SeqCst);
        let active = vpn_active.clone();
        let mut s = DesyncService::new(
            temp_dir(),
            Box::new(Host {
                readiness: Readiness::NotImplemented("профиль не проверен".into()),
                elevated,
                ..good(flags.clone())
            }),
            Box::new(move || routing::VpnSignal {
                active: active.load(Ordering::SeqCst),
                verified: false,
            }),
        );
        s.candidate = profile();
        s.installable = true;
        s.settle = (2, Duration::from_millis(1));
        s.stop_wait = (2, Duration::from_millis(1));
        Arc::new(s)
    }

    #[test]
    fn enabling_bypass_turns_a_running_vpn_off_first() {
        let flags = Arc::new(Flags::default());
        let vpn = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let s = trial_with_switchable_vpn(&flags, &vpn, true);
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let (v, c) = (vpn.clone(), calls.clone());
        s.set_vpn_stopper(Box::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
            v.store(false, Ordering::SeqCst);
            Ok(())
        }));
        s.start_trial(true, &mut |_| {}).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!vpn.load(Ordering::SeqCst));
        assert!(s.is_running());
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 1);
        s.stop_trial().unwrap();
    }

    #[test]
    fn no_vpn_running_means_the_stopper_is_not_called() {
        let flags = Arc::new(Flags::default());
        let vpn = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let s = trial_with_switchable_vpn(&flags, &vpn, true);
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = calls.clone();
        s.set_vpn_stopper(Box::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        s.start_trial(true, &mut |_| {}).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        s.stop_trial().unwrap();
    }

    #[test]
    fn failed_or_ineffective_vpn_stop_refuses_and_starts_nothing() {
        let flags = Arc::new(Flags::default());
        let vpn = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let s = trial_with_switchable_vpn(&flags, &vpn, true);
        s.set_vpn_stopper(Box::new(|| Err("не удалось отключить VPN".into())));
        let err = s.start_trial(true, &mut |_| {}).unwrap_err();
        assert!(matches!(err, DesyncError::NotAllowed(_)), "{err:?}");
        // «Успех» стоппера, но VPN всё ещё работает — тоже отказ.
        s.set_vpn_stopper(Box::new(|| Ok(())));
        assert_eq!(
            s.start_trial(true, &mut |_| {}).unwrap_err(),
            DesyncError::VpnRunning
        );
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
        assert!(vpn.load(Ordering::SeqCst));
    }

    #[test]
    fn vpn_is_not_touched_when_the_bypass_cannot_start_anyway() {
        let flags = Arc::new(Flags::default());
        let vpn = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let s = trial_with_switchable_vpn(&flags, &vpn, false); // нет прав администратора
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = calls.clone();
        s.set_vpn_stopper(Box::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        assert_eq!(
            s.start_trial(true, &mut |_| {}).unwrap_err(),
            DesyncError::NeedsAdmin
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(vpn.load(Ordering::SeqCst));
    }

    #[test]
    fn enabling_vpn_stops_a_running_bypass_through_the_guard() {
        let flags = Arc::new(Flags::default());
        let vpn = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let s = trial_with_switchable_vpn(&flags, &vpn, true);
        s.start_trial(true, &mut |_| {}).unwrap();
        let guard = desync_conflict_guard(Arc::downgrade(&s));
        assert!(guard().is_none());
        assert!(!s.is_running());
        assert!(flags.killed.load(Ordering::SeqCst));
        assert!(s.coord.begin(coordinator::Owner::Vpn).is_ok());
        // Если остановить не удалось, VPN получает отказ с причиной.
        s.coord.release(coordinator::Owner::Vpn);
        s.start_trial(true, &mut |_| {}).unwrap();
        flags.ignore_kill.store(true, Ordering::SeqCst);
        assert!(guard().is_some());
        flags.ignore_kill.store(false, Ordering::SeqCst);
        flags.exited.store(true, Ordering::SeqCst);
        s.stop_trial().unwrap();
    }

    #[test]
    fn merged_profile_command_line_fits_the_windows_limit_with_margin() {
        let merged = windows_merged_candidate_profile().unwrap();
        let exe = r"C:\Users\Someone\AppData\Local\OCHISTKA\desync\winws2.exe";
        let len = windows_command_line_len(exe, &merged.args);
        // CreateProcess принимает не больше 32 767 символов; нужен запас (не больше 30 000).
        assert!(
            len <= 30_000,
            "командная строка {len} символов, предел Windows 32 767"
        );
        assert!(check_command_line(exe, &merged.args).is_ok());
    }

    #[test]
    fn too_long_command_line_is_refused_with_a_clear_reason_not_os_error_206() {
        let exe = r"C:\x\winws2.exe";
        let long: Vec<String> = (0..2000)
            .map(|i| format!("--lua-desync=fake:blob={i:030}"))
            .collect();
        let err = check_command_line(exe, &long).unwrap_err();
        assert!(
            err.contains("слишком длинн") && err.contains("32767"),
            "{err}"
        );
    }

    #[test]
    fn windows_command_line_length_counts_quotes_for_arguments_with_spaces() {
        let exe = r"C:\a\w.exe";
        let plain = windows_command_line_len(exe, &["--a=1".to_string()]);
        let spaced = windows_command_line_len(exe, &["--name=A B".to_string()]);
        assert_eq!(plain, exe.len() + 2 + 1 + 5);
        assert_eq!(spaced, exe.len() + 2 + 1 + "--name=A B".len() + 2);
    }

    #[test]
    fn launch_arguments_differ_from_the_full_preset_only_by_profile_names() {
        let m = preset::ZapretPreset::parse(include_str!(
            "../../../assets/desync/presets/OCHISTKA Merged.txt"
        ))
        .unwrap();
        let full = m.command_arguments();
        let launch = m.launch_arguments();
        let without_names: Vec<String> = full
            .iter()
            .filter(|a| !a.starts_with("--name="))
            .cloned()
            .collect();
        assert_eq!(launch, without_names);
        // Ни одно правило не потеряно: все не-именные аргументы на месте, порядок прежний.
        assert!(launch.len() < full.len());
        assert_eq!(
            launch.iter().filter(|a| a.as_str() == "--new").count(),
            m.sections.len()
        );
        assert_eq!(windows_merged_candidate_profile().unwrap().args, launch);
    }

    // ----- служба драйвера WinDivert -----

    fn driver_flags(initial: DriverState) -> Arc<Flags> {
        let flags = Arc::new(Flags::default());
        *flags.driver.lock().unwrap() = Some(initial);
        flags.spawn_starts_driver.store(true, Ordering::SeqCst);
        flags
    }

    fn driver_now(flags: &Flags) -> DriverState {
        flags.driver.lock().unwrap().clone().unwrap()
    }

    #[test]
    fn driver_started_by_the_app_is_stopped_with_the_bypass_and_the_mark_is_cleared() {
        let flags = driver_flags(DriverState::Stopped);
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        assert_eq!(driver_now(&flags), DriverState::Running);
        assert!(s.driver_marked(), "запуск службы записан в файл состояния");
        s.stop_trial().unwrap();
        assert_eq!(driver_now(&flags), DriverState::Stopped);
        assert_eq!(flags.driver_stops.load(Ordering::SeqCst), 1);
        assert!(!s.driver_marked());
        assert_eq!(s.status().0, DesyncState::Off);
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn a_driver_already_running_before_us_is_never_stopped() {
        let flags = driver_flags(DriverState::Running);
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        assert!(!s.driver_marked());
        s.stop_trial().unwrap();
        assert_eq!(flags.driver_stops.load(Ordering::SeqCst), 0);
        assert_eq!(driver_now(&flags), DriverState::Running);
    }

    #[test]
    fn failed_driver_stop_keeps_the_bypass_not_off_with_a_short_reason() {
        let flags = driver_flags(DriverState::Stopped);
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        flags.driver_stop_fails.store(true, Ordering::SeqCst);
        let err = s.stop_trial().unwrap_err();
        assert!(matches!(err, DesyncError::Stop(_)), "{err:?}");
        assert_eq!(s.status().0, DesyncState::Error);
        let st = s.trial_status(std::time::Instant::now());
        assert_eq!(st.error.as_deref(), Some(DRIVER_NOT_STOPPED));
        assert!(st.active && !st.message.contains("Выключено"));
        assert!(s.driver_marked(), "отметка остаётся, пока служба работает");
        assert_eq!(
            s.coord.mode(),
            Some(coordinator::Mode::Unclean(coordinator::Owner::Desync))
        );
        // Ни новый запуск, ни VPN не получают разрешения.
        assert!(s.coord.begin(coordinator::Owner::Vpn).is_err());
        // Повтор после устранения причины доводит остановку до конца.
        flags.driver_stop_fails.store(false, Ordering::SeqCst);
        s.stop_trial().unwrap();
        assert!(!s.driver_marked());
        assert_eq!(s.status().0, DesyncState::Off);
        assert!(s.trial_status(std::time::Instant::now()).error.is_none());
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn a_service_that_stays_running_after_stop_is_a_failure_too() {
        let flags = driver_flags(DriverState::Stopped);
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        // stop_driver «удаётся», но служба по-прежнему RUNNING.
        *flags.driver.lock().unwrap() = Some(DriverState::Running);
        flags.driver_stop_fails.store(true, Ordering::SeqCst);
        assert!(s.stop_trial().is_err());
        assert!(s.driver_marked());
    }

    #[test]
    fn timer_and_exit_paths_also_stop_the_driver_and_exit_reports_failure() {
        let flags = driver_flags(DriverState::Stopped);
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        let late = std::time::Instant::now() + Duration::from_secs(121);
        assert!(s.tick(late));
        assert_eq!(driver_now(&flags), DriverState::Stopped);
        assert!(!s.driver_marked());
        // Выход из программы: остановка службы удалась / не удалась.
        s.start_trial(true, &mut |_| {}).unwrap();
        flags.driver_stop_fails.store(true, Ordering::SeqCst);
        assert!(matches!(s.shutdown().unwrap_err(), DesyncError::Stop(_)));
        assert!(s.driver_marked());
        flags.driver_stop_fails.store(false, Ordering::SeqCst);
        s.shutdown().unwrap();
        assert_eq!(driver_now(&flags), DriverState::Stopped);
    }

    #[test]
    fn startup_recovery_stops_a_driver_left_running_by_a_forced_exit() {
        let flags = driver_flags(DriverState::Running);
        let s = trial_service(&flags, false);
        s.set_driver_mark(true); // прошлый запуск программы пометил службу и был закрыт принудительно
        s.recover_on_start();
        assert_eq!(driver_now(&flags), DriverState::Stopped);
        assert!(!s.driver_marked());
        assert_eq!(s.status().0, DesyncState::Off);
        // Нет отметки (службу запустил кто-то другой) — служба не трогается.
        let flags = driver_flags(DriverState::Running);
        let s = trial_service(&flags, false);
        s.recover_on_start();
        assert_eq!(driver_now(&flags), DriverState::Running);
        assert_eq!(flags.driver_stops.load(Ordering::SeqCst), 0);
        // Остановить не удалось: ошибка в состоянии, запуск заблокирован, отметка остаётся.
        let flags = driver_flags(DriverState::Running);
        let s = trial_service(&flags, false);
        s.set_driver_mark(true);
        flags.driver_stop_fails.store(true, Ordering::SeqCst);
        s.recover_on_start();
        assert_eq!(s.status().0, DesyncState::Error);
        assert!(s.driver_marked());
        assert!(s.start_trial(true, &mut |_| {}).is_err());
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn driver_marker_is_a_small_json_file_in_the_app_folder() {
        let flags = driver_flags(DriverState::Stopped);
        let s = trial_service(&flags, false);
        s.start_trial(true, &mut |_| {}).unwrap();
        let text = std::fs::read_to_string(s.dir.join("desync-state.json")).unwrap();
        assert!(text.contains("driver_started_by_app"));
        s.stop_trial().unwrap();
        assert!(!s.dir.join("desync-state.json").exists());
    }

    // ----- замер скорости и откат -----

    use speed::testing::Fake as SpeedFake;
    use speed::{Timing, URLS};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Ответы: на каждый адрес сначала базовый замер, потом замер с обходом.
    fn speed_fake(base: Timing, after: Timing) -> Arc<SpeedFake> {
        let items: Vec<(&str, Vec<Timing>)> =
            URLS.iter().map(|u| (*u, vec![base, after])).collect();
        Arc::new(SpeedFake::new(&items))
    }

    fn speed_service(flags: &Arc<Flags>, fake: Arc<SpeedFake>) -> Arc<DesyncService> {
        flags.present.store(true, Ordering::SeqCst);
        let mut s = make(
            flags,
            |f| Host {
                readiness: Readiness::NotImplemented("профиль не проверен".into()),
                ..good(f)
            },
            true,
            false,
        );
        s.candidate = profile();
        s.stop_wait = (2, Duration::from_millis(1));
        s.speed_async = false;
        s.set_speed_probe(fake);
        Arc::new(s)
    }

    fn finish_speed_check(s: &Arc<DesyncService>) {
        let (generation, baseline) = {
            let t = lock(&s.trial);
            (
                t.generation,
                t.pending_baseline
                    .clone()
                    .expect("замер после запуска не запланирован"),
            )
        };
        s.run_speed_check(generation, &baseline);
    }

    #[test]
    fn slower_than_the_baseline_turns_the_bypass_off_with_the_reason() {
        let flags = driver_flags(DriverState::Stopped);
        let s = speed_service(
            &flags,
            speed_fake(
                Timing::done(ms(300), ms(900)),
                Timing::done(ms(300), ms(3000)),
            ),
        );
        s.start_trial(true, &mut |_| {}).unwrap();
        assert!(s.is_running());
        finish_speed_check(&s);
        assert!(!s.is_running(), "обход выключен");
        assert_eq!(
            driver_now(&flags),
            DriverState::Stopped,
            "откат прошёл через общий путь остановки"
        );
        let st = s.trial_status(std::time::Instant::now());
        assert_eq!(st.speed_note.as_deref(), Some(speed::SLOWER_MESSAGE));
        assert!(!st.active && st.error.is_none());
        s.coord.begin(coordinator::Owner::Vpn).unwrap();
    }

    #[test]
    fn same_speed_keeps_the_bypass_until_the_end_and_says_it_proves_nothing() {
        let flags = Arc::new(Flags::default());
        let s = speed_service(
            &flags,
            speed_fake(
                Timing::done(ms(300), ms(900)),
                Timing::done(ms(280), ms(850)),
            ),
        );
        s.start_trial(true, &mut |_| {}).unwrap();
        finish_speed_check(&s);
        assert!(s.is_running());
        let st = s.trial_status(std::time::Instant::now());
        assert!(st.active);
        assert_eq!(st.speed_note.as_deref(), Some(speed::NOT_SLOWER_MESSAGE));
        assert!(st.speed_note.unwrap().contains("не доказывает"));
        // «Функционально подтверждено» этим замером не становится.
        assert!(routing::CONFIRMED_EXTERNAL_PROFILES.is_empty());
        s.stop_trial().unwrap();
    }

    #[test]
    fn a_timeout_with_the_bypass_rolls_back() {
        let flags = Arc::new(Flags::default());
        let s = speed_service(
            &flags,
            speed_fake(
                Timing::done(ms(300), ms(900)),
                Timing::timeout(speed::BASELINE.per_request),
            ),
        );
        s.start_trial(true, &mut |_| {}).unwrap();
        finish_speed_check(&s);
        assert!(!s.is_running());
        assert_eq!(
            s.trial_status(std::time::Instant::now())
                .speed_note
                .as_deref(),
            Some(speed::SLOWER_MESSAGE)
        );
    }

    #[test]
    fn nothing_opening_without_the_bypass_never_blocks_or_rolls_back_the_bypass() {
        // Сайты закрыты без обхода (это и есть цель) и открываются с обходом.
        let flags = Arc::new(Flags::default());
        let s = speed_service(
            &flags,
            speed_fake(Timing::failed(ms(5)), Timing::done(ms(300), ms(900))),
        );
        s.start_trial(true, &mut |_| {}).unwrap();
        assert_eq!(flags.spawned.load(Ordering::SeqCst), 1, "обход включился");
        finish_speed_check(&s);
        assert!(s.is_running(), "откат не выполняется");
        assert_eq!(
            s.trial_status(std::time::Instant::now())
                .speed_note
                .as_deref(),
            Some(speed::NO_COMPARE_MESSAGE)
        );
        s.stop_trial().unwrap();
    }

    #[test]
    fn no_network_at_all_says_so_and_still_does_not_roll_back() {
        let flags = Arc::new(Flags::default());
        let s = speed_service(
            &flags,
            speed_fake(Timing::failed(ms(5)), Timing::failed(ms(5))),
        );
        s.start_trial(true, &mut |_| {}).unwrap();
        finish_speed_check(&s);
        assert!(s.is_running());
        assert_eq!(
            s.trial_status(std::time::Instant::now())
                .speed_note
                .as_deref(),
            Some(speed::NO_INTERNET_MESSAGE)
        );
        assert_eq!(speed::NO_INTERNET_MESSAGE, "Нет подключения к интернету");
        s.stop_trial().unwrap();
    }

    #[test]
    fn the_button_is_on_right_after_the_engine_starts_without_waiting_for_the_check() {
        // Замер с обходом (фоновый) «висит» на замке: start_trial всё равно возвращается сразу.
        struct Gated {
            calls: AtomicU32,
            gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
        }
        impl speed::TimedFetch for Gated {
            fn fetch(&self, _: &str, _: Duration) -> Timing {
                // Первые три вызова — замер до включения: сразу. Остальные ждут разрешения.
                if self.calls.fetch_add(1, Ordering::SeqCst) >= 3 {
                    let (lock_, cv) = &*self.gate;
                    let mut open = lock_.lock().unwrap();
                    let started = std::time::Instant::now();
                    while !*open && started.elapsed() < Duration::from_secs(5) {
                        open = cv.wait_timeout(open, Duration::from_millis(50)).unwrap().0;
                    }
                }
                Timing::done(ms(100), ms(300))
            }
        }
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let mut svc = make(
            &flags,
            |f| Host {
                readiness: Readiness::NotImplemented("профиль не проверен".into()),
                ..good(f)
            },
            true,
            false,
        );
        svc.candidate = profile();
        svc.stop_wait = (2, Duration::from_millis(1));
        svc.set_speed_probe(Arc::new(Gated {
            calls: AtomicU32::new(0),
            gate: gate.clone(),
        }));
        let s = Arc::new(svc);
        let started = std::time::Instant::now();
        s.start_trial(true, &mut |_| {}).unwrap();
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "кнопка не ждёт замер с обходом"
        );
        assert!(s.is_running());
        // Пока замер идёт, интерфейс видит «Проверяем…», обход уже включён.
        let mut phase = TrialPhase::Idle;
        for _ in 0..100 {
            phase = s.trial_status(std::time::Instant::now()).phase;
            if phase == TrialPhase::MeasuringAfter {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(phase, TrialPhase::MeasuringAfter);
        assert!(s.trial_status(std::time::Instant::now()).active);
        // Замер закончился: результат появляется отдельной строкой, обход остаётся.
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        for _ in 0..200 {
            if s.trial_status(std::time::Instant::now())
                .speed_note
                .is_some()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(
            s.trial_status(std::time::Instant::now())
                .speed_note
                .as_deref(),
            Some(speed::NOT_SLOWER_MESSAGE)
        );
        s.stop_trial().unwrap();
    }

    #[test]
    fn a_check_that_finishes_after_a_manual_stop_changes_nothing() {
        let flags = Arc::new(Flags::default());
        let s = speed_service(
            &flags,
            speed_fake(
                Timing::done(ms(300), ms(900)),
                Timing::done(ms(300), ms(9000)),
            ),
        );
        s.start_trial(true, &mut |_| {}).unwrap();
        let (generation, baseline) = {
            let t = lock(&s.trial);
            (t.generation, t.pending_baseline.clone().unwrap())
        };
        s.stop_trial().unwrap();
        s.run_speed_check(generation, &baseline);
        assert!(s
            .trial_status(std::time::Instant::now())
            .speed_note
            .is_none());
    }

    #[test]
    fn the_ui_sees_the_checking_phase_while_measuring_and_nothing_blocks() {
        struct Spy {
            svc: Mutex<Option<Weak<DesyncService>>>,
            seen: Mutex<Vec<TrialPhase>>,
        }
        impl speed::TimedFetch for Spy {
            fn fetch(&self, _: &str, _: Duration) -> Timing {
                if let Some(s) = self.svc.lock().unwrap().as_ref().and_then(Weak::upgrade) {
                    // Статус читается без ожидания: замер не держит блокировки интерфейса.
                    self.seen
                        .lock()
                        .unwrap()
                        .push(s.trial_status(std::time::Instant::now()).phase);
                }
                Timing::done(ms(100), ms(200))
            }
        }
        let spy = Arc::new(Spy {
            svc: Mutex::new(None),
            seen: Mutex::new(Vec::new()),
        });
        let flags = Arc::new(Flags::default());
        let s = speed_service(
            &flags,
            speed_fake(Timing::done(ms(1), ms(1)), Timing::done(ms(1), ms(1))),
        );
        s.set_speed_probe(spy.clone());
        *spy.svc.lock().unwrap() = Some(Arc::downgrade(&s));
        s.start_trial(true, &mut |_| {}).unwrap();
        finish_speed_check(&s);
        let seen = spy.seen.lock().unwrap().clone();
        assert!(seen.contains(&TrialPhase::MeasuringBaseline));
        assert!(seen.contains(&TrialPhase::MeasuringAfter));
        assert_eq!(
            s.trial_status(std::time::Instant::now()).phase,
            TrialPhase::Idle
        );
        s.stop_trial().unwrap();
    }

    // ----- этапы запуска, параллельный замер, кеш -----

    #[test]
    fn stage_timings_are_recorded_and_logged() {
        let flags = Arc::new(Flags::default());
        let s = speed_service(
            &flags,
            speed_fake(Timing::done(ms(10), ms(20)), Timing::done(ms(10), ms(20))),
        );
        assert!(s.last_start_timings().is_none());
        s.start_trial(true, &mut |_| {}).unwrap();
        let t = s.last_start_timings().unwrap();
        assert!(t.total >= t.prepare + t.spawn_settle, "{t:?}");
        let log = std::fs::read_to_string(s.dir.join("desync-start.log")).unwrap();
        assert!(
            log.contains("запуск: всего") && log.contains("подготовка"),
            "{log}"
        );
        // Цифры на фейках (без диска и сети) выводятся для docs/status/bypass-optimization.md.
        println!("TIMINGS(fake) {t:?}");
        s.stop_trial().unwrap();
    }

    #[test]
    fn the_baseline_runs_in_parallel_with_preparation_not_after_it() {
        // Подготовка (проверка файлов) 400 мс и замер 400 мс: вместе ≈ 400, а не 800.
        struct Slow(Duration);
        impl speed::TimedFetch for Slow {
            fn fetch(&self, _: &str, _: Duration) -> Timing {
                std::thread::sleep(self.0);
                Timing::done(self.0, self.0)
            }
        }
        let flags = Arc::new(Flags::default());
        flags.present.store(true, Ordering::SeqCst);
        let mut svc = make(
            &flags,
            |f| Host {
                readiness: Readiness::NotImplemented("профиль не проверен".into()),
                locate_delay: Duration::from_millis(400),
                ..good(f)
            },
            true,
            false,
        );
        svc.candidate = profile();
        svc.speed_async = false;
        svc.set_speed_probe(Arc::new(Slow(Duration::from_millis(400))));
        let s = Arc::new(svc);
        let started = std::time::Instant::now();
        s.start_trial(true, &mut |_| {}).unwrap();
        let took = started.elapsed();
        assert!(
            took < Duration::from_millis(760),
            "последовательно было бы ≥ 800 мс, вышло {took:?}"
        );
        s.stop_trial().unwrap();
    }

    #[test]
    fn a_fresh_baseline_is_reused_so_the_next_start_does_not_wait_for_it() {
        let flags = Arc::new(Flags::default());
        let fake = speed_fake(
            Timing::done(ms(300), ms(900)),
            Timing::done(ms(300), ms(900)),
        );
        let s = speed_service(&flags, fake.clone());
        s.start_trial(true, &mut |_| {}).unwrap();
        finish_speed_check(&s);
        let after_first = fake.calls.lock().unwrap().len();
        assert_eq!(after_first, 6, "три адреса без обхода и три с обходом");
        s.stop_trial().unwrap();
        s.start_trial(true, &mut |_| {}).unwrap();
        // Второй запуск: замера без обхода нет (взят прошлый), новых запросов пока не было.
        assert_eq!(fake.calls.lock().unwrap().len(), after_first);
        s.stop_trial().unwrap();
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
