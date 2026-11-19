//! Закоммиченные TypeScript-типы (`apps/desktop/src/lib/generated`) должны совпадать с тем,
//! что порождают DTO на Rust. Расхождение роняет `cargo test`.
//! Обновить файлы: `OCHISTKA_BLESS=1 cargo test -p ochistka-core --test generated_types`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ochistka_core::assess::Suspicion;
use ochistka_core::hardware::{GpuInfo, HardwareOverview, MotherboardInfo};
use ochistka_core::netinfo::{GeoConsent, GeoInfo};
use ochistka_core::network::{NetworkAdapter, TcpConnection};
use ochistka_core::scan::{EngineStatus, Finding, ScanReport};
use ochistka_core::speedtest::{SpeedEvent, SpeedFailure, SpeedPhase, SpeedResult};
use ochistka_core::system::{CpuStatus, DiskKind, DiskStatus, MemoryStatus, SystemOverview};
use ochistka_core::tweaks::{
    ActivationInfo, ActivationStatus, BoostState, TweakAvailability, TweakState, TweaksOverview,
};
use ochistka_core::vpn::engine::PrepareProgress;
use ochistka_core::vpn::routes::RoutingRules;
use ochistka_core::vpn::service::{
    ConnState, ConnectionStatus, CoreAvailability, ImportSummary, PresetInfo, ServerView,
    VpnSnapshot,
};
use ts_rs::{Config, TS};

fn committed_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src/lib/generated")
}

fn read_dir_files(dir: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "ts") {
                let name = entry.file_name().to_string_lossy().into_owned();
                // Git на Windows может заменить переводы строк.
                let text = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
                files.insert(name, text);
            }
        }
    }
    files
}

fn export_all_into(dir: &Path) {
    let cfg = Config::new().with_out_dir(dir);
    SystemOverview::export_all(&cfg).unwrap();
    CpuStatus::export_all(&cfg).unwrap();
    MemoryStatus::export_all(&cfg).unwrap();
    DiskKind::export_all(&cfg).unwrap();
    DiskStatus::export_all(&cfg).unwrap();
    ScanReport::export_all(&cfg).unwrap();
    Finding::export_all(&cfg).unwrap();
    EngineStatus::export_all(&cfg).unwrap();
    Suspicion::export_all(&cfg).unwrap();
    GpuInfo::export_all(&cfg).unwrap();
    MotherboardInfo::export_all(&cfg).unwrap();
    HardwareOverview::export_all(&cfg).unwrap();
    NetworkAdapter::export_all(&cfg).unwrap();
    TcpConnection::export_all(&cfg).unwrap();
    GeoInfo::export_all(&cfg).unwrap();
    GeoConsent::export_all(&cfg).unwrap();
    TweaksOverview::export_all(&cfg).unwrap();
    TweakState::export_all(&cfg).unwrap();
    BoostState::export_all(&cfg).unwrap();
    TweakAvailability::export_all(&cfg).unwrap();
    ActivationInfo::export_all(&cfg).unwrap();
    ActivationStatus::export_all(&cfg).unwrap();
    VpnSnapshot::export_all(&cfg).unwrap();
    ServerView::export_all(&cfg).unwrap();
    ConnectionStatus::export_all(&cfg).unwrap();
    ConnState::export_all(&cfg).unwrap();
    CoreAvailability::export_all(&cfg).unwrap();
    PresetInfo::export_all(&cfg).unwrap();
    ImportSummary::export_all(&cfg).unwrap();
    RoutingRules::export_all(&cfg).unwrap();
    PrepareProgress::export_all(&cfg).unwrap();
    SpeedEvent::export_all(&cfg).unwrap();
    SpeedResult::export_all(&cfg).unwrap();
    SpeedPhase::export_all(&cfg).unwrap();
    SpeedFailure::export_all(&cfg).unwrap();
}

#[test]
fn committed_typescript_bindings_match_rust_dtos() {
    let scratch = std::env::temp_dir().join(format!("ochistka-ts-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    export_all_into(&scratch);
    let produced = read_dir_files(&scratch);
    let committed = committed_dir();

    if std::env::var("OCHISTKA_BLESS").is_ok_and(|v| v == "1") {
        fs::create_dir_all(&committed).unwrap();
        for (name, text) in &produced {
            fs::write(committed.join(name), text).unwrap();
        }
    }

    let on_disk = read_dir_files(&committed);
    let _ = fs::remove_dir_all(&scratch);
    assert_eq!(
        on_disk.keys().collect::<Vec<_>>(),
        produced.keys().collect::<Vec<_>>(),
        "набор файлов в apps/desktop/src/lib/generated разошёлся с DTO; запустите с OCHISTKA_BLESS=1"
    );
    for (name, text) in &produced {
        assert_eq!(
            &on_disk[name], text,
            "{name} устарел; запустите с OCHISTKA_BLESS=1"
        );
    }
}
