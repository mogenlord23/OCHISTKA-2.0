//! Белый список путей для очистки и карантина. Чистый код: работает со строками, диск не трогает.
//! Системные папки отсекаются и здесь, даже если кто-то добавил их в белый список.

use crate::fs::CleanupError;

/// Верхние папки системного диска, которые никогда не трогаем.
const SYSTEM_TOP_LEVEL: [&str; 9] = [
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "system volume information",
    "$recycle.bin",
    "boot",
    "recovery",
    "perflogs",
];

/// Приводит путь к виду для сравнения: нижний регистр, разделители `\`, без хвостового `\`.
pub fn normalize(path: &str) -> String {
    let replaced = path.trim().replace('/', "\\").to_lowercase();
    let trimmed = replaced.trim_end_matches('\\');
    if trimmed.is_empty() {
        "\\".to_string()
    } else {
        trimmed.to_string()
    }
}

fn has_drive(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Путь с корня: `X:\...`, `\\сервер\...` или `\...`. Относительные пути отвергаются.
fn is_rooted(path: &str) -> bool {
    (has_drive(path) && path.as_bytes().get(2) == Some(&b'\\')) || path.starts_with('\\')
}

fn is_drive_root(normalized: &str) -> bool {
    normalized == "\\" || (has_drive(normalized) && normalized.len() == 2)
}

fn has_parent_reference(normalized: &str) -> bool {
    normalized.split('\\').any(|part| part == "..")
}

/// Системная папка: верхний уровень диска — из списка защищённых папок Windows.
pub fn is_system_path(normalized: &str) -> bool {
    let rest = if has_drive(normalized) {
        &normalized[2..]
    } else {
        normalized
    };
    let top = rest
        .trim_start_matches('\\')
        .split('\\')
        .next()
        .unwrap_or("");
    SYSTEM_TOP_LEVEL.contains(&top)
}

/// Общая проверка пути: абсолютный, без `..` и управляющих символов, не корень диска и не системная папка.
fn validate_shape(normalized: &str, original: &str) -> Result<(), CleanupError> {
    if original.chars().any(char::is_control) {
        return Err(CleanupError::Refused(format!(
            "управляющие символы: {original:?}"
        )));
    }
    if !is_rooted(normalized) {
        return Err(CleanupError::Refused(format!(
            "путь не абсолютный: {original}"
        )));
    }
    if has_parent_reference(normalized) {
        return Err(CleanupError::Refused(format!(
            "путь содержит «..»: {original}"
        )));
    }
    if is_drive_root(normalized) {
        return Err(CleanupError::Refused(format!("корень диска: {original}")));
    }
    if is_system_path(normalized) {
        return Err(CleanupError::Refused(format!(
            "системная папка: {original}"
        )));
    }
    Ok(())
}

/// Проверка корня белого списка и карантина.
pub fn validate_root(original: &str) -> Result<String, CleanupError> {
    let normalized = normalize(original);
    validate_shape(&normalized, original)?;
    Ok(normalized)
}

/// Белый список: путь разрешён, если он равен одному из корней или лежит внутри него.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPolicy {
    roots: Vec<String>,
}

impl PathPolicy {
    /// Строит список из корней. Корень, который нарушает правила, — ошибка всей конфигурации.
    pub fn new(roots: &[&str]) -> Result<Self, CleanupError> {
        let mut normalized: Vec<String> = Vec::new();
        for root in roots {
            let value = validate_root(root)?;
            if !normalized.contains(&value) {
                normalized.push(value);
            }
        }
        Ok(Self { roots: normalized })
    }

    /// Нормализованные корни, для проверок вложенности.
    pub fn roots(&self) -> &[String] {
        &self.roots
    }

    /// Разрешает путь, только если он внутри белого списка и не нарушает правил.
    pub fn check(&self, path: &str) -> Result<(), CleanupError> {
        let normalized = normalize(path);
        validate_shape(&normalized, path)?;
        let inside = self
            .roots
            .iter()
            .any(|root| normalized == *root || normalized.starts_with(&format!("{root}\\")));
        if inside {
            Ok(())
        } else {
            Err(CleanupError::Refused(format!("вне белого списка: {path}")))
        }
    }
}

/// Проверка «X:» или «X:\» и возврат канонической формы `X:\`.
pub fn validate_drive(input: &str) -> Result<String, CleanupError> {
    let trimmed = input.trim();
    let bytes = trimmed.as_bytes();
    let shaped = match bytes.len() {
        2 => bytes[1] == b':',
        3 => bytes[1] == b':' && bytes[2] == b'\\',
        _ => false,
    };
    if shaped && bytes[0].is_ascii_alphabetic() {
        Ok(format!("{}:\\", (bytes[0] as char).to_ascii_uppercase()))
    } else {
        Err(CleanupError::Refused(format!("некорректный диск: {input}")))
    }
}

/// Последний компонент пути (имя файла). Пустая строка, если его нет.
pub fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/'])
        .find(|part| !part.is_empty())
        .unwrap_or("")
}

/// Папка, в которой лежит путь.
pub fn parent_dir(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches(['\\', '/']);
    trimmed
        .rfind(['\\', '/'])
        .map(|index| &trimmed[..index])
        .filter(|parent| !parent.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMP: &str = "C:\\Users\\Slava\\AppData\\Local\\Temp";

    #[test]
    fn normalize_unifies_case_separators_and_trailing_slash() {
        assert_eq!(normalize("C:/Users/Slava/"), "c:\\users\\slava");
        assert_eq!(normalize("c:\\users\\slava"), "c:\\users\\slava");
    }

    #[test]
    fn file_inside_allowed_root_is_allowed() {
        let policy = PathPolicy::new(&[TEMP]).unwrap();
        assert!(policy.check(&format!("{TEMP}\\a\\b.tmp")).is_ok());
        assert!(policy.check(TEMP).is_ok());
    }

    #[test]
    fn sibling_with_same_prefix_is_not_inside_root() {
        let policy = PathPolicy::new(&[TEMP]).unwrap();
        assert!(matches!(
            policy.check("C:\\Users\\Slava\\AppData\\Local\\Temp2\\x.tmp"),
            Err(CleanupError::Refused(_))
        ));
    }

    #[test]
    fn system_folder_is_refused_even_when_a_parent_is_allowed() {
        let policy = PathPolicy::new(&["C:\\"]).err();
        assert!(matches!(policy, Some(CleanupError::Refused(_))));
        let policy = PathPolicy::new(&["C:\\Users\\Slava\\AppData\\Local"]).unwrap();
        assert!(matches!(
            policy.check("C:\\Windows\\Temp\\a.tmp"),
            Err(CleanupError::Refused(_))
        ));
        assert!(matches!(
            policy.check("C:\\Program Files\\x\\a.dll"),
            Err(CleanupError::Refused(_))
        ));
        assert!(is_system_path("c:\\programdata\\x"));
    }

    #[test]
    fn parent_reference_is_refused() {
        let policy = PathPolicy::new(&[TEMP]).unwrap();
        assert!(matches!(
            policy.check(&format!("{TEMP}\\..\\..\\Documents\\a.txt")),
            Err(CleanupError::Refused(_))
        ));
    }

    #[test]
    fn relative_and_control_character_paths_are_refused() {
        let policy = PathPolicy::new(&[TEMP]).unwrap();
        assert!(policy.check("Temp\\a.tmp").is_err());
        assert!(policy.check(&format!("{TEMP}\\a\nb.tmp")).is_err());
    }

    #[test]
    fn drive_root_cannot_be_a_root() {
        assert!(PathPolicy::new(&["D:\\"]).is_err());
        assert!(PathPolicy::new(&["D:"]).is_err());
    }

    #[test]
    fn validate_drive_accepts_letter_with_optional_slash() {
        assert_eq!(validate_drive("c:").unwrap(), "C:\\");
        assert_eq!(validate_drive(" D:\\ ").unwrap(), "D:\\");
        assert!(validate_drive("C:\\Windows").is_err());
        assert!(validate_drive("1:").is_err());
    }

    #[test]
    fn file_name_and_parent_split_on_both_separators() {
        assert_eq!(file_name("C:\\a\\b.txt"), "b.txt");
        assert_eq!(file_name("/tmp/x/y"), "y");
        assert_eq!(parent_dir("C:\\a\\b.txt"), Some("C:\\a"));
        assert_eq!(parent_dir("C:"), None);
    }
}
