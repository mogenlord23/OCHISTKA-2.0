//! Значки программ и процессов: разбор `DisplayIcon` (кавычки, `,индекс`, `%VAR%`), выбор файла
//! значка, ключ дискового кэша (путь + время изменения) и LRU-кэш в памяти.
//! Чистый код. Сам значок извлекает адаптер платформы (`ProgramsPort::icon_png_base64`).

use std::collections::VecDeque;
use std::sync::Mutex;

use super::{InstalledProgram, ProgramKind, ProgramsPort};

/// Расширения, из которых разрешено брать значок.
const ICON_EXTENSIONS: [&str; 4] = ["exe", "dll", "ico", "lnk"];

/// Сторона готового значка в пикселях: 2x от самой крупной плитки (32 px) для HiDPI.
pub const ICON_PX: u32 = 64;

/// Подставляет `%NAME%` через `env`. Незнакомая переменная или одинокий `%` дают `None`.
pub fn expand_env(raw: &str, env: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('%')?;
        let name = &after[..end];
        if name.is_empty() {
            return None;
        }
        out.push_str(&env(name)?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Файл значка и его индекс из значения `DisplayIcon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconTarget {
    pub path: String,
    pub index: i32,
}

/// Разбор `DisplayIcon`: `"C:\A\a.exe",0`, `C:\A\a.exe,-101`, `%ProgramFiles%\A\a.ico`.
/// Переменные окружения подставляются через `env`; путь проходит `validate_icon_path`.
pub fn display_icon_target(raw: &str, env: &dyn Fn(&str) -> Option<String>) -> Option<IconTarget> {
    let raw = raw.trim();
    let (path, index) = if let Some(rest) = raw.strip_prefix('"') {
        let (path, tail) = rest.split_once('"')?;
        let index = tail
            .trim()
            .strip_prefix(',')
            .and_then(|n| n.trim().parse::<i32>().ok())
            .unwrap_or(0);
        (path.to_string(), index)
    } else {
        match raw.rsplit_once(',') {
            Some((head, tail)) if tail.trim().parse::<i32>().is_ok() => {
                (head.trim().to_string(), tail.trim().parse().unwrap_or(0))
            }
            _ => (raw.to_string(), 0),
        }
    };
    let path = validate_icon_path(&expand_env(&path, env)?).ok()?;
    Some(IconTarget { path, index })
}

/// Путь файла из `DisplayIcon` без переменных окружения (они здесь не подставляются).
pub fn parse_display_icon(raw: &str) -> Option<String> {
    display_icon_target(raw, &|_| None).map(|target| target.path)
}

/// Путь программы удаления из `UninstallString`: в кавычках или до первого `.exe`.
/// `MsiExec.exe` без пути и прочее неабсолютное отбрасывается проверкой пути.
pub fn uninstall_exe_path(raw: &str, env: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let raw = raw.trim();
    let candidate = if let Some(rest) = raw.strip_prefix('"') {
        rest.split('"').next()?.to_string()
    } else {
        let end = raw.to_ascii_lowercase().find(".exe")? + 4;
        raw[..end].to_string()
    };
    validate_icon_path(&expand_env(&candidate, env)?).ok()
}

/// Папка файла: всё до последнего `\`.
pub fn parent_dir(path: &str) -> Option<&str> {
    path.rsplit_once('\\').map(|(dir, _)| dir)
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

/// Отпечаток файла для ключа кэша: время изменения (секунды) и размер.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileStamp {
    pub modified_secs: u64,
    pub len: u64,
}

/// Ключ дискового кэша: хэш пути (без учёта регистра), времени изменения, размера и стороны
/// значка. Изменился файл или размер значка, и ключ другой.
pub fn icon_cache_key(path: &str, stamp: Option<FileStamp>, px: u32) -> String {
    let stamp = stamp.unwrap_or_default();
    let text = format!(
        "v1|{}|{}|{}|{px}",
        path.to_lowercase(),
        stamp.modified_secs,
        stamp.len
    );
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Хранилище готовых значков на диске (папка данных приложения). Ошибки молча игнорируются:
/// кэш только ускоряет, без него значок просто извлекается заново.
pub trait IconStore: Send + Sync {
    fn load(&self, key: &str) -> Option<String>;
    fn save(&self, key: &str, data_url: &str);
}

/// Хранилище-пустышка: ничего не помнит.
pub struct NoIconStore;

impl IconStore for NoIconStore {
    fn load(&self, _key: &str) -> Option<String> {
        None
    }
    fn save(&self, _key: &str, _data_url: &str) {}
}

/// Всё, что нужно для получения значка: порт, дисковый и memory-кэш.
pub struct IconContext<'a> {
    pub port: &'a dyn ProgramsPort,
    pub store: &'a dyn IconStore,
    pub memory: &'a IconCache,
}

impl IconContext<'_> {
    fn env(&self) -> impl Fn(&str) -> Option<String> + '_ {
        |name| self.port.env_var(name)
    }

    /// Значок файла: память, затем диск (ключ путь + mtime), затем извлечение. Неудача
    /// запоминается только в памяти. Путь должен быть проверен `validate_icon_path`.
    pub fn icon_for_path(&self, path: &str) -> Option<String> {
        if let Some(known) = self.memory.get(path) {
            return known;
        }
        let key = icon_cache_key(path, self.port.file_stamp(path), ICON_PX);
        let found = self.store.load(&key).or_else(|| {
            let url = self
                .port
                .icon_png_base64(path)
                .ok()
                .map(|png| png_data_url(&png))?;
            self.store.save(&key, &url);
            Some(url)
        });
        self.memory.put(path, found.clone());
        found
    }

    /// Значок по пути от интерфейса (процесс): проверка и `icon_for_path`.
    pub fn icon_for_untrusted_path(&self, path: &str) -> Result<Option<String>, String> {
        let safe = validate_icon_path(path)?;
        Ok(self.icon_for_path(&safe))
    }

    fn main_exe_in(&self, dir: &str, program_name: &str) -> Option<String> {
        let files = self.port.list_file_names(dir).ok()?;
        let exe = pick_program_exe(program_name, &files)?;
        validate_icon_path(&format!("{dir}\\{exe}")).ok()
    }

    /// Значок программы. Порядок: `DisplayIcon`; логотип пакета Store; основной exe в папке
    /// установки; основной exe рядом с деинсталлятором; сам деинсталлятор. `None` — буква.
    pub fn program_icon(&self, program: &InstalledProgram) -> Option<String> {
        let env = self.env();
        if let Some(target) = program
            .display_icon
            .as_deref()
            .and_then(|raw| display_icon_target(raw, &env))
        {
            if let Some(url) = self.icon_for_path(&target.path) {
                return Some(url);
            }
        }
        let location = program
            .install_location
            .as_deref()
            .and_then(|raw| expand_env(raw.trim_end_matches('\\'), &env))
            .filter(|l| valid_location(l));
        if program.kind == ProgramKind::Store {
            return self.store_logo(location.as_deref()?);
        }
        if let Some(path) = location
            .as_deref()
            .and_then(|dir| self.main_exe_in(dir, &program.name))
        {
            if let Some(url) = self.icon_for_path(&path) {
                return Some(url);
            }
        }
        let uninstaller = program
            .uninstall_string
            .as_deref()
            .and_then(|raw| uninstall_exe_path(raw, &env))?;
        if let Some(path) = parent_dir(&uninstaller)
            .filter(|dir| location.as_deref() != Some(*dir))
            .and_then(|dir| self.main_exe_in(dir, &program.name))
        {
            if let Some(url) = self.icon_for_path(&path) {
                return Some(url);
            }
        }
        self.icon_for_path(&uninstaller)
    }

    /// Логотип пакета Store из манифеста (только чтение).
    fn store_logo(&self, location: &str) -> Option<String> {
        let manifest = self
            .port
            .read_small_file(&format!("{location}\\AppxManifest.xml"), MAX_MANIFEST_BYTES)
            .ok()?;
        let relative = manifest_logo(&String::from_utf8_lossy(&manifest))?.replace('/', "\\");
        let dir = relative.rsplit_once('\\').map_or("", |(dir, _)| dir);
        let folder = if dir.is_empty() {
            location.to_string()
        } else {
            format!("{location}\\{dir}")
        };
        let name = pick_logo_file(&relative, &self.port.list_file_names(&folder).ok()?)?;
        let bytes = self
            .port
            .read_small_file(&format!("{folder}\\{name}"), MAX_LOGO_BYTES)
            .ok()?;
        png_data_url_from_bytes(&bytes)
    }
}

/// Первые 8 байт любого PNG.
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// Предел размера файла логотипа, который читаем из пакета.
pub const MAX_LOGO_BYTES: usize = 1024 * 1024;
/// Предел размера манифеста пакета.
pub const MAX_MANIFEST_BYTES: usize = 256 * 1024;

/// Файл начинается с PNG-заголовка.
pub fn looks_like_png(bytes: &[u8]) -> bool {
    bytes.len() > PNG_MAGIC.len() && bytes[..PNG_MAGIC.len()] == PNG_MAGIC
}

/// Base64 (RFC 4648, с `=`) без внешних крейтов: логотип пакета читается как есть.
pub fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Адрес `data:` из байтов PNG; `None`, если это не PNG.
pub fn png_data_url_from_bytes(bytes: &[u8]) -> Option<String> {
    looks_like_png(bytes).then(|| png_data_url(&base64_encode(bytes)))
}

/// Путь логотипа из `AppxManifest.xml`: значок списка приложений (`Square44x44Logo`),
/// затем плитка (`Square150x150Logo`), затем `<Logo>` свойств пакета. Только чтение текста.
pub fn manifest_logo(xml: &str) -> Option<String> {
    let attribute = |name: &str| -> Option<String> {
        let marker = format!("{name}=\"");
        let rest = &xml[xml.find(&marker)? + marker.len()..];
        let value = rest[..rest.find('"')?].trim();
        (!value.is_empty() && !value.contains("..") && !value.starts_with("ms-"))
            .then(|| value.to_string())
    };
    attribute("Square44x44Logo")
        .or_else(|| attribute("Square150x150Logo"))
        .or_else(|| {
            let rest = xml.split_once("<Logo>")?.1;
            let value = rest.split_once("</Logo>")?.0.trim();
            (!value.is_empty() && !value.contains("..")).then(|| value.to_string())
        })
}

/// Разбор имени варианта логотипа: `(числовой признак, это targetsize)`; `None` — не вариант
/// этого логотипа или вариант для высокой контрастности.
fn logo_variant(stem: &str, ext: &str, candidate: &str) -> Option<(u32, bool)> {
    let lower = candidate.to_ascii_lowercase();
    let stem = stem.to_ascii_lowercase();
    let ext = format!(".{}", ext.to_ascii_lowercase());
    let middle = lower.strip_prefix(&stem)?.strip_suffix(&ext)?;
    if middle.is_empty() {
        return Some((0, false));
    }
    let middle = middle.strip_prefix('.')?;
    if middle.contains("contrast") {
        return None;
    }
    let number = |prefix: &str| -> Option<u32> {
        let digits: String = middle
            .strip_prefix(prefix)?
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    };
    if let Some(size) = number("targetsize-") {
        return Some((size, true));
    }
    number("scale-").map(|scale| (scale, false))
}

/// Выбирает файл логотипа среди соседей по папке. Пакет хранит не сам `Logo.png`,
/// а варианты `Logo.scale-200.png` и `Logo.targetsize-48.png`. Берём `targetsize`,
/// ближайший к 48 px, иначе масштаб, ближайший к 200 %, иначе файл как в манифесте.
/// Возвращает имя файла в папке (без пути).
pub fn pick_logo_file(relative: &str, siblings: &[String]) -> Option<String> {
    let file = relative.rsplit(['\\', '/']).next()?;
    let (stem, ext) = file.rsplit_once('.')?;
    let mut sized: Vec<(u32, &String)> = Vec::new();
    let mut scaled: Vec<(u32, &String)> = Vec::new();
    let mut exact: Option<&String> = None;
    for name in siblings {
        match logo_variant(stem, ext, name) {
            Some((0, false)) => exact = Some(name),
            Some((size, true)) => sized.push((size, name)),
            Some((scale, false)) => scaled.push((scale, name)),
            None => {}
        }
    }
    let nearest = |items: &mut Vec<(u32, &String)>, goal: u32| {
        items.sort_by_key(|(value, name)| (value.abs_diff(goal), (*name).clone()));
        items.first().map(|(_, name)| (*name).clone())
    };
    nearest(&mut sized, 48)
        .or_else(|| nearest(&mut scaled, 200))
        .or_else(|| exact.cloned())
}

fn normalized(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Имена исполняемых файлов, которые не годятся как значок программы.
const SKIPPED_EXE: [&str; 8] = [
    "unins", "uninst", "setup", "update", "crash", "helper", "install", "report",
];

/// Выбирает основной exe программы в её папке по имени программы. Без уверенности
/// возвращает `None` (лучше заглушка, чем значок чужого файла).
pub fn pick_program_exe(program_name: &str, files: &[String]) -> Option<String> {
    let wanted = normalized(program_name);
    let exes: Vec<(&String, String)> = files
        .iter()
        .filter(|name| name.to_ascii_lowercase().ends_with(".exe"))
        .map(|name| (name, normalized(&name[..name.len() - 4])))
        .filter(|(_, stem)| !stem.is_empty() && !SKIPPED_EXE.iter().any(|bad| stem.contains(bad)))
        .collect();
    if let Some((name, _)) = exes.iter().find(|(_, stem)| *stem == wanted) {
        return Some((*name).clone());
    }
    let related = exes
        .iter()
        .filter(|(_, stem)| {
            stem.len() >= 3 && (wanted.contains(stem.as_str()) || stem.contains(&wanted))
        })
        .min_by_key(|(_, stem)| stem.len());
    if let Some((name, _)) = related {
        return Some((*name).clone());
    }
    match exes.as_slice() {
        [(name, _)] => Some((*name).clone()),
        _ => None,
    }
}

/// Папка установки пригодна для чтения: абсолютная, с буквой диска, без `..` и `%`.
pub fn valid_location(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'\\'
        && !path.chars().any(char::is_control)
        && !path.contains('%')
        && !path.split('\\').any(|part| part == "..")
}

/// Адрес данных для `<img src>`: PNG в base64. Лишний префикс `data:` и пробелы убираются.
pub fn png_data_url(base64_png: &str) -> String {
    let body = base64_png
        .trim()
        .strip_prefix("data:image/png;base64,")
        .unwrap_or_else(|| base64_png.trim());
    let body: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    format!("data:image/png;base64,{body}")
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
    use crate::programs::{
        InstalledProgram, ProgramKind, ProgramsPort, RawProgramRecord, RawStartup,
    };
    use crate::system::CoreError;
    use std::collections::BTreeMap;

    /// Реальный PNG 1x1 (70 байт).
    const PNG_1X1: [u8; 70] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8,
        0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99, 0x3D, 0x1D, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[derive(Default)]
    struct FakePort {
        files: BTreeMap<String, Vec<u8>>,
        dirs: BTreeMap<String, Vec<String>>,
        icons: BTreeMap<String, String>,
        env: BTreeMap<String, String>,
        stamps: BTreeMap<String, FileStamp>,
        extractions: std::sync::atomic::AtomicUsize,
    }

    impl ProgramsPort for FakePort {
        fn installed_records(&self) -> Result<Vec<RawProgramRecord>, CoreError> {
            Ok(Vec::new())
        }
        fn startup_records(&self) -> Result<Vec<RawStartup>, CoreError> {
            Ok(Vec::new())
        }
        fn env_var(&self, name: &str) -> Option<String> {
            self.env.get(&name.to_ascii_lowercase()).cloned()
        }
        fn file_stamp(&self, path: &str) -> Option<FileStamp> {
            self.stamps.get(path).copied()
        }
        fn icon_png_base64(&self, path: &str) -> Result<String, CoreError> {
            self.extractions
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.icons
                .get(path)
                .cloned()
                .ok_or_else(|| CoreError::Probe("нет значка".into()))
        }
        fn list_file_names(&self, dir: &str) -> Result<Vec<String>, CoreError> {
            self.dirs
                .get(dir)
                .cloned()
                .ok_or_else(|| CoreError::Probe("нет папки".into()))
        }
        fn read_small_file(&self, path: &str, _max: usize) -> Result<Vec<u8>, CoreError> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| CoreError::Probe("нет файла".into()))
        }
    }

    fn program(
        name: &str,
        kind: ProgramKind,
        location: Option<&str>,
        icon: Option<&str>,
    ) -> InstalledProgram {
        InstalledProgram {
            name: name.into(),
            publisher: None,
            version: None,
            size_bytes: None,
            install_location: location.map(str::to_string),
            disk_letter: None,
            kind,
            id: name.to_lowercase(),
            icon_path: None,
            display_icon: icon.map(str::to_string),
            can_uninstall: false,
            uninstall_reason: None,
            uninstall_string: None,
        }
    }

    fn resolve(port: &FakePort, program: &InstalledProgram) -> Option<String> {
        let memory = IconCache::new(16);
        let ctx = IconContext {
            port,
            store: &NoIconStore,
            memory: &memory,
        };
        ctx.program_icon(program)
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn real_png_becomes_a_data_url_and_other_bytes_do_not() {
        let url = png_data_url_from_bytes(&PNG_1X1).unwrap();
        assert!(url.starts_with("data:image/png;base64,iVBORw0KGgo"));
        assert!(png_data_url_from_bytes(b"<svg/>").is_none());
        assert!(png_data_url_from_bytes(&[]).is_none());
    }

    #[test]
    fn data_url_does_not_double_the_prefix_or_keep_whitespace() {
        assert_eq!(png_data_url("QUJD"), "data:image/png;base64,QUJD");
        assert_eq!(
            png_data_url("data:image/png;base64,QUJD\r\n"),
            "data:image/png;base64,QUJD"
        );
        assert_eq!(png_data_url(" QU\nJD "), "data:image/png;base64,QUJD");
    }

    #[test]
    fn manifest_logo_prefers_the_app_list_logo() {
        let xml = r#"<Package><Properties><Logo>Assets\Store.png</Logo></Properties>
            <Applications><Application><uap:VisualElements Square150x150Logo="Assets\Med.png"
            Square44x44Logo="Assets\Small.png"/></Application></Applications></Package>"#;
        assert_eq!(manifest_logo(xml).as_deref(), Some(r"Assets\Small.png"));
        let only_properties = "<Properties><Logo>Assets\\Store.png</Logo></Properties>";
        assert_eq!(
            manifest_logo(only_properties).as_deref(),
            Some(r"Assets\Store.png")
        );
        assert_eq!(manifest_logo("<Package/>"), None);
        assert_eq!(manifest_logo(r#"Square44x44Logo="..\x.png""#), None);
    }

    #[test]
    fn logo_variant_prefers_target_size_near_48_then_scale_200_without_contrast() {
        let files = names(&[
            "Logo.scale-100.png",
            "Logo.scale-200.png",
            "Logo.scale-400.png",
            "Logo.targetsize-24.png",
            "Logo.targetsize-48.png",
            "Logo.targetsize-256.png",
            "Logo.targetsize-48_contrast-white.png",
            "Other.targetsize-48.png",
        ]);
        assert_eq!(
            pick_logo_file(r"Assets\Logo.png", &files).as_deref(),
            Some("Logo.targetsize-48.png")
        );
        let scaled = names(&[
            "Logo.scale-100.png",
            "Logo.scale-200.png",
            "Logo.scale-400.png",
        ]);
        assert_eq!(
            pick_logo_file("Assets/Logo.png", &scaled).as_deref(),
            Some("Logo.scale-200.png")
        );
        assert_eq!(
            pick_logo_file("Logo.png", &names(&["logo.png"])).as_deref(),
            Some("logo.png")
        );
        assert_eq!(pick_logo_file("Logo.png", &names(&["Nope.png"])), None);
    }

    #[test]
    fn main_exe_is_chosen_by_program_name_and_never_guessed() {
        let files = names(&[
            "unins000.exe",
            "Chrome.exe",
            "crashpad_handler.exe",
            "readme.txt",
        ]);
        assert_eq!(
            pick_program_exe("Google Chrome", &files).as_deref(),
            Some("Chrome.exe")
        );
        assert_eq!(
            pick_program_exe("7-Zip", &names(&["7zFM.exe", "7z.exe", "Uninstall.exe"])),
            None
        );
        assert_eq!(
            pick_program_exe("Tool", &names(&["Tool.exe", "Other.exe"])).as_deref(),
            Some("Tool.exe")
        );
        assert_eq!(
            pick_program_exe("Whatever", &names(&["run.exe", "setup.exe"])).as_deref(),
            Some("run.exe")
        );
        assert_eq!(pick_program_exe("X", &names(&["a.dll"])), None);
    }

    #[test]
    fn display_icon_wins_when_it_loads() {
        let mut port = FakePort::default();
        port.icons.insert(r"C:\A\a.exe".into(), "QUJD".into());
        let p = program("A", ProgramKind::Win32, Some(r"C:\A"), Some(r"C:\A\a.exe"));
        assert_eq!(
            resolve(&port, &p).as_deref(),
            Some("data:image/png;base64,QUJD")
        );
    }

    #[test]
    fn win32_without_display_icon_uses_the_main_exe_in_its_folder() {
        let mut port = FakePort::default();
        port.dirs
            .insert(r"C:\Apps\Tool".into(), names(&["Tool.exe", "unins000.exe"]));
        port.icons
            .insert(r"C:\Apps\Tool\Tool.exe".into(), "QUJD".into());
        let p = program("Tool", ProgramKind::Win32, Some(r"C:\Apps\Tool"), None);
        assert_eq!(
            resolve(&port, &p).as_deref(),
            Some("data:image/png;base64,QUJD")
        );
        let lost = program("Tool", ProgramKind::Win32, None, None);
        assert_eq!(resolve(&port, &lost), None);
    }

    #[test]
    fn store_program_takes_the_package_logo_from_its_manifest() {
        let root = r"C:\Program Files\WindowsApps\Calc_1.0_x64__abc";
        let mut port = FakePort::default();
        port.files.insert(
            format!(r"{root}\AppxManifest.xml"),
            br#"<Package><uap:VisualElements Square44x44Logo="Assets\Calc44.png"/></Package>"#
                .to_vec(),
        );
        port.dirs.insert(
            format!(r"{root}\Assets"),
            names(&["Calc44.scale-100.png", "Calc44.scale-200.png"]),
        );
        port.files.insert(
            format!(r"{root}\Assets\Calc44.scale-200.png"),
            PNG_1X1.to_vec(),
        );
        let p = program("Calc", ProgramKind::Store, Some(root), None);
        let url = resolve(&port, &p).unwrap();
        assert!(url.starts_with("data:image/png;base64,iVBORw0KGgo"));
    }

    #[test]
    fn store_logo_that_is_not_a_png_is_ignored() {
        let root = r"C:\WindowsApps\X_1_x64__a";
        let mut port = FakePort::default();
        port.files.insert(
            format!(r"{root}\AppxManifest.xml"),
            b"<Properties><Logo>Logo.png</Logo></Properties>".to_vec(),
        );
        port.dirs.insert(root.into(), names(&["Logo.png"]));
        port.files
            .insert(format!(r"{root}\Logo.png"), b"not a png".to_vec());
        let p = program("X", ProgramKind::Store, Some(root), None);
        assert_eq!(resolve(&port, &p), None);
    }

    #[test]
    fn unsafe_locations_are_never_read() {
        assert!(valid_location(r"C:\Apps\A"));
        assert!(!valid_location(r"..\A"));
        assert!(!valid_location(r"C:\A\..\B"));
        assert!(!valid_location(r"%ProgramFiles%\A"));
        assert!(!valid_location(r"\\server\share\A"));
    }

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

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), (*v).to_string()))
            .collect();
        move |name| map.get(&name.to_ascii_lowercase()).cloned()
    }

    #[test]
    fn env_variables_are_expanded_case_insensitively_or_rejected() {
        let env = env_of(&[("ProgramFiles", r"C:\Program Files")]);
        assert_eq!(
            expand_env(r"%PROGRAMFILES%\A\a.exe", &env).as_deref(),
            Some(r"C:\Program Files\A\a.exe")
        );
        assert_eq!(
            expand_env(r"C:\plain.exe", &env).as_deref(),
            Some(r"C:\plain.exe")
        );
        assert_eq!(expand_env(r"%Nope%\a.exe", &env), None);
        assert_eq!(expand_env(r"50%\a.exe", &env), None);
        assert_eq!(expand_env(r"%%\a.exe", &env), None);
    }

    #[test]
    fn display_icon_target_handles_quotes_index_and_env() {
        let env = env_of(&[
            ("ProgramFiles", r"C:\Program Files"),
            ("SystemRoot", r"C:\Windows"),
        ]);
        let t = |raw: &str| display_icon_target(raw, &env);
        assert_eq!(
            t(r#""%ProgramFiles%\qBittorrent\qbittorrent.exe",0"#),
            Some(IconTarget {
                path: r"C:\Program Files\qBittorrent\qbittorrent.exe".into(),
                index: 0
            })
        );
        assert_eq!(t(r"C:\A B\a.exe,3").map(|x| x.index), Some(3));
        assert_eq!(t(r"C:\A\a.exe,-101").map(|x| x.index), Some(-101));
        assert_eq!(t(r#""C:\A\a.exe" , 2"#).map(|x| x.index), Some(2));
        assert_eq!(t(r#""C:\A\a.exe""#).map(|x| x.index), Some(0));
        assert_eq!(
            t(r"%SystemRoot%\system32\shell32.dll,-4").map(|x| x.path),
            Some(r"C:\Windows\system32\shell32.dll".into())
        );
        assert_eq!(t(r"%Unknown%\a.exe"), None);
        assert_eq!(t(r#""C:\A\..\a.exe",0"#), None);
        assert_eq!(t(r#""C:\A\a.txt",0"#), None);
        assert_eq!(t(r#""C:\A\a.exe"#), None, "unterminated quote");
    }

    #[test]
    fn uninstall_exe_is_found_with_and_without_quotes() {
        let env = env_of(&[("ProgramFiles", r"C:\Program Files")]);
        let u = |raw: &str| uninstall_exe_path(raw, &env);
        assert_eq!(
            u(r#""C:\Program Files\A\unins000.exe" /SILENT"#).as_deref(),
            Some(r"C:\Program Files\A\unins000.exe")
        );
        assert_eq!(
            u(r"C:\Program Files\A\uninstall.exe /S").as_deref(),
            Some(r"C:\Program Files\A\uninstall.exe")
        );
        assert_eq!(
            u(r"%ProgramFiles%\A\u.EXE -x").as_deref(),
            Some(r"C:\Program Files\A\u.EXE")
        );
        assert_eq!(u(r"MsiExec.exe /X{ABC}"), None);
        assert_eq!(u(r"rundll32 foo"), None);
        assert_eq!(parent_dir(r"C:\A\b.exe"), Some(r"C:\A"));
    }

    #[test]
    fn cache_key_depends_on_path_mtime_size_and_px_but_not_case() {
        let s = |m| {
            Some(FileStamp {
                modified_secs: m,
                len: 10,
            })
        };
        let a = icon_cache_key(r"C:\A\a.exe", s(1), 64);
        assert_eq!(a.len(), 16);
        assert_eq!(a, icon_cache_key(r"c:\a\A.EXE", s(1), 64));
        assert_ne!(a, icon_cache_key(r"C:\A\a.exe", s(2), 64), "mtime");
        assert_ne!(a, icon_cache_key(r"C:\A\a.exe", s(1), 32), "px");
        assert_ne!(
            a,
            icon_cache_key(
                r"C:\A\a.exe",
                Some(FileStamp {
                    modified_secs: 1,
                    len: 11
                }),
                64
            ),
            "size"
        );
        assert_ne!(a, icon_cache_key(r"C:\A\b.exe", s(1), 64), "path");
    }

    #[derive(Default)]
    struct MemStore(Mutex<BTreeMap<String, String>>);
    impl IconStore for MemStore {
        fn load(&self, key: &str) -> Option<String> {
            self.0.lock().unwrap().get(key).cloned()
        }
        fn save(&self, key: &str, data_url: &str) {
            self.0.lock().unwrap().insert(key.into(), data_url.into());
        }
    }

    #[test]
    fn extraction_is_saved_to_disk_store_and_reused_after_restart() {
        use std::sync::atomic::Ordering;
        let mut port = FakePort::default();
        port.icons.insert(r"C:\A\a.exe".into(), "QUJD".into());
        port.stamps.insert(
            r"C:\A\a.exe".into(),
            FileStamp {
                modified_secs: 5,
                len: 9,
            },
        );
        let store = MemStore::default();
        let first = IconCache::new(8);
        let ctx = IconContext {
            port: &port,
            store: &store,
            memory: &first,
        };
        let url = ctx.icon_for_path(r"C:\A\a.exe");
        assert_eq!(url.as_deref(), Some("data:image/png;base64,QUJD"));
        ctx.icon_for_path(r"C:\A\a.exe");
        assert_eq!(port.extractions.load(Ordering::SeqCst), 1, "memory cache");
        // "Перезапуск": память пуста, диск помнит.
        let second = IconCache::new(8);
        let ctx = IconContext {
            port: &port,
            store: &store,
            memory: &second,
        };
        assert_eq!(ctx.icon_for_path(r"C:\A\a.exe"), url);
        assert_eq!(port.extractions.load(Ordering::SeqCst), 1, "disk cache");
        // Файл изменился: другой ключ, значок извлекается заново.
        let mut changed = FakePort::default();
        changed.icons.insert(r"C:\A\a.exe".into(), "WFla".into());
        changed.stamps.insert(
            r"C:\A\a.exe".into(),
            FileStamp {
                modified_secs: 6,
                len: 9,
            },
        );
        let third = IconCache::new(8);
        let ctx = IconContext {
            port: &changed,
            store: &store,
            memory: &third,
        };
        assert_eq!(
            ctx.icon_for_path(r"C:\A\a.exe").as_deref(),
            Some("data:image/png;base64,WFla")
        );
    }

    #[test]
    fn failed_extraction_is_remembered_in_memory_only() {
        use std::sync::atomic::Ordering;
        let port = FakePort::default();
        let store = MemStore::default();
        let memory = IconCache::new(8);
        let ctx = IconContext {
            port: &port,
            store: &store,
            memory: &memory,
        };
        assert_eq!(ctx.icon_for_path(r"C:\A\none.exe"), None);
        assert_eq!(ctx.icon_for_path(r"C:\A\none.exe"), None);
        assert_eq!(port.extractions.load(Ordering::SeqCst), 1);
        assert!(store.0.lock().unwrap().is_empty());
        assert!(ctx.icon_for_untrusted_path("calc.exe").is_err());
    }

    #[test]
    fn display_icon_with_env_and_index_is_used_for_a_program() {
        let mut port = FakePort::default();
        port.env
            .insert("programfiles".into(), r"C:\Program Files".into());
        port.icons.insert(
            r"C:\Program Files\qBittorrent\qbittorrent.exe".into(),
            "QUJD".into(),
        );
        let p = program(
            "qBittorrent",
            ProgramKind::Win32,
            None,
            Some(r#""%ProgramFiles%\qBittorrent\qbittorrent.exe",0"#),
        );
        assert_eq!(
            resolve(&port, &p).as_deref(),
            Some("data:image/png;base64,QUJD")
        );
    }

    #[test]
    fn install_location_with_env_and_trailing_slash_finds_the_main_exe() {
        let mut port = FakePort::default();
        port.env
            .insert("programfiles".into(), r"C:\Program Files".into());
        port.dirs.insert(
            r"C:\Program Files\Tool".into(),
            names(&["Tool.exe", "unins000.exe"]),
        );
        port.icons
            .insert(r"C:\Program Files\Tool\Tool.exe".into(), "QUJD".into());
        let p = program(
            "Tool",
            ProgramKind::Win32,
            Some(r"%ProgramFiles%\Tool\"),
            None,
        );
        assert!(resolve(&port, &p).is_some());
    }

    #[test]
    fn falls_back_to_the_uninstaller_adjacent_exe_then_the_uninstaller_itself() {
        let mut port = FakePort::default();
        port.dirs
            .insert(r"C:\Apps\Tool".into(), names(&["Tool.exe", "unins000.exe"]));
        port.icons
            .insert(r"C:\Apps\Tool\Tool.exe".into(), "QUJD".into());
        let mut p = program("Tool", ProgramKind::Win32, None, None);
        p.uninstall_string = Some(r#""C:\Apps\Tool\unins000.exe" /SILENT"#.into());
        assert_eq!(
            resolve(&port, &p).as_deref(),
            Some("data:image/png;base64,QUJD")
        );
        // Основной exe выбрать нельзя (7-Zip): значок самого деинсталлятора.
        let mut seven = FakePort::default();
        seven.dirs.insert(
            r"C:\7z".into(),
            names(&["7zFM.exe", "7z.exe", "Uninstall.exe"]),
        );
        seven
            .icons
            .insert(r"C:\7z\Uninstall.exe".into(), "WFla".into());
        let mut p = program("7-Zip", ProgramKind::Win32, None, None);
        p.uninstall_string = Some(r"C:\7z\Uninstall.exe".into());
        assert_eq!(
            resolve(&seven, &p).as_deref(),
            Some("data:image/png;base64,WFla")
        );
    }

    #[test]
    fn program_without_any_usable_file_has_no_icon() {
        let port = FakePort::default();
        let mut p = program("Ghost", ProgramKind::Win32, None, Some(r"C:\Gone\g.exe"));
        p.uninstall_string = Some("MsiExec.exe /X{1}".into());
        assert_eq!(resolve(&port, &p), None);
    }
}
