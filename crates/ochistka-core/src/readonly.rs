//! Разделы «Очистка файлов», «Анализ пространства» и «Приватность»: только чтение, ничего не переносится.

use std::sync::atomic::AtomicBool;

use serde::Serialize;

use crate::fs::{walk, Budget, CleanupError, FsPort, Limits};
use crate::policy::validate_drive;

const DAY_SECONDS: u64 = 86_400;
const MIB: u64 = 1024 * 1024;

/// Условия поиска больших старых файлов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LargeFilesQuery {
    pub min_age_days: u64,
    pub min_bytes: u64,
    pub top: usize,
}

impl Default for LargeFilesQuery {
    fn default() -> Self {
        Self {
            min_age_days: 30,
            min_bytes: 10 * MIB,
            top: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFile {
    pub path: String,
    pub size: u64,
    pub modified_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFilesReport {
    pub files: Vec<LargeFile>,
    pub scanned: u64,
    pub complete: bool,
}

/// Большие и давно не менявшиеся файлы в папке (обычно «Загрузки»). Сортировка по размеру, по убыванию.
pub fn large_old_files(
    fs: &dyn FsPort,
    root: &str,
    query: LargeFilesQuery,
    now_unix: u64,
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<LargeFilesReport, CleanupError> {
    let cutoff = now_unix.saturating_sub(query.min_age_days * DAY_SECONDS);
    let mut files = Vec::new();
    let mut scanned = 0u64;
    let mut budget = Budget::new(limits.max_entries);
    let complete = walk(
        fs,
        root,
        cancel,
        &mut budget,
        limits.max_depth,
        &mut |entry| {
            scanned += 1;
            let meta = entry.meta;
            if !meta.is_dir && meta.size >= query.min_bytes && meta.modified_unix <= cutoff {
                files.push(LargeFile {
                    path: entry.path.clone(),
                    size: meta.size,
                    modified_unix: meta.modified_unix,
                });
            }
        },
    )?;
    files.sort_by(|a, b| b.size.cmp(&a.size).then(a.path.cmp(&b.path)));
    files.truncate(query.top);
    Ok(LargeFilesReport {
        files,
        scanned,
        complete,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSize {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub files: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceReport {
    pub root: String,
    /// Папки верхнего уровня, по убыванию объёма.
    pub folders: Vec<FolderSize>,
    /// Файлы прямо в корне диска.
    pub loose_bytes: u64,
    pub complete: bool,
}

/// Объём папок верхнего уровня диска. Лимит записей общий на весь диск.
pub fn folder_sizes(
    fs: &dyn FsPort,
    drive: &str,
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<SpaceReport, CleanupError> {
    let root = validate_drive(drive)?;
    let entries = fs.read_dir(&root)?;
    let mut budget = Budget::new(limits.max_entries);
    let mut folders = Vec::new();
    let mut loose_bytes = 0u64;
    let mut complete = true;
    for entry in entries {
        if entry.meta.is_symlink {
            continue;
        }
        if !entry.meta.is_dir {
            loose_bytes += entry.meta.size;
            continue;
        }
        let mut bytes = 0u64;
        let mut files = 0u64;
        let folder_complete = walk(
            fs,
            &entry.path,
            cancel,
            &mut budget,
            limits.max_depth,
            &mut |item| {
                if !item.meta.is_dir {
                    bytes += item.meta.size;
                    files += 1;
                }
            },
        )
        .unwrap_or(false);
        complete &= folder_complete;
        folders.push(FolderSize {
            name: entry.name,
            path: entry.path,
            bytes,
            files,
            complete: folder_complete,
        });
    }
    folders.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.name.cmp(&b.name)));
    Ok(SpaceReport {
        root,
        folders,
        loose_bytes,
        complete,
    })
}

/// Место для проверки приватности: название и путь. Читаются только метаданные, не содержимое.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyCandidate {
    pub id: String,
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyItem {
    pub id: String,
    pub title: String,
    pub path: String,
    pub found: bool,
    pub files: u64,
    pub bytes: u64,
    pub complete: bool,
}

/// Наличие и размер мест, где хранится история. Ничего не открывает и не удаляет.
pub fn privacy_report(
    fs: &dyn FsPort,
    candidates: &[PrivacyCandidate],
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<Vec<PrivacyItem>, CleanupError> {
    let mut items = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let mut item = PrivacyItem {
            id: candidate.id.clone(),
            title: candidate.title.clone(),
            path: candidate.path.clone(),
            found: false,
            files: 0,
            bytes: 0,
            complete: true,
        };
        if let Ok(meta) = fs.metadata(&candidate.path) {
            item.found = true;
            if meta.is_dir {
                let mut budget = Budget::new(limits.max_entries);
                item.complete = walk(
                    fs,
                    &candidate.path,
                    cancel,
                    &mut budget,
                    limits.max_depth,
                    &mut |entry| {
                        if !entry.meta.is_dir {
                            item.files += 1;
                            item.bytes += entry.meta.size;
                        }
                    },
                )
                .unwrap_or(false);
            } else {
                item.files = 1;
                item.bytes = meta.size;
            }
        }
        items.push(item);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake_fs::FakeFs;

    const DOWNLOADS: &str = "C:\\Users\\Slava\\Downloads";
    const NOW: u64 = 100 * DAY_SECONDS;

    fn cancel() -> AtomicBool {
        AtomicBool::new(false)
    }

    #[test]
    fn large_old_files_filters_by_age_and_size_and_sorts_by_size() {
        let fs = FakeFs::new();
        let old = NOW - 40 * DAY_SECONDS;
        let fresh = NOW - DAY_SECONDS;
        fs.file(&format!("{DOWNLOADS}\\small.zip"), &[0u8; 10], old);
        fs.file(&format!("{DOWNLOADS}\\big.iso"), &[0u8; 3 * 1024], old);
        fs.file(&format!("{DOWNLOADS}\\huge.iso"), &[0u8; 5 * 1024], old);
        fs.file(&format!("{DOWNLOADS}\\new.iso"), &[0u8; 9 * 1024], fresh);
        fs.file(&format!("{DOWNLOADS}\\sub\\mid.bin"), &[0u8; 2 * 1024], old);
        let query = LargeFilesQuery {
            min_age_days: 30,
            min_bytes: 1024,
            top: 10,
        };

        let report =
            large_old_files(&fs, DOWNLOADS, query, NOW, &cancel(), Limits::default()).unwrap();
        let names: Vec<&str> = report.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "C:\\Users\\Slava\\Downloads\\huge.iso",
                "C:\\Users\\Slava\\Downloads\\big.iso",
                "C:\\Users\\Slava\\Downloads\\sub\\mid.bin",
            ]
        );
        assert!(report.complete);
    }

    #[test]
    fn folder_sizes_sum_top_level_folders_and_loose_files() {
        let fs = FakeFs::new();
        fs.file("D:\\a\\1.bin", &[0u8; 100], 0);
        fs.file("D:\\a\\deep\\2.bin", &[0u8; 100], 0);
        fs.file("D:\\b\\3.bin", &[0u8; 300], 0);
        fs.file("D:\\loose.txt", &[0u8; 7], 0);
        fs.dir("D:\\");

        let report = folder_sizes(&fs, "d:", &cancel(), Limits::default()).unwrap();
        assert_eq!(report.root, "D:\\");
        assert_eq!(report.loose_bytes, 7);
        let rows: Vec<(&str, u64, u64)> = report
            .folders
            .iter()
            .map(|f| (f.name.as_str(), f.bytes, f.files))
            .collect();
        assert_eq!(rows, vec![("b", 300, 1), ("a", 200, 2)]);
        assert!(report.complete);
    }

    #[test]
    fn folder_sizes_rejects_a_path_that_is_not_a_drive() {
        let fs = FakeFs::new();
        assert!(folder_sizes(&fs, "C:\\Windows", &cancel(), Limits::default()).is_err());
    }

    #[test]
    fn privacy_report_counts_present_items_and_marks_missing_ones() {
        let fs = FakeFs::new();
        fs.file("C:\\P\\Recent\\a.lnk", &[0u8; 20], 0);
        fs.file("C:\\P\\Recent\\b.lnk", &[0u8; 30], 0);
        fs.file("C:\\P\\History", &[0u8; 512], 0);
        let candidates = vec![
            PrivacyCandidate {
                id: "recent".into(),
                title: "Недавние".into(),
                path: "C:\\P\\Recent".into(),
            },
            PrivacyCandidate {
                id: "history".into(),
                title: "История".into(),
                path: "C:\\P\\History".into(),
            },
            PrivacyCandidate {
                id: "ff".into(),
                title: "Firefox".into(),
                path: "C:\\P\\places.sqlite".into(),
            },
        ];

        let items = privacy_report(&fs, &candidates, &cancel(), Limits::default()).unwrap();
        assert_eq!(
            (items[0].found, items[0].files, items[0].bytes),
            (true, 2, 50)
        );
        assert_eq!(
            (items[1].found, items[1].files, items[1].bytes),
            (true, 1, 512)
        );
        assert!(!items[2].found);
    }

    #[test]
    fn shared_entry_budget_makes_later_folders_incomplete() {
        let fs = FakeFs::new();
        fs.file("D:\\a\\1.bin", &[0u8; 1], 0);
        fs.file("D:\\b\\2.bin", &[0u8; 1], 0);
        let limits = Limits {
            max_entries: 1,
            max_depth: 24,
        };
        let report = folder_sizes(&fs, "D:\\", &cancel(), limits).unwrap();
        assert!(!report.complete);
    }
}
