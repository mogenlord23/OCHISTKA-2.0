//! Договор установщика службы VPN: что должно быть в скрипте и хуках (проверяется текстом, потому что
//! настоящая установка идёт только на Windows и проверяется в CI).

use std::path::PathBuf;

fn read(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join(relative)).unwrap();
    text.trim_start_matches('\u{feff}').to_string()
}

const SCRIPT: &str = "apps/desktop/src-tauri/resources/vpn-service/vpn-service.ps1";
const HOOKS: &str = "apps/desktop/src-tauri/windows/hooks.nsh";

#[test]
fn the_install_script_writes_every_step_to_the_install_log() {
    let s = read(SCRIPT);
    assert!(s.contains("$env:ProgramData") && s.contains("install.log"));
    assert!(s.contains("function Write-Log"));
    // Файлы, права, команды и ошибки попадают в журнал; при ошибке код выхода не нулевой.
    for needle in [
        "Права администратора",
        "sc sdset",
        "sc qc",
        "ОШИБКА",
        "exit 1",
    ] {
        assert!(s.contains(needle), "{needle}");
    }
}

#[test]
fn the_service_starts_only_by_hand_with_rights_for_users_on_that_service_only() {
    let s = read(SCRIPT);
    assert!(s.contains("-StartupType Manual"), "тип запуска «вручную»");
    assert!(!s.to_lowercase().contains("startuptype automatic"));
    assert!(!s.contains("start= auto"));
    assert!(s.contains(";;;AU)"), "права для обычных пользователей");
    assert!(s.contains("sdset"));
    // Системная программа — по абсолютному пути.
    assert!(s.contains("$env:SystemRoot 'System32\\sc.exe'"));
}

#[test]
fn the_hooks_log_the_result_and_never_stop_the_install_on_failure() {
    let h = read(HOOKS);
    assert!(h.contains("install.log") && h.contains("ProgramData"));
    assert!(h.contains("NSIS_HOOK_POSTINSTALL") && h.contains("NSIS_HOOK_PREUNINSTALL"));
    assert!(h.contains("-Action install") && h.contains("-Action uninstall"));
    assert!(h.contains("$WINDIR\\System32\\WindowsPowerShell"));
    // Ни прерывания установки, ни окон с вопросами при ошибке регистрации.
    assert!(!h.contains("Abort") && !h.contains("MessageBox"));
}

#[test]
fn the_installer_is_per_machine_and_bundles_the_service_and_the_core() {
    let conf = read("apps/desktop/src-tauri/tauri.conf.json");
    for needle in [
        "\"perMachine\"",
        "windows/hooks.nsh",
        "resources/vpn-service/*",
        "resources/vpn-core/*",
    ] {
        assert!(conf.contains(needle), "{needle}");
    }
}
