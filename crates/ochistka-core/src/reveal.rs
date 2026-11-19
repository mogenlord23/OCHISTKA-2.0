//! «Показать в Проводнике»: проверка пути и разбор причин. Без кода ОС.
//!
//! Проводник запускает платформа (`C:\Windows\explorer.exe` по абсолютному пути). Здесь только
//! правила: путь должен быть полным, без символов, которые ломают командную строку, и без `..`.

use thiserror::Error;

/// Причины отказа. Текст уже на русском, без технических слов.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RevealError {
    #[error("Путь не указан.")]
    Empty,
    #[error("Путь неполный, Проводник не сможет его открыть.")]
    NotAbsolute,
    #[error("В пути есть недопустимые символы, открывать его небезопасно.")]
    Unsafe,
    #[error("Файл или папка не найдены: их, возможно, уже удалили или перенесли.")]
    Missing,
    #[error("Не удалось открыть Проводник.")]
    Launch(String),
    #[error("{feature} недоступно на {os}.")]
    Unsupported { feature: String, os: String },
}

/// Приводит путь к виду для Проводника: убирает пробелы, кавычки по краям и префикс `\\?\`.
/// Принимает только полные пути вида `C:\...` или `\\сервер\ресурс\...`.
pub fn normalize_path(raw: &str) -> Result<String, RevealError> {
    let mut path = raw.trim();
    if path.len() >= 2 && path.starts_with('"') && path.ends_with('"') {
        path = path[1..path.len() - 1].trim();
    }
    path = path.strip_prefix(r"\\?\").unwrap_or(path);
    if path.is_empty() {
        return Err(RevealError::Empty);
    }
    if path
        .chars()
        .any(|c| c.is_control() || matches!(c, '"' | '<' | '>' | '|' | '*' | '?'))
    {
        return Err(RevealError::Unsafe);
    }
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    let unc = path.starts_with(r"\\") && path.len() > 2;
    if !drive && !unc {
        return Err(RevealError::NotAbsolute);
    }
    if path.split(['\\', '/']).any(|part| part == "..") {
        return Err(RevealError::Unsafe);
    }
    // `:` допустим только после буквы диска (иначе это поток данных NTFS или мусор).
    let rest = if drive { &path[2..] } else { path };
    if rest.contains(':') {
        return Err(RevealError::Unsafe);
    }
    let normalized = path.replace('/', "\\");
    let trimmed = normalized.trim_end_matches('\\');
    // «C:» без слэша означает текущую папку диска, возвращаем корень.
    if trimmed.len() == 2 && trimmed.ends_with(':') {
        return Ok(format!("{trimmed}\\"));
    }
    Ok(trimmed.to_string())
}

/// Аргументы для `explorer.exe`: файл выделяется в своей папке, папка открывается.
/// Путь уже прошёл `normalize_path`, значит кавычек внутри нет.
pub fn explorer_args(path: &str, is_dir: bool) -> String {
    if is_dir {
        format!("\"{path}\"")
    } else {
        format!("/select,\"{path}\"")
    }
}

/// Причина, по которой у процесса нет пути к файлу. Показывается человеку.
pub fn no_path_reason(pid: u32) -> String {
    if pid == 0 || pid == 4 {
        "Это системный процесс Windows, у него нет файла на диске.".to_string()
    } else {
        "Windows не отдаёт путь к этому процессу: он защищён или принадлежит другому \
         пользователю. С правами администратора такие пути иногда видны."
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_drive_path_is_accepted() {
        assert_eq!(
            normalize_path(r"C:\Windows\System32\notepad.exe").unwrap(),
            r"C:\Windows\System32\notepad.exe"
        );
    }

    #[test]
    fn quotes_spaces_and_long_prefix_are_removed() {
        assert_eq!(
            normalize_path(r#"  "C:\Program Files\App\app.exe"  "#).unwrap(),
            r"C:\Program Files\App\app.exe"
        );
        assert_eq!(
            normalize_path(r"\\?\C:\Tools\a.exe").unwrap(),
            r"C:\Tools\a.exe"
        );
    }

    #[test]
    fn trailing_slash_is_cut_but_drive_root_stays() {
        assert_eq!(
            normalize_path(r"C:\Program Files\App\").unwrap(),
            r"C:\Program Files\App"
        );
        assert_eq!(normalize_path(r"C:\").unwrap(), r"C:\");
    }

    #[test]
    fn relative_and_empty_paths_are_refused() {
        assert_eq!(normalize_path(""), Err(RevealError::Empty));
        assert_eq!(normalize_path("   "), Err(RevealError::Empty));
        assert_eq!(normalize_path("notepad.exe"), Err(RevealError::NotAbsolute));
        assert_eq!(
            normalize_path(r"\Windows\x.exe"),
            Err(RevealError::NotAbsolute)
        );
    }

    #[test]
    fn dangerous_characters_and_parent_parts_are_refused() {
        for bad in [
            r#"C:\a" /e,cmd"#,
            "C:\\a\\b|c",
            "C:\\a\\*.exe",
            "C:\\a\nb",
            r"C:\a\..\b",
            r"C:\a\b.txt:stream",
        ] {
            assert_eq!(normalize_path(bad), Err(RevealError::Unsafe), "{bad}");
        }
    }

    #[test]
    fn unc_path_is_accepted() {
        assert_eq!(
            normalize_path(r"\\server\share\a.exe").unwrap(),
            r"\\server\share\a.exe"
        );
    }

    #[test]
    fn explorer_arguments_select_files_and_open_folders() {
        assert_eq!(
            explorer_args(r"C:\Program Files\A\a.exe", false),
            r#"/select,"C:\Program Files\A\a.exe""#
        );
        assert_eq!(
            explorer_args(r"C:\Program Files\A", true),
            r#""C:\Program Files\A""#
        );
    }

    #[test]
    fn missing_path_reason_is_human() {
        assert!(no_path_reason(4).contains("системный"));
        assert!(no_path_reason(1234).contains("защищён"));
    }
}
