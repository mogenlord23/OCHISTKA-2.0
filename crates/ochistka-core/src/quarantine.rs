//! Карантин по ADR 0005: перенос вместо удаления, манифест с путём, SHA-256, датой и причиной,
//! восстановление и окончательное удаление только с токеном подтверждения.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::fs::{read_all, sha256_file, sha256_hex, CleanupError, FsPort};
use crate::policy::{file_name, parent_dir, validate_root, PathPolicy};

pub const MANIFEST_FILE: &str = "manifest.json";
const MANIFEST_TEMP: &str = "manifest.json.tmp";
/// Сколько переносов копится до записи манифеста.
const COMMIT_EVERY: usize = 100;
/// Сколько причин ошибок переноса показываем интерфейсу (остальные только счётчиком).
pub const MAX_RECORDED_FAILURES: usize = 20;

/// Запись манифеста: всё, что нужно, чтобы вернуть файл или проверить его целостность.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineEntry {
    pub id: String,
    pub original_path: String,
    pub size: u64,
    pub sha256: String,
    pub moved_at_unix: u64,
    pub reason: String,
    /// Токен, который интерфейс должен вернуть при окончательном удалении.
    pub purge_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    #[serde(default = "manifest_version")]
    version: u32,
    #[serde(default)]
    entries: Vec<QuarantineEntry>,
}

fn manifest_version() -> u32 {
    1
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: manifest_version(),
            entries: Vec::new(),
        }
    }
}

/// Токен окончательного удаления для записи. Интерфейс показывает отдельный диалог и передаёт его обратно.
pub fn purge_token(id: &str) -> String {
    format!("purge:{id}")
}

/// Идентификатор записи: 16 шестнадцатеричных строчных символов.
pub fn is_valid_id(id: &str) -> bool {
    id.len() == 16
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn check_id(id: &str) -> Result<(), CleanupError> {
    if is_valid_id(id) {
        Ok(())
    } else {
        Err(CleanupError::InvalidId)
    }
}

/// Результат переноса пачки файлов.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcome {
    pub moved: u32,
    pub bytes: u64,
    pub failed: u32,
    pub failures: Vec<MoveFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveFailure {
    pub path: String,
    pub message: String,
}

impl MoveOutcome {
    fn record_failure(&mut self, path: &str, err: &CleanupError) {
        self.failed += 1;
        if self.failures.len() < MAX_RECORDED_FAILURES {
            self.failures.push(MoveFailure {
                path: path.to_string(),
                message: err.to_string(),
            });
        }
    }

    /// Складывает результаты нескольких пачек (например, по категориям).
    pub fn merge(&mut self, other: MoveOutcome) {
        self.moved += other.moved;
        self.bytes += other.bytes;
        self.failed += other.failed;
        for failure in other.failures {
            if self.failures.len() < MAX_RECORDED_FAILURES {
                self.failures.push(failure);
            }
        }
    }
}

/// Перенесённый, но ещё не записанный в манифест файл.
struct Staged {
    source: String,
    dest: String,
    entry: QuarantineEntry,
}

/// Карантин в одной папке. Корень не может лежать внутри папок очистки.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarantine {
    root: String,
    policy: PathPolicy,
}

impl Quarantine {
    pub fn new(root: &str, policy: PathPolicy) -> Result<Self, CleanupError> {
        let normalized = validate_root(root)?;
        for cleanup_root in policy.roots() {
            let nested = normalized == *cleanup_root
                || normalized.starts_with(&format!("{cleanup_root}\\"))
                || cleanup_root.starts_with(&format!("{normalized}\\"));
            if nested {
                return Err(CleanupError::Refused(
                    "карантин не может пересекаться с папками очистки".into(),
                ));
            }
        }
        Ok(Self {
            root: root.trim().to_string(),
            policy,
        })
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    /// Имя файла внутри записи. Служебные имена отвергаются, чтобы путь не ушёл из папки записи.
    fn stored_path(
        &self,
        fs: &dyn FsPort,
        id: &str,
        original: &str,
    ) -> Result<(String, String), CleanupError> {
        let name = file_name(original);
        if name.is_empty() || name == "." || name == ".." {
            return Err(CleanupError::InvalidId);
        }
        let dir = fs.child(&self.root, id);
        let stored = fs.child(&dir, name);
        Ok((dir, stored))
    }

    fn load(&self, fs: &dyn FsPort) -> Result<Manifest, CleanupError> {
        let path = fs.child(&self.root, MANIFEST_FILE);
        match fs.metadata(&path) {
            Ok(_) => {}
            Err(CleanupError::NotFound(_)) => return Ok(Manifest::default()),
            Err(err) => return Err(err),
        }
        let bytes = read_all(fs, &path)?;
        serde_json::from_slice(&bytes).map_err(|_| CleanupError::ManifestCorrupt)
    }

    /// Пишет манифест во временный файл и переименовывает, чтобы не оставить полузаписанный файл.
    fn save(&self, fs: &dyn FsPort, manifest: &Manifest) -> Result<(), CleanupError> {
        fs.create_dir_all(&self.root)?;
        let bytes =
            serde_json::to_vec_pretty(manifest).map_err(|err| CleanupError::Io(err.to_string()))?;
        let temp = fs.child(&self.root, MANIFEST_TEMP);
        let target = fs.child(&self.root, MANIFEST_FILE);
        fs.write_file(&temp, &bytes)?;
        fs.rename(&temp, &target)
    }

    /// Все записи, новые сверху.
    pub fn list(&self, fs: &dyn FsPort) -> Result<Vec<QuarantineEntry>, CleanupError> {
        let mut entries = self.load(fs)?.entries;
        entries.sort_by(|a, b| b.moved_at_unix.cmp(&a.moved_at_unix).then(a.id.cmp(&b.id)));
        Ok(entries)
    }

    /// Переносит файлы в карантин. Ошибки отдельных файлов копятся, остальные файлы продолжают переноситься.
    pub fn move_many(
        &self,
        fs: &dyn FsPort,
        sources: &[String],
        reason: &str,
        now_unix: u64,
    ) -> Result<MoveOutcome, CleanupError> {
        let mut manifest = self.load(fs)?;
        let mut known: HashSet<String> = manifest.entries.iter().map(|e| e.id.clone()).collect();
        let mut outcome = MoveOutcome::default();
        let mut pending: Vec<Staged> = Vec::new();

        for source in sources {
            match self.stage(fs, source, reason, now_unix, &known) {
                Ok(staged) => {
                    known.insert(staged.entry.id.clone());
                    pending.push(staged);
                }
                Err(err) => outcome.record_failure(source, &err),
            }
            if pending.len() >= COMMIT_EVERY
                && !self.commit(fs, &mut manifest, &mut pending, &mut outcome)
            {
                // Запись манифеста не удалась: пачка возвращена на место, дальше не переносим.
                return Ok(outcome);
            }
        }
        if !pending.is_empty() {
            self.commit(fs, &mut manifest, &mut pending, &mut outcome);
        }
        Ok(outcome)
    }

    fn stage(
        &self,
        fs: &dyn FsPort,
        source: &str,
        reason: &str,
        now_unix: u64,
        known: &HashSet<String>,
    ) -> Result<Staged, CleanupError> {
        self.policy.check(source)?;
        let meta = fs.metadata(source)?;
        if meta.is_dir || meta.is_symlink {
            return Err(CleanupError::Refused(format!("не обычный файл: {source}")));
        }
        let sha256 = sha256_file(fs, source)?;
        let id = sha256_hex(format!("{source}|{now_unix}|{sha256}").as_bytes())[..16].to_string();
        if known.contains(&id) {
            return Err(CleanupError::Io("коллизия идентификатора карантина".into()));
        }
        let (dir, dest) = self.stored_path(fs, &id, source)?;
        match fs.metadata(&dest) {
            Ok(_) => return Err(CleanupError::Occupied(dest)),
            Err(CleanupError::NotFound(_)) => {}
            Err(err) => return Err(err),
        }
        fs.create_dir_all(&dir)?;
        fs.rename(source, &dest)?;
        Ok(Staged {
            source: source.to_string(),
            dest,
            entry: QuarantineEntry {
                purge_token: purge_token(&id),
                id,
                original_path: source.to_string(),
                size: meta.size,
                sha256,
                moved_at_unix: now_unix,
                reason: reason.to_string(),
            },
        })
    }

    /// Записывает пачку в манифест. При ошибке возвращает файлы назад и записывает причины.
    fn commit(
        &self,
        fs: &dyn FsPort,
        manifest: &mut Manifest,
        pending: &mut Vec<Staged>,
        outcome: &mut MoveOutcome,
    ) -> bool {
        let before = manifest.entries.len();
        manifest
            .entries
            .extend(pending.iter().map(|staged| staged.entry.clone()));
        match self.save(fs, manifest) {
            Ok(()) => {
                for staged in pending.drain(..) {
                    outcome.moved += 1;
                    outcome.bytes += staged.entry.size;
                }
                true
            }
            Err(err) => {
                manifest.entries.truncate(before);
                for staged in pending.drain(..) {
                    // Если вернуть не удалось, файл остаётся в карантине без записи: причина уходит в отчёт.
                    let message = match fs.rename(&staged.dest, &staged.source) {
                        Ok(()) => err.clone(),
                        Err(back) => CleanupError::Io(format!("{err}; не возвращён: {back}")),
                    };
                    outcome.record_failure(&staged.source, &message);
                }
                false
            }
        }
    }

    /// Возвращает файл на исходный путь (или на `alternative`, если исходный занят).
    /// Проверяет хэш; если место занято, возвращает `Occupied`, и интерфейс спрашивает другой путь.
    pub fn restore(
        &self,
        fs: &dyn FsPort,
        id: &str,
        alternative: Option<&str>,
    ) -> Result<QuarantineEntry, CleanupError> {
        check_id(id)?;
        let mut manifest = self.load(fs)?;
        let index = manifest
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(CleanupError::EntryNotFound)?;
        let entry = manifest.entries[index].clone();
        let (dir, stored) = self.stored_path(fs, id, &entry.original_path)?;
        fs.metadata(&stored)?;
        if sha256_file(fs, &stored)? != entry.sha256 {
            return Err(CleanupError::HashMismatch(stored));
        }

        let target = alternative
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&entry.original_path)
            .to_string();
        self.policy.check(&target)?;
        match fs.metadata(&target) {
            Ok(_) => return Err(CleanupError::Occupied(target)),
            Err(CleanupError::NotFound(_)) => {}
            Err(err) => return Err(err),
        }
        if let Some(parent) = parent_dir(&target) {
            fs.create_dir_all(parent)?;
        }
        fs.rename(&stored, &target)?;

        manifest.entries.remove(index);
        if let Err(err) = self.save(fs, &manifest) {
            // Без записи в манифесте файл не должен оставаться на новом месте как «восстановленный».
            let _ = fs.rename(&target, &stored);
            return Err(err);
        }
        let _ = fs.remove_dir(&dir);
        Ok(entry)
    }

    /// Окончательное удаление. Токен проверяется до любой операции с диском.
    pub fn purge(
        &self,
        fs: &dyn FsPort,
        id: &str,
        token: &str,
    ) -> Result<QuarantineEntry, CleanupError> {
        check_id(id)?;
        if token != purge_token(id) {
            return Err(CleanupError::ConfirmationMismatch);
        }
        let mut manifest = self.load(fs)?;
        let index = manifest
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(CleanupError::EntryNotFound)?;
        let entry = manifest.entries[index].clone();
        // Путь строится внутри папки записи (id уже проверен), поэтому за пределы карантина не выйдет.
        let (dir, stored) = self.stored_path(fs, id, &entry.original_path)?;
        fs.metadata(&stored)?;
        fs.remove_file(&stored)?;
        manifest.entries.remove(index);
        self.save(fs, &manifest)?;
        let _ = fs.remove_dir(&dir);
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake_fs::FakeFs;

    const TEMP: &str = "C:\\Users\\Slava\\AppData\\Local\\Temp";
    const QROOT: &str = "C:\\Users\\Slava\\AppData\\Local\\OCHISTKA\\Quarantine";

    fn quarantine() -> Quarantine {
        Quarantine::new(QROOT, PathPolicy::new(&[TEMP]).unwrap()).unwrap()
    }

    struct MetadataDenied<'a> {
        inner: &'a FakeFs,
        blocked: String,
    }

    impl FsPort for MetadataDenied<'_> {
        fn metadata(&self, path: &str) -> Result<crate::fs::Meta, CleanupError> {
            if path == self.blocked {
                Err(CleanupError::Io("нет доступа к метаданным".into()))
            } else {
                self.inner.metadata(path)
            }
        }
        fn read_dir(&self, path: &str) -> Result<Vec<crate::fs::DirEntry>, CleanupError> {
            self.inner.read_dir(path)
        }
        fn child(&self, dir: &str, name: &str) -> String {
            self.inner.child(dir, name)
        }
        fn open(&self, path: &str) -> Result<Box<dyn std::io::Read + Send>, CleanupError> {
            self.inner.open(path)
        }
        fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), CleanupError> {
            self.inner.write_file(path, bytes)
        }
        fn create_dir_all(&self, path: &str) -> Result<(), CleanupError> {
            self.inner.create_dir_all(path)
        }
        fn rename(&self, from: &str, to: &str) -> Result<(), CleanupError> {
            self.inner.rename(from, to)
        }
        fn remove_file(&self, path: &str) -> Result<(), CleanupError> {
            self.inner.remove_file(path)
        }
        fn remove_dir(&self, path: &str) -> Result<(), CleanupError> {
            self.inner.remove_dir(path)
        }
    }

    #[test]
    fn inaccessible_manifest_cannot_be_treated_as_empty() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        let denied = MetadataDenied {
            inner: &fs,
            blocked: fs.child(QROOT, MANIFEST_FILE),
        };
        assert!(matches!(
            q.move_many(&denied, std::slice::from_ref(&source), "test", 1),
            Err(CleanupError::Io(_))
        ));
        assert!(fs.exists(&source));
    }

    #[test]
    fn inaccessible_source_is_reported_and_not_moved() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        let denied = MetadataDenied {
            inner: &fs,
            blocked: source.clone(),
        };
        let outcome = q
            .move_many(&denied, std::slice::from_ref(&source), "test", 1)
            .unwrap();
        assert_eq!((outcome.moved, outcome.failed), (0, 1));
        assert!(fs.exists(&source));
        assert!(q.list(&fs).unwrap().is_empty());
    }

    #[test]
    fn inaccessible_restore_destination_cannot_be_treated_as_vacant() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "test", 1)
            .unwrap();
        let entry = q.list(&fs).unwrap().remove(0);
        let denied = MetadataDenied {
            inner: &fs,
            blocked: source.clone(),
        };
        assert!(matches!(
            q.restore(&denied, &entry.id, None),
            Err(CleanupError::Io(_))
        ));
        assert_eq!(q.list(&fs).unwrap(), vec![entry]);
        assert!(!fs.exists(&source));
    }

    #[test]
    fn inaccessible_quarantined_file_keeps_its_manifest_entry() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "test", 1)
            .unwrap();
        let entry = q.list(&fs).unwrap().remove(0);
        let blocked = format!("{QROOT}\\{}\\a.tmp", entry.id);
        let denied = MetadataDenied {
            inner: &fs,
            blocked: blocked.clone(),
        };
        assert!(matches!(
            q.purge(&denied, &entry.id, &entry.purge_token),
            Err(CleanupError::Io(_))
        ));
        assert!(fs.exists(&blocked));
        assert_eq!(q.list(&fs).unwrap(), vec![entry]);
    }

    #[test]
    fn missing_quarantined_file_keeps_its_manifest_entry() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "test", 1)
            .unwrap();
        let entry = q.list(&fs).unwrap().remove(0);
        let stored = format!("{QROOT}\\{}\\a.tmp", entry.id);
        fs.remove_file(&stored).unwrap();
        assert!(matches!(
            q.restore(&fs, &entry.id, None),
            Err(CleanupError::NotFound(_))
        ));
        assert!(matches!(
            q.purge(&fs, &entry.id, &entry.purge_token),
            Err(CleanupError::NotFound(_))
        ));
        assert_eq!(q.list(&fs).unwrap(), vec![entry]);
    }

    #[test]
    fn move_then_restore_round_trip_keeps_hash_and_manifest_entry() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\cache\\a.tmp");
        fs.file(&source, b"hello", 100);
        let q = quarantine();

        let outcome = q
            .move_many(&fs, std::slice::from_ref(&source), "тест", 1000)
            .unwrap();
        assert_eq!((outcome.moved, outcome.bytes, outcome.failed), (1, 5, 0));
        assert!(!fs.exists(&source), "source must be moved away");

        let entries = q.list(&fs).unwrap();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.original_path, source);
        assert_eq!(entry.sha256, sha256_hex(b"hello"));
        assert_eq!(entry.moved_at_unix, 1000);
        assert_eq!(entry.reason, "тест");
        assert_eq!(entry.purge_token, purge_token(&entry.id));

        let restored = q.restore(&fs, &entry.id, None).unwrap();
        assert_eq!(restored.original_path, source);
        assert_eq!(fs.bytes_of(&source).unwrap(), b"hello");
        assert!(q.list(&fs).unwrap().is_empty());
    }

    #[test]
    fn system_path_is_refused_before_any_io() {
        let fs = FakeFs::new();
        fs.file("C:\\Windows\\Temp\\x.tmp", b"x", 0);
        let policy = PathPolicy::new(&["C:\\Windows\\Temp"]);
        assert!(matches!(policy, Err(CleanupError::Refused(_))));

        let q = quarantine();
        let outcome = q
            .move_many(&fs, &["C:\\Windows\\Temp\\x.tmp".to_string()], "тест", 1)
            .unwrap();
        assert_eq!(outcome.failed, 1);
        assert_eq!(outcome.moved, 0);
        assert!(fs.exists("C:\\Windows\\Temp\\x.tmp"));
    }

    #[test]
    fn purge_is_refused_without_matching_token_and_keeps_file() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "тест", 5)
            .unwrap();
        let id = q.list(&fs).unwrap()[0].id.clone();

        assert_eq!(
            q.purge(&fs, &id, ""),
            Err(CleanupError::ConfirmationMismatch)
        );
        assert_eq!(
            q.purge(&fs, &id, "purge:0000000000000000"),
            Err(CleanupError::ConfirmationMismatch)
        );
        assert_eq!(
            q.list(&fs).unwrap().len(),
            1,
            "entry must survive a refused purge"
        );
    }

    #[test]
    fn purge_with_token_removes_file_and_manifest_entry() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "тест", 5)
            .unwrap();
        let entry = q.list(&fs).unwrap()[0].clone();

        q.purge(&fs, &entry.id, &entry.purge_token).unwrap();
        assert!(q.list(&fs).unwrap().is_empty());
        assert!(!fs.exists(&format!("{QROOT}\\{}\\a.tmp", entry.id)));
    }

    #[test]
    fn restore_reports_occupied_target_and_accepts_alternative() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "тест", 5)
            .unwrap();
        let id = q.list(&fs).unwrap()[0].id.clone();
        fs.file(&source, b"new", 0);

        assert!(matches!(
            q.restore(&fs, &id, None),
            Err(CleanupError::Occupied(_))
        ));
        let alt = format!("{TEMP}\\b.tmp");
        q.restore(&fs, &id, Some(&alt)).unwrap();
        assert_eq!(fs.bytes_of(&alt).unwrap(), b"data");
        assert_eq!(fs.bytes_of(&source).unwrap(), b"new");
    }

    #[test]
    fn restore_to_an_alternative_outside_the_whitelist_is_refused_and_keeps_the_entry() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "тест", 5)
            .unwrap();
        let entry = q.list(&fs).unwrap()[0].clone();

        let outside = "C:\\Users\\Slava\\Documents\\a.tmp";
        assert!(matches!(
            q.restore(&fs, &entry.id, Some(outside)),
            Err(CleanupError::Refused(_))
        ));
        assert!(
            !fs.exists(outside),
            "nothing may be written outside the whitelist"
        );
        assert_eq!(q.list(&fs).unwrap(), vec![entry.clone()]);
        assert!(fs.exists(&format!("{QROOT}\\{}\\a.tmp", entry.id)));
    }

    #[test]
    fn restore_to_an_occupied_alternative_keeps_the_file_in_quarantine() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "тест", 5)
            .unwrap();
        let entry = q.list(&fs).unwrap()[0].clone();
        let alt = format!("{TEMP}\\b.tmp");
        fs.file(&alt, b"other", 0);

        assert!(matches!(
            q.restore(&fs, &entry.id, Some(&alt)),
            Err(CleanupError::Occupied(_))
        ));
        assert_eq!(fs.bytes_of(&alt).unwrap(), b"other");
        assert_eq!(q.list(&fs).unwrap(), vec![entry.clone()]);
        assert!(fs.exists(&format!("{QROOT}\\{}\\a.tmp", entry.id)));
    }

    #[test]
    fn restore_refuses_a_file_changed_after_the_move() {
        let fs = FakeFs::new();
        let source = format!("{TEMP}\\a.tmp");
        fs.file(&source, b"data", 0);
        let q = quarantine();
        q.move_many(&fs, std::slice::from_ref(&source), "тест", 5)
            .unwrap();
        let entry = q.list(&fs).unwrap()[0].clone();
        fs.file(&format!("{QROOT}\\{}\\a.tmp", entry.id), b"tampered", 0);

        assert!(matches!(
            q.restore(&fs, &entry.id, None),
            Err(CleanupError::HashMismatch(_))
        ));
    }

    #[test]
    fn invalid_ids_are_rejected_before_touching_the_tree() {
        let fs = FakeFs::new();
        let q = quarantine();
        assert_eq!(
            q.restore(&fs, "..\\..\\x", None),
            Err(CleanupError::InvalidId)
        );
        assert_eq!(
            q.purge(&fs, "../x", "purge:../x"),
            Err(CleanupError::InvalidId)
        );
    }

    #[test]
    fn quarantine_inside_a_cleanup_root_is_refused() {
        let policy = PathPolicy::new(&[TEMP]).unwrap();
        assert!(Quarantine::new(&format!("{TEMP}\\OCHISTKA"), policy.clone()).is_err());
        assert!(Quarantine::new("C:\\Users\\Slava\\AppData\\Local", policy).is_err());
    }
}
