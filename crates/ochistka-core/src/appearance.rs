//! Оформление Windows: акцентный цвет, прозрачность, тёмная и светлая тема, обои.
//!
//! Чистая логика без системных вызовов: реестр и смена обоев приходят через порты
//! (`RegistryPort`, `WallpaperPort`). Каждое изменение требует `confirmed`, перед первым
//! изменением прежние значения копируются в резервную ветку, и их можно вернуть.
//! Реальные записи живут только в `crates/platform` под `cfg(windows)`.

use serde::Serialize;
use thiserror::Error;

/// Значение реестра, которое трогает это окно: число или строка.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Dword(u32),
    Text(String),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppearanceError {
    #[error("Действие не подтверждено пользователем")]
    NotConfirmed,
    #[error("Недопустимое значение: {0}")]
    Invalid(String),
    #[error("Сохранённого оформления нет: сначала что-нибудь измените")]
    NoBackup,
    #[error("Не удалось работать с реестром: {0}")]
    Registry(String),
    #[error("Не удалось сменить обои: {0}")]
    Wallpaper(String),
    #[error("Эта функция доступна только в Windows")]
    Unsupported,
    #[error("Выбор отменён")]
    Cancelled,
}

/// Реестр текущего пользователя: ключи заданы относительно HKCU.
pub trait RegistryPort: Send + Sync {
    fn read(&self, key: &str, name: &str) -> Result<Option<RegValue>, AppearanceError>;
    fn write(&self, key: &str, name: &str, value: &RegValue) -> Result<(), AppearanceError>;
    fn delete(&self, key: &str, name: &str) -> Result<(), AppearanceError>;
}

/// Файлы приложения в `%LOCALAPPDATA%\OCHISTKA`: курсоры, обои. Пути полные, с обратной косой чертой.
pub trait FilePort: Send + Sync {
    fn data_dir(&self) -> Result<String, AppearanceError>;
    fn exists(&self, path: &str) -> bool;
    /// Пишет файл, создавая папки. Существующий файл заменяется.
    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), AppearanceError>;
}

/// Просит Windows перечитать курсоры: `%SystemRoot%\System32\rundll32.exe user32.dll,UpdatePerUserSystemParameters`.
pub trait CursorRefreshPort: Send + Sync {
    fn refresh(&self) -> Result<(), AppearanceError>;
}

/// Обои рабочего стола Windows.
pub trait WallpaperPort: Send + Sync {
    fn current(&self) -> Result<Option<String>, AppearanceError>;
    fn set(&self, path: &str) -> Result<(), AppearanceError>;
}

const DWM: &str = r"Software\Microsoft\Windows\DWM";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const ACCENT: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent";
/// Резервная ветка: прежние значения до первого изменения.
pub const BACKUP_KEY: &str = r"Software\OCHISTKA\Backup\Appearance";
const BACKUP_DONE: &str = "Created";
const BACKUP_WALLPAPER: &str = "PreviousWallpaper";
/// Какая плитка обоев сейчас поставлена (`custom` — свой файл).
const APPLIED_WALLPAPER: &str = "AppliedWallpaper";
/// Акцентный цвет `#RRGGBB`, поставленный плиткой.
const APPLIED_ACCENT: &str = "AppliedAccent";
/// Идентификаторы четырёх значений акцента внутри `TARGETS`.
const ACCENT_IDS: [&str; 4] = ["dwmAccent", "dwmColorization", "accentMenu", "accentStart"];

struct Target {
    id: &'static str,
    key: &'static str,
    name: &'static str,
}

const TARGETS: &[Target] = &[
    Target {
        id: "dwmAccent",
        key: DWM,
        name: "AccentColor",
    },
    Target {
        id: "dwmColorization",
        key: DWM,
        name: "ColorizationColor",
    },
    Target {
        id: "dwmPrevalence",
        key: DWM,
        name: "ColorPrevalence",
    },
    Target {
        id: "accentMenu",
        key: ACCENT,
        name: "AccentColorMenu",
    },
    Target {
        id: "accentStart",
        key: ACCENT,
        name: "StartColorMenu",
    },
    Target {
        id: "prevalence",
        key: PERSONALIZE,
        name: "ColorPrevalence",
    },
    Target {
        id: "transparency",
        key: PERSONALIZE,
        name: "EnableTransparency",
    },
    Target {
        id: "appsLight",
        key: PERSONALIZE,
        name: "AppsUseLightTheme",
    },
    Target {
        id: "systemLight",
        key: PERSONALIZE,
        name: "SystemUsesLightTheme",
    },
];

fn target(id: &str) -> &'static Target {
    TARGETS.iter().find(|t| t.id == id).expect("известная цель")
}

/// Текущее состояние для интерфейса. `None` — в реестре значения нет.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceStatus {
    /// Акцентный цвет в виде `#RRGGBB`.
    pub accent: Option<String>,
    pub transparency: Option<bool>,
    /// `true` — тёмная тема приложений.
    pub dark_apps: Option<bool>,
    /// `true` — тёмная тема Пуска и панели задач.
    pub dark_system: Option<bool>,
    /// Цвет акцента на Пуске и панели задач.
    pub fill_taskbar: Option<bool>,
    /// Есть сохранённые прежние значения: можно вернуть оформление.
    pub has_backup: bool,
    /// Есть сохранённый прежний фон рабочего стола.
    pub has_previous_wallpaper: bool,
    /// Плитка обоев, поставленная из OCHISTKA (`custom` — свой файл).
    pub wallpaper_tile: Option<String>,
    /// Акцент, поставленный плиткой (`#RRGGBB`).
    pub accent_tile: Option<String>,
    /// Схема курсора, поставленная из OCHISTKA.
    pub cursor_scheme: Option<String>,
    /// Оригиналов обоев в `%LOCALAPPDATA%\OCHISTKA\Wallpapers` ещё нет.
    pub originals_missing: bool,
}

fn text(reg: &dyn RegistryPort, key: &str, name: &str) -> Result<Option<String>, AppearanceError> {
    Ok(match reg.read(key, name)? {
        Some(RegValue::Text(t)) if !t.is_empty() => Some(t),
        _ => None,
    })
}

/// `#RRGGBB` -> (r, g, b). Принимает только строго шесть шестнадцатеричных цифр.
pub fn parse_hex(hex: &str) -> Result<(u8, u8, u8), AppearanceError> {
    let body = hex
        .strip_prefix('#')
        .filter(|b| b.len() == 6 && b.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| AppearanceError::Invalid("цвет должен быть вида #RRGGBB".into()))?;
    let n = u32::from_str_radix(body, 16).expect("проверено выше");
    Ok(((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// Формат `AccentColor`: 0xAABBGGRR.
pub fn abgr(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | (u32::from(b) << 16) | (u32::from(g) << 8) | u32::from(r)
}

/// Формат `ColorizationColor`: 0xAARRGGBB.
pub fn argb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

fn hex_from_abgr(v: u32) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        v as u8,
        (v >> 8) as u8,
        (v >> 16) as u8
    )
}

fn dword(reg: &dyn RegistryPort, id: &str) -> Result<Option<u32>, AppearanceError> {
    let t = target(id);
    Ok(match reg.read(t.key, t.name)? {
        Some(RegValue::Dword(v)) => Some(v),
        _ => None,
    })
}

fn write_dword(reg: &dyn RegistryPort, id: &str, value: u32) -> Result<(), AppearanceError> {
    let t = target(id);
    reg.write(t.key, t.name, &RegValue::Dword(value))
}

fn has_backup(reg: &dyn RegistryPort) -> Result<bool, AppearanceError> {
    Ok(reg.read(BACKUP_KEY, BACKUP_DONE)?.is_some())
}

pub fn status(reg: &dyn RegistryPort) -> Result<AppearanceStatus, AppearanceError> {
    Ok(AppearanceStatus {
        accent: dword(reg, "dwmAccent")?.map(hex_from_abgr),
        transparency: dword(reg, "transparency")?.map(|v| v != 0),
        dark_apps: dword(reg, "appsLight")?.map(|v| v == 0),
        dark_system: dword(reg, "systemLight")?.map(|v| v == 0),
        fill_taskbar: dword(reg, "prevalence")?.map(|v| v != 0),
        has_backup: has_backup(reg)?,
        has_previous_wallpaper: reg.read(BACKUP_KEY, BACKUP_WALLPAPER)?.is_some(),
        wallpaper_tile: text(reg, BACKUP_KEY, APPLIED_WALLPAPER)?,
        accent_tile: text(reg, BACKUP_KEY, APPLIED_ACCENT)?,
        cursor_scheme: text(reg, CURSOR_BACKUP_KEY, APPLIED_SCHEME)?,
        originals_missing: false,
    })
}

fn need_confirm(confirmed: bool) -> Result<(), AppearanceError> {
    if confirmed {
        Ok(())
    } else {
        Err(AppearanceError::NotConfirmed)
    }
}

/// Перед первым изменением копирует прежние значения. Повторно не перезаписывает:
/// резервная копия всегда хранит то, что было до OCHISTKA.
fn ensure_backup(reg: &dyn RegistryPort) -> Result<(), AppearanceError> {
    if has_backup(reg)? {
        return Ok(());
    }
    for t in TARGETS {
        match reg.read(t.key, t.name)? {
            Some(value) => reg.write(BACKUP_KEY, t.id, &value)?,
            None => reg.write(BACKUP_KEY, &format!("{}.absent", t.id), &RegValue::Dword(1))?,
        }
    }
    reg.write(BACKUP_KEY, BACKUP_DONE, &RegValue::Dword(1))
}

/// Акцентный цвет Windows. `hex` — `#RRGGBB`.
pub fn set_accent(
    reg: &dyn RegistryPort,
    hex: &str,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    let (r, g, b) = parse_hex(hex)?;
    ensure_backup(reg)?;
    write_dword(reg, "dwmAccent", abgr(r, g, b))?;
    write_dword(reg, "dwmColorization", argb(r, g, b))?;
    write_dword(reg, "accentMenu", abgr(r, g, b))?;
    write_dword(reg, "accentStart", abgr(r, g, b))?;
    reg.write(
        BACKUP_KEY,
        APPLIED_ACCENT,
        &RegValue::Text(format!("#{r:02X}{g:02X}{b:02X}")),
    )
}

/// Возвращает прежний акцент (повторный клик по выбранному цвету). Без поставленного
/// акцента ничего не делает.
pub fn restore_accent(reg: &dyn RegistryPort, confirmed: bool) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    if reg.read(BACKUP_KEY, APPLIED_ACCENT)?.is_none() {
        return Ok(());
    }
    for t in TARGETS.iter().filter(|t| ACCENT_IDS.contains(&t.id)) {
        match reg.read(BACKUP_KEY, t.id)? {
            Some(value) => reg.write(t.key, t.name, &value)?,
            None => reg.delete(t.key, t.name)?,
        }
    }
    reg.delete(BACKUP_KEY, APPLIED_ACCENT)
}

pub fn set_transparency(
    reg: &dyn RegistryPort,
    enabled: bool,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    ensure_backup(reg)?;
    write_dword(reg, "transparency", u32::from(enabled))
}

/// Цвет акцента на Пуске и панели задач (и в заголовках окон).
pub fn set_fill_taskbar(
    reg: &dyn RegistryPort,
    enabled: bool,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    ensure_backup(reg)?;
    write_dword(reg, "prevalence", u32::from(enabled))?;
    write_dword(reg, "dwmPrevalence", u32::from(enabled))
}

/// Тёмная или светлая тема приложений; при `also_system` — ещё и Пуска с панелью задач.
pub fn set_theme(
    reg: &dyn RegistryPort,
    dark: bool,
    also_system: bool,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    ensure_backup(reg)?;
    let light = u32::from(!dark);
    write_dword(reg, "appsLight", light)?;
    if also_system {
        write_dword(reg, "systemLight", light)?;
    }
    Ok(())
}

/// Тёмная или светлая тема только Пуска и панели задач.
pub fn set_system_theme(
    reg: &dyn RegistryPort,
    dark: bool,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    ensure_backup(reg)?;
    write_dword(reg, "systemLight", u32::from(!dark))
}

/// Возвращает значения, сохранённые до первого изменения.
pub fn restore(reg: &dyn RegistryPort, confirmed: bool) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    if !has_backup(reg)? {
        return Err(AppearanceError::NoBackup);
    }
    for t in TARGETS {
        match reg.read(BACKUP_KEY, t.id)? {
            Some(value) => reg.write(t.key, t.name, &value)?,
            None => reg.delete(t.key, t.name)?,
        }
    }
    reg.delete(BACKUP_KEY, APPLIED_ACCENT)
}

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp"];

fn is_absolute(path: &str) -> bool {
    let b = path.as_bytes();
    let drive = b.len() > 3
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b[2] == b'\\' || b[2] == b'/');
    drive || path.starts_with('/')
}

/// Путь к картинке: абсолютный, без `..`, расширение из списка.
pub fn validate_image_path(path: &str) -> Result<(), AppearanceError> {
    let bad = |why: &str| Err(AppearanceError::Invalid(why.into()));
    if !is_absolute(path) {
        return bad("нужен полный путь к файлу");
    }
    if path.split(['\\', '/']).any(|part| part == "..") {
        return bad("путь не должен содержать «..»");
    }
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !path.contains('.') || !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return bad("поддерживаются PNG, JPG и BMP");
    }
    Ok(())
}

/// Ставит картинку обоями Windows. Прежний фон запоминается один раз.
pub fn apply_wallpaper(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    path: &str,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    apply_wallpaper_as(reg, wallpaper, path, "custom", confirmed)
}

/// То же, но с запоминанием плитки, из которой взята картинка.
pub fn apply_wallpaper_as(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    path: &str,
    tile: &str,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    validate_image_path(path)?;
    if reg.read(BACKUP_KEY, BACKUP_WALLPAPER)?.is_none() {
        let previous = wallpaper.current()?.unwrap_or_default();
        reg.write(BACKUP_KEY, BACKUP_WALLPAPER, &RegValue::Text(previous))?;
    }
    wallpaper.set(path)?;
    reg.write(BACKUP_KEY, APPLIED_WALLPAPER, &RegValue::Text(tile.into()))
}

/// Возвращает прежний фон. Записи о нём удаляются: следующая смена запомнит новый.
/// Если вернуть нечего, ничего не делает.
pub fn restore_wallpaper(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    let Some(RegValue::Text(previous)) = reg.read(BACKUP_KEY, BACKUP_WALLPAPER)? else {
        return Ok(());
    };
    if !previous.is_empty() {
        wallpaper.set(&previous)?;
    }
    reg.delete(BACKUP_KEY, BACKUP_WALLPAPER)?;
    reg.delete(BACKUP_KEY, APPLIED_WALLPAPER)
}

fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 48
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Папка оригиналов обоев (набор «Offline» владельца), в неё интерфейс ничего не пишет.
pub fn wallpapers_dir(files: &dyn FilePort) -> Result<String, AppearanceError> {
    Ok(format!("{}\\Wallpapers", files.data_dir()?))
}

fn original_path(files: &dyn FilePort, id: &str) -> Result<Option<String>, AppearanceError> {
    let dir = wallpapers_dir(files)?;
    Ok(["jpg", "jpeg", "png", "bmp"]
        .iter()
        .map(|ext| format!("{dir}\\{id}.{ext}"))
        .find(|path| files.exists(path)))
}

/// Есть ли в папке оригиналов хотя бы один файл из галереи (проверяется по `sample_id`).
pub fn originals_missing(files: &dyn FilePort, sample_id: &str) -> bool {
    match original_path(files, sample_id) {
        Ok(found) => found.is_none(),
        Err(_) => true,
    }
}

/// Откуда взята картинка для плитки обоев.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WallpaperSource {
    Original(String),
    /// Оригинала нет, взято превью: в журнал, не в интерфейс.
    Thumbnail(String),
}

/// Плитка обоев: оригинал из `%LOCALAPPDATA%\OCHISTKA\Wallpapers`, а если его нет, превью
/// (`thumb_base64`), записанное рядом. Сплошные цвета (`solid-…`) рисуются как PNG 64×64.
pub fn apply_wallpaper_tile(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    files: &dyn FilePort,
    id: &str,
    solid_color: Option<&str>,
    thumb_base64: Option<&str>,
    confirmed: bool,
) -> Result<WallpaperSource, AppearanceError> {
    need_confirm(confirmed)?;
    if !is_safe_id(id) {
        return Err(AppearanceError::Invalid("недопустимое имя обоев".into()));
    }
    let dir = wallpapers_dir(files)?;
    let source = if let Some(hex) = solid_color {
        let (r, g, b) = parse_hex(hex)?;
        let rgba: Vec<u8> = (0..64 * 64).flat_map(|_| [r, g, b, 255]).collect();
        let path = format!("{dir}\\solid\\{id}.png");
        files.write(&path, &crate::appearance_cursor::encode_png(&rgba, 64))?;
        WallpaperSource::Original(path)
    } else if let Some(path) = original_path(files, id)? {
        WallpaperSource::Original(path)
    } else {
        let bytes = decode_base64(thumb_base64.unwrap_or(""))
            .filter(|b| !b.is_empty())
            .ok_or_else(|| AppearanceError::Invalid("нет ни оригинала, ни превью обоев".into()))?;
        let path = format!("{dir}\\thumbs\\{id}.jpg");
        files.write(&path, &bytes)?;
        WallpaperSource::Thumbnail(path)
    };
    let (WallpaperSource::Original(path) | WallpaperSource::Thumbnail(path)) = &source;
    apply_wallpaper_as(reg, wallpaper, path, id, true)?;
    Ok(source)
}

/// Заглушка кнопки «Скачать оригиналы»: загрузка ещё не подключена, причина пишется в файл
/// рядом с папкой оригиналов. Возвращает путь к файлу с причиной.
pub fn note_originals_download(files: &dyn FilePort) -> Result<String, AppearanceError> {
    let path = format!("{}\\Wallpapers\\ORIGINALS.txt", files.data_dir()?);
    let reason = "Скачивание оригиналов обоев пока не подключено: набор Offline (около 105 МБ) \
        не лежит в репозитории, а сервер раздачи ещё не выбран. Положите файлы вида photo-<id>.jpg \
        в эту папку вручную; установщик положит их сюда сам. См. docs/status/decisions.md.\n";
    files.write(&path, reason.as_bytes())?;
    Ok(path)
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in input.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' => continue,
            _ => return None,
        };
        buf = (buf << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

// ---------- Курсоры ----------

const CURSORS_KEY: &str = r"Control Panel\Cursors";
/// Резервная ветка курсоров: прежняя схема до первого изменения.
pub const CURSOR_BACKUP_KEY: &str = r"Software\OCHISTKA\Backup\Cursors";
const APPLIED_SCHEME: &str = "AppliedScheme";

/// Значения `Control Panel\Cursors`, которые трогает схема: (id в резерве, имя значения).
fn cursor_targets() -> Vec<(String, &'static str)> {
    let mut list: Vec<(String, &'static str)> = crate::appearance_cursor::Role::ALL
        .iter()
        .map(|r| (r.registry_name().to_string(), r.registry_name()))
        .collect();
    list.push(("SchemeName".into(), ""));
    list.push(("SchemeSource".into(), "Scheme Source"));
    list
}

fn ensure_cursor_backup(reg: &dyn RegistryPort) -> Result<(), AppearanceError> {
    if reg.read(CURSOR_BACKUP_KEY, BACKUP_DONE)?.is_some() {
        return Ok(());
    }
    for (id, name) in cursor_targets() {
        match reg.read(CURSORS_KEY, name)? {
            Some(value) => reg.write(CURSOR_BACKUP_KEY, &id, &value)?,
            None => reg.write(
                CURSOR_BACKUP_KEY,
                &format!("{id}.absent"),
                &RegValue::Dword(1),
            )?,
        }
    }
    reg.write(CURSOR_BACKUP_KEY, BACKUP_DONE, &RegValue::Dword(1))
}

/// Описание схемы, пришедшее из таблицы интерфейса (проверяется здесь).
pub struct CursorSpec<'a> {
    pub id: &'a str,
    pub color: &'a str,
    pub secondary: &'a str,
    pub pattern: &'a str,
}

/// Ставит схему курсора Windows: рисует `.cur`, пишет `HKCU\Control Panel\Cursors`, обновляет.
/// Схема с узором `pulse` кладёт стрелку как анимированный `.ani`.
pub fn apply_cursor_scheme(
    reg: &dyn RegistryPort,
    files: &dyn FilePort,
    refresh: &dyn CursorRefreshPort,
    spec: &CursorSpec,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    if !is_safe_id(spec.id) {
        return Err(AppearanceError::Invalid("недопустимое имя схемы".into()));
    }
    let pattern = crate::appearance_cursor::Pattern::parse(spec.pattern)?;
    // Сначала рисуем всё: ошибка в данных не должна оставить реестр наполовину изменённым.
    let mut cursors = Vec::new();
    for role in crate::appearance_cursor::Role::ALL {
        let ext = crate::appearance_cursor::file_extension(role, pattern);
        let bytes = if ext == "ani" {
            crate::appearance_cursor::build_ani(role, spec.color, spec.secondary)?
        } else {
            crate::appearance_cursor::build_cur(role, pattern, spec.color, spec.secondary)?
        };
        cursors.push((role, ext, bytes));
    }
    let dir = format!("{}\\Cursors\\{}", files.data_dir()?, spec.id);
    let mut paths = Vec::new();
    for (role, ext, bytes) in &cursors {
        let path = format!("{dir}\\{}.{ext}", role.registry_name());
        files.write(&path, bytes)?;
        paths.push((role.registry_name(), path));
    }
    ensure_cursor_backup(reg)?;
    for (name, path) in paths {
        reg.write(CURSORS_KEY, name, &RegValue::Text(path))?;
    }
    reg.write(
        CURSORS_KEY,
        "",
        &RegValue::Text(format!("OCHISTKA {}", spec.id)),
    )?;
    reg.write(CURSORS_KEY, "Scheme Source", &RegValue::Dword(1))?;
    reg.write(
        CURSOR_BACKUP_KEY,
        APPLIED_SCHEME,
        &RegValue::Text(spec.id.into()),
    )?;
    refresh.refresh()
}

/// Возвращает прежнюю схему курсоров. Без поставленной схемы ничего не делает.
pub fn restore_cursor_scheme(
    reg: &dyn RegistryPort,
    refresh: &dyn CursorRefreshPort,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    if reg.read(CURSOR_BACKUP_KEY, BACKUP_DONE)?.is_none() {
        return Ok(());
    }
    for (id, name) in cursor_targets() {
        match reg.read(CURSOR_BACKUP_KEY, &id)? {
            Some(value) => reg.write(CURSORS_KEY, name, &value)?,
            None => reg.delete(CURSORS_KEY, name)?,
        }
        reg.delete(CURSOR_BACKUP_KEY, &id)?;
        reg.delete(CURSOR_BACKUP_KEY, &format!("{id}.absent"))?;
    }
    reg.delete(CURSOR_BACKUP_KEY, APPLIED_SCHEME)?;
    reg.delete(CURSOR_BACKUP_KEY, BACKUP_DONE)?;
    refresh.refresh()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeReg {
        data: Mutex<HashMap<(String, String), RegValue>>,
        writes: Mutex<Vec<(String, String)>>,
    }

    impl FakeReg {
        fn with(items: &[(&str, &str, RegValue)]) -> Self {
            let reg = Self::default();
            for (k, n, v) in items {
                reg.data
                    .lock()
                    .unwrap()
                    .insert((k.to_string(), n.to_string()), v.clone());
            }
            reg
        }
        fn get(&self, key: &str, name: &str) -> Option<RegValue> {
            self.data
                .lock()
                .unwrap()
                .get(&(key.to_string(), name.to_string()))
                .cloned()
        }
    }

    impl RegistryPort for FakeReg {
        fn read(&self, key: &str, name: &str) -> Result<Option<RegValue>, AppearanceError> {
            Ok(self.get(key, name))
        }
        fn write(&self, key: &str, name: &str, value: &RegValue) -> Result<(), AppearanceError> {
            self.writes.lock().unwrap().push((key.into(), name.into()));
            self.data
                .lock()
                .unwrap()
                .insert((key.into(), name.into()), value.clone());
            Ok(())
        }
        fn delete(&self, key: &str, name: &str) -> Result<(), AppearanceError> {
            self.data
                .lock()
                .unwrap()
                .remove(&(key.to_string(), name.to_string()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeWallpaper {
        path: Mutex<Option<String>>,
    }

    impl WallpaperPort for FakeWallpaper {
        fn current(&self) -> Result<Option<String>, AppearanceError> {
            Ok(self.path.lock().unwrap().clone())
        }
        fn set(&self, path: &str) -> Result<(), AppearanceError> {
            *self.path.lock().unwrap() = Some(path.into());
            Ok(())
        }
    }

    #[test]
    fn hex_parsing_and_channel_orders() {
        assert_eq!(parse_hex("#FF8000").unwrap(), (255, 128, 0));
        assert!(parse_hex("FF8000").is_err());
        assert!(parse_hex("#FF80").is_err());
        assert!(parse_hex("#GG8000").is_err());
        assert_eq!(abgr(0xFF, 0x80, 0x00), 0xFF00_80FF);
        assert_eq!(argb(0xFF, 0x80, 0x00), 0xFFFF_8000);
        assert_eq!(hex_from_abgr(0xFF00_80FF), "#FF8000");
    }

    #[test]
    fn nothing_is_written_without_confirmation() {
        let reg = FakeReg::default();
        assert_eq!(
            set_accent(&reg, "#FF0000", false),
            Err(AppearanceError::NotConfirmed)
        );
        assert_eq!(
            set_transparency(&reg, true, false),
            Err(AppearanceError::NotConfirmed)
        );
        assert_eq!(
            set_theme(&reg, true, false, false),
            Err(AppearanceError::NotConfirmed)
        );
        assert_eq!(
            set_fill_taskbar(&reg, true, false),
            Err(AppearanceError::NotConfirmed)
        );
        assert_eq!(restore(&reg, false), Err(AppearanceError::NotConfirmed));
        assert!(reg.writes.lock().unwrap().is_empty());
    }

    #[test]
    fn invalid_colour_writes_nothing() {
        let reg = FakeReg::default();
        assert!(matches!(
            set_accent(&reg, "red", true),
            Err(AppearanceError::Invalid(_))
        ));
        assert!(reg.writes.lock().unwrap().is_empty());
    }

    #[test]
    fn first_change_backs_up_then_writes_accent() {
        let reg = FakeReg::with(&[(DWM, "AccentColor", RegValue::Dword(0xFF11_2233))]);
        set_accent(&reg, "#FF8000", true).unwrap();
        assert_eq!(
            reg.get(BACKUP_KEY, "dwmAccent"),
            Some(RegValue::Dword(0xFF11_2233))
        );
        assert_eq!(
            reg.get(BACKUP_KEY, "dwmColorization.absent"),
            Some(RegValue::Dword(1))
        );
        assert_eq!(reg.get(BACKUP_KEY, "Created"), Some(RegValue::Dword(1)));
        assert_eq!(
            reg.get(DWM, "AccentColor"),
            Some(RegValue::Dword(0xFF00_80FF))
        );
        assert_eq!(
            reg.get(DWM, "ColorizationColor"),
            Some(RegValue::Dword(0xFFFF_8000))
        );
        assert_eq!(status(&reg).unwrap().accent.as_deref(), Some("#FF8000"));
    }

    #[test]
    fn backup_is_not_overwritten_by_second_change() {
        let reg = FakeReg::with(&[(PERSONALIZE, "EnableTransparency", RegValue::Dword(1))]);
        set_transparency(&reg, false, true).unwrap();
        set_transparency(&reg, true, true).unwrap();
        set_transparency(&reg, false, true).unwrap();
        assert_eq!(
            reg.get(BACKUP_KEY, "transparency"),
            Some(RegValue::Dword(1))
        );
    }

    #[test]
    fn theme_changes_system_only_when_asked() {
        let reg = FakeReg::with(&[
            (PERSONALIZE, "AppsUseLightTheme", RegValue::Dword(1)),
            (PERSONALIZE, "SystemUsesLightTheme", RegValue::Dword(1)),
        ]);
        set_theme(&reg, true, false, true).unwrap();
        let s = status(&reg).unwrap();
        assert_eq!((s.dark_apps, s.dark_system), (Some(true), Some(false)));
        set_theme(&reg, true, true, true).unwrap();
        assert_eq!(status(&reg).unwrap().dark_system, Some(true));
        set_theme(&reg, false, true, true).unwrap();
        let s = status(&reg).unwrap();
        assert_eq!((s.dark_apps, s.dark_system), (Some(false), Some(false)));
    }

    #[test]
    fn system_theme_leaves_apps_alone() {
        let reg = FakeReg::with(&[
            (PERSONALIZE, "AppsUseLightTheme", RegValue::Dword(1)),
            (PERSONALIZE, "SystemUsesLightTheme", RegValue::Dword(1)),
        ]);
        assert_eq!(
            set_system_theme(&reg, true, false),
            Err(AppearanceError::NotConfirmed)
        );
        set_system_theme(&reg, true, true).unwrap();
        let s = status(&reg).unwrap();
        assert_eq!((s.dark_apps, s.dark_system), (Some(false), Some(true)));
    }

    #[test]
    fn restore_returns_values_and_removes_added_ones() {
        let reg = FakeReg::with(&[
            (DWM, "AccentColor", RegValue::Dword(0xFF11_2233)),
            (PERSONALIZE, "EnableTransparency", RegValue::Dword(1)),
        ]);
        set_accent(&reg, "#FF8000", true).unwrap();
        set_transparency(&reg, false, true).unwrap();
        set_fill_taskbar(&reg, true, true).unwrap();
        restore(&reg, true).unwrap();
        assert_eq!(
            reg.get(DWM, "AccentColor"),
            Some(RegValue::Dword(0xFF11_2233))
        );
        assert_eq!(
            reg.get(PERSONALIZE, "EnableTransparency"),
            Some(RegValue::Dword(1))
        );
        assert_eq!(reg.get(DWM, "ColorizationColor"), None);
        assert_eq!(reg.get(PERSONALIZE, "ColorPrevalence"), None);
        assert!(status(&reg).unwrap().has_backup);
    }

    #[test]
    fn restore_without_backup_fails() {
        let reg = FakeReg::default();
        assert_eq!(restore(&reg, true), Err(AppearanceError::NoBackup));
    }

    #[test]
    fn image_path_validation() {
        assert!(validate_image_path(r"C:\Users\u\Pictures\a.JPG").is_ok());
        assert!(validate_image_path(r"C:\x\..\a.png").is_err());
        assert!(validate_image_path("a.png").is_err());
        assert!(validate_image_path(r"C:\x\a.exe").is_err());
        assert!(validate_image_path(r"C:\x\noext").is_err());
    }

    #[test]
    fn wallpaper_remembers_previous_once_and_restores() {
        let reg = FakeReg::default();
        let wp = FakeWallpaper::default();
        wp.set(r"C:\old.png").unwrap();
        assert_eq!(
            apply_wallpaper(&reg, &wp, r"C:\a.png", false),
            Err(AppearanceError::NotConfirmed)
        );
        apply_wallpaper(&reg, &wp, r"C:\a.png", true).unwrap();
        apply_wallpaper(&reg, &wp, r"C:\b.png", true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(r"C:\b.png"));
        assert!(status(&reg).unwrap().has_previous_wallpaper);
        restore_wallpaper(&reg, &wp, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(r"C:\old.png"));
        assert!(!status(&reg).unwrap().has_previous_wallpaper);
        // Повторное возвращение ничего не меняет.
        wp.set(r"C:\other.png").unwrap();
        restore_wallpaper(&reg, &wp, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(r"C:\other.png"));
    }

    #[derive(Default)]
    struct FakeFiles {
        files: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl FilePort for FakeFiles {
        fn data_dir(&self) -> Result<String, AppearanceError> {
            Ok(r"C:\Data\OCHISTKA".into())
        }
        fn exists(&self, path: &str) -> bool {
            self.files.lock().unwrap().contains_key(path)
        }
        fn write(&self, path: &str, bytes: &[u8]) -> Result<(), AppearanceError> {
            self.files
                .lock()
                .unwrap()
                .insert(path.into(), bytes.to_vec());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeRefresh(Mutex<u32>);

    impl CursorRefreshPort for FakeRefresh {
        fn refresh(&self) -> Result<(), AppearanceError> {
            *self.0.lock().unwrap() += 1;
            Ok(())
        }
    }

    const CUR: &str = r"Control Panel\Cursors";

    fn spec(id: &str) -> CursorSpec<'_> {
        CursorSpec {
            id,
            color: "#FF0000",
            secondary: "#00FF00",
            pattern: "stripes",
        }
    }

    #[test]
    fn cursor_scheme_apply_restore_and_restore_again() {
        let reg = FakeReg::with(&[
            (CUR, "Arrow", RegValue::Text(r"C:\old\arrow.cur".into())),
            (CUR, "", RegValue::Text("Windows Default".into())),
        ]);
        let (files, refresh) = (FakeFiles::default(), FakeRefresh::default());
        assert_eq!(
            apply_cursor_scheme(&reg, &files, &refresh, &spec("cursor-red"), false),
            Err(AppearanceError::NotConfirmed)
        );
        assert!(reg.writes.lock().unwrap().is_empty());
        apply_cursor_scheme(&reg, &files, &refresh, &spec("cursor-red"), true).unwrap();
        assert_eq!(
            reg.get(CUR, "Arrow"),
            Some(RegValue::Text(
                r"C:\Data\OCHISTKA\Cursors\cursor-red\Arrow.cur".into()
            ))
        );
        assert!(files.exists(r"C:\Data\OCHISTKA\Cursors\cursor-red\IBeam.cur"));
        assert_eq!(
            status(&reg).unwrap().cursor_scheme.as_deref(),
            Some("cursor-red")
        );
        // Вторая схема не перезаписывает резерв.
        apply_cursor_scheme(&reg, &files, &refresh, &spec("cursor-blue"), true).unwrap();
        assert_eq!(
            reg.get(CURSOR_BACKUP_KEY, "Arrow"),
            Some(RegValue::Text(r"C:\old\arrow.cur".into()))
        );
        restore_cursor_scheme(&reg, &refresh, true).unwrap();
        assert_eq!(
            reg.get(CUR, "Arrow"),
            Some(RegValue::Text(r"C:\old\arrow.cur".into()))
        );
        assert_eq!(reg.get(CUR, "IBeam"), None, "добавленное значение убрано");
        assert_eq!(
            reg.get(CUR, ""),
            Some(RegValue::Text("Windows Default".into()))
        );
        assert_eq!(status(&reg).unwrap().cursor_scheme, None);
        let refreshed = *refresh.0.lock().unwrap();
        assert_eq!(refreshed, 3);
        // Повторное возвращение: ничего не пишет и не обновляет.
        let before = reg.writes.lock().unwrap().len();
        restore_cursor_scheme(&reg, &refresh, true).unwrap();
        assert_eq!(reg.writes.lock().unwrap().len(), before);
        assert_eq!(*refresh.0.lock().unwrap(), refreshed);
    }

    #[test]
    fn animated_scheme_writes_ani_for_arrow_and_restores_the_stored_value_once() {
        let reg = FakeReg::with(&[(CUR, "Arrow", RegValue::Text(r"C:\old\arrow.cur".into()))]);
        let (files, refresh) = (FakeFiles::default(), FakeRefresh::default());
        let pulse = CursorSpec {
            id: "cursor-pulse",
            color: "#FF009D",
            secondary: "#00FFFF",
            pattern: "pulse",
        };
        apply_cursor_scheme(&reg, &files, &refresh, &pulse, true).unwrap();
        assert_eq!(
            reg.get(CUR, "Arrow"),
            Some(RegValue::Text(
                r"C:\Data\OCHISTKA\Cursors\cursor-pulse\Arrow.ani".into()
            ))
        );
        assert!(files.exists(r"C:\Data\OCHISTKA\Cursors\cursor-pulse\IBeam.cur"));
        apply_cursor_scheme(&reg, &files, &refresh, &spec("cursor-red"), true).unwrap();
        restore_cursor_scheme(&reg, &refresh, true).unwrap();
        assert_eq!(
            reg.get(CUR, "Arrow"),
            Some(RegValue::Text(r"C:\old\arrow.cur".into()))
        );
    }

    #[test]
    fn bad_cursor_data_leaves_registry_untouched() {
        let reg = FakeReg::default();
        let (files, refresh) = (FakeFiles::default(), FakeRefresh::default());
        let mut bad = spec("cursor-red");
        bad.color = "red";
        assert!(apply_cursor_scheme(&reg, &files, &refresh, &bad, true).is_err());
        assert!(apply_cursor_scheme(&reg, &files, &refresh, &spec("..\\x"), true).is_err());
        assert!(reg.writes.lock().unwrap().is_empty());
    }

    const THUMB: &str = "/9j/4A=="; // несколько байт JPEG-заголовка

    #[test]
    fn wallpaper_tile_uses_original_when_present_else_thumbnail() {
        let reg = FakeReg::default();
        let wp = FakeWallpaper::default();
        let files = FakeFiles::default();
        let first =
            apply_wallpaper_tile(&reg, &wp, &files, "photo-1", None, Some(THUMB), true).unwrap();
        assert_eq!(
            first,
            WallpaperSource::Thumbnail(r"C:\Data\OCHISTKA\Wallpapers\thumbs\photo-1.jpg".into())
        );
        files
            .write(r"C:\Data\OCHISTKA\Wallpapers\photo-1.jpg", b"x")
            .unwrap();
        let second = apply_wallpaper_tile(&reg, &wp, &files, "photo-1", None, None, true).unwrap();
        assert_eq!(
            second,
            WallpaperSource::Original(r"C:\Data\OCHISTKA\Wallpapers\photo-1.jpg".into())
        );
        assert_eq!(
            status(&reg).unwrap().wallpaper_tile.as_deref(),
            Some("photo-1")
        );
        assert!(apply_wallpaper_tile(&reg, &wp, &files, "photo-9", None, None, true).is_err());
        assert!(apply_wallpaper_tile(&reg, &wp, &files, "..\\a", None, Some(THUMB), true).is_err());
    }

    #[test]
    fn solid_wallpaper_is_generated_and_previous_backup_written_once() {
        let reg = FakeReg::default();
        let wp = FakeWallpaper::default();
        let files = FakeFiles::default();
        wp.set(r"C:\old.jpg").unwrap();
        apply_wallpaper_tile(
            &reg,
            &wp,
            &files,
            "solid-black",
            Some("#080808"),
            None,
            true,
        )
        .unwrap();
        apply_wallpaper_tile(
            &reg,
            &wp,
            &files,
            "solid-white",
            Some("#FFFFFF"),
            None,
            true,
        )
        .unwrap();
        assert_eq!(
            reg.get(BACKUP_KEY, BACKUP_WALLPAPER),
            Some(RegValue::Text(r"C:\old.jpg".into()))
        );
        restore_wallpaper(&reg, &wp, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(r"C:\old.jpg"));
        assert_eq!(status(&reg).unwrap().wallpaper_tile, None);
    }

    #[test]
    fn accent_twice_restores_previous_accent_and_again_is_noop() {
        let reg = FakeReg::with(&[(DWM, "AccentColor", RegValue::Dword(0xFF11_2233))]);
        set_accent(&reg, "#087FFF", true).unwrap();
        assert_eq!(
            status(&reg).unwrap().accent_tile.as_deref(),
            Some("#087FFF")
        );
        set_accent(&reg, "#FF009D", true).unwrap();
        restore_accent(&reg, true).unwrap();
        assert_eq!(
            reg.get(DWM, "AccentColor"),
            Some(RegValue::Dword(0xFF11_2233))
        );
        assert_eq!(status(&reg).unwrap().accent_tile, None);
        let before = reg.writes.lock().unwrap().len();
        restore_accent(&reg, true).unwrap();
        assert_eq!(reg.writes.lock().unwrap().len(), before);
    }

    #[test]
    fn download_originals_is_a_documented_stub() {
        let files = FakeFiles::default();
        let path = note_originals_download(&files).unwrap();
        assert!(files.exists(&path));
        assert!(originals_missing(&files, "photo-1"));
    }

    #[test]
    fn base64_decoder_works() {
        assert_eq!(decode_base64("aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(decode_base64("a$b"), None);
    }
}
