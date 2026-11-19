//! Дисковый кэш значков в папке данных приложения: один маленький файл на ключ
//! (путь + время изменения + размер). Ошибки записи и чтения не критичны: без кэша значок
//! просто извлекается заново.

use std::fs;
use std::path::{Path, PathBuf};

use ochistka_core::programs::icons::IconStore;

/// Не больше стольких файлов: при превышении удаляются самые старые.
const MAX_FILES: usize = 6000;

pub struct DiskIconStore {
    dir: PathBuf,
}

impl DiskIconStore {
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }

    /// Папка кэша по умолчанию: `%LOCALAPPDATA%\OCHISTKA\icon-cache`, иначе папка временных файлов.
    pub fn default_dir() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("XDG_CACHE_HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("OCHISTKA")
            .join("icon-cache")
    }

    fn file(&self, key: &str) -> Option<PathBuf> {
        // Ключ — только шестнадцатеричные цифры: вне папки кэша писать нельзя.
        (!key.is_empty() && key.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| self.dir.join(format!("{key}.txt")))
    }

    /// Удаляет самые старые файлы сверх лимита. Вызывается один раз при запуске.
    pub fn prune(&self, max_files: usize) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(Result::ok)
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        if files.len() <= max_files {
            return;
        }
        files.sort();
        for (_, path) in files.iter().take(files.len() - max_files) {
            let _ = fs::remove_file(path);
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Default for DiskIconStore {
    fn default() -> Self {
        let store = Self::new(Self::default_dir());
        store.prune(MAX_FILES);
        store
    }
}

impl IconStore for DiskIconStore {
    fn load(&self, key: &str) -> Option<String> {
        let text = fs::read_to_string(self.file(key)?).ok()?;
        text.starts_with("data:image/png;base64,").then_some(text)
    }

    fn save(&self, key: &str, data_url: &str) {
        let Some(path) = self.file(key) else {
            return;
        };
        let temp = path.with_extension("tmp");
        if fs::write(&temp, data_url).is_ok() && fs::rename(&temp, &path).is_err() {
            let _ = fs::remove_file(&temp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-icons-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn saved_icon_is_loaded_back_by_a_new_store() {
        let dir = temp("roundtrip");
        DiskIconStore::new(dir.clone()).save("abcdef0123456789", "data:image/png;base64,QUJD");
        let again = DiskIconStore::new(dir.clone());
        assert_eq!(
            again.load("abcdef0123456789").as_deref(),
            Some("data:image/png;base64,QUJD")
        );
        assert_eq!(again.load("0000000000000000"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn keys_that_could_escape_the_folder_are_ignored() {
        let dir = temp("escape");
        let store = DiskIconStore::new(dir.clone());
        store.save(r"..\evil", "data:image/png;base64,QUJD");
        store.save("../evil", "data:image/png;base64,QUJD");
        assert_eq!(store.load("../evil"), None);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_is_treated_as_a_miss() {
        let dir = temp("corrupt");
        let store = DiskIconStore::new(dir.clone());
        fs::write(dir.join("abc123.txt"), "garbage").unwrap();
        assert_eq!(store.load("abc123"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prune_keeps_only_the_newest_files() {
        let dir = temp("prune");
        let store = DiskIconStore::new(dir.clone());
        for i in 0..5 {
            store.save(&format!("a{i}"), "data:image/png;base64,QQ==");
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        store.prune(2);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        assert!(store.load("a4").is_some());
        assert!(store.load("a0").is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
