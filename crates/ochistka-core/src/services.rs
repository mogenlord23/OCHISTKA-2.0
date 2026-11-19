//! Службы Windows: состояние, тип запуска, запуск и остановка. Без кода ОС.
//!
//! Системные вызовы делает порт `ServiceBackend` (в `crates/platform`): чтение реестра и
//! `C:\Windows\System32\sc.exe` по абсолютному пути. Здесь живут правила: чёрный список
//! критичных служб, служб с последствиями, разбор вывода `sc`, коды ошибок, запоминание
//! прежнего типа запуска и проверка результата повторным чтением.

use serde::Serialize;
use thiserror::Error;

/// Сколько раз и с каким шагом проверяем, что служба дошла до нужного состояния.
const POLL_TIMES: u32 = 20;
const POLL_STEP_MS: u64 = 500;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ServiceState {
    Running,
    Stopped,
    StartPending,
    StopPending,
    ContinuePending,
    PausePending,
    Paused,
    Unknown,
}

impl ServiceState {
    /// Код состояния из вывода `sc` (`SERVICE_STOPPED` = 1 ... `SERVICE_PAUSED` = 7).
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Stopped,
            2 => Self::StartPending,
            3 => Self::StopPending,
            4 => Self::Running,
            5 => Self::ContinuePending,
            6 => Self::PausePending,
            7 => Self::Paused,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "Работает",
            Self::Stopped => "Остановлена",
            Self::StartPending => "Запускается",
            Self::StopPending => "Останавливается",
            Self::ContinuePending => "Возобновляется",
            Self::PausePending => "Приостанавливается",
            Self::Paused => "Приостановлена",
            Self::Unknown => "Неизвестно",
        }
    }
}

/// Тип запуска. «Авто» отложенный отличается флагом `DelayedAutostart`.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StartType {
    Boot,
    System,
    Auto,
    AutoDelayed,
    Manual,
    Disabled,
    Unknown,
}

impl StartType {
    /// По значению `Start` из реестра и флагу отложенного запуска.
    pub fn from_registry(start: Option<u32>, delayed: bool) -> Self {
        match start {
            Some(0) => Self::Boot,
            Some(1) => Self::System,
            Some(2) if delayed => Self::AutoDelayed,
            Some(2) => Self::Auto,
            Some(3) => Self::Manual,
            Some(4) => Self::Disabled,
            _ => Self::Unknown,
        }
    }

    /// Код для хранения прежнего значения: числа реестра, отложенный автозапуск — 102.
    pub fn code(self) -> Option<u32> {
        match self {
            Self::Boot => Some(0),
            Self::System => Some(1),
            Self::Auto => Some(2),
            Self::AutoDelayed => Some(102),
            Self::Manual => Some(3),
            Self::Disabled => Some(4),
            Self::Unknown => None,
        }
    }

    pub fn from_code(code: u32) -> Self {
        match code {
            102 => Self::AutoDelayed,
            other => Self::from_registry(Some(other), false),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Boot => "Загрузка драйвера",
            Self::System => "Системная",
            Self::Auto => "Автоматически",
            Self::AutoDelayed => "Автоматически (отложенный)",
            Self::Manual => "Вручную",
            Self::Disabled => "Отключена",
            Self::Unknown => "Неизвестно",
        }
    }

    /// Значение для `sc config <имя> start= <значение>`.
    fn sc_value(self) -> Option<&'static str> {
        match self {
            Self::Auto => Some("auto"),
            Self::AutoDelayed => Some("delayed-auto"),
            Self::Manual => Some("demand"),
            Self::Disabled => Some("disabled"),
            _ => None,
        }
    }
}

/// Какой тип запуска просит пользователь.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestedStart {
    Auto,
    Manual,
    Disabled,
}

impl RequestedStart {
    fn start_type(self) -> StartType {
        match self {
            Self::Auto => StartType::Auto,
            Self::Manual => StartType::Manual,
            Self::Disabled => StartType::Disabled,
        }
    }
}

/// Насколько осторожно обращаться со службой.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Guard {
    /// Обычная служба: можно менять без вопросов.
    Free,
    /// Остановка или отключение имеют заметные последствия: нужно явное подтверждение.
    Careful,
    /// Критичная для Windows: не меняем никогда.
    Locked,
}

/// Служба для списка. `note` — причина блокировки или последствия остановки.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: ServiceState,
    pub start_type: StartType,
    /// Прежний тип запуска, который запомнили перед первым изменением из OCHISTKA.
    pub previous_start_type: Option<StartType>,
    pub guard: Guard,
    pub note: Option<String>,
}

/// Список для интерфейса: `elevated` — есть ли права администратора для действий.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServicesSnapshot {
    pub elevated: bool,
    pub services: Vec<ServiceInfo>,
}

/// Запись службы из реестра (то, что отдаёт порт).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawService {
    pub name: String,
    pub display_name: String,
    pub start: Option<u32>,
    pub delayed: bool,
}

/// Результат запуска `sc.exe`: код выхода и стандартный вывод.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScOutput {
    pub code: i32,
    pub stdout: String,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    #[error("Имя службы указано неверно.")]
    InvalidName,
    #[error("Службы «{0}» нет в системе.")]
    NotFound(String),
    #[error("{0}")]
    Protected(String),
    /// Нужно подтверждение: внутри последствия остановки или отключения.
    #[error("{0}")]
    NotConfirmed(String),
    #[error("Нужны права администратора. Запусти OCHISTKA от имени администратора.")]
    NeedsAdmin,
    #[error("{0}")]
    Refused(String),
    #[error("Прежнего типа запуска для этой службы не запомнено.")]
    NoSavedStart,
    #[error("{0}")]
    Failed(String),
    #[error("Не удалось прочитать службы: {0}")]
    Probe(String),
    #[error("{feature} недоступно на {os}.")]
    Unsupported { feature: String, os: String },
}

/// Порт служб. Реализация на Windows — в `crates/platform`.
pub trait ServiceBackend: Send + Sync {
    /// Службы Win32 из реестра (без драйверов и шаблонов пользовательских служб).
    fn registry_entries(&self) -> Result<Vec<RawService>, ServiceError>;
    /// Запускает `sc.exe` с аргументами (без поиска по PATH, с ограничением по времени).
    fn sc(&self, args: &[&str]) -> Result<ScOutput, ServiceError>;
    /// Есть ли права администратора для действий над службами.
    fn is_elevated(&self) -> bool;
    /// Прежний тип запуска (код `StartType::code`), запомненный OCHISTKA.
    fn saved_start(&self, name: &str) -> Option<u32>;
    fn save_start(&self, name: &str, code: u32) -> Result<(), ServiceError>;
    fn clear_saved_start(&self, name: &str);
    /// Пауза между проверками состояния. У двойника в тестах ничего не делает.
    fn wait_ms(&self, _ms: u64) {}
}

// ---------------------------------------------------------------------------------------------
// Защита

/// Критичные службы Windows: остановка или отключение ломают систему, сеть или безопасность.
const LOCKED: &[&str] = &[
    "rpcss",
    "rpceptmapper",
    "dcomlaunch",
    "lsm",
    "samss",
    "eventlog",
    "plugplay",
    "power",
    "profsvc",
    "gpsvc",
    "brokerinfrastructure",
    "systemeventsbroker",
    "coremessagingregistrar",
    "cryptsvc",
    "keyiso",
    "bfe",
    "mpssvc",
    "windefend",
    "wdnissvc",
    "securityhealthservice",
    "wscsvc",
    "trustedinstaller",
    "staterepository",
    "usermanager",
    "schedule",
    "timebrokersvc",
    "winmgmt",
    "dhcp",
    "dnscache",
    "nsi",
    "appinfo",
    "lanmanworkstation",
    "sppsvc",
    "clipsvc",
    "tokenbroker",
    "wlidsvc",
];

/// Службы, которые менять можно, но с понятными последствиями (нужно подтверждение).
const CAREFUL: &[(&str, &str)] = &[
    ("spooler", "Перестанет работать печать на всех принтерах."),
    (
        "wuauserv",
        "Центр обновления Windows не сможет скачивать и ставить обновления.",
    ),
    (
        "usosvc",
        "Windows перестанет запускать установку обновлений.",
    ),
    (
        "bits",
        "Фоновая загрузка обновлений и некоторых программ остановится.",
    ),
    (
        "dosvc",
        "Оптимизация доставки обновлений перестанет работать.",
    ),
    (
        "wsearch",
        "Поиск в Windows станет медленным: индекс не будет обновляться.",
    ),
    (
        "sysmain",
        "Windows перестанет заранее подгружать часто используемые программы.",
    ),
    ("audiosrv", "Пропадёт звук во всех программах."),
    (
        "audioendpointbuilder",
        "Пропадёт звук: устройства вывода не будут находиться.",
    ),
    (
        "themes",
        "Оформление Windows станет упрощённым, окна могут выглядеть иначе.",
    ),
    (
        "netman",
        "Перестанут открываться сетевые подключения и их настройки.",
    ),
    (
        "nlasvc",
        "Windows не будет определять, к какой сети подключён компьютер.",
    ),
    ("wlansvc", "Перестанет работать Wi-Fi."),
    ("bthserv", "Перестанет работать Bluetooth."),
    (
        "lanmanserver",
        "Другие компьютеры не смогут открывать общие папки и принтеры этого.",
    ),
    (
        "iphlpsvc",
        "Часть сетевых возможностей (IPv6 через туннели) перестанет работать.",
    ),
    (
        "diagtrack",
        "Windows перестанет отправлять диагностические данные. Безопасно, если они не нужны.",
    ),
    ("wpnservice", "Не будут приходить уведомления приложений."),
    (
        "eventsystem",
        "Часть системных событий перестанет доставляться программам.",
    ),
    (
        "shellhwdetection",
        "Не будут появляться окна при подключении флешек и дисков.",
    ),
    (
        "trkwks",
        "Ярлыки на файлы в сети могут перестать находить перенесённые файлы.",
    ),
    ("dps", "Диагностика неполадок Windows перестанет работать."),
    (
        "lmhosts",
        "Часть старых сетевых имён перестанет определяться.",
    ),
    (
        "camsvc",
        "Приложения не смогут спросить разрешение на камеру и микрофон.",
    ),
];

/// Правила защиты по имени службы без учёта регистра.
pub fn guard_for(name: &str) -> (Guard, Option<String>) {
    let lower = name.to_lowercase();
    if LOCKED.contains(&lower.as_str()) {
        return (
            Guard::Locked,
            Some("Критичная служба Windows: менять её нельзя, система перестанет работать правильно.".to_string()),
        );
    }
    if let Some((_, text)) = CAREFUL.iter().find(|(n, _)| *n == lower) {
        return (Guard::Careful, Some((*text).to_string()));
    }
    (Guard::Free, None)
}

/// Допустимое имя службы: буквы, цифры и `_ . - $ @`; не начинается со слэша и не пустое.
pub fn validate_name(name: &str) -> Result<&str, ServiceError> {
    let name = name.trim();
    let ok = !name.is_empty()
        && name.len() <= 256
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-' | '$' | '@'))
        && !name.starts_with(['-', '.']);
    if ok {
        Ok(name)
    } else {
        Err(ServiceError::InvalidName)
    }
}

// ---------------------------------------------------------------------------------------------
// Разбор вывода `sc`

/// Разбирает вывод `sc query`. Названия строк в выводе переведены на язык Windows, поэтому
/// опираемся на строение: первая строка блока (без отступа) — имя службы после двоеточия, а
/// строка состояния — отступ, двоеточие, число и непереведённое слово (`4  RUNNING`).
pub fn parse_sc_query(output: &str) -> Vec<(String, ServiceState)> {
    const STATE_WORDS: [&str; 7] = [
        "STOPPED",
        "START_PENDING",
        "STOP_PENDING",
        "RUNNING",
        "CONTINUE_PENDING",
        "PAUSE_PENDING",
        "PAUSED",
    ];
    let mut result = Vec::new();
    let mut name: Option<String> = None;
    // Блоки разделены пустой строкой: имя только в первой строке блока (вторая — отображаемое имя).
    let mut block_start = true;
    for line in output.lines() {
        if line.trim().is_empty() {
            block_start = true;
            continue;
        }
        let indented = line.starts_with([' ', '\t']);
        if !indented {
            if block_start && !line.starts_with('[') {
                name = line
                    .split_once(':')
                    .map(|(_, value)| value.trim().to_string())
                    .filter(|value| !value.is_empty());
            }
            block_start = false;
            continue;
        }
        let Some((_, value)) = line.split_once(':') else {
            continue;
        };
        let mut parts = value.split_whitespace();
        let (Some(code), Some(word)) = (parts.next(), parts.next()) else {
            continue;
        };
        if !STATE_WORDS.contains(&word) {
            continue;
        }
        if let (Some(service), Ok(code)) = (name.take(), code.parse::<u32>()) {
            result.push((service, ServiceState::from_code(code)));
        }
    }
    result
}

/// Человеческая причина по коду выхода `sc`.
pub fn sc_error(code: i32, what: &str) -> ServiceError {
    match code {
        5 => ServiceError::NeedsAdmin,
        1060 => ServiceError::NotFound(String::new()),
        1051 => ServiceError::Refused(
            "Нельзя остановить: от этой службы зависят другие работающие службы.".into(),
        ),
        1052 | 1056 | 1062 => ServiceError::Refused("Служба уже в нужном состоянии.".into()),
        1053 => ServiceError::Failed("Служба не ответила вовремя.".into()),
        1058 => ServiceError::Refused(
            "Служба отключена, запустить её нельзя. Сначала поставь тип запуска «Вручную» или «Автоматически».".into(),
        ),
        1075 => ServiceError::Refused("Не запускается: нужная ей служба отсутствует.".into()),
        _ => ServiceError::Failed(format!("Не удалось {what} (код {code}).")),
    }
}

// ---------------------------------------------------------------------------------------------
// Операции

fn state_of(backend: &dyn ServiceBackend, name: &str) -> ServiceState {
    backend
        .sc(&["query", name])
        .ok()
        .and_then(|out| parse_sc_query(&out.stdout).into_iter().next())
        .map_or(ServiceState::Unknown, |(_, state)| state)
}

fn build_info(backend: &dyn ServiceBackend, raw: &RawService, state: ServiceState) -> ServiceInfo {
    let (guard, note) = guard_for(&raw.name);
    ServiceInfo {
        name: raw.name.clone(),
        display_name: raw.display_name.clone(),
        state,
        start_type: StartType::from_registry(raw.start, raw.delayed),
        previous_start_type: backend.saved_start(&raw.name).map(StartType::from_code),
        guard,
        note,
    }
}

/// Список служб: реестр (имена, типы запуска) плюс состояния из `sc query`.
pub fn list_services(backend: &dyn ServiceBackend) -> Result<ServicesSnapshot, ServiceError> {
    let mut entries = backend.registry_entries()?;
    entries.sort_by_key(|entry| entry.name.to_lowercase());
    let states: std::collections::HashMap<String, ServiceState> = backend
        .sc(&["query", "type=", "service", "state=", "all"])
        .map(|out| {
            parse_sc_query(&out.stdout)
                .into_iter()
                .map(|(name, state)| (name.to_lowercase(), state))
                .collect()
        })
        .unwrap_or_default();
    let services = entries
        .iter()
        .map(|raw| {
            let state = states
                .get(&raw.name.to_lowercase())
                .copied()
                .unwrap_or(ServiceState::Unknown);
            build_info(backend, raw, state)
        })
        .collect();
    Ok(ServicesSnapshot {
        elevated: backend.is_elevated(),
        services,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceAction {
    Start,
    Stop,
    SetStart(RequestedStart),
    /// Вернуть тип запуска, который был до первого изменения из OCHISTKA.
    RestoreStart,
}

fn find(backend: &dyn ServiceBackend, name: &str) -> Result<RawService, ServiceError> {
    backend
        .registry_entries()?
        .into_iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| ServiceError::NotFound(name.to_string()))
}

fn run_sc(
    backend: &dyn ServiceBackend,
    args: &[&str],
    what: &str,
    name: &str,
) -> Result<(), ServiceError> {
    let out = backend.sc(args)?;
    if out.code == 0 {
        return Ok(());
    }
    Err(match sc_error(out.code, what) {
        ServiceError::NotFound(_) => ServiceError::NotFound(name.to_string()),
        other => other,
    })
}

/// Ждёт, пока служба дойдёт до нужного состояния; возвращает последнее увиденное.
fn wait_for(backend: &dyn ServiceBackend, name: &str, want: ServiceState) -> ServiceState {
    let mut state = state_of(backend, name);
    for _ in 0..POLL_TIMES {
        if state == want {
            break;
        }
        backend.wait_ms(POLL_STEP_MS);
        state = state_of(backend, name);
    }
    state
}

/// Выполняет действие и возвращает фактическое состояние службы после него.
pub fn apply(
    backend: &dyn ServiceBackend,
    name: &str,
    action: ServiceAction,
    confirmed: bool,
) -> Result<ServiceInfo, ServiceError> {
    let name = validate_name(name)?;
    let raw = find(backend, name)?;
    let name = raw.name.clone();
    let (guard, note) = guard_for(&name);
    if guard == Guard::Locked {
        return Err(ServiceError::Protected(
            note.unwrap_or_else(|| "Эту службу менять нельзя.".into()),
        ));
    }
    let current = StartType::from_registry(raw.start, raw.delayed);
    let risky = matches!(
        action,
        ServiceAction::Stop | ServiceAction::SetStart(RequestedStart::Disabled)
    );
    if guard == Guard::Careful && risky && !confirmed {
        return Err(ServiceError::NotConfirmed(
            note.unwrap_or_else(|| "Последствия остановки не определены.".into()),
        ));
    }
    if !backend.is_elevated() {
        return Err(ServiceError::NeedsAdmin);
    }

    match action {
        ServiceAction::Start => {
            if current == StartType::Disabled {
                return Err(sc_error(1058, "запустить"));
            }
            if state_of(backend, &name) != ServiceState::Running {
                run_sc(backend, &["start", &name], "запустить службу", &name)?;
                let state = wait_for(backend, &name, ServiceState::Running);
                if state != ServiceState::Running {
                    return Err(ServiceError::Failed(format!(
                        "Служба не запустилась. Сейчас: {}.",
                        state.label().to_lowercase()
                    )));
                }
            }
        }
        ServiceAction::Stop => {
            if state_of(backend, &name) != ServiceState::Stopped {
                run_sc(backend, &["stop", &name], "остановить службу", &name)?;
                let state = wait_for(backend, &name, ServiceState::Stopped);
                if state != ServiceState::Stopped {
                    return Err(ServiceError::Failed(format!(
                        "Служба не остановилась. Сейчас: {}.",
                        state.label().to_lowercase()
                    )));
                }
            }
        }
        ServiceAction::SetStart(requested) => {
            set_start(backend, &name, current, requested.start_type())?;
        }
        ServiceAction::RestoreStart => {
            let code = backend
                .saved_start(&name)
                .ok_or(ServiceError::NoSavedStart)?;
            let target = StartType::from_code(code);
            set_start_inner(backend, &name, target)?;
            backend.clear_saved_start(&name);
        }
    }

    let raw = find(backend, &name)?;
    Ok(build_info(backend, &raw, state_of(backend, &name)))
}

/// Меняет тип запуска, запомнив прежний, если он ещё не запомнен.
fn set_start(
    backend: &dyn ServiceBackend,
    name: &str,
    current: StartType,
    target: StartType,
) -> Result<(), ServiceError> {
    if current == target {
        return Ok(());
    }
    let saved_now = if backend.saved_start(name).is_none() {
        if let Some(code) = current.code() {
            backend.save_start(name, code)?;
            true
        } else {
            false
        }
    } else {
        false
    };
    match set_start_inner(backend, name, target) {
        Ok(()) => Ok(()),
        Err(err) => {
            if saved_now {
                backend.clear_saved_start(name);
            }
            Err(err)
        }
    }
}

fn set_start_inner(
    backend: &dyn ServiceBackend,
    name: &str,
    target: StartType,
) -> Result<(), ServiceError> {
    let value = target
        .sc_value()
        .ok_or_else(|| ServiceError::Failed("Этот тип запуска менять нельзя.".into()))?;
    run_sc(
        backend,
        &["config", name, "start=", value],
        "изменить тип запуска",
        name,
    )?;
    // Проверка чтением: значение должно реально поменяться.
    let raw = find(backend, name)?;
    let actual = StartType::from_registry(raw.start, raw.delayed);
    if actual != target {
        return Err(ServiceError::Failed(format!(
            "Тип запуска не изменился: сейчас «{}».",
            actual.label()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
