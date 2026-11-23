//! Проверка папок через порт движка на настоящей файловой системе (временная папка).
//! Движок — либо заглушка `ClamAvEngine::not_built`, либо фейк, который реализует `VirusScanPort`.
//! Строка EICAR — стандартный тестовый файл антивирусов (68 байт), она не является вредоносной.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use ochistka_core::fs::Limits;
use ochistka_core::scan::{
    sweep, EngineStatus, FileVerdict, ScanReport, VirusScanPort, MAX_FILE_BYTES,
};
use ochistka_core::system::CoreError;
use ochistka_platform::windows::clamav::ClamAvEngine;
use ochistka_platform::windows_cleanup::WindowsFs;

const EICAR: &[u8] = b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*";

/// Фейковый движок: видит EICAR по содержимому; файлы с именем `damaged*` дают ошибку чтения.
#[derive(Default)]
struct FakeEngine {
    calls: AtomicU32,
}

impl VirusScanPort for FakeEngine {
    fn engine_status(&self) -> EngineStatus {
        EngineStatus {
            available: true,
            reason: String::new(),
        }
    }

    fn scan_file(&self, path: &str) -> Result<FileVerdict, CoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if Path::new(path)
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("damaged"))
        {
            return Err(CoreError::Probe("файл повреждён".into()));
        }
        let bytes = std::fs::read(path).map_err(|err| CoreError::Probe(err.to_string()))?;
        if bytes.windows(EICAR.len()).any(|window| window == EICAR) {
            return Ok(FileVerdict::Found {
                signature: "Eicar-Test-Signature".into(),
            });
        }
        Ok(FileVerdict::Clean)
    }

    fn load_signatures(&self, _dir: &str) -> Result<(), CoreError> {
        Ok(())
    }
}

/// Своя временная папка на тест; старые остатки удаляются.
fn temp_folder(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("ochistka-scan-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn no_cancel() -> AtomicBool {
    AtomicBool::new(false)
}

fn as_target(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn eicar_string_is_the_standard_68_byte_test_file() {
    assert_eq!(EICAR.len(), 68);
}

#[test]
fn eicar_with_stub_engine_reports_engine_not_built() {
    let root = temp_folder("eicar-stub");
    std::fs::write(root.join("eicar.com"), EICAR).unwrap();

    let err = sweep(
        &WindowsFs,
        &ClamAvEngine::not_built(),
        &as_target(&root),
        &no_cancel(),
        Limits::default(),
    )
    .unwrap_err();
    std::fs::remove_dir_all(&root).ok();

    assert!(matches!(err, CoreError::EngineMissing(ref m) if m.contains("движок не собран")));
}

#[test]
fn eicar_with_engine_that_detects_it_is_reported_as_finding() {
    let root = temp_folder("eicar-fake");
    std::fs::write(root.join("eicar.com"), EICAR).unwrap();
    std::fs::write(root.join("notes.txt"), b"plain text").unwrap();
    let engine = FakeEngine::default();

    let report = sweep(
        &WindowsFs,
        &engine,
        &as_target(&root),
        &no_cancel(),
        Limits::default(),
    )
    .unwrap();
    std::fs::remove_dir_all(&root).ok();

    assert_eq!(report.scanned_files, 2);
    assert_eq!(report.infected_files, 1);
    assert_eq!(report.findings.len(), 1);
    assert!(report.findings[0].path.ends_with("eicar.com"));
    assert_eq!(report.findings[0].signature, "Eicar-Test-Signature");
    assert_eq!(engine.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn empty_folder_gives_empty_report() {
    let root = temp_folder("empty");
    let report = sweep(
        &WindowsFs,
        &FakeEngine::default(),
        &as_target(&root),
        &no_cancel(),
        Limits::default(),
    )
    .unwrap();
    std::fs::remove_dir_all(&root).ok();
    assert_eq!(report, ScanReport::default());
}

#[test]
fn zero_byte_and_damaged_files_do_not_stop_the_sweep() {
    let root = temp_folder("damaged");
    std::fs::write(root.join("empty.bin"), b"").unwrap();
    std::fs::write(root.join("damaged.zip"), b"PK\x03").unwrap();
    std::fs::write(root.join("eicar.com"), EICAR).unwrap();

    let report = sweep(
        &WindowsFs,
        &FakeEngine::default(),
        &as_target(&root),
        &no_cancel(),
        Limits::default(),
    )
    .unwrap();
    std::fs::remove_dir_all(&root).ok();

    assert_eq!(report.scanned_files, 2, "empty file and EICAR are scanned");
    assert_eq!(report.errors, 1, "damaged file is counted, not fatal");
    assert_eq!(report.infected_files, 1);
}

#[test]
fn files_over_200_mb_are_skipped_and_not_sent_to_engine() {
    let root = temp_folder("size-limit");
    // Разреженный файл: на диске почти ничего не занимает, но размер больше лимита.
    let big = std::fs::File::create(root.join("big.iso")).unwrap();
    big.set_len(MAX_FILE_BYTES + 1).unwrap();
    drop(big);
    std::fs::write(root.join("small.txt"), b"ok").unwrap();
    let engine = FakeEngine::default();

    let report = sweep(
        &WindowsFs,
        &engine,
        &as_target(&root),
        &no_cancel(),
        Limits::default(),
    )
    .unwrap();
    std::fs::remove_dir_all(&root).ok();

    assert_eq!(report.skipped_files, 1);
    assert_eq!(report.scanned_files, 1);
    assert_eq!(engine.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn no_signatures_folder_reports_the_reason() {
    let root = temp_folder("no-signatures");
    let missing = root.join("signatures");
    let err = ClamAvEngine::not_built()
        .load_signatures(&as_target(&missing))
        .unwrap_err();
    std::fs::remove_dir_all(&root).ok();
    assert!(matches!(err, CoreError::Probe(ref m) if m.contains("папка сигнатур не найдена")));
}

#[test]
fn empty_signatures_folder_reports_the_reason() {
    let root = temp_folder("empty-signatures");
    let signatures = root.join("signatures");
    std::fs::create_dir_all(&signatures).unwrap();
    let err = ClamAvEngine::not_built()
        .load_signatures(&as_target(&signatures))
        .unwrap_err();
    std::fs::remove_dir_all(&root).ok();
    assert!(matches!(err, CoreError::Probe(ref m) if m.contains("папка сигнатур пуста")));
}

#[test]
fn cancel_before_sweep_stops_without_calling_engine() {
    let root = temp_folder("cancel");
    std::fs::write(root.join("eicar.com"), EICAR).unwrap();
    let engine = FakeEngine::default();
    let cancel = AtomicBool::new(true);

    let err = sweep(
        &WindowsFs,
        &engine,
        &as_target(&root),
        &cancel,
        Limits::default(),
    )
    .unwrap_err();
    std::fs::remove_dir_all(&root).ok();

    assert_eq!(err, CoreError::Cancelled);
    assert_eq!(engine.calls.load(Ordering::SeqCst), 0);
}

#[cfg(unix)]
#[test]
fn inaccessible_subfolder_marks_scan_incomplete() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_folder("unreadable");
    let restricted = root.join("restricted");
    std::fs::create_dir(&restricted).unwrap();
    std::fs::write(restricted.join("file.txt"), b"not checked").unwrap();
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read_dir(&restricted).is_ok() {
        std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::remove_dir_all(&root).ok();
        eprintln!("permission test skipped: current user can read the restricted directory");
        return;
    }
    let report = sweep(
        &WindowsFs,
        &FakeEngine::default(),
        &as_target(&root),
        &no_cancel(),
        Limits::default(),
    );
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir_all(&root).ok();
    let report = report.unwrap();
    assert!(
        report.incomplete,
        "unreadable folder must not produce a complete report"
    );
    assert_eq!(report.scanned_files, 0);
}

#[cfg(feature = "libclamav")]
#[test]
fn embedded_clamav_loads_test_signature_and_detects_eicar() {
    let root = temp_folder("eicar-clamav");
    let signatures = root.join("signatures");
    std::fs::create_dir(&signatures).unwrap();
    std::fs::write(
        signatures.join("test.hdb"),
        b"44d88612fea8a8f36de82e1278abb02f:68:OCHISTKA.EICAR.Test\n",
    )
    .unwrap();
    let eicar = root.join("eicar.txt");
    let clean = root.join("clean.txt");
    std::fs::write(&eicar, EICAR).unwrap();
    std::fs::write(&clean, b"clean test file").unwrap();
    let engine = ClamAvEngine::build();
    engine.load_signatures(&as_target(&signatures)).unwrap();
    assert!(engine.engine_status().available);
    assert!(matches!(
        engine.scan_file(&as_target(&eicar)),
        Ok(FileVerdict::Found { .. })
    ));
    assert_eq!(
        engine.scan_file(&as_target(&clean)).unwrap(),
        FileVerdict::Clean
    );
    let corrupted = root.join("corrupted");
    std::fs::create_dir(&corrupted).unwrap();
    std::fs::write(
        corrupted.join("bad.hdb"),
        b"not a valid signature database\n",
    )
    .unwrap();
    assert!(engine.load_signatures(&as_target(&corrupted)).is_err());
    assert!(matches!(
        engine.scan_file(&as_target(&eicar)),
        Ok(FileVerdict::Found { .. })
    ));

    std::fs::remove_dir_all(&root).ok();
}

#[cfg(feature = "libclamav")]
#[test]
fn embedded_clamav_stays_unavailable_without_signatures() {
    let root = temp_folder("missing-clamav-signatures");
    let engine = ClamAvEngine::build();
    assert!(!engine.engine_status().available);
    assert!(engine
        .load_signatures(&as_target(&root.join("missing")))
        .is_err());
    assert!(engine.load_signatures(&as_target(&root)).is_err());
    assert!(!engine.engine_status().available);
    assert!(matches!(
        engine.scan_file(&as_target(&root.join("file"))),
        Err(CoreError::EngineMissing(_))
    ));
    std::fs::remove_dir_all(&root).ok();
}
