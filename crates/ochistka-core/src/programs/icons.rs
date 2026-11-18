//! Значки программ и процессов: разбор пути из `DisplayIcon`, проверка пути и небольшой LRU-кэш.
//! Чистый код. Сам значок извлекает адаптер платформы (`ProgramsPort::icon_png_base64`).

use std::collections::VecDeque;
use std::sync::Mutex;

/// Расширения, из которых разрешено брать значок.
const ICON_EXTENSIONS: [&str; 4] = ["exe", "dll", "ico", "lnk"];

/// Путь файла из значения `DisplayIcon`: `"C:\A\a.exe",0`, `C:\A\a.exe,-101`, `C:\A\a.ico`.
/// Индекс значка отбрасывается (берётся значок файла по умолчанию). `None` — путь не годится.
pub fn parse_display_icon(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let path = if let Some(rest) = raw.strip_prefix('"') {
        rest.split('"').next()?.to_string()
    } else {
        match raw.rsplit_once(',') {
            Some((head, tail)) if tail.trim().parse::<i32>().is_ok() => head.to_string(),
            _ => raw.to_string(),
        }
    };
    validate_icon_path(&path).ok()
}

/// Проверка пути для извлечения значка: абсолютный путь с буквой диска, без `..`, `%`,
/// управляющих символов и с известным расширением. Возвращает путь без пробелов по краям.
pub fn validate_icon_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.chars().any(char::is_control) || trimmed.contains('%') {
        return Err("недопустимые символы в пути".into());
    }
    let bytes = trimmed.as_bytes();
    let absolute =
        bytes.len() > 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\';
    if !absolute {
        return Err("путь не абсолютный".into());
    }
    if trimmed.split('\\').any(|part| part == "..") {
        return Err("путь содержит «..»".into());
    }
    let extension = trimmed
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();
    if !ICON_EXTENSIONS.contains(&extension.as_str()) {
        return Err("значок можно брать только из exe, dll, ico и lnk".into());
    }
    Ok(trimmed.to_string())
}

/// Адрес данных для `<img src>`: PNG в base64.
pub fn png_data_url(base64_png: &str) -> String {
    format!("data:image/png;base64,{base64_png}")
}

/// Кэш значков с вытеснением давно не использованных. Ключ — путь в нижнем регистре.
/// Запоминает и неудачи (`None`), чтобы не открывать один и тот же файл снова и снова.
pub struct IconCache {
    capacity: usize,
    inner: Mutex<VecDeque<(String, Option<String>)>>,
}

impl IconCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            inner: Mutex::new(VecDeque::new()),
        }
    }

    /// `Some(значок или None-неудача)`, если путь уже встречался; запись становится «свежей».
    pub fn get(&self, path: &str) -> Option<Option<String>> {
        let key = path.to_lowercase();
        let mut items = self.inner.lock().ok()?;
        let position = items.iter().position(|(k, _)| *k == key)?;
        let entry = items.remove(position)?;
        let value = entry.1.clone();
        items.push_back(entry);
        Some(value)
    }

    pub fn put(&self, path: &str, value: Option<String>) {
        let key = path.to_lowercase();
        let Ok(mut items) = self.inner.lock() else {
            return;
        };
        items.retain(|(k, _)| *k != key);
        items.push_back((key, value));
        while items.len() > self.capacity {
            items.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|items| items.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_icon_forms_are_parsed() {
        assert_eq!(
            parse_display_icon(r#""C:\Program Files\A\a.exe",0"#).as_deref(),
            Some(r"C:\Program Files\A\a.exe")
        );
        assert_eq!(
            parse_display_icon(r"C:\A\a.exe,-101").as_deref(),
            Some(r"C:\A\a.exe")
        );
        assert_eq!(
            parse_display_icon(r"C:\A\a.ico").as_deref(),
            Some(r"C:\A\a.ico")
        );
    }

    #[test]
    fn display_icon_rejects_unsafe_values() {
        assert_eq!(parse_display_icon(r"a.exe"), None);
        assert_eq!(parse_display_icon(r"%SystemRoot%\x.exe"), None);
        assert_eq!(parse_display_icon(r"C:\A\..\b.exe"), None);
        assert_eq!(parse_display_icon(r"C:\A\data.txt"), None);
        assert_eq!(parse_display_icon(""), None);
    }

    #[test]
    fn data_url_has_png_prefix() {
        assert_eq!(png_data_url("QUJD"), "data:image/png;base64,QUJD");
    }

    #[test]
    fn cache_evicts_the_least_recently_used_and_ignores_case() {
        let cache = IconCache::new(2);
        cache.put(r"C:\A.exe", Some("a".into()));
        cache.put(r"C:\B.exe", None);
        assert_eq!(cache.get(r"c:\a.exe"), Some(Some("a".into())));
        cache.put(r"C:\C.exe", Some("c".into()));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(r"C:\B.exe"), None, "B was the oldest");
        assert_eq!(cache.get(r"C:\A.exe"), Some(Some("a".into())));
    }

    #[test]
    fn cache_remembers_failures() {
        let cache = IconCache::new(4);
        cache.put(r"C:\X.exe", None);
        assert_eq!(cache.get(r"C:\X.exe"), Some(None));
        assert!(!cache.is_empty());
    }
}
