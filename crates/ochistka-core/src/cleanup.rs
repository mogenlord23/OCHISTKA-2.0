//! Глубокая очистка: анализ кэшей и временных папок (только чтение) и перенос выбранного в карантин.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::atomic::AtomicBool;

use serde::Serialize;

use crate::fs::{walk, Budget, CleanupError, FsPort, Limits};
use crate::policy::{normalize, PathPolicy};
use crate::quarantine::{MoveOutcome, Quarantine};

/// Категория очистки, которую выдаёт платформа: id, название и корень (уже внутри белого списка).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupLocation {
    pub id: String,
    pub title: String,
    pub root: String,
}

/// Итог по категории для интерфейса: только числа, без списка файлов.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCategory {
    pub id: String,
    pub title: String,
    /// Папка существует на этом компьютере.
    pub found: bool,
    pub file_count: u64,
    pub bytes: u64,
    /// `false`, если обход остановлен лимитом или часть папок недоступна.
    pub complete: bool,
}

/// Результат анализа. Список файлов остаётся на стороне ядра: интерфейс переносит только по категориям.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupAnalysis {
    pub categories: Vec<CleanupCategory>,
    files: BTreeMap<String, Vec<String>>,
}

impl CleanupAnalysis {
    /// Пути файлов категории (для проверок и тестов).
    pub fn files_of(&self, id: &str) -> &[String] {
        self.files.get(id).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Считает файлы и объём в категориях. Ссылки не переходим; файл, попавший в две категории, учитываем один раз.
/// `cancel` останавливает обход ошибкой `Cancelled`.
pub fn analyze(
    fs: &dyn FsPort,
    policy: &PathPolicy,
    locations: &[CleanupLocation],
    cancel: &AtomicBool,
    limits: Limits,
) -> Result<CleanupAnalysis, CleanupError> {
    let mut categories = Vec::with_capacity(locations.len());
    let mut files = BTreeMap::new();
    let mut seen: HashSet<String> = HashSet::new();

    for location in locations {
        policy.check(&location.root)?;
        let mut category = CleanupCategory {
            id: location.id.clone(),
            title: location.title.clone(),
            found: false,
            file_count: 0,
            bytes: 0,
            complete: true,
        };
        let mut paths = Vec::new();
        if let Ok(meta) = fs.metadata(&location.root) {
            if meta.is_dir && !meta.is_symlink {
                category.found = true;
                let mut budget = Budget::new(limits.max_entries);
                let outcome = walk(
                    fs,
                    &location.root,
                    cancel,
                    &mut budget,
                    limits.max_depth,
                    &mut |entry| {
                        if !entry.meta.is_dir && seen.insert(normalize(&entry.path)) {
                            category.file_count += 1;
                            category.bytes += entry.meta.size;
                            paths.push(entry.path.clone());
                        }
                    },
                );
                category.complete = match outcome {
                    Ok(complete) => complete,
                    Err(CleanupError::Cancelled) => return Err(CleanupError::Cancelled),
                    Err(_) => false,
                };
            }
        }
        files.insert(location.id.clone(), paths);
        categories.push(category);
    }
    Ok(CleanupAnalysis { categories, files })
}

/// Переносит файлы выбранных категорий в карантин. Удаления нет.
/// Идентификаторы сверяются с анализом: пути из интерфейса не принимаются.
pub fn clean_selected(
    fs: &dyn FsPort,
    quarantine: &Quarantine,
    analysis: &CleanupAnalysis,
    category_ids: &[String],
    now_unix: u64,
) -> Result<MoveOutcome, CleanupError> {
    let mut outcome = MoveOutcome::default();
    let selected: BTreeSet<&String> = category_ids.iter().collect();
    for category in &analysis.categories {
        if !selected.contains(&category.id) {
            continue;
        }
        let reason = format!("глубокая очистка: {}", category.title);
        let paths = analysis.files_of(&category.id).to_vec();
        if paths.is_empty() {
            continue;
        }
        outcome.merge(quarantine.move_many(fs, &paths, &reason, now_unix)?);
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake_fs::FakeFs;
    use std::sync::atomic::Ordering;

    const TEMP: &str = "C:\\Users\\Slava\\AppData\\Local\\Temp";
    const QROOT: &str = "C:\\Users\\Slava\\AppData\\Local\\OCHISTKA\\Quarantine";

    fn locations() -> Vec<CleanupLocation> {
        vec![
            CleanupLocation {
                id: "temp".into(),
                title: "Временные файлы".into(),
                root: TEMP.into(),
            },
            CleanupLocation {
                id: "chrome".into(),
                title: "Кэш Chrome".into(),
                root: "C:\\Users\\Slava\\AppData\\Local\\Google\\Chrome\\Cache".into(),
            },
        ]
    }

    fn sample_tree(fs: &FakeFs) {
        fs.file(&format!("{TEMP}\\a.tmp"), &[0u8; 100], 0);
        fs.file(&format!("{TEMP}\\sub\\b.tmp"), &[0u8; 250], 0);
        fs.file(&format!("{TEMP}\\sub\\deeper\\c.tmp"), &[0u8; 50], 0);
        fs.link(&format!("{TEMP}\\link"));
        fs.dir("C:\\Users\\Slava\\AppData\\Local\\Google\\Chrome\\Cache");
        fs.file(
            "C:\\Users\\Slava\\AppData\\Local\\Google\\Chrome\\Cache\\data_0",
            &[0u8; 1000],
            0,
        );
    }

    fn policy() -> PathPolicy {
        PathPolicy::new(&[
            TEMP,
            "C:\\Users\\Slava\\AppData\\Local\\Google\\Chrome\\Cache",
        ])
        .unwrap()
    }

    #[test]
    fn analysis_sums_sizes_and_counts_on_fake_tree() {
        let fs = FakeFs::new();
        sample_tree(&fs);
        let cancel = AtomicBool::new(false);
        let analysis = analyze(&fs, &policy(), &locations(), &cancel, Limits::default()).unwrap();

        let temp = &analysis.categories[0];
        assert!(temp.found && temp.complete);
        assert_eq!((temp.file_count, temp.bytes), (3, 400));
        let chrome = &analysis.categories[1];
        assert_eq!((chrome.file_count, chrome.bytes), (1, 1000));
        assert!(analysis
            .files_of("temp")
            .iter()
            .all(|p| !p.ends_with("link")));
    }

    #[test]
    fn missing_category_is_reported_as_not_found_with_zero_size() {
        let fs = FakeFs::new();
        let cancel = AtomicBool::new(false);
        let analysis = analyze(&fs, &policy(), &locations(), &cancel, Limits::default()).unwrap();
        assert!(analysis.categories.iter().all(|c| !c.found && c.bytes == 0));
    }

    #[test]
    fn file_in_two_categories_is_counted_once() {
        let fs = FakeFs::new();
        fs.file(&format!("{TEMP}\\x.tmp"), &[0u8; 10], 0);
        let mut locs = locations();
        locs.push(CleanupLocation {
            id: "again".into(),
            title: "Повтор".into(),
            root: TEMP.into(),
        });
        let cancel = AtomicBool::new(false);
        let analysis = analyze(&fs, &policy(), &locs, &cancel, Limits::default()).unwrap();
        assert_eq!(analysis.categories[0].file_count, 1);
        assert_eq!(analysis.categories[2].file_count, 0);
    }

    #[test]
    fn cancel_stops_analysis_with_cancelled_error() {
        let fs = FakeFs::new();
        sample_tree(&fs);
        let cancel = AtomicBool::new(true);
        let result = analyze(&fs, &policy(), &locations(), &cancel, Limits::default());
        assert_eq!(result, Err(CleanupError::Cancelled));
        assert_eq!(
            *fs.reads.lock().unwrap(),
            0,
            "no directory may be read after cancel"
        );
        cancel.store(false, Ordering::SeqCst);
    }

    #[test]
    fn entry_budget_marks_category_incomplete() {
        let fs = FakeFs::new();
        sample_tree(&fs);
        let cancel = AtomicBool::new(false);
        let limits = Limits {
            max_entries: 2,
            max_depth: 24,
        };
        let analysis = analyze(&fs, &policy(), &locations(), &cancel, limits).unwrap();
        assert!(!analysis.categories[0].complete);
    }

    #[test]
    fn clean_selected_moves_only_chosen_categories_and_never_deletes() {
        let fs = FakeFs::new();
        sample_tree(&fs);
        let cancel = AtomicBool::new(false);
        let analysis = analyze(&fs, &policy(), &locations(), &cancel, Limits::default()).unwrap();
        let quarantine = Quarantine::new(QROOT, policy()).unwrap();

        let outcome =
            clean_selected(&fs, &quarantine, &analysis, &["temp".to_string()], 42).unwrap();
        assert_eq!((outcome.moved, outcome.bytes, outcome.failed), (3, 400, 0));
        assert!(!fs.exists(&format!("{TEMP}\\a.tmp")));
        assert!(fs.exists("C:\\Users\\Slava\\AppData\\Local\\Google\\Chrome\\Cache\\data_0"));
        assert_eq!(quarantine.list(&fs).unwrap().len(), 3);
    }
}
