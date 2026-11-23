//! Синхронизированные исследования гонок, не реализация переноса карантина.
#![cfg(target_os = "linux")]

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::sync_channel;
use std::thread;

use ochistka_core::scan::FileVerdict;
use ochistka_platform::linux::file_identity::{
    identify, open_regular_no_links, open_verified_source, scan_open_file,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ochistka-race-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn replacing_name_after_verification_makes_path_rename_move_the_wrong_object() {
    let tmp = Temp::new();
    let source = tmp.0.join("source");
    let original = tmp.0.join("original");
    let destination = tmp.0.join("destination");
    let manifest = tmp.0.join("manifest");
    fs::write(&source, b"checked").unwrap();
    fs::write(&manifest, b"previous").unwrap();
    let identity = identify(&open_regular_no_links(&source).unwrap()).unwrap();
    let verified = open_verified_source(&source, &identity).unwrap();
    let (start, proceed) = sync_channel::<()>(0);
    let (finished, done) = sync_channel::<()>(0);
    thread::scope(|scope| {
        let source_ref = &source;
        let original_ref = &original;
        scope.spawn(move || {
            proceed.recv().unwrap();
            fs::rename(source_ref, original_ref).unwrap();
            fs::write(source_ref, b"replacement").unwrap();
            finished.send(()).unwrap();
        });
        start.send(()).unwrap();
        done.recv().unwrap();
        // Демонстрация небезопасной идеи в изолированной папке, не код приложения.
        fs::rename(&source, &destination).unwrap();
    });
    let mut held = &verified;
    held.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    held.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"checked");
    assert_eq!(fs::read(&original).unwrap(), b"checked");
    assert_eq!(fs::read(&destination).unwrap(), b"replacement");
    assert_eq!(fs::read(&manifest).unwrap(), b"previous");
}

#[test]
fn concurrent_replacement_before_open_is_refused_without_disk_mutations() {
    let tmp = Temp::new();
    let source = tmp.0.join("source");
    let original = tmp.0.join("original");
    let destination = tmp.0.join("destination");
    let manifest = tmp.0.join("manifest");
    fs::write(&source, b"checked").unwrap();
    fs::write(&destination, b"existing").unwrap();
    fs::write(&manifest, b"previous").unwrap();
    let identity = identify(&open_regular_no_links(&source).unwrap()).unwrap();
    let (start, proceed) = sync_channel::<()>(0);
    let (finished, done) = sync_channel::<()>(0);
    thread::scope(|scope| {
        let source_ref = &source;
        let original_ref = &original;
        scope.spawn(move || {
            proceed.recv().unwrap();
            fs::rename(source_ref, original_ref).unwrap();
            fs::write(source_ref, b"replacement").unwrap();
            finished.send(()).unwrap();
        });
        start.send(()).unwrap();
        done.recv().unwrap();
        assert!(open_verified_source(&source, &identity).is_err());
    });
    assert_eq!(fs::read(&source).unwrap(), b"replacement");
    assert_eq!(fs::read(&original).unwrap(), b"checked");
    assert_eq!(fs::read(&destination).unwrap(), b"existing");
    assert_eq!(fs::read(&manifest).unwrap(), b"previous");
}

#[test]
fn concurrent_content_change_during_scan_is_refused() {
    let tmp = Temp::new();
    let path = tmp.0.join("source");
    fs::write(&path, b"checked").unwrap();
    let file = open_regular_no_links(&path).unwrap();
    let (start, proceed) = sync_channel::<()>(0);
    let (finished, done) = sync_channel::<()>(0);
    thread::scope(|scope| {
        let path_ref = &path;
        scope.spawn(move || {
            proceed.recv().unwrap();
            fs::write(path_ref, b"changed").unwrap();
            finished.send(()).unwrap();
        });
        let result = scan_open_file(&file, |_| {
            start.send(()).unwrap();
            done.recv().unwrap();
            Ok(FileVerdict::Clean)
        });
        assert!(result.is_err());
    });
    assert_eq!(fs::read(&path).unwrap(), b"changed");
}

#[test]
fn hash_reader_propagates_an_injected_read_error() {
    struct FailingReader;
    impl Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected read failure"))
        }
    }
    assert!(ochistka_core::fs::sha256_reader(&mut FailingReader).is_err());
}
