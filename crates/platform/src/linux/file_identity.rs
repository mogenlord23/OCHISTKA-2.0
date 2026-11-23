//! Предварительная проверка идентичности Linux-файла. Не выполняет перенос.
//!
//! Каждый компонент открывается относительно уже открытой директории через procfs;
//! отсутствие procfs или поддерживаемых флагов приводит к отказу. Финальный файл
//! хэшируется через тот же дескриптор, из которого получены метаданные. Это не
//! связывает результат с текущим path-based API сканера и не делает rename безопасным.

use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path};

use ochistka_core::fs::{sha256_reader, CleanupError};
use ochistka_core::quarantine::FileIdentity;
use ochistka_core::scan::FileVerdict;
use ochistka_core::system::CoreError;

// Linux UAPI <asm-generic/fcntl.h>; модуль компилируется только для Linux.
const O_NOFOLLOW: i32 = 0o400000;
const O_DIRECTORY: i32 = 0o200000;
const O_NONBLOCK: i32 = 0o4000;

fn io_error(err: std::io::Error) -> CleanupError {
    CleanupError::Io(err.to_string())
}

/// Открывает абсолютный путь по компонентам, запрещая ссылки на каждом шаге.
/// Возвращаемый дескриптор остаётся привязан к открытому объекту при переименовании пути.
pub fn open_regular_no_links(path: &Path) -> Result<File, CleanupError> {
    let mut components = path.components();
    if components.next() != Some(Component::RootDir) {
        return Err(CleanupError::Refused("нужен абсолютный путь".into()));
    }
    let parts: Vec<_> = components.collect();
    if parts.is_empty()
        || parts
            .iter()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(CleanupError::Refused("небезопасный путь".into()));
    }
    let mut dir = File::open("/").map_err(io_error)?;
    for (index, part) in parts.iter().enumerate() {
        let Component::Normal(name) = part else {
            unreachable!()
        };
        let last = index + 1 == parts.len();
        let anchored = Path::new("/proc/self/fd")
            .join(dir.as_raw_fd().to_string())
            .join(name);
        let mut opts = OpenOptions::new();
        opts.read(true)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK | if last { 0 } else { O_DIRECTORY });
        let opened = opts.open(anchored).map_err(io_error)?;
        if last {
            if !opened.metadata().map_err(io_error)?.is_file() {
                return Err(CleanupError::Refused(
                    "объект не является обычным файлом".into(),
                ));
            }
            return Ok(opened);
        }
        dir = opened;
    }
    Err(CleanupError::Refused("не указан файл".into()))
}

/// Снимок метаданных и SHA-256 открытого объекта; изменение во время чтения отклоняется.
pub fn identify(mut file: &File) -> Result<FileIdentity, CleanupError> {
    let before = file.metadata().map_err(io_error)?;
    if !before.is_file() {
        return Err(CleanupError::Refused(
            "объект не является обычным файлом".into(),
        ));
    }
    file.seek(SeekFrom::Start(0)).map_err(io_error)?;
    let sha256 = sha256_reader(&mut file)?;
    let after = file.metadata().map_err(io_error)?;
    if (
        before.dev(),
        before.ino(),
        before.len(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    ) != (
        after.dev(),
        after.ino(),
        after.len(),
        after.mtime(),
        after.mtime_nsec(),
        after.ctime(),
        after.ctime_nsec(),
    ) {
        return Err(CleanupError::Refused(
            "файл изменился во время чтения".into(),
        ));
    }
    Ok(FileIdentity {
        platform: "linux".into(),
        volume: before.dev().to_string(),
        file_id: before.ino().to_string(),
        size: before.len(),
        sha256,
    })
}

/// Только проверка: не изменяет источник и не разрешает последующее перемещение по имени.
/// Возвращённый дескриптор удерживает именно проверенный объект открытым.
pub fn open_verified_source(path: &Path, expected: &FileIdentity) -> Result<File, CleanupError> {
    let file = open_regular_no_links(path)?;
    if identify(&file)? != *expected {
        return Err(CleanupError::Refused(
            "файл отличается от проверенной находки; перенос не выполнен".into(),
        ));
    }
    Ok(file)
}

/// Сканирует открытый объект между двумя проверками его содержимого.
pub fn scan_open_file(
    file: &File,
    scan: impl FnOnce(&File) -> Result<FileVerdict, CoreError>,
) -> Result<(FileVerdict, FileIdentity), CoreError> {
    let before = identify(file).map_err(|err| CoreError::Probe(err.to_string()))?;
    let metadata_before = file
        .metadata()
        .map_err(|err| CoreError::Probe(err.to_string()))?;
    (&*file)
        .seek(SeekFrom::Start(0))
        .map_err(|err| CoreError::Probe(err.to_string()))?;
    let verdict = scan(file)?;
    let after = identify(file).map_err(|err| CoreError::Probe(err.to_string()))?;
    let metadata_after = file
        .metadata()
        .map_err(|err| CoreError::Probe(err.to_string()))?;
    if before != after
        || (
            metadata_before.mtime(),
            metadata_before.mtime_nsec(),
            metadata_before.ctime(),
            metadata_before.ctime_nsec(),
        ) != (
            metadata_after.mtime(),
            metadata_after.mtime_nsec(),
            metadata_after.ctime(),
            metadata_after.ctime_nsec(),
        )
    {
        return Err(CoreError::Probe("файл изменился во время проверки".into()));
    }
    Ok((verdict, before))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "ochistka-identity-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn identifies_open_object_and_detects_replacement_and_changed_content() {
        let tmp = Temp::new();
        let path = tmp.0.join("file");
        std::fs::write(&path, b"test").unwrap();
        let open = open_regular_no_links(&path).unwrap();
        let original = identify(&open).unwrap();
        assert_eq!(original.sha256, ochistka_core::fs::sha256_hex(b"test"));
        assert_eq!(identify(&open).unwrap(), original);
        std::fs::rename(&path, tmp.0.join("old")).unwrap();
        std::fs::write(&path, b"test").unwrap();
        let replaced = identify(&open_regular_no_links(&path).unwrap()).unwrap();
        assert_ne!(original.file_id, replaced.file_id);
        std::fs::write(&path, b"diff").unwrap();
        let changed = identify(&open_regular_no_links(&path).unwrap()).unwrap();
        assert_ne!(replaced.sha256, changed.sha256);
    }

    #[test]
    fn refuses_symlink_in_file_or_parent_and_non_regular_object() {
        let tmp = Temp::new();
        std::fs::create_dir(tmp.0.join("dir")).unwrap();
        std::fs::write(tmp.0.join("dir/file"), b"safe").unwrap();
        symlink("dir/file", tmp.0.join("link")).unwrap();
        symlink("dir", tmp.0.join("parent")).unwrap();
        assert!(open_regular_no_links(&tmp.0.join("link")).is_err());
        assert!(open_regular_no_links(&tmp.0.join("parent/file")).is_err());
        assert!(open_regular_no_links(&tmp.0.join("dir")).is_err());
    }

    #[test]
    fn rejects_content_changed_during_scan() {
        let tmp = Temp::new();
        let path = tmp.0.join("file");
        std::fs::write(&path, b"first").unwrap();
        let file = open_regular_no_links(&path).unwrap();
        let result = scan_open_file(&file, |_| {
            std::fs::write(&path, b"other").unwrap();
            Ok(FileVerdict::Clean)
        });
        assert!(matches!(result, Err(CoreError::Probe(_))));
    }

    #[test]
    fn scan_uses_original_file_when_path_is_replaced() {
        let tmp = Temp::new();
        let path = tmp.0.join("file");
        std::fs::write(&path, b"first").unwrap();
        let file = open_regular_no_links(&path).unwrap();
        let result = scan_open_file(&file, |opened| {
            use std::io::Read;
            std::fs::rename(&path, tmp.0.join("old")).unwrap();
            std::fs::write(&path, b"other").unwrap();
            let mut data = Vec::new();
            (&*opened).read_to_end(&mut data).unwrap();
            assert_eq!(data, b"first");
            Ok(FileVerdict::Clean)
        });
        if let Ok((_, identity)) = result {
            assert_ne!(
                identity,
                identify(&open_regular_no_links(&path).unwrap()).unwrap()
            );
        }
    }

    #[test]
    fn verified_source_refuses_replacement_and_preserves_both_files() {
        let tmp = Temp::new();
        let path = tmp.0.join("file");
        let old = tmp.0.join("old");
        let manifest = tmp.0.join("manifest.json");
        let destination = tmp.0.join("destination");
        std::fs::write(&manifest, b"manifest").unwrap();
        std::fs::write(&destination, b"existing").unwrap();
        std::fs::write(&path, b"same").unwrap();
        let identity = identify(&open_regular_no_links(&path).unwrap()).unwrap();
        assert!(open_verified_source(&path, &identity).is_ok());
        std::fs::rename(&path, &old).unwrap();
        std::fs::write(&path, b"same").unwrap();
        assert!(matches!(
            open_verified_source(&path, &identity),
            Err(CleanupError::Refused(_))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), b"same");
        assert_eq!(std::fs::read(&old).unwrap(), b"same");
        assert_eq!(std::fs::read(&manifest).unwrap(), b"manifest");
        assert_eq!(std::fs::read(&destination).unwrap(), b"existing");
    }

    #[test]
    fn verified_source_refuses_changed_content_and_each_identity_mismatch() {
        let tmp = Temp::new();
        let path = tmp.0.join("file");
        std::fs::write(&path, b"first").unwrap();
        let identity = identify(&open_regular_no_links(&path).unwrap()).unwrap();
        let mut variants = Vec::new();
        let mut other = identity.clone();
        other.platform = "other".into();
        variants.push(other);
        let mut other = identity.clone();
        other.volume.push('0');
        variants.push(other);
        let mut other = identity.clone();
        other.file_id.push('0');
        variants.push(other);
        let mut other = identity.clone();
        other.size += 1;
        variants.push(other);
        let mut other = identity.clone();
        other.sha256.push('0');
        variants.push(other);
        for other in variants {
            assert!(matches!(
                open_verified_source(&path, &other),
                Err(CleanupError::Refused(_))
            ));
            assert_eq!(std::fs::read(&path).unwrap(), b"first");
        }
        std::fs::write(&path, b"other").unwrap();
        assert!(matches!(
            open_verified_source(&path, &identity),
            Err(CleanupError::Refused(_))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), b"other");
    }

    #[test]
    fn verified_source_refuses_links_without_touching_their_targets() {
        let tmp = Temp::new();
        let dir = tmp.0.join("dir");
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("file");
        std::fs::write(&path, b"data").unwrap();
        let identity = identify(&open_regular_no_links(&path).unwrap()).unwrap();
        symlink(&path, tmp.0.join("link")).unwrap();
        symlink(&dir, tmp.0.join("parent")).unwrap();
        assert!(open_verified_source(&tmp.0.join("link"), &identity).is_err());
        assert!(open_verified_source(&tmp.0.join("parent/file"), &identity).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"data");
    }
}
