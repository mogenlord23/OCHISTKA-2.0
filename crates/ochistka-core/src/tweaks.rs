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
}

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

/// Профили Boost: схема питания (GUID) или `None`, если схема не меняется.
const PROFILES: &[(&str, Option<&str>)] = &[
    ("basic", None),
    ("gaming", Some(HIGH_SCHEME)),
    ("max", Some(ULTIMATE_SCHEME)),
];

/// Параметры профиля применяются к этим настройкам.
const PROFILE_TWEAKS: &[&str] = &["game_dvr", "game_mode"];

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

fn entry_matches(reg: &dyn RegistryPort, entry: &Entry) -> Result<bool, CoreError> {
    Ok(match entry.value {
        Val::Dword(want) => reg.read_dword(entry.key, entry.name)? == Some(want),
        Val::Text(want) => reg.read_string(entry.key, entry.name)?.as_deref() == Some(want),
    })
}

fn is_applied(port: &dyn TweaksPort, tweak: &TweakDef, build: Option<u32>) -> bool {
    if let Some(guid) = tweak.power_scheme {
        return port
            .active_scheme()
            .map(|active| active.eq_ignore_ascii_case(guid))
            .unwrap_or(false);
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
    CoreError::Probe("Windows не приняла изменение: значение осталось прежним".into())
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
        let existed = match entry.value {
            Val::Dword(_) => {
                let current = reg.read_dword(entry.key, entry.name)?;
                reg.write_dword(&key, &format!("e{index}_value"), current.unwrap_or(0))?;
                current.is_some()
            }
            Val::Text(_) => {
                let current = reg.read_string(entry.key, entry.name)?;
                reg.write_string(
                    &key,
                    &format!("e{index}_text"),
                    current.as_deref().unwrap_or(""),
                )?;
                current.is_some()
            }
        };
        reg.write_dword(&key, &format!("e{index}_existed"), u32::from(existed))?;
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
    let (availability, reason) = availability(tweak, build, elevated);
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
    }
    match (enabled, tweak.power_scheme) {
        (true, Some(guid)) => enable_power(port, guid),
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

/// Текущий профиль Boost: записанный и всё ещё действующий.
pub fn boost_state(port: &dyn TweaksPort) -> Result<BoostState, CoreError> {
    let reg: &dyn RegistryPort = port;
    let build = port.windows_build();
    let marker = reg.read_string(BOOST_KEY, PROFILE_VALUE)?;
    let has_backup = marker.is_some() || reg.read_string(BOOST_KEY, POWER_VALUE)?.is_some();
    let profile = match marker
        .as_deref()
        .and_then(|p| PROFILES.iter().find(|(id, _)| id == &p))
    {
        Some((id, scheme)) => {
            let tweaks_on = PROFILE_TWEAKS
                .iter()
                .all(|tweak| def(tweak).is_ok_and(|t| is_applied(port, t, build)));
            let power_ok = match scheme {
                Some(guid) => port.active_scheme()?.eq_ignore_ascii_case(guid),
                None => true,
            };
            (tweaks_on && power_ok).then(|| (*id).to_string())
        }
        None => None,
    };
    Ok(BoostState {
        profile,
        has_backup,
    })
}

/// Включает профиль Boost или (при `None`) возвращает всё, что сделал Boost.
pub fn set_boost_profile(port: &dyn TweaksPort, profile: Option<&str>) -> Result<(), CoreError> {
    let reg: &dyn RegistryPort = port;
    let Some(profile) = profile else {
        return revert_group(port, Group::Boost);
    };
    let (_, scheme) = PROFILES
        .iter()
        .find(|(id, _)| *id == profile)
        .ok_or_else(|| CoreError::InvalidTarget(format!("неизвестный профиль: {profile}")))?;
    let build = port.windows_build();
    // Сначала схема питания: если powercfg откажет, реестр ещё не тронут.
    match scheme {
        Some(guid) => enable_power(port, guid)?,
        None => restore_power(port)?,
    }
    for id in PROFILE_TWEAKS {
        apply(port, def(id)?, build)?;
    }
    reg.write_string(BOOST_KEY, PROFILE_VALUE, profile)
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
