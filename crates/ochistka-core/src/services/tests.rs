use super::*;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Поддельная система: службы с состоянием и типом запуска, `sc` отвечает текстом как настоящий.
struct Fake {
    elevated: bool,
    services: Mutex<BTreeMap<String, (ServiceState, Option<u32>, bool)>>,
    saved: Mutex<BTreeMap<String, u32>>,
    calls: Mutex<Vec<Vec<String>>>,
    /// Код, которым `sc` отвечает на start/stop/config (0 — успех).
    fail_code: Mutex<i32>,
    /// Если true, `sc start` не меняет состояние (служба «зависла»).
    stuck: Mutex<bool>,
    /// Если true, `sc config` ничего не записывает, но отвечает успехом.
    config_noop: Mutex<bool>,
}

impl Fake {
    fn new(elevated: bool) -> Self {
        let mut map = BTreeMap::new();
        map.insert("Spooler".into(), (ServiceState::Running, Some(2), false));
        map.insert("Fax".into(), (ServiceState::Stopped, Some(3), false));
        map.insert("RpcSs".into(), (ServiceState::Running, Some(2), false));
        map.insert("Off".into(), (ServiceState::Stopped, Some(4), false));
        map.insert("Late".into(), (ServiceState::Running, Some(2), true));
        Self {
            elevated,
            services: Mutex::new(map),
            saved: Mutex::new(BTreeMap::new()),
            calls: Mutex::new(Vec::new()),
            fail_code: Mutex::new(0),
            stuck: Mutex::new(false),
            config_noop: Mutex::new(false),
        }
    }

    fn state(&self, name: &str) -> ServiceState {
        self.services.lock().unwrap()[name].0
    }

    fn sc_calls(&self) -> Vec<Vec<String>> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c[0] != "query")
            .cloned()
            .collect()
    }
}

fn state_text(state: ServiceState) -> (u32, &'static str) {
    match state {
        ServiceState::Stopped => (1, "STOPPED"),
        ServiceState::StartPending => (2, "START_PENDING"),
        ServiceState::Running => (4, "RUNNING"),
        _ => (1, "STOPPED"),
    }
}

impl ServiceBackend for Fake {
    fn registry_entries(&self) -> Result<Vec<RawService>, ServiceError> {
        Ok(self
            .services
            .lock()
            .unwrap()
            .iter()
            .map(|(name, (_, start, delayed))| RawService {
                name: name.clone(),
                display_name: format!("{name} display"),
                start: *start,
                delayed: *delayed,
            })
            .collect())
    }

    fn sc(&self, args: &[&str]) -> Result<ScOutput, ServiceError> {
        self.calls
            .lock()
            .unwrap()
            .push(args.iter().map(|a| (*a).to_string()).collect());
        let fail = *self.fail_code.lock().unwrap();
        let ok = |stdout: String| Ok(ScOutput { code: 0, stdout });
        match args[0] {
            "query" => {
                let map = self.services.lock().unwrap();
                let mut out = String::new();
                for (name, (state, _, _)) in map.iter() {
                    if args.len() == 2 && args[1] != name {
                        continue;
                    }
                    let (code, word) = state_text(*state);
                    // Русская локализация: названия строк переведены, слова состояния нет.
                    out.push_str(&format!(
                        "ИМЯ_СЛУЖБЫ: {name}\nОТОБРАЖАЕМОЕ_ИМЯ: {name} display\n        ТИП                : 20  WIN32_SHARE_PROCESS\n        СОСТОЯНИЕ          : {code}  {word}\n                                (STOPPABLE, NOT_PAUSABLE, ACCEPTS_SHUTDOWN)\n        КОД_ВЫХОДА_WIN32   : 0  (0x0)\n\n"
                    ));
                }
                ok(out)
            }
            "start" | "stop" | "config" if fail != 0 => Ok(ScOutput {
                code: fail,
                stdout: "ОШИБКА".into(),
            }),
            "start" => {
                if !*self.stuck.lock().unwrap() {
                    self.services.lock().unwrap().get_mut(args[1]).unwrap().0 =
                        ServiceState::Running;
                }
                ok(String::new())
            }
            "stop" => {
                self.services.lock().unwrap().get_mut(args[1]).unwrap().0 = ServiceState::Stopped;
                ok(String::new())
            }
            "config" => {
                if !*self.config_noop.lock().unwrap() {
                    let (start, delayed) = match args[3] {
                        "auto" => (2, false),
                        "delayed-auto" => (2, true),
                        "demand" => (3, false),
                        "disabled" => (4, false),
                        other => panic!("неизвестный тип {other}"),
                    };
                    let mut map = self.services.lock().unwrap();
                    let entry = map.get_mut(args[1]).unwrap();
                    entry.1 = Some(start);
                    entry.2 = delayed;
                }
                ok(String::new())
            }
            other => panic!("неожиданный вызов sc {other}"),
        }
    }

    fn is_elevated(&self) -> bool {
        self.elevated
    }

    fn saved_start(&self, name: &str) -> Option<u32> {
        self.saved.lock().unwrap().get(name).copied()
    }

    fn save_start(&self, name: &str, code: u32) -> Result<(), ServiceError> {
        self.saved.lock().unwrap().insert(name.into(), code);
        Ok(())
    }

    fn clear_saved_start(&self, name: &str) {
        self.saved.lock().unwrap().remove(name);
    }
}

#[test]
fn state_codes_map_to_states() {
    assert_eq!(ServiceState::from_code(4), ServiceState::Running);
    assert_eq!(ServiceState::from_code(1), ServiceState::Stopped);
    assert_eq!(ServiceState::from_code(2), ServiceState::StartPending);
    assert_eq!(ServiceState::from_code(99), ServiceState::Unknown);
}

#[test]
fn start_types_come_from_registry_value_and_delay_flag() {
    assert_eq!(StartType::from_registry(Some(2), false), StartType::Auto);
    assert_eq!(
        StartType::from_registry(Some(2), true),
        StartType::AutoDelayed
    );
    assert_eq!(StartType::from_registry(Some(3), false), StartType::Manual);
    assert_eq!(
        StartType::from_registry(Some(4), false),
        StartType::Disabled
    );
    assert_eq!(StartType::from_registry(None, false), StartType::Unknown);
    assert_eq!(
        StartType::from_code(StartType::AutoDelayed.code().unwrap()),
        StartType::AutoDelayed
    );
    assert_eq!(StartType::from_code(3), StartType::Manual);
}

#[test]
fn sc_query_is_parsed_with_english_and_russian_labels() {
    let english = "SERVICE_NAME: Spooler\nDISPLAY_NAME: Print Spooler\n        TYPE               : 110  WIN32_OWN_PROCESS  (interactive)\n        STATE              : 4  RUNNING\n                                (STOPPABLE, NOT_PAUSABLE, ACCEPTS_SHUTDOWN)\n        WIN32_EXIT_CODE    : 0  (0x0)\n\nSERVICE_NAME: Fax\nDISPLAY_NAME: Fax\n        TYPE               : 10  WIN32_OWN_PROCESS\n        STATE              : 1  STOPPED\n";
    assert_eq!(
        parse_sc_query(english),
        vec![
            ("Spooler".to_string(), ServiceState::Running),
            ("Fax".to_string(), ServiceState::Stopped)
        ]
    );
    let russian = "ИМЯ_СЛУЖБЫ: wuauserv\nОТОБРАЖАЕМОЕ_ИМЯ: Центр обновления Windows\n        ТИП                : 30  WIN32\n        СОСТОЯНИЕ          : 2  START_PENDING\n";
    assert_eq!(
        parse_sc_query(russian),
        vec![("wuauserv".to_string(), ServiceState::StartPending)]
    );
}

#[test]
fn sc_error_lines_and_garbage_are_ignored() {
    assert!(parse_sc_query(
        "[SC] EnumQueryServicesStatus:OpenService FAILED 5:\n\nОтказано в доступе.\n"
    )
    .is_empty());
    assert!(parse_sc_query("").is_empty());
}

#[test]
fn sc_exit_codes_become_human_errors() {
    assert_eq!(sc_error(5, "x"), ServiceError::NeedsAdmin);
    assert!(matches!(sc_error(1058, "x"), ServiceError::Refused(m) if m.contains("отключена")));
    assert!(matches!(sc_error(1051, "x"), ServiceError::Refused(m) if m.contains("зависят")));
    assert!(matches!(sc_error(1, "запустить"), ServiceError::Failed(m) if m.contains("код 1")));
}

#[test]
fn guard_denylist_locks_critical_and_marks_careful() {
    assert_eq!(guard_for("RpcSs").0, Guard::Locked);
    assert_eq!(guard_for("WinDefend").0, Guard::Locked);
    assert_eq!(guard_for("mpssvc").0, Guard::Locked);
    let (guard, note) = guard_for("Spooler");
    assert_eq!(guard, Guard::Careful);
    assert!(note.unwrap().contains("печать"));
    assert_eq!(guard_for("SomeVendorService").0, Guard::Free);
}

#[test]
fn names_with_option_like_or_odd_characters_are_refused() {
    assert!(validate_name("Spooler").is_ok());
    assert!(validate_name("CDPUserSvc_1a2b3").is_ok());
    for bad in ["", "-x", "/?", "a b", "a;b", "a\"b", "a&b", ".."] {
        assert_eq!(validate_name(bad), Err(ServiceError::InvalidName), "{bad}");
    }
}

#[test]
fn list_merges_registry_states_and_guards() {
    let fake = Fake::new(true);
    let snapshot = list_services(&fake).unwrap();
    assert!(snapshot.elevated);
    let spooler = snapshot
        .services
        .iter()
        .find(|s| s.name == "Spooler")
        .unwrap();
    assert_eq!(spooler.state, ServiceState::Running);
    assert_eq!(spooler.start_type, StartType::Auto);
    assert_eq!(spooler.guard, Guard::Careful);
    let rpc = snapshot
        .services
        .iter()
        .find(|s| s.name == "RpcSs")
        .unwrap();
    assert_eq!(rpc.guard, Guard::Locked);
    let late = snapshot.services.iter().find(|s| s.name == "Late").unwrap();
    assert_eq!(late.start_type, StartType::AutoDelayed);
    let names: Vec<_> = snapshot
        .services
        .iter()
        .map(|s| s.name.to_lowercase())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}

#[test]
fn start_runs_sc_and_reports_actual_state() {
    let fake = Fake::new(true);
    let info = apply(&fake, "Fax", ServiceAction::Start, false).unwrap();
    assert_eq!(info.state, ServiceState::Running);
    assert_eq!(
        fake.sc_calls(),
        vec![vec!["start".to_string(), "Fax".to_string()]]
    );
}

#[test]
fn start_of_running_service_does_not_call_sc() {
    let fake = Fake::new(true);
    let info = apply(&fake, "Late", ServiceAction::Start, false).unwrap();
    assert_eq!(info.state, ServiceState::Running);
    assert!(fake.sc_calls().is_empty());
}

#[test]
fn start_of_disabled_service_explains_what_to_do() {
    let fake = Fake::new(true);
    let err = apply(&fake, "Off", ServiceAction::Start, false).unwrap_err();
    assert!(matches!(err, ServiceError::Refused(m) if m.contains("Вручную")));
    assert!(fake.sc_calls().is_empty());
}

#[test]
fn stuck_start_is_reported_not_faked() {
    let fake = Fake::new(true);
    *fake.stuck.lock().unwrap() = true;
    let err = apply(&fake, "Fax", ServiceAction::Start, false).unwrap_err();
    assert!(matches!(err, ServiceError::Failed(m) if m.contains("не запустилась")));
    assert_eq!(fake.state("Fax"), ServiceState::Stopped);
}

#[test]
fn locked_service_is_never_touched() {
    let fake = Fake::new(true);
    for action in [
        ServiceAction::Stop,
        ServiceAction::Start,
        ServiceAction::SetStart(RequestedStart::Disabled),
        ServiceAction::RestoreStart,
    ] {
        let err = apply(&fake, "RpcSs", action, true).unwrap_err();
        assert!(matches!(err, ServiceError::Protected(_)), "{action:?}");
    }
    assert!(fake.sc_calls().is_empty());
}

#[test]
fn careful_service_needs_confirmation_with_consequences() {
    let fake = Fake::new(true);
    let err = apply(&fake, "Spooler", ServiceAction::Stop, false).unwrap_err();
    assert!(matches!(err, ServiceError::NotConfirmed(m) if m.contains("печать")));
    let err = apply(
        &fake,
        "Spooler",
        ServiceAction::SetStart(RequestedStart::Disabled),
        false,
    )
    .unwrap_err();
    assert!(matches!(err, ServiceError::NotConfirmed(_)));
    assert!(fake.sc_calls().is_empty());
    let info = apply(&fake, "Spooler", ServiceAction::Stop, true).unwrap();
    assert_eq!(info.state, ServiceState::Stopped);
}

#[test]
fn switching_careful_service_to_manual_needs_no_confirmation() {
    let fake = Fake::new(true);
    let info = apply(
        &fake,
        "Spooler",
        ServiceAction::SetStart(RequestedStart::Manual),
        false,
    )
    .unwrap();
    assert_eq!(info.start_type, StartType::Manual);
}

#[test]
fn without_admin_rights_nothing_is_called() {
    let fake = Fake::new(false);
    let err = apply(&fake, "Fax", ServiceAction::Start, false).unwrap_err();
    assert_eq!(err, ServiceError::NeedsAdmin);
    assert!(fake.sc_calls().is_empty());
    assert!(!list_services(&fake).unwrap().elevated);
}

#[test]
fn unknown_service_and_bad_name_are_refused() {
    let fake = Fake::new(true);
    assert!(matches!(
        apply(&fake, "Nope", ServiceAction::Start, false),
        Err(ServiceError::NotFound(_))
    ));
    assert_eq!(
        apply(&fake, "/?", ServiceAction::Start, false),
        Err(ServiceError::InvalidName)
    );
}

#[test]
fn start_type_change_saves_previous_value_and_verifies() {
    let fake = Fake::new(true);
    let info = apply(
        &fake,
        "Fax",
        ServiceAction::SetStart(RequestedStart::Auto),
        false,
    )
    .unwrap();
    assert_eq!(info.start_type, StartType::Auto);
    assert_eq!(info.previous_start_type, Some(StartType::Manual));
    assert_eq!(
        fake.sc_calls(),
        vec![["config", "Fax", "start=", "auto"]
            .map(String::from)
            .to_vec()]
    );
    // Второе изменение не затирает самое первое значение.
    let info = apply(
        &fake,
        "Fax",
        ServiceAction::SetStart(RequestedStart::Disabled),
        false,
    )
    .unwrap();
    assert_eq!(info.start_type, StartType::Disabled);
    assert_eq!(info.previous_start_type, Some(StartType::Manual));
}

#[test]
fn restore_returns_saved_start_type_and_clears_it() {
    let fake = Fake::new(true);
    apply(
        &fake,
        "Late",
        ServiceAction::SetStart(RequestedStart::Disabled),
        false,
    )
    .unwrap();
    let info = apply(&fake, "Late", ServiceAction::RestoreStart, false).unwrap();
    assert_eq!(info.start_type, StartType::AutoDelayed);
    assert_eq!(info.previous_start_type, None);
    assert_eq!(
        apply(&fake, "Late", ServiceAction::RestoreStart, false),
        Err(ServiceError::NoSavedStart)
    );
}

#[test]
fn failed_config_does_not_leave_a_saved_value() {
    let fake = Fake::new(true);
    *fake.fail_code.lock().unwrap() = 5;
    let err = apply(
        &fake,
        "Fax",
        ServiceAction::SetStart(RequestedStart::Auto),
        false,
    )
    .unwrap_err();
    assert_eq!(err, ServiceError::NeedsAdmin);
    assert_eq!(fake.saved_start("Fax"), None);
}

#[test]
fn silent_config_that_changes_nothing_is_an_error() {
    let fake = Fake::new(true);
    *fake.config_noop.lock().unwrap() = true;
    let err = apply(
        &fake,
        "Fax",
        ServiceAction::SetStart(RequestedStart::Auto),
        false,
    )
    .unwrap_err();
    assert!(matches!(err, ServiceError::Failed(m) if m.contains("не изменился")));
    assert_eq!(fake.saved_start("Fax"), None);
}

#[test]
fn same_start_type_is_a_no_op() {
    let fake = Fake::new(true);
    apply(
        &fake,
        "Fax",
        ServiceAction::SetStart(RequestedStart::Manual),
        false,
    )
    .unwrap();
    assert!(fake.sc_calls().is_empty());
    assert_eq!(fake.saved_start("Fax"), None);
}
