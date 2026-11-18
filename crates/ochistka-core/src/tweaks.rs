//! Безопасные настройки Windows и профили Boost. Только значения реестра текущего пользователя
//! и схема электропитания. Перед изменением прежнее значение записывается в
//! `HKCU\Software\OCHISTKA\Backup`, откат возвращает его (или удаляет значение, если его не было).
//! Здесь только логика над портами; настоящие записи делает адаптер платформы, в тестах — только
//! поддельный реестр.

use serde::Serialize;
use ts_rs::TS;

use crate::system::CoreError;

/// Реестр текущего пользователя (HKCU). Пути заданы относительно HKCU.
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
    fn set_active_scheme(&self, guid: &str) -> Result<(), CoreError>;
}

/// Всё, что нужно настройкам и Boost, в одном объекте.
pub trait TweaksPort: RegistryPort + PowerPort {}
impl<T: RegistryPort + PowerPort> TweaksPort for T {}

/// Раздел интерфейса, которому принадлежит настройка.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Settings,
    Boost,
}

impl Group {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "settings" => Some(Self::Settings),
            "boost" => Some(Self::Boost),
            _ => None,
        }
    }
}

struct Entry {
    key: &'static str,
    name: &'static str,
    /// Значение, которое включает настройку.
    value: u32,
}

struct TweakDef {
    id: &'static str,
    group: Group,
    entries: &'static [Entry],
}

const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";

const fn cdm(name: &'static str) -> Entry {
    Entry {
        key: CDM,
        name,
        value: 0,
    }
}

const TWEAKS: &[TweakDef] = &[
    TweakDef {
        id: "transparency",
        group: Group::Settings,
        entries: &[Entry {
            key: r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            name: "EnableTransparency",
            value: 0,
        }],
    },
    TweakDef {
        id: "web_search",
        group: Group::Settings,
        entries: &[Entry {
            key: r"Software\Policies\Microsoft\Windows\Explorer",
            name: "DisableSearchBoxSuggestions",
            value: 1,
        }],
    },
    TweakDef {
        id: "spotlight",
        group: Group::Settings,
        entries: &[
            cdm("SubscribedContent-338387Enabled"),
            cdm("SubscribedContent-338388Enabled"),
            cdm("SubscribedContent-338389Enabled"),
            cdm("SubscribedContent-338393Enabled"),
            cdm("SubscribedContent-353694Enabled"),
            cdm("SubscribedContent-353696Enabled"),
            cdm("SubscribedContent-310093Enabled"),
            cdm("RotatingLockScreenEnabled"),
            cdm("RotatingLockScreenOverlayEnabled"),
        ],
    },
    TweakDef {
        id: "suggestions",
        group: Group::Settings,
        entries: &[cdm("SystemPaneSuggestionsEnabled")],
    },
    TweakDef {
        id: "game_dvr",
        group: Group::Boost,
        entries: &[Entry {
            key: r"System\GameConfigStore",
            name: "GameDVR_Enabled",
            value: 0,
        }],
    },
    TweakDef {
        id: "game_mode",
        group: Group::Boost,
        entries: &[Entry {
            key: r"Software\Microsoft\GameBar",
            name: "AutoGameModeEnabled",
            value: 1,
        }],
    },
];

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
}

/// Профили Boost: схема питания (GUID) или `None`, если схема не меняется.
const PROFILES: &[(&str, Option<&str>)] = &[
    ("basic", None),
    ("gaming", Some("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c")),
    ("max", Some("e9a42b02-d5df-448d-aa00-03f14749eb61")),
];

/// Параметры профиля применяются к этим настройкам.
const PROFILE_TWEAKS: &[&str] = &["game_dvr", "game_mode"];

fn def(id: &str) -> Result<&'static TweakDef, CoreError> {
    TWEAKS
        .iter()
        .find(|tweak| tweak.id == id)
        .ok_or_else(|| CoreError::InvalidTarget(format!("неизвестная настройка: {id}")))
}

fn backup_key(id: &str) -> String {
    format!(r"{BACKUP_ROOT}\{id}")
}

fn has_backup(reg: &dyn RegistryPort, id: &str) -> Result<bool, CoreError> {
    Ok(reg.read_dword(&backup_key(id), "count")?.is_some())
}

fn is_applied(reg: &dyn RegistryPort, tweak: &TweakDef) -> Result<bool, CoreError> {
    for entry in tweak.entries {
        if reg.read_dword(entry.key, entry.name)? != Some(entry.value) {
            return Ok(false);
        }
    }
    Ok(true)
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

/// Применяет настройку, предварительно сохранив прежние значения (только если копии ещё нет).
fn apply(reg: &dyn RegistryPort, tweak: &TweakDef) -> Result<(), CoreError> {
    let key = backup_key(tweak.id);
    if !has_backup(reg, tweak.id)? {
        for (index, entry) in tweak.entries.iter().enumerate() {
            let current = reg.read_dword(entry.key, entry.name)?;
            reg.write_dword(
                &key,
                &format!("e{index}_existed"),
                u32::from(current.is_some()),
            )?;
            reg.write_dword(&key, &format!("e{index}_value"), current.unwrap_or(0))?;
        }
        // Счётчик пишется последним: он показывает, что копия целая.
        reg.write_dword(&key, "count", tweak.entries.len() as u32)?;
    }
    for entry in tweak.entries {
        reg.write_dword(entry.key, entry.name, entry.value)?;
    }
    Ok(())
}

/// Возвращает сохранённые значения и удаляет копию. Без копии ничего не трогает.
fn revert(reg: &dyn RegistryPort, tweak: &TweakDef) -> Result<(), CoreError> {
    if !has_backup(reg, tweak.id)? {
        return Ok(());
    }
    let key = backup_key(tweak.id);
    for (index, entry) in tweak.entries.iter().enumerate() {
        let existed = reg.read_dword(&key, &format!("e{index}_existed"))?;
        let value = reg.read_dword(&key, &format!("e{index}_value"))?;
        match (existed, value) {
            (Some(1), Some(value)) => reg.write_dword(entry.key, entry.name, value)?,
            _ => reg.delete_value(entry.key, entry.name)?,
        }
    }
    reg.delete_key(&key)
}

fn states_for(reg: &dyn RegistryPort, group: Group) -> Result<Vec<TweakState>, CoreError> {
    TWEAKS
        .iter()
        .filter(|tweak| tweak.group == group)
        .map(|tweak| {
            Ok(TweakState {
                id: tweak.id.to_string(),
                applied: is_applied(reg, tweak)?,
                has_backup: has_backup(reg, tweak.id)?,
            })
        })
        .collect()
}

/// Включает или выключает одну настройку (выключение = откат к сохранённому значению).
pub fn set_tweak(reg: &dyn RegistryPort, id: &str, enabled: bool) -> Result<(), CoreError> {
    let tweak = def(id)?;
    if enabled {
        apply(reg, tweak)
    } else {
        revert(reg, tweak)
    }
}

/// Текущий профиль Boost: записанный и всё ещё действующий.
pub fn boost_state(reg: &dyn RegistryPort, power: &dyn PowerPort) -> Result<BoostState, CoreError> {
    let marker = reg.read_string(BOOST_KEY, PROFILE_VALUE)?;
    let has_backup = marker.is_some() || reg.read_string(BOOST_KEY, POWER_VALUE)?.is_some();
    let profile = match marker
        .as_deref()
        .and_then(|p| PROFILES.iter().find(|(id, _)| id == &p))
    {
        Some((id, scheme)) => {
            let tweaks_on = PROFILE_TWEAKS
                .iter()
                .map(|tweak| is_applied(reg, def(tweak)?))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .all(|applied| applied);
            let power_ok = match scheme {
                Some(guid) => power.active_scheme()?.eq_ignore_ascii_case(guid),
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

fn restore_power(reg: &dyn RegistryPort, power: &dyn PowerPort) -> Result<(), CoreError> {
    if let Some(saved) = reg.read_string(BOOST_KEY, POWER_VALUE)? {
        if is_guid(&saved) {
            power.set_active_scheme(&saved)?;
        }
        reg.delete_value(BOOST_KEY, POWER_VALUE)?;
    }
    Ok(())
}

/// Включает профиль Boost или (при `None`) возвращает всё, что сделал Boost.
pub fn set_boost_profile(
    reg: &dyn RegistryPort,
    power: &dyn PowerPort,
    profile: Option<&str>,
) -> Result<(), CoreError> {
    let Some(profile) = profile else {
        return revert_group(reg, power, Group::Boost);
    };
    let (_, scheme) = PROFILES
        .iter()
        .find(|(id, _)| *id == profile)
        .ok_or_else(|| CoreError::InvalidTarget(format!("неизвестный профиль: {profile}")))?;
    // Сначала схема питания: если powercfg откажет, реестр ещё не тронут.
    match scheme {
        Some(guid) => {
            if reg.read_string(BOOST_KEY, POWER_VALUE)?.is_none() {
                let current = power.active_scheme()?;
                reg.write_string(BOOST_KEY, POWER_VALUE, &current)?;
            }
            power.set_active_scheme(guid)?;
        }
        None => restore_power(reg, power)?,
    }
    for id in PROFILE_TWEAKS {
        apply(reg, def(id)?)?;
    }
    reg.write_string(BOOST_KEY, PROFILE_VALUE, profile)
}

/// Возвращает все изменения раздела к сохранённым значениям.
pub fn revert_group(
    reg: &dyn RegistryPort,
    power: &dyn PowerPort,
    group: Group,
) -> Result<(), CoreError> {
    for tweak in TWEAKS.iter().filter(|tweak| tweak.group == group) {
        revert(reg, tweak)?;
    }
    if group == Group::Boost {
        restore_power(reg, power)?;
        reg.delete_key(BOOST_KEY)?;
    }
    Ok(())
}

/// Полное состояние для интерфейса.
pub fn overview(
    reg: &dyn RegistryPort,
    power: &dyn PowerPort,
) -> Result<TweaksOverview, CoreError> {
    let mut tweaks = states_for(reg, Group::Settings)?;
    tweaks.extend(states_for(reg, Group::Boost)?);
    Ok(TweaksOverview {
        tweaks,
        boost: boost_state(reg, power)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fake {
        dwords: Mutex<BTreeMap<(String, String), u32>>,
        strings: Mutex<BTreeMap<(String, String), String>>,
        scheme: Mutex<String>,
        fail_power: Mutex<bool>,
    }

    fn k(key: &str, name: &str) -> (String, String) {
        (key.to_string(), name.to_string())
    }

    impl RegistryPort for Fake {
        fn read_dword(&self, key: &str, name: &str) -> Result<Option<u32>, CoreError> {
            Ok(self.dwords.lock().unwrap().get(&k(key, name)).copied())
        }
        fn write_dword(&self, key: &str, name: &str, value: u32) -> Result<(), CoreError> {
            self.dwords.lock().unwrap().insert(k(key, name), value);
            Ok(())
        }
        fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, CoreError> {
            Ok(self.strings.lock().unwrap().get(&k(key, name)).cloned())
        }
        fn write_string(&self, key: &str, name: &str, value: &str) -> Result<(), CoreError> {
            self.strings
                .lock()
                .unwrap()
                .insert(k(key, name), value.to_string());
            Ok(())
        }
        fn delete_value(&self, key: &str, name: &str) -> Result<(), CoreError> {
            self.dwords.lock().unwrap().remove(&k(key, name));
            self.strings.lock().unwrap().remove(&k(key, name));
            Ok(())
        }
        fn delete_key(&self, key: &str) -> Result<(), CoreError> {
            let prefix = format!("{key}\\");
            let under = |entry: &str| entry == key || entry.starts_with(&prefix);
            self.dwords
                .lock()
                .unwrap()
                .retain(|(path, _), _| !under(path));
            self.strings
                .lock()
                .unwrap()
                .retain(|(path, _), _| !under(path));
            Ok(())
        }
    }

    impl PowerPort for Fake {
        fn active_scheme(&self) -> Result<String, CoreError> {
            Ok(self.scheme.lock().unwrap().clone())
        }
        fn set_active_scheme(&self, guid: &str) -> Result<(), CoreError> {
            if *self.fail_power.lock().unwrap() {
                return Err(CoreError::Probe("powercfg отказал".into()));
            }
            *self.scheme.lock().unwrap() = guid.to_string();
            Ok(())
        }
    }

    const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
    const TRANSPARENCY: (&str, &str) = (
        r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        "EnableTransparency",
    );

    fn fake() -> Fake {
        let fake = Fake::default();
        *fake.scheme.lock().unwrap() = BALANCED.to_string();
        fake
    }

    fn state(reg: &Fake, id: &str) -> TweakState {
        overview(reg, reg)
            .unwrap()
            .tweaks
            .into_iter()
            .find(|tweak| tweak.id == id)
            .unwrap()
    }

    #[test]
    fn apply_saves_previous_value_and_revert_restores_it() {
        let reg = fake();
        reg.write_dword(TRANSPARENCY.0, TRANSPARENCY.1, 1).unwrap();
        assert!(!state(&reg, "transparency").applied);

        set_tweak(&reg, "transparency", true).unwrap();
        assert_eq!(
            reg.read_dword(TRANSPARENCY.0, TRANSPARENCY.1).unwrap(),
            Some(0)
        );
        let now = state(&reg, "transparency");
        assert!(now.applied && now.has_backup);

        set_tweak(&reg, "transparency", false).unwrap();
        assert_eq!(
            reg.read_dword(TRANSPARENCY.0, TRANSPARENCY.1).unwrap(),
            Some(1)
        );
        let back = state(&reg, "transparency");
        assert!(!back.applied && !back.has_backup);
    }

    #[test]
    fn revert_deletes_a_value_that_did_not_exist_before() {
        let reg = fake();
        set_tweak(&reg, "web_search", true).unwrap();
        set_tweak(&reg, "web_search", false).unwrap();
        let key = r"Software\Policies\Microsoft\Windows\Explorer";
        assert_eq!(
            reg.read_dword(key, "DisableSearchBoxSuggestions").unwrap(),
            None
        );
    }

    #[test]
    fn second_apply_keeps_the_original_backup() {
        let reg = fake();
        reg.write_dword(TRANSPARENCY.0, TRANSPARENCY.1, 1).unwrap();
        set_tweak(&reg, "transparency", true).unwrap();
        set_tweak(&reg, "transparency", true).unwrap();
        set_tweak(&reg, "transparency", false).unwrap();
        assert_eq!(
            reg.read_dword(TRANSPARENCY.0, TRANSPARENCY.1).unwrap(),
            Some(1)
        );
    }

    #[test]
    fn unknown_tweak_is_refused() {
        let reg = fake();
        assert!(matches!(
            set_tweak(&reg, "format_c", true),
            Err(CoreError::InvalidTarget(_))
        ));
    }

    #[test]
    fn spotlight_covers_every_flag_and_reverts_each() {
        let reg = fake();
        reg.write_dword(CDM, "SubscribedContent-338388Enabled", 1)
            .unwrap();
        set_tweak(&reg, "spotlight", true).unwrap();
        assert!(state(&reg, "spotlight").applied);
        revert_group(&reg, &reg, Group::Settings).unwrap();
        assert_eq!(
            reg.read_dword(CDM, "SubscribedContent-338388Enabled")
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            reg.read_dword(CDM, "SubscribedContent-338387Enabled")
                .unwrap(),
            None
        );
        assert!(!state(&reg, "spotlight").has_backup);
    }

    #[test]
    fn gaming_profile_switches_power_and_revert_restores_the_old_scheme() {
        let reg = fake();
        set_boost_profile(&reg, &reg, Some("gaming")).unwrap();
        assert_eq!(*reg.scheme.lock().unwrap(), PROFILES[1].1.unwrap());
        let boost = overview(&reg, &reg).unwrap().boost;
        assert_eq!(boost.profile.as_deref(), Some("gaming"));
        assert!(boost.has_backup);

        set_boost_profile(&reg, &reg, None).unwrap();
        assert_eq!(*reg.scheme.lock().unwrap(), BALANCED);
        let boost = overview(&reg, &reg).unwrap().boost;
        assert_eq!(boost.profile, None);
        assert!(!boost.has_backup);
        assert!(!state(&reg, "game_dvr").applied);
    }

    #[test]
    fn switching_profiles_keeps_the_very_first_scheme() {
        let reg = fake();
        set_boost_profile(&reg, &reg, Some("gaming")).unwrap();
        set_boost_profile(&reg, &reg, Some("max")).unwrap();
        set_boost_profile(&reg, &reg, Some("basic")).unwrap();
        // Базовый профиль не меняет схему питания: прежняя схема возвращается.
        assert_eq!(*reg.scheme.lock().unwrap(), BALANCED);
        assert_eq!(
            overview(&reg, &reg).unwrap().boost.profile.as_deref(),
            Some("basic")
        );
    }

    #[test]
    fn failed_power_change_leaves_registry_untouched() {
        let reg = fake();
        *reg.fail_power.lock().unwrap() = true;
        assert!(set_boost_profile(&reg, &reg, Some("max")).is_err());
        assert!(!state(&reg, "game_dvr").applied);
        assert_eq!(overview(&reg, &reg).unwrap().boost.profile, None);
    }

    #[test]
    fn profile_is_not_reported_when_the_power_scheme_was_changed_by_the_user() {
        let reg = fake();
        set_boost_profile(&reg, &reg, Some("gaming")).unwrap();
        *reg.scheme.lock().unwrap() = BALANCED.to_string();
        assert_eq!(overview(&reg, &reg).unwrap().boost.profile, None);
    }

    #[test]
    fn tampered_power_backup_is_never_passed_to_powercfg() {
        let reg = fake();
        reg.write_string(BOOST_KEY, POWER_VALUE, "x /duplicatescheme y")
            .unwrap();
        revert_group(&reg, &reg, Group::Boost).unwrap();
        assert_eq!(*reg.scheme.lock().unwrap(), BALANCED);
    }

    #[test]
    fn parses_guid_from_localized_powercfg_output() {
        let out = "GUID схемы питания: 381B4222-F694-41F0-9685-FF5BB260DF2E  (Сбалансированная)";
        assert_eq!(parse_active_scheme(out).as_deref(), Some(BALANCED));
        assert_eq!(parse_active_scheme("ничего"), None);
        assert!(!is_guid("381b4222-f694-41f0-9685-ff5bb260df2e; calc"));
    }

    #[test]
    fn unknown_profile_is_refused() {
        let reg = fake();
        assert!(set_boost_profile(&reg, &reg, Some("turbo")).is_err());
    }

    #[test]
    fn overview_serializes_with_camel_case_keys() {
        let json = serde_json::to_value(overview(&fake(), &fake()).unwrap()).unwrap();
        assert!(json["tweaks"][0]["hasBackup"].is_boolean());
        assert!(json["boost"]["profile"].is_null());
    }
}
