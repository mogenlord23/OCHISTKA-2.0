use super::*;
use crate::tweaks_catalog::{WIN11, WIN11_24H2};
use std::collections::BTreeMap;
use std::sync::Mutex;

struct Fake {
    dwords: Mutex<BTreeMap<(String, String), u32>>,
    strings: Mutex<BTreeMap<(String, String), String>>,
    scheme: Mutex<String>,
    fail_power: Mutex<bool>,
    /// Windows молча не принимает запись этого значения (как защищённые ключи).
    ignore_name: Mutex<Option<String>>,
    build: u32,
    elevated: bool,
}

fn k(key: &str, name: &str) -> (String, String) {
    (key.to_string(), name.to_string())
}

impl RegistryPort for Fake {
    fn read_dword(&self, key: &str, name: &str) -> Result<Option<u32>, CoreError> {
        Ok(self.dwords.lock().unwrap().get(&k(key, name)).copied())
    }
    fn write_dword(&self, key: &str, name: &str, value: u32) -> Result<(), CoreError> {
        if self.ignore_name.lock().unwrap().as_deref() == Some(name) {
            return Ok(());
        }
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

impl SystemPort for Fake {
    fn windows_build(&self) -> Option<u32> {
        Some(self.build)
    }
    fn is_elevated(&self) -> bool {
        self.elevated
    }
    fn activation(&self) -> Result<ActivationInfo, CoreError> {
        Ok(parse_activation("1;0"))
    }
    fn open_activation_settings(&self) -> Result<(), CoreError> {
        Ok(())
    }
}

const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
const TRANSPARENCY: (&str, &str) = (
    r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
    "EnableTransparency",
);
const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const WIN10_BUILD: u32 = 19045;
const WIN11_BUILD: u32 = 22631;

fn fake_on(build: u32, elevated: bool) -> Fake {
    Fake {
        dwords: Default::default(),
        strings: Default::default(),
        scheme: Mutex::new(BALANCED.to_string()),
        fail_power: Default::default(),
        ignore_name: Default::default(),
        build,
        elevated,
    }
}

fn fake() -> Fake {
    fake_on(WIN11_BUILD, false)
}

fn state(reg: &Fake, id: &str) -> TweakState {
    overview(reg)
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
fn web_search_writes_both_the_policy_and_the_bing_switch() {
    let reg = fake();
    set_tweak(&reg, "web_search", true).unwrap();
    assert_eq!(
        reg.read_dword(
            r"Software\Policies\Microsoft\Windows\Explorer",
            "DisableSearchBoxSuggestions"
        )
        .unwrap(),
        Some(1)
    );
    assert_eq!(
        reg.read_dword(
            r"Software\Microsoft\Windows\CurrentVersion\Search",
            "BingSearchEnabled"
        )
        .unwrap(),
        Some(0)
    );
    assert!(state(&reg, "web_search").applied);
}

#[test]
fn game_dvr_writes_all_three_user_keys() {
    let reg = fake();
    set_tweak(&reg, "game_dvr", true).unwrap();
    assert_eq!(
        reg.read_dword(r"System\GameConfigStore", "GameDVR_Enabled")
            .unwrap(),
        Some(0)
    );
    let dvr = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
    assert_eq!(reg.read_dword(dvr, "AppCaptureEnabled").unwrap(), Some(0));
    assert_eq!(
        reg.read_dword(dvr, "HistoricalCaptureEnabled").unwrap(),
        Some(0)
    );
    assert!(state(&reg, "game_dvr").applied);
    // Одно значение вернулось к 1 (например, Xbox Game Bar): настройка уже не «включена».
    reg.write_dword(dvr, "AppCaptureEnabled", 1).unwrap();
    assert!(!state(&reg, "game_dvr").applied);
}

#[test]
fn a_value_windows_refuses_to_take_is_not_shown_as_applied_and_everything_is_rolled_back() {
    let reg = fake();
    reg.write_dword(TRANSPARENCY.0, TRANSPARENCY.1, 1).unwrap();
    *reg.ignore_name.lock().unwrap() = Some("EnableTransparency".into());
    let err = set_tweak(&reg, "transparency", true).unwrap_err();
    assert!(matches!(err, CoreError::Probe(_)));
    let now = state(&reg, "transparency");
    assert!(!now.applied && !now.has_backup);
    assert_eq!(
        reg.read_dword(TRANSPARENCY.0, TRANSPARENCY.1).unwrap(),
        Some(1)
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
    revert_group(&reg, Group::Settings).unwrap();
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
fn text_values_are_backed_up_and_restored() {
    let reg = fake();
    let key =
        r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\webcam";
    reg.write_string(key, "Value", "Allow").unwrap();
    set_tweak(&reg, "camera", true).unwrap();
    assert_eq!(
        reg.read_string(key, "Value").unwrap().as_deref(),
        Some("Deny")
    );
    assert!(state(&reg, "camera").applied);
    set_tweak(&reg, "camera", false).unwrap();
    assert_eq!(
        reg.read_string(key, "Value").unwrap().as_deref(),
        Some("Allow")
    );
}

#[test]
fn version_specific_keys_are_chosen_by_build() {
    let win11 = fake_on(WIN11_BUILD, false);
    set_tweak(&win11, "widgets", true).unwrap();
    let adv = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
    assert_eq!(win11.read_dword(adv, "TaskbarDa").unwrap(), Some(0));
    assert_eq!(
        win11
            .read_dword(
                r"Software\Microsoft\Windows\CurrentVersion\Feeds",
                "ShellFeedsTaskbarViewMode"
            )
            .unwrap(),
        None
    );

    let win10 = fake_on(WIN10_BUILD, false);
    set_tweak(&win10, "widgets", true).unwrap();
    assert_eq!(win10.read_dword(adv, "TaskbarDa").unwrap(), None);
    assert!(state(&win10, "widgets").applied);
}

#[test]
fn unsupported_version_is_greyed_out_with_a_reason_and_never_reports_success() {
    let win10 = fake_on(WIN10_BUILD, false);
    let copilot = state(&win10, "copilot");
    assert_eq!(copilot.availability, TweakAvailability::UnsupportedVersion);
    assert!(copilot.reason.as_deref().unwrap().contains("Windows 11"));
    assert!(matches!(
        set_tweak(&win10, "copilot", true),
        Err(CoreError::NotApplicable(_))
    ));
    assert!(!state(&win10, "copilot").applied);

    let recall_on_23h2 = state(&fake_on(WIN11_BUILD, false), "recall");
    assert_eq!(
        recall_on_23h2.availability,
        TweakAvailability::UnsupportedVersion
    );
    assert!(recall_on_23h2.reason.unwrap().contains("24H2"));
    let recall_on_24h2 = state(&fake_on(WIN11_24H2, false), "recall");
    assert_eq!(recall_on_24h2.availability, TweakAvailability::Available);

    let cortana = state(&fake_on(WIN11, false), "cortana");
    assert_eq!(cortana.availability, TweakAvailability::UnsupportedVersion);
    assert!(cortana.reason.unwrap().contains("Windows 10"));
}

#[test]
fn hklm_tweaks_need_administrator_and_do_not_write_without_it() {
    let reg = fake_on(WIN11_BUILD, false);
    let blur = state(&reg, "login_blur");
    assert_eq!(blur.availability, TweakAvailability::NeedsAdmin);
    assert!(blur.reason.unwrap().contains("администратора"));
    assert!(matches!(
        set_tweak(&reg, "login_blur", true),
        Err(CoreError::NeedsAdmin)
    ));
    assert!(reg.dwords.lock().unwrap().is_empty());

    let admin = fake_on(WIN11_BUILD, true);
    assert_eq!(
        state(&admin, "login_blur").availability,
        TweakAvailability::Available
    );
    set_tweak(&admin, "login_blur", true).unwrap();
    assert!(state(&admin, "login_blur").applied);
    set_tweak(&admin, "login_blur", false).unwrap();
    assert!(!state(&admin, "login_blur").applied);
}

#[test]
fn reverting_a_group_without_administrator_skips_machine_wide_values_and_says_so() {
    let admin = fake_on(WIN11_BUILD, true);
    set_tweak(&admin, "login_blur", true).unwrap();
    set_tweak(&admin, "transparency", true).unwrap();
    let user = Fake {
        dwords: Mutex::new(admin.dwords.lock().unwrap().clone()),
        strings: Mutex::new(admin.strings.lock().unwrap().clone()),
        ..fake_on(WIN11_BUILD, false)
    };
    assert!(matches!(
        revert_group(&user, Group::Settings),
        Err(CoreError::NeedsAdmin)
    ));
    assert!(!state(&user, "transparency").has_backup);
    assert!(state(&user, "login_blur").has_backup);
}

#[test]
fn gaming_profile_switches_power_and_revert_restores_the_old_scheme() {
    let reg = fake();
    set_boost_profile(&reg, Some("gaming")).unwrap();
    assert_eq!(*reg.scheme.lock().unwrap(), HIGH_SCHEME);
    let boost = overview(&reg).unwrap().boost;
    assert_eq!(boost.profile.as_deref(), Some("gaming"));
    assert!(boost.has_backup);

    set_boost_profile(&reg, None).unwrap();
    assert_eq!(*reg.scheme.lock().unwrap(), BALANCED);
    let boost = overview(&reg).unwrap().boost;
    assert_eq!(boost.profile, None);
    assert!(!boost.has_backup);
    assert!(!state(&reg, "game_dvr").applied);
}

#[test]
fn switching_profiles_keeps_the_very_first_scheme() {
    let reg = fake();
    set_boost_profile(&reg, Some("gaming")).unwrap();
    set_boost_profile(&reg, Some("max")).unwrap();
    set_boost_profile(&reg, Some("basic")).unwrap();
    // Базовый профиль не меняет схему питания: прежняя схема возвращается.
    assert_eq!(*reg.scheme.lock().unwrap(), BALANCED);
    assert_eq!(
        overview(&reg).unwrap().boost.profile.as_deref(),
        Some("basic")
    );
}

#[test]
fn failed_power_change_leaves_registry_untouched() {
    let reg = fake();
    *reg.fail_power.lock().unwrap() = true;
    assert!(set_boost_profile(&reg, Some("max")).is_err());
    assert!(!state(&reg, "game_dvr").applied);
    assert_eq!(overview(&reg).unwrap().boost.profile, None);
}

#[test]
fn profile_is_not_reported_when_the_power_scheme_was_changed_by_the_user() {
    let reg = fake();
    set_boost_profile(&reg, Some("gaming")).unwrap();
    *reg.scheme.lock().unwrap() = BALANCED.to_string();
    assert_eq!(overview(&reg).unwrap().boost.profile, None);
}

#[test]
fn ultimate_performance_is_a_separate_switch_that_remembers_the_old_scheme() {
    let reg = fake();
    set_tweak(&reg, "ultimate", true).unwrap();
    assert_eq!(*reg.scheme.lock().unwrap(), ULTIMATE_SCHEME);
    let now = state(&reg, "ultimate");
    assert!(now.applied && now.has_backup);
    // Профили при этом не включены: это отдельная оптимизация.
    assert_eq!(overview(&reg).unwrap().boost.profile, None);
    set_tweak(&reg, "ultimate", false).unwrap();
    assert_eq!(*reg.scheme.lock().unwrap(), BALANCED);
    assert!(!state(&reg, "ultimate").applied);
}

#[test]
fn ultimate_is_not_reported_when_powercfg_refuses() {
    let reg = fake();
    *reg.fail_power.lock().unwrap() = true;
    assert!(set_tweak(&reg, "ultimate", true).is_err());
    assert!(!state(&reg, "ultimate").applied);
}

#[test]
fn tampered_power_backup_is_never_passed_to_powercfg() {
    let reg = fake();
    reg.write_string(BOOST_KEY, POWER_VALUE, "x /duplicatescheme y")
        .unwrap();
    revert_group(&reg, Group::Boost).unwrap();
    assert_eq!(*reg.scheme.lock().unwrap(), BALANCED);
}

#[test]
fn vbs_is_dangerous_and_reboots_the_pc_and_it_is_the_only_dangerous_one() {
    let dangerous: Vec<_> = all_ids()
        .into_iter()
        .filter(|id| is_dangerous(id))
        .collect();
    assert_eq!(dangerous, vec!["vbs_off"]);
    let admin = fake_on(WIN11_BUILD, true);
    let vbs = state(&admin, "vbs_off");
    assert!(vbs.dangerous);
    assert_eq!(vbs.restart, "reboot");
    set_tweak(&admin, "vbs_off", true).unwrap();
    set_tweak(&admin, "vbs_off", false).unwrap();
    assert!(!state(&admin, "vbs_off").has_backup);
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
    assert!(set_boost_profile(&reg, Some("turbo")).is_err());
}

#[test]
fn activation_output_is_parsed_without_depending_on_language() {
    assert_eq!(parse_activation("1;0\n").status, ActivationStatus::Licensed);
    assert_eq!(
        parse_activation("0;0").status,
        ActivationStatus::NotActivated
    );
    let grace = parse_activation("2;14400");
    assert_eq!(grace.status, ActivationStatus::Grace);
    assert_eq!(grace.grace_days, Some(10));
    assert_eq!(
        parse_activation("5;0").status,
        ActivationStatus::Notification
    );
    assert_eq!(parse_activation("none").status, ActivationStatus::Unknown);
    assert_eq!(parse_activation("").status, ActivationStatus::Unknown);
}

#[test]
fn overview_serializes_with_camel_case_keys() {
    let json = serde_json::to_value(overview(&fake()).unwrap()).unwrap();
    assert!(json["tweaks"][0]["hasBackup"].is_boolean());
    assert!(json["tweaks"][0]["availability"].is_string());
    assert!(json["boost"]["profile"].is_null());
    assert!(json["elevated"].is_boolean());
}

fn repo_file(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

#[test]
fn every_tweak_has_a_text_in_the_interface_and_a_row_in_the_keys_table() {
    let texts = repo_file("../../apps/desktop/src/lib/tweaksText.ts");
    let table = repo_file("../../docs/status/settings-keys.md");
    let mut seen = std::collections::BTreeSet::new();
    for id in all_ids() {
        assert!(seen.insert(id), "повторный id {id}");
        assert!(
            texts.contains(&format!("id: '{id}'")),
            "нет текста для {id}"
        );
        assert!(
            table.contains(&format!("`{id}`")),
            "нет строки в таблице для {id}"
        );
    }
}
