//! Признаки файла для эвристики: имя, размер, папка загрузок и автозагрузки.

use std::path::Path;

use ochistka_core::scan::FileFacts;
use ochistka_core::system::CoreError;

/// Папка «Загрузки» пользователя по пути профиля. Только строка, диск не читается.
pub fn downloads_from_profile(profile: &str) -> String {
    format!("{}\\Downloads", profile.trim_end_matches(['\\', '/']))
}

/// Папка «Загрузки» текущего пользователя (из USERPROFILE), если переменная задана.
pub fn default_downloads() -> Option<String> {
    std::env::var("USERPROFILE")
        .ok()
        .map(|profile| downloads_from_profile(&profile))
}

/// Собирает признаки файла с диска. Читает только метаданные, содержимое не открывает.
pub fn file_facts(path: &Path) -> Result<FileFacts, CoreError> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| CoreError::Probe(format!("файл недоступен: {err}")))?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut facts = classify(path, metadata.len());
    facts.file_name = file_name;
    Ok(facts)
}

/// Определяет папку по пути. Вынесено отдельно, чтобы проверять без файловой системы.
fn classify(path: &Path, size_bytes: u64) -> FileFacts {
    let lower = path.to_string_lossy().to_lowercase();
    FileFacts {
        file_name: String::new(),
        size_bytes,
        in_downloads: lower.contains("\\downloads\\") || lower.contains("/downloads/"),
        in_startup: lower.contains("\\startup\\") || lower.contains("/startup/"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir_with(name: &str, bytes: &[u8]) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "ochistka-files-{}-{}",
            std::process::id(),
            name.len()
        ));
        let downloads = root.join("Downloads");
        std::fs::create_dir_all(&downloads).unwrap();
        let file = downloads.join(name);
        std::fs::write(&file, bytes).unwrap();
        (root, file)
    }

    #[test]
    fn file_in_downloads_is_marked_with_its_size_and_name() {
        let (root, file) = temp_dir_with("invoice.pdf.exe", &[0u8; 300]);
        let facts = file_facts(&file).expect("file exists");
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(facts.file_name, "invoice.pdf.exe");
        assert_eq!(facts.size_bytes, 300);
        assert!(facts.in_downloads);
        assert!(!facts.in_startup);
    }

    #[test]
    fn startup_folder_is_detected() {
        let path = Path::new("C:\\Users\\Slava\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\tool.exe");
        let facts = classify(path, 10);
        assert!(facts.in_startup);
        assert!(!facts.in_downloads);
    }

    #[test]
    fn missing_file_is_a_probe_error() {
        assert!(matches!(
            file_facts(Path::new("Z:/definitely/missing.exe")),
            Err(CoreError::Probe(_))
        ));
    }
    #[test]
    fn downloads_folder_is_built_from_the_profile_path() {
        assert_eq!(
            downloads_from_profile("C:\\Users\\Slava\\"),
            "C:\\Users\\Slava\\Downloads"
        );
        assert_eq!(
            downloads_from_profile("C:\\Users\\Slava"),
            "C:\\Users\\Slava\\Downloads"
        );
    }
}
