//! Безопасные настройки Windows и профили Boost. Реестр (HKCU, а для политик HKLM только при
//! правах администратора) и схема электропитания. Перед изменением прежнее значение записывается в
//! `HKCU\Software\OCHISTKA\Backup`, откат возвращает его (или удаляет значение, если его не было).
//! После каждой записи значения читаются обратно: «включено» показывается только если Windows
//! действительно приняла значение, иначе всё возвращается и возникает ошибка.
//! Здесь только логика над портами; настоящие записи делает адаптер платформы, в тестах — только
//! поддельный реестр.

use serde::Serialize;
use ts_rs::TS;

use crate::system::CoreError;
use crate::tweaks_catalog::{
    Entry, TweakDef, Val, HIGH_SCHEME, TWEAKS, ULTIMATE_SCHEME, WIN11, WIN11_22H2, WIN11_24H2,
};
pub use crate::tweaks_catalog::{Group, RestartKind};

/// Реестр. Пути заданы относительно HKCU; приставка `HKLM\` означает HKLM.
pub trait RegistryPort: Send + Sync {
    fn read_dword(&self, key: &str, name: &str) -> Result<Option<u32>, CoreError>;
    fn write_dword(&self, key: &str, name: &str, value: u32) -> Result<(), CoreError>;
    fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, CoreError>;
    fn write_string(&self, key: &str, name: &str, value: &str) -> Result<(), CoreError>;
    fn delete_value(&self, key: &str, name: &str) -> Result<(), CoreError>;
    fn delete_key(&self, key: &str) -> Result<(), CoreError>;
}

/// Схема электропитания Windows.
pub trait PowerPort: Send + Sync {
    /// GUID активной схемы.
    fn active_scheme(&self) -> Result<String, CoreError>;
    /// Делает схему активной (при необходимости создаёт её: «Максимальная производительность»).
    fn set_active_scheme(&self, guid: &str) -> Result<(), CoreError>;
}

/// Сведения о системе и переходы в системные окна.
pub trait SystemPort: Send + Sync {
    /// Номер сборки Windows (CurrentBuildNumber): 19045 для Windows 10 22H2, 22631 для Windows 11 23H2.
    fn windows_build(&self) -> Option<u32>;
    /// Короткая пауза перед проверочным чтением: Windows и Проводник успевают перезаписать
    /// значение, если оно им не нравится. В тестах ничего не ждёт.
    fn settle(&self) {}
    /// Программа запущена с правами администратора (запись в HKLM возможна).
    fn is_elevated(&self) -> bool;
    /// Статус активации Windows.
    fn activation(&self) -> Result<ActivationInfo, CoreError>;
    /// Открывает «Параметры → Активация» (`ms-settings:activation`).
    fn open_activation_settings(&self) -> Result<(), CoreError>;
}

/// Всё, что нужно настройкам и Boost, в одном объекте.
pub trait TweaksPort: RegistryPort + PowerPort + SystemPort {}
impl<T: RegistryPort + PowerPort + SystemPort> TweaksPort for T {}

/// Доступна ли настройка на этом компьютере.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "TweakAvailability.ts")]
pub enum TweakAvailability {
    Available,
    /// Ключ не действует на этой версии Windows.
    UnsupportedVersion,
    /// Запись в HKLM: нужны права администратора.
    NeedsAdmin,
    /// Параметр задан политикой компьютера: Windows не даст изменить его отсюда.
    ManagedByPolicy,
}

pub const POLICY_REASON: &str =
    "Включено политикой компьютера (администратор или организация): здесь не меняется.";
pub const NOT_KEPT_REASON: &str = "Windows сразу вернула прежнее значение. Скорее всего, эта редакция или сборка Windows не поддерживает параметр.";
pub const PROTECTED_REASON: &str =
    "Windows не разрешила записать параметр: раздел реестра защищён (политикой или защитной программой).";

pub const NEEDS_ADMIN_REASON: &str =
    "Нужны права администратора: запусти OCHISTKA от имени администратора.";

const BACKUP_ROOT: &str = r"Software\OCHISTKA\Backup";
const BOOST_KEY: &str = r"Software\OCHISTKA\Backup\boost";
const POWER_VALUE: &str = "PowerScheme";
const PROFILE_VALUE: &str = "Profile";

/// Состояние одной настройки для интерфейса.
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "TweakState.ts")]
pub struct TweakState {
    pub id: String,
    /// Все значения настройки сейчас такие, как после применения.
    pub applied: bool,
    /// Прежнее значение сохранено, откат возможен.
    pub has_backup: bool,
    pub availability: TweakAvailability,
    /// Причина недоступности человеческими словами.
    pub reason: Option<String>,
    /// Когда вступает в силу: `none`, `explorer`, `signOut`, `reboot`.
    pub restart: String,
    /// Включается только после явного подтверждения.
    pub dangerous: bool,
}

/// Состояние профиля Boost: `basic`, `gaming`, `max` или ничего.
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "BoostState.ts")]
pub struct BoostState {
    pub profile: Option<String>,
    pub has_backup: bool,
    /// Состав выбранного профиля и фактическое состояние каждого параметра.
    pub steps: Vec<BoostStep>,
}

/// Один параметр профиля Boost: применён он или пропущен (и почему).
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "BoostStep.ts")]
pub struct BoostStep {
    /// Id настройки или `power` для схемы электропитания.
    pub id: String,
    pub applied: bool,
    /// Почему не применён (пропущен), человеческими словами.
    pub reason: Option<String>,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "TweaksOverview.ts")]
pub struct TweaksOverview {
    pub tweaks: Vec<TweakState>,
    pub boost: BoostState,
    /// Номер сборки Windows, если удалось определить.
    pub build: Option<u32>,
    pub elevated: bool,
}

#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "ActivationStatus.ts")]
pub enum ActivationStatus {
    Licensed,
    NotActivated,
    Grace,
    Notification,
    Unknown,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "ActivationInfo.ts")]
pub struct ActivationInfo {
    pub status: ActivationStatus,
    /// Сколько полных дней осталось до конца льготного периода (если Windows сообщила).
    pub grace_days: Option<u32>,
}

/// Разбирает вывод запроса лицензии: `статус;минуты` (число LicenseStatus не зависит от языка).
pub fn parse_activation(output: &str) -> ActivationInfo {
    let line = output.lines().map(str::trim).find(|l| !l.is_empty());
    let mut parts = line.unwrap_or("").split(';');
    let code = parts.next().and_then(|c| c.trim().parse::<u32>().ok());
    let minutes = parts.next().and_then(|m| m.trim().parse::<u64>().ok());
    let status = match code {
        Some(1) => ActivationStatus::Licensed,
        Some(0) => ActivationStatus::NotActivated,
        Some(2..=4) | Some(6) => ActivationStatus::Grace,
        Some(5) => ActivationStatus::Notification,
        _ => ActivationStatus::Unknown,
    };
    let grace_days = match status {
        ActivationStatus::Grace => minutes
            .filter(|m| *m > 0)
            .map(|m| u32::try_from(m / 1440).unwrap_or(u32::MAX)),
        _ => None,
    };
    ActivationInfo { status, grace_days }
}

/// Профиль Boost: набор отдельных параметров. Состав описан в `docs/status/boost-profiles.md`.
struct ProfileDef {
    id: &'static str,
    /// Обязательные параметры (без прав администратора).
    tweaks: &'static [&'static str],
    /// Параметры, которые пропускаются с причиной, если недоступны (права, версия Windows).
    optional: &'static [&'static str],
    /// Схема электропитания или `None`, если схема не меняется.
    power: Option<&'static str>,
}

const BASE_TWEAKS: &[&str] = &["game_dvr", "game_mode"];

const PROFILES: &[ProfileDef] = &[
    ProfileDef {
        id: "basic",
        tweaks: BASE_TWEAKS,
        optional: &[],
        power: None,
    },
    ProfileDef {
        id: "gaming",
        tweaks: BASE_TWEAKS,
        optional: &[],
        power: Some(HIGH_SCHEME),
    },
    ProfileDef {
        id: "max",
        tweaks: BASE_TWEAKS,
        optional: &["power_throttling"],
        power: Some(ULTIMATE_SCHEME),
    },
];

const POWER_STEP: &str = "power";
const POWER_UNAVAILABLE: &str = "Эта схема электропитания недоступна на этом компьютере.";

fn skip_value(id: &str) -> String {
    format!("skip_{id}")
}

fn def(id: &str) -> Result<&'static TweakDef, CoreError> {
    TWEAKS
        .iter()
        .find(|tweak| tweak.id == id)
        .ok_or_else(|| CoreError::InvalidTarget(format!("неизвестная настройка: {id}")))
}

/// Нужно ли подтверждение, чтобы включить настройку (единственное окно с вопросом: VBS/HVCI).
pub fn is_dangerous(id: &str) -> bool {
    def(id).map(|tweak| tweak.dangerous).unwrap_or(false)
}

/// Все идентификаторы настроек (для проверок и документации).
pub fn all_ids() -> Vec<&'static str> {
    TWEAKS.iter().map(|tweak| tweak.id).collect()
}

fn backup_key(id: &str) -> String {
    format!(r"{BACKUP_ROOT}\{id}")
}

fn has_backup(reg: &dyn RegistryPort, tweak: &TweakDef) -> Result<bool, CoreError> {
    if tweak.power_scheme.is_some() {
        return Ok(reg.read_string(BOOST_KEY, POWER_VALUE)?.is_some());
    }
    Ok(reg.read_dword(&backup_key(tweak.id), "count")?.is_some())
}

fn applicable(tweak: &TweakDef, build: Option<u32>) -> impl Iterator<Item = (usize, &Entry)> {
    tweak
        .entries
        .iter()
        .enumerate()
        .filter(move |(_, entry)| entry.applies_to(build))
}

fn unsupported_reason(tweak: &TweakDef) -> String {
    let min = tweak.entries.iter().map(|e| e.min_build).min().unwrap_or(0);
    let max = tweak.entries.iter().map(|e| e.max_build).max().unwrap_or(0);
    if min >= WIN11_24H2 {
        "Нужна Windows 11 версии 24H2 или новее.".into()
    } else if min >= WIN11_22H2 {
        "Нужна Windows 11 версии 22H2 или новее.".into()
    } else if min >= WIN11 {
        "Есть только в Windows 11.".into()
    } else if max < WIN11 {
        "Есть только в Windows 10.".into()
    } else {
        "Эта версия Windows не поддерживает настройку.".into()
    }
}

fn availability(
    tweak: &TweakDef,
    build: Option<u32>,
    elevated: bool,
) -> (TweakAvailability, Option<String>) {
    if tweak.power_scheme.is_some() {
        return (TweakAvailability::Available, None);
    }
    let mut any = false;
    let mut hklm = false;
    for (_, entry) in applicable(tweak, build) {
        any = true;
        hklm |= entry.is_hklm();
    }
    if !any {
        return (
            TweakAvailability::UnsupportedVersion,
            Some(unsupported_reason(tweak)),
        );
    }
    if hklm && !elevated {
        return (
            TweakAvailability::NeedsAdmin,
            Some(NEEDS_ADMIN_REASON.into()),
        );
    }
    (TweakAvailability::Available, None)
}

/// Число из строки: десятичное или `0x..`. Так читаются значения, записанные как REG_SZ.
pub fn dword_from_text(text: &str) -> Option<u32> {
    let text = text.trim();
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

/// Читает числовой параметр, даже если Windows или другая программа хранит его строкой.
/// Второе значение: число было прочитано из строки (тип надо вернуть при откате).
fn read_flag(
    reg: &dyn RegistryPort,
    key: &str,
    name: &str,
) -> Result<(Option<u32>, Option<String>), CoreError> {
    if let Some(value) = reg.read_dword(key, name)? {
        return Ok((Some(value), None));
    }
    match reg.read_string(key, name)? {
        Some(text) => Ok((dword_from_text(&text), Some(text))),
        None => Ok((None, None)),
    }
}

fn entry_matches(reg: &dyn RegistryPort, entry: &Entry) -> Result<bool, CoreError> {
    Ok(match entry.value {
        Val::Dword(want) => read_flag(reg, entry.key, entry.name)?.0 == Some(want),
        Val::Text(want) => reg.read_string(entry.key, entry.name)?.as_deref() == Some(want),
    })
}

/// Политика компьютера уже включает этот параметр (и сильнее значений пользователя).
fn policy_forces(reg: &dyn RegistryPort, tweak: &TweakDef) -> bool {
    tweak.policies.iter().any(|policy| {
        read_flag(reg, policy.key, policy.name)
            .map(|(value, _)| value == Some(policy.on))
            .unwrap_or(false)
    })
}

fn is_applied(port: &dyn TweaksPort, tweak: &TweakDef, build: Option<u32>) -> bool {
    if let Some(guid) = tweak.power_scheme {
        return port
            .active_scheme()
            .map(|active| active.eq_ignore_ascii_case(guid))
            .unwrap_or(false);
    }
    if policy_forces(port, tweak) {
        return true;
    }
    let mut any = false;
    for (_, entry) in applicable(tweak, build) {
        any = true;
        if !entry_matches(port, entry).unwrap_or(false) {
            return false;
        }
    }
    any
}

/// GUID вида `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`: только такие значения попадают в powercfg.
pub fn is_guid(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let lens = [8, 4, 4, 4, 12];
    parts.len() == 5
        && parts
            .iter()
            .zip(lens)
            .all(|(part, len)| part.len() == len && part.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Достаёт GUID из вывода `powercfg /getactivescheme`; язык системы не важен.
pub fn parse_active_scheme(output: &str) -> Option<String> {
    output
        .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .find(|token| is_guid(token))
        .map(str::to_ascii_lowercase)
}

fn not_accepted() -> CoreError {
    CoreError::NotApplicable(NOT_KEPT_REASON.into())
}

/// Сохраняет прежние значения применимых ключей (только если копии ещё нет).
fn save_backup(
    reg: &dyn RegistryPort,
    tweak: &TweakDef,
    build: Option<u32>,
) -> Result<(), CoreError> {
    if has_backup(reg, tweak)? {
        return Ok(());
    }
    let key = backup_key(tweak.id);
    let mut count = 0;
    for (index, entry) in applicable(tweak, build) {
        // existed: 0 — значения не было, 1 — было как есть, 2 — число хранилось строкой.
        let existed = match entry.value {
            Val::Dword(_) => match read_flag(reg, entry.key, entry.name)? {
                (Some(current), None) => {
                    reg.write_dword(&key, &format!("e{index}_value"), current)?;
                    1
                }
                (_, Some(text)) => {
                    reg.write_string(&key, &format!("e{index}_text"), &text)?;
                    2
                }
                (None, None) => {
                    reg.write_dword(&key, &format!("e{index}_value"), 0)?;
                    0
                }
            },
            Val::Text(_) => {
                let current = reg.read_string(entry.key, entry.name)?;
                reg.write_string(
                    &key,
                    &format!("e{index}_text"),
                    current.as_deref().unwrap_or(""),
                )?;
                u32::from(current.is_some())
            }
        };
        reg.write_dword(&key, &format!("e{index}_existed"), existed)?;
        count += 1;
    }
    // Счётчик пишется последним: он показывает, что копия целая.
    reg.write_dword(&key, "count", count)
}

/// Применяет настройку и проверяет чтением; при отказе Windows возвращает всё назад.
fn apply(port: &dyn TweaksPort, tweak: &TweakDef, build: Option<u32>) -> Result<(), CoreError> {
    let reg: &dyn RegistryPort = port;
    save_backup(reg, tweak, build)?;
    let written: Result<(), CoreError> =
        applicable(tweak, build).try_for_each(|(_, entry)| match entry.value {
            Val::Dword(value) => reg.write_dword(entry.key, entry.name, value),
            Val::Text(value) => reg.write_string(entry.key, entry.name, value),
        });
    // Пауза: Проводник и Windows могут перезаписать значение сразу после записи.
    if written.is_ok() {
        port.settle();
    }
    let verified = written.and_then(|()| {
        for (_, entry) in applicable(tweak, build) {
            if !entry_matches(reg, entry)? {
                return Err(not_accepted());
            }
        }
        Ok(())
    });
    if let Err(err) = verified {
        // Откат: если и он не удался, это важнее исходной причины.
        revert(port, tweak, build)?;
        return Err(err);
    }
    Ok(())
}

/// Возвращает сохранённые значения, проверяет чтением и удаляет копию. Без копии ничего не трогает.
fn revert(port: &dyn TweaksPort, tweak: &TweakDef, build: Option<u32>) -> Result<(), CoreError> {
    let reg: &dyn RegistryPort = port;
    if tweak.power_scheme.is_some() {
        return restore_power(port);
    }
    if !has_backup(reg, tweak)? {
        return Ok(());
    }
    let key = backup_key(tweak.id);
    for (index, entry) in applicable(tweak, build) {
        let Some(existed) = reg.read_dword(&key, &format!("e{index}_existed"))? else {
            continue;
        };
        match (entry.value, existed) {
            (Val::Dword(_), 2) => {
                let text = reg
                    .read_string(&key, &format!("e{index}_text"))?
                    .unwrap_or_default();
                reg.write_string(entry.key, entry.name, &text)?;
            }
            (Val::Dword(_), 1) => {
                let value = reg
                    .read_dword(&key, &format!("e{index}_value"))?
                    .unwrap_or(0);
                reg.write_dword(entry.key, entry.name, value)?;
            }
            (Val::Text(_), 1) => {
                let value = reg
                    .read_string(&key, &format!("e{index}_text"))?
                    .unwrap_or_default();
                reg.write_string(entry.key, entry.name, &value)?;
            }
            _ => reg.delete_value(entry.key, entry.name)?,
        }
        // Чтение обратно: значение больше не равно «включённому».
        if existed == 0 && entry_matches(reg, entry)? {
            return Err(not_accepted());
        }
    }
    reg.delete_key(&key)
}

fn state_of(
    port: &dyn TweaksPort,
    tweak: &TweakDef,
    build: Option<u32>,
    elevated: bool,
) -> TweakState {
    let (mut availability, mut reason) = availability(tweak, build, elevated);
    if availability != TweakAvailability::UnsupportedVersion && policy_forces(port, tweak) {
        availability = TweakAvailability::ManagedByPolicy;
        reason = Some(POLICY_REASON.into());
    }
    TweakState {
        id: tweak.id.to_string(),
        applied: is_applied(port, tweak, build),
        has_backup: has_backup(port, tweak).unwrap_or(false),
        availability,
        reason,
        restart: tweak.restart.as_str().to_string(),
        dangerous: tweak.dangerous,
    }
}

/// Включает или выключает одну настройку (выключение = откат к сохранённому значению).
pub fn set_tweak(port: &dyn TweaksPort, id: &str, enabled: bool) -> Result<(), CoreError> {
    let tweak = def(id)?;
    let build = port.windows_build();
    match availability(tweak, build, port.is_elevated()) {
        (TweakAvailability::Available, _) => {}
        (TweakAvailability::NeedsAdmin, _) => return Err(CoreError::NeedsAdmin),
        (TweakAvailability::UnsupportedVersion, reason) => {
            return Err(CoreError::NotApplicable(reason.unwrap_or_default()))
        }
        (TweakAvailability::ManagedByPolicy, _) => {}
    }
    if policy_forces(port, tweak) {
        // Политика уже включила параметр: включать нечего, выключить отсюда нельзя.
        return if enabled {
            Ok(())
        } else {
            Err(CoreError::NotApplicable(POLICY_REASON.into()))
        };
    }
    match (enabled, tweak.power_scheme) {
        (true, Some(guid)) => {
            enable_power(port, guid).map_err(|_| CoreError::NotApplicable(POWER_UNAVAILABLE.into()))
        }
        (true, None) => apply(port, tweak, build),
        (false, _) => revert(port, tweak, build),
    }
}

/// Запоминает прежнюю схему (один раз) и включает новую; результат проверяется чтением.
fn enable_power(port: &dyn TweaksPort, guid: &str) -> Result<(), CoreError> {
    let reg: &dyn RegistryPort = port;
    if reg.read_string(BOOST_KEY, POWER_VALUE)?.is_none() {
        let current = port.active_scheme()?;
        reg.write_string(BOOST_KEY, POWER_VALUE, &current)?;
    }
    port.set_active_scheme(guid)?;
    if !port.active_scheme()?.eq_ignore_ascii_case(guid) {
        return Err(not_accepted());
    }
    Ok(())
}

fn restore_power(port: &dyn TweaksPort) -> Result<(), CoreError> {
    let reg: &dyn RegistryPort = port;
    if let Some(saved) = reg.read_string(BOOST_KEY, POWER_VALUE)? {
        if is_guid(&saved) {
            port.set_active_scheme(&saved)?;
        }
        reg.delete_value(BOOST_KEY, POWER_VALUE)?;
    }
    Ok(())
}

fn profile_def(id: &str) -> Option<&'static ProfileDef> {
    PROFILES.iter().find(|profile| profile.id == id)
}

/// Состав профиля и фактическое состояние каждого параметра (читается из системы).
fn profile_steps(
    port: &dyn TweaksPort,
    profile: &ProfileDef,
    build: Option<u32>,
) -> Result<Vec<BoostStep>, CoreError> {
    let reg: &dyn RegistryPort = port;
    let elevated = port.is_elevated();
    let mut steps = Vec::new();
    for id in profile.tweaks.iter().chain(profile.optional) {
        let tweak = def(id)?;
        let applied = is_applied(port, tweak, build);
        let reason = if applied {
            None
        } else {
            match reg.read_string(BOOST_KEY, &skip_value(id))? {
                Some(reason) => Some(reason),
                None => availability(tweak, build, elevated).1,
            }
        };
        steps.push(BoostStep {
            id: (*id).to_string(),
            applied,
            reason,
        });
    }
    if let Some(guid) = profile.power {
        let applied = port
            .active_scheme()
            .map(|active| active.eq_ignore_ascii_case(guid))
            .unwrap_or(false);
        let reason = if applied {
            None
        } else {
            reg.read_string(BOOST_KEY, &skip_value(POWER_STEP))?
        };
        steps.push(BoostStep {
            id: POWER_STEP.to_string(),
            applied,
            reason,
        });
    }
    Ok(steps)
}

/// Текущий профиль Boost: записанный и всё ещё действующий. Пропущенные с причиной параметры
/// (нет прав, схемы питания нет на этом компьютере) профиль не отменяют.
pub fn boost_state(port: &dyn TweaksPort) -> Result<BoostState, CoreError> {
    let reg: &dyn RegistryPort = port;
    let build = port.windows_build();
    let marker = reg.read_string(BOOST_KEY, PROFILE_VALUE)?;
    let has_backup = marker.is_some() || reg.read_string(BOOST_KEY, POWER_VALUE)?.is_some();
    let Some(profile) = marker.as_deref().and_then(profile_def) else {
        return Ok(BoostState {
            profile: None,
            has_backup,
            steps: Vec::new(),
        });
    };
    let steps = profile_steps(port, profile, build)?;
    let required = |id: &str| profile.tweaks.contains(&id) || id == POWER_STEP;
    let active = steps
        .iter()
        .filter(|step| required(&step.id))
        .all(|step| step.applied || (step.id == POWER_STEP && step.reason.is_some()));
    Ok(BoostState {
        profile: active.then(|| profile.id.to_string()),
        has_backup,
        steps,
    })
}

fn clear_skips(reg: &dyn RegistryPort) -> Result<(), CoreError> {
    for id in [POWER_STEP, "power_throttling"] {
        reg.delete_value(BOOST_KEY, &skip_value(id))?;
    }
    Ok(())
}

/// Включает профиль Boost или (при `None`) возвращает всё, что сделал Boost.
/// Параметр, который недоступен (нет прав, схемы питания нет на этом компьютере),
/// пропускается с причиной и не считается ошибкой всего профиля.
pub fn set_boost_profile(port: &dyn TweaksPort, profile: Option<&str>) -> Result<(), CoreError> {
    let reg: &dyn RegistryPort = port;
    let Some(profile) = profile else {
        return revert_group(port, Group::Boost);
    };
    let profile = profile_def(profile)
        .ok_or_else(|| CoreError::InvalidTarget(format!("неизвестный профиль: {profile}")))?;
    let build = port.windows_build();
    let elevated = port.is_elevated();
    clear_skips(reg)?;
    // Сначала схема питания: если powercfg откажет, реестр ещё не тронут.
    match profile.power {
        Some(guid) => {
            if enable_power(port, guid).is_err() {
                reg.write_string(BOOST_KEY, &skip_value(POWER_STEP), POWER_UNAVAILABLE)?;
            }
        }
        None => restore_power(port)?,
    }
    for id in profile.tweaks {
        apply(port, def(id)?, build)?;
    }
    for id in profile.optional {
        let tweak = def(id)?;
        match availability(tweak, build, elevated) {
            (TweakAvailability::Available, _) => {
                if let Err(err) = apply(port, tweak, build) {
                    reg.write_string(BOOST_KEY, &skip_value(id), &err.to_string())?;
                }
            }
            (_, reason) => {
                let reason = reason.unwrap_or_else(|| NEEDS_ADMIN_REASON.into());
                reg.write_string(BOOST_KEY, &skip_value(id), &reason)?;
            }
        }
    }
    reg.write_string(BOOST_KEY, PROFILE_VALUE, profile.id)
}

/// Возвращает все изменения раздела к сохранённым значениям. То, что требует прав
/// администратора, без них пропускается, и тогда в конце возвращается `NeedsAdmin`.
pub fn revert_group(port: &dyn TweaksPort, group: Group) -> Result<(), CoreError> {
    let reg: &dyn RegistryPort = port;
    let build = port.windows_build();
    let elevated = port.is_elevated();
    let mut skipped = false;
    for tweak in TWEAKS.iter().filter(|tweak| tweak.group == group) {
        if !has_backup(reg, tweak)? {
            continue;
        }
        match availability(tweak, build, elevated).0 {
            TweakAvailability::NeedsAdmin => skipped = true,
            _ => revert(port, tweak, build)?,
        }
    }
    if group == Group::Boost {
        restore_power(port)?;
        reg.delete_key(BOOST_KEY)?;
    }
    if skipped {
        return Err(CoreError::NeedsAdmin);
    }
    Ok(())
}

/// Полное состояние для интерфейса.
pub fn overview(port: &dyn TweaksPort) -> Result<TweaksOverview, CoreError> {
    let build = port.windows_build();
    let elevated = port.is_elevated();
    let tweaks = TWEAKS
        .iter()
        .map(|tweak| state_of(port, tweak, build, elevated))
        .collect();
    Ok(TweaksOverview {
        tweaks,
        boost: boost_state(port)?,
        build,
        elevated,
    })
}

#[cfg(test)]
mod tests;
