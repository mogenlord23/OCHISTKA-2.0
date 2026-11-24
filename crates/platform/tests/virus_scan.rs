//! Проверка папок через порт движка на настоящей файловой системе (временная папка).
//! Движок — либо заглушка `ClamAvEngine::not_built`, либо фейк, который реализует `VirusScanPort`.
//! Строка EICAR — стандартный тестовый файл антивирусов (68 байт), она не является вредоносной.
//! На диск EICAR не пишется: Defender удаляет такой файл, поэтому тесты используют собственный
//! безвредный образец `MARKER`, и движок проверяется по-настоящему.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use ochistka_core::fs::Limits;
use ochistka_core::scan::{
    sweep, EngineStatus, FileVerdict, ScanReport, VirusScanPort, MAX_FILE_BYTES,
};
use ochistka_core::system::CoreError;
use ochistka_platform::windows::clamav::ClamAvEngine;
use ochistka_platform::windows_cleanup::WindowsFs;

#[cfg(all(windows, feature = "libclamav"))]
#[test]
#[ignore = "requires a verified official CVD in OCHISTKA_OFFICIAL_CLAMDB"]
fn embedded_clamav_scans_with_official_database() {
    let database = std::env::var("OCHISTKA_OFFICIAL_CLAMDB")
        .expect("provide the isolated directory containing a verified official CVD");
    let scanner = ClamAvEngine::build();
    scanner
        .load_signatures(&database)
        .expect("load official ClamAV signatures in OCHISTKA");
    assert!(scanner.engine_status().available);
    let signatures = scanner.signature_count().expect("signature count");
    assert!(signatures > 0);

    let sample = temp_folder("official-clamav-clean");
    std::fs::write(
        sample.join("note.txt"),
        b"OCHISTKA official database smoke test",
    )
    .unwrap();
    let report = ochistka_core::scan::sweep_computer(
        &WindowsFs,
        &scanner,
        &[as_target(&sample)],
        &no_cancel(),
        &mut |_| {},
    )
    .unwrap();
    std::fs::remove_dir_all(&sample).unwrap();
    assert_eq!(report.mode, ochistka_core::scan::ScanMode::Signatures);
    assert_eq!(report.scanned_files, 1);
    assert_eq!(report.infected_files, 0);
    assert!(!report.incomplete);
    println!("official database: {signatures} loaded signatures; clean test file scanned");
}

const EICAR: &[u8] = b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*";

/// Собственный безвредный образец для тестов на диске.
const MARKER: &[u8] = b"OCHISTKA-CLAMAV-TEST-MARKER-0123456789ABCDEF";

/// Фейковый движок: видит MARKER по содержимому; файлы с именем `damaged*` дают ошибку чтения.
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
        if bytes.windows(MARKER.len()).any(|window| window == MARKER) {
            return Ok(FileVerdict::Found {
                signature: "OCHISTKA.Test".into(),
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
fn stub_engine_reports_not_built_for_a_marker_file() {
    let root = temp_folder("marker-stub");
    std::fs::write(root.join("sample.bin"), MARKER).unwrap();

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
fn engine_that_detects_the_marker_reports_a_finding() {
    let root = temp_folder("marker-fake");
    std::fs::write(root.join("sample.bin"), MARKER).unwrap();
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
    assert!(report.findings[0].path.ends_with("sample.bin"));
    assert_eq!(report.findings[0].signature, "OCHISTKA.Test");
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
    std::fs::write(root.join("sample.bin"), MARKER).unwrap();

    let report = sweep(
        &WindowsFs,
        &FakeEngine::default(),
        &as_target(&root),
        &no_cancel(),
        Limits::default(),
    )
    .unwrap();
    std::fs::remove_dir_all(&root).ok();

    assert_eq!(report.scanned_files, 2, "empty file and marker are scanned");
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
fn embedded_clamav_loads_a_test_base_and_detects_the_marker() {
    let root = temp_folder("marker-clamav");
    let signatures = root.join("signatures");
    std::fs::create_dir(&signatures).unwrap();
    let hex: String = MARKER.iter().map(|byte| format!("{byte:02x}")).collect();
    std::fs::write(
        signatures.join("test.ndb"),
        format!("OCHISTKA.Test:0:*:{hex}\n"),
    )
    .unwrap();
    let sample = root.join("sample.bin");
    let clean = root.join("чистый.txt");
    let compressed = root.join("sample.txt.gz");
    std::fs::write(&sample, MARKER).unwrap();
    std::fs::write(&clean, b"clean test file").unwrap();
    // gzip с тем же безвредным маркером, сохранённым в сжатом виде.
    std::fs::write(
        &compressed,
        [
            0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x0a, 0xf3, 0x77, 0xf6, 0xf0,
            0x0c, 0x0e, 0xf1, 0x76, 0xd4, 0x75, 0xf6, 0x71, 0xf4, 0x75, 0x0c, 0xd3, 0x0d, 0x71,
            0x0d, 0x0e, 0xd1, 0xf5, 0x75, 0x0c, 0xf2, 0x76, 0x0d, 0xd2, 0x35, 0x30, 0x34, 0x32,
            0x36, 0x31, 0x35, 0x33, 0xb7, 0xb0, 0x74, 0x74, 0x72, 0x76, 0x71, 0x75, 0x03, 0x00,
            0x75, 0x76, 0x73, 0x79, 0x2c, 0x00, 0x00, 0x00,
        ],
    )
    .unwrap();
    let engine = ClamAvEngine::build();
    engine.load_signatures(&as_target(&signatures)).unwrap();
    assert!(engine.engine_status().available);
    assert!(matches!(
        engine.scan_file(&as_target(&sample)),
        Ok(FileVerdict::Found { .. })
    ));
    assert_eq!(
        engine.scan_file(&as_target(&clean)).unwrap(),
        FileVerdict::Clean
    );
    assert!(matches!(
        engine.scan_file(&as_target(&compressed)),
        Ok(FileVerdict::Found { .. })
    ));
    let corrupted = root.join("corrupted");
    std::fs::create_dir(&corrupted).unwrap();
    std::fs::write(
        corrupted.join("bad.ndb"),
        b"not a valid signature database\n",
    )
    .unwrap();
    assert!(engine.load_signatures(&as_target(&corrupted)).is_err());
    // Неудачная загрузка не ломает уже загруженную базу.
    assert!(matches!(
        engine.scan_file(&as_target(&sample)),
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

#[cfg(all(feature = "libclamav", target_os = "linux"))]
#[test]
fn embedded_scan_records_open_file_identity_and_detects_replacement() {
    use ochistka_platform::linux::file_identity::{identify, open_regular_no_links};
    use std::os::unix::fs::symlink;

    let root = temp_folder("identity-clamav");
    let signatures = root.join("signatures");
    let samples = root.join("samples");
    std::fs::create_dir(&signatures).unwrap();
    std::fs::create_dir(&samples).unwrap();
    std::fs::write(
        signatures.join("test.hdb"),
        b"44d88612fea8a8f36de82e1278abb02f:68:OCHISTKA.EICAR.Test\n",
    )
    .unwrap();
    let path = samples.join("eicar.txt");
    std::fs::write(&path, EICAR).unwrap();
    let engine = ClamAvEngine::build();
    engine.load_signatures(&as_target(&signatures)).unwrap();
    let report = sweep(
        &WindowsFs,
        &engine,
        &as_target(&samples),
        &no_cancel(),
        Limits::default(),
    )
    .unwrap();
    let identity = report.findings[0]
        .identity
        .as_ref()
        .expect("scan must carry identity");
    assert_eq!(identity.sha256, ochistka_core::fs::sha256_hex(EICAR));
    std::fs::rename(&path, root.join("old")).unwrap();
    std::fs::write(&path, EICAR).unwrap();
    assert_ne!(
        *identity,
        identify(&open_regular_no_links(&path).unwrap()).unwrap()
    );
    std::fs::remove_file(&path).unwrap();
    symlink(root.join("old"), &path).unwrap();
    assert!(engine.scan_file_with_identity(&as_target(&path)).is_err());
    std::fs::remove_dir_all(&root).unwrap();
}
