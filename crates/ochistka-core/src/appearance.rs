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
    /// REG_BINARY (например, `AccentPalette`).
    Binary(Vec<u8>),
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
    /// Копирует файл `from` в `to`, создавая папки. Существующий файл заменяется.
    fn copy(&self, from: &str, to: &str) -> Result<(), AppearanceError>;
    /// Папка Windows (`%SystemRoot%`), откуда берутся обои по умолчанию.
    fn windows_dir(&self) -> String {
        r"C:\Windows".into()
    }
}

/// Просит Windows перечитать курсоры (`SystemParametersInfoW(SPI_SETCURSORS)`) и сообщает
/// системе о смене цветов.
pub trait CursorRefreshPort: Send + Sync {
    fn refresh(&self) -> Result<(), AppearanceError>;
    /// Одно широковещательное `WM_SETTINGCHANGE` («ImmersiveColorSet») с коротким таймаутом.
    /// Больше ничего не перезапускается: ни проводник, ни повторные рассылки.
    fn colors_changed(&self) -> Result<(), AppearanceError> {
        Ok(())
    }
}

/// Обои рабочего стола Windows.
pub trait WallpaperPort: Send + Sync {
    /// Полный путь к текущему файлу обоев (`SPI_GETDESKWALLPAPER`); `None` — обоев нет.
    fn current(&self) -> Result<Option<String>, AppearanceError>;
    /// Ставит обои; пустая строка убирает картинку.
    fn set(&self, path: &str) -> Result<(), AppearanceError>;
    /// Размер основного экрана в пикселях: по нему рисуются сплошные обои.
    fn screen_size(&self) -> Option<(u32, u32)> {
        None
    }
}

const DWM: &str = r"Software\Microsoft\Windows\DWM";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const ACCENT: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent";
/// Резервная ветка: прежние значения до первого изменения.
pub const BACKUP_KEY: &str = r"Software\OCHISTKA\Backup\Appearance";
const BACKUP_DONE: &str = "Created";
/// Прежние обои: путь к копии в `Backup`, пусто (обоев не было) или `@default` (обои Windows).
const BACKUP_WALLPAPER: &str = "PreviousWallpaperFile";
const PREVIOUS_DEFAULT: &str = "@default";
/// Ветка рабочего стола: положение обоев и качество импорта JPEG.
const DESKTOP: &str = r"Control Panel\Desktop";
/// Значения ветки рабочего стола, которые ставит смена обоев (прежние сохраняются).
const DESKTOP_TARGETS: [&str; 3] = ["WallpaperStyle", "TileWallpaper", "JPEGImportQuality"];
const DESKTOP_SNAPSHOT: &str = "Desktop.";
/// Прозрачность и заливка панели задач включены нами (значение — снимок прежнего в `Toggle.*`).
const APPLIED_TRANSPARENCY: &str = "AppliedTransparency";
const APPLIED_FILL: &str = "AppliedFill";
const TOGGLE_SNAPSHOT: &str = "Toggle.";
/// Какая плитка обоев сейчас поставлена (`custom` — свой файл).
const APPLIED_WALLPAPER: &str = "AppliedWallpaper";
/// Акцентный цвет `#RRGGBB`, поставленный плиткой.
const APPLIED_ACCENT: &str = "AppliedAccent";
/// Идентификаторы значений цвета Windows внутри `TARGETS`: их плитка цвета снимает и возвращает.
const ACCENT_IDS: [&str; 4] = [
    "dwmAccent",
    "dwmColorization",
    "accentMenu",
    "accentPalette",
];
/// Снимок цвета перед плиткой: `Accent.<id>` или `Accent.<id>.absent`.
const ACCENT_SNAPSHOT: &str = "Accent.";

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
        id: "accentPalette",
        key: ACCENT,
        name: "AccentPalette",
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
];

fn target(id: &str) -> &'static Target {
    TARGETS.iter().find(|t| t.id == id).expect("известная цель")
}

/// Текущее состояние для интерфейса.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceStatus {
    /// Акцентный цвет в виде `#RRGGBB`.
    pub accent: Option<String>,
    /// Эффект прозрачности включён из OCHISTKA (по умолчанию «выключено»).
    pub transparency_on: bool,
    /// Заливка панели задач и Пуска включена из OCHISTKA.
    pub fill_on: bool,
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
        transparency_on: reg.read(BACKUP_KEY, APPLIED_TRANSPARENCY)?.is_some(),
        fill_on: reg.read(BACKUP_KEY, APPLIED_FILL)?.is_some(),
        has_backup: has_backup(reg)?,
        has_previous_wallpaper: reg.read(BACKUP_KEY, BACKUP_WALLPAPER)?.is_some(),
        wallpaper_tile: text(reg, BACKUP_KEY, APPLIED_WALLPAPER)?,
        accent_tile: text(reg, BACKUP_KEY, APPLIED_ACCENT)?,
        cursor_scheme: text(reg, CURSOR_BACKUP_KEY, APPLIED_SCHEME)?,
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

/// Палитра `AccentPalette`: 8 цветов RGBA от светлого к тёмному, четвёртый — сам акцент.
pub fn accent_palette(r: u8, g: u8, b: u8) -> Vec<u8> {
    // Доля смешивания: положительная — с белым, отрицательная — с чёрным.
    const MIX: [f64; 8] = [0.6, 0.4, 0.2, 0.0, -0.2, -0.4, -0.6, -0.8];
    let mut out = Vec::with_capacity(32);
    for (i, mix) in MIX.iter().enumerate() {
        for channel in [r, g, b] {
            let c = f64::from(channel);
            let v = if *mix >= 0.0 {
                c + (255.0 - c) * mix
            } else {
                c * (1.0 + mix)
            };
            out.push(v.round().clamp(0.0, 255.0) as u8);
        }
        out.push(if i == 7 { 0 } else { 0xFF });
    }
    out
}

/// Акцентный цвет Windows. `hex` — `#RRGGBB`. Меняются только `AccentColor`, `AccentPalette`,
/// `ColorizationColor` и `AccentColorMenu`; `ColorPrevalence` (заливка Пуска и панели задач)
/// не трогается: она включается отдельным переключателем.
/// Перед первой плиткой прежние значения снимаются и возвращаются `restore_accent`.
pub fn set_accent(
    reg: &dyn RegistryPort,
    hex: &str,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    let (r, g, b) = parse_hex(hex)?;
    ensure_backup(reg)?;
    if reg.read(BACKUP_KEY, APPLIED_ACCENT)?.is_none() {
        snapshot(reg, &ACCENT_IDS, ACCENT_SNAPSHOT)?;
    }
    write_dword(reg, "dwmAccent", abgr(r, g, b))?;
    write_dword(reg, "dwmColorization", argb(r, g, b))?;
    write_dword(reg, "accentMenu", abgr(r, g, b))?;
    let palette = target("accentPalette");
    reg.write(
        palette.key,
        palette.name,
        &RegValue::Binary(accent_palette(r, g, b)),
    )?;
    reg.write(
        BACKUP_KEY,
        APPLIED_ACCENT,
        &RegValue::Text(format!("#{r:02X}{g:02X}{b:02X}")),
    )
}

/// Снимает прежние значения целей `ids` в резерв под именами `<prefix><id>[.absent]`.
fn snapshot(reg: &dyn RegistryPort, ids: &[&str], prefix: &str) -> Result<(), AppearanceError> {
    for id in ids {
        let t = target(id);
        let name = format!("{prefix}{id}");
        reg.delete(BACKUP_KEY, &name)?;
        reg.delete(BACKUP_KEY, &format!("{name}.absent"))?;
        match reg.read(t.key, t.name)? {
            Some(value) => reg.write(BACKUP_KEY, &name, &value)?,
            None => reg.write(BACKUP_KEY, &format!("{name}.absent"), &RegValue::Dword(1))?,
        }
    }
    Ok(())
}

/// Возвращает значения целей `ids` из снимка и стирает сам снимок.
fn restore_snapshot(
    reg: &dyn RegistryPort,
    ids: &[&str],
    prefix: &str,
) -> Result<(), AppearanceError> {
    for id in ids {
        let t = target(id);
        let name = format!("{prefix}{id}");
        match reg.read(BACKUP_KEY, &name)? {
            Some(value) => reg.write(t.key, t.name, &value)?,
            None => reg.delete(t.key, t.name)?,
        }
        reg.delete(BACKUP_KEY, &name)?;
        reg.delete(BACKUP_KEY, &format!("{name}.absent"))?;
    }
    Ok(())
}

/// Возвращает цвет, который был до плитки (повторный клик по выбранному цвету). Без
/// поставленной плитки ничего не делает.
pub fn restore_accent(reg: &dyn RegistryPort, confirmed: bool) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    if reg.read(BACKUP_KEY, APPLIED_ACCENT)?.is_none() {
        return Ok(());
    }
    restore_snapshot(reg, &ACCENT_IDS, ACCENT_SNAPSHOT)?;
    reg.delete(BACKUP_KEY, APPLIED_ACCENT)
}

/// Включает или выключает переключатель: при включении запоминает прежние значения `ids` и
/// пишет `1`, при выключении возвращает прежние. Выключение без включения ничего не делает.
fn toggle(
    reg: &dyn RegistryPort,
    ids: &[&str],
    marker: &str,
    enabled: bool,
) -> Result<(), AppearanceError> {
    let prefix = format!("{TOGGLE_SNAPSHOT}{marker}.");
    let applied = reg.read(BACKUP_KEY, marker)?.is_some();
    if enabled {
        ensure_backup(reg)?;
        if !applied {
            snapshot(reg, ids, &prefix)?;
            reg.write(BACKUP_KEY, marker, &RegValue::Dword(1))?;
        }
        for id in ids {
            write_dword(reg, id, 1)?;
        }
        Ok(())
    } else if applied {
        restore_snapshot(reg, ids, &prefix)?;
        reg.delete(BACKUP_KEY, marker)
    } else {
        Ok(())
    }
}

/// Эффект прозрачности: включение запоминает прежнее значение, повторный клик возвращает его.
pub fn set_transparency(
    reg: &dyn RegistryPort,
    enabled: bool,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    toggle(reg, &["transparency"], APPLIED_TRANSPARENCY, enabled)
}

/// Цвет акцента на Пуске, панели задач и в заголовках окон (`ColorPrevalence`).
pub fn set_fill_taskbar(
    reg: &dyn RegistryPort,
    enabled: bool,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    toggle(reg, &["prevalence", "dwmPrevalence"], APPLIED_FILL, enabled)
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
    for marker in [APPLIED_ACCENT, APPLIED_TRANSPARENCY, APPLIED_FILL] {
        reg.delete(BACKUP_KEY, marker)?;
    }
    Ok(())
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

/// Папка резерва в данных приложения: копия прежних обоев.
fn backup_dir(files: &dyn FilePort) -> Result<String, AppearanceError> {
    Ok(format!("{}\\Backup", files.data_dir()?))
}

fn same_path(a: &str, b: &str) -> bool {
    a.replace('/', "\\")
        .eq_ignore_ascii_case(&b.replace('/', "\\"))
}

fn extension(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or("");
    match name.rsplit_once('.') {
        Some((_, ext))
            if !ext.is_empty() && ext.len() <= 5 && ext.chars().all(char::is_alphanumeric) =>
        {
            ext.to_ascii_lowercase()
        }
        // `TranscodedWallpaper` без расширения: Windows распознаёт картинку по содержимому.
        _ => "jpg".into(),
    }
}

/// Перед первой сменой копирует текущий файл обоев в `Backup\wallpaper-previous.<ext>` и
/// запоминает прежние значения ветки рабочего стола. Повторно не перезаписывает.
/// Копия нужна потому, что Windows хранит текущие обои в `TranscodedWallpaper` и перезаписывает
/// этот файл при следующей смене: путь к нему прежние обои уже не вернёт.
fn ensure_wallpaper_backup(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    files: &dyn FilePort,
) -> Result<(), AppearanceError> {
    if reg.read(BACKUP_KEY, BACKUP_WALLPAPER)?.is_some() {
        return Ok(());
    }
    for name in DESKTOP_TARGETS {
        let id = format!("{DESKTOP_SNAPSHOT}{name}");
        reg.delete(BACKUP_KEY, &id)?;
        reg.delete(BACKUP_KEY, &format!("{id}.absent"))?;
        match reg.read(DESKTOP, name)? {
            Some(value) => reg.write(BACKUP_KEY, &id, &value)?,
            None => reg.write(BACKUP_KEY, &format!("{id}.absent"), &RegValue::Dword(1))?,
        }
    }
    let current = wallpaper.current()?.filter(|p| !p.is_empty());
    let previous = match current {
        None => String::new(),
        Some(path) => {
            let ours = [wallpapers_dir(files)?, backup_dir(files)?]
                .iter()
                .any(|dir| {
                    path.to_ascii_lowercase()
                        .starts_with(&dir.to_ascii_lowercase())
                });
            if ours && !path.to_ascii_lowercase().contains("wallpaper-previous") {
                // Фон уже поставлен нами раньше (старая версия без копии): вернём обои Windows.
                PREVIOUS_DEFAULT.into()
            } else if !files.exists(&path) {
                PREVIOUS_DEFAULT.into()
            } else {
                let copy = format!(
                    "{}\\wallpaper-previous.{}",
                    backup_dir(files)?,
                    extension(&path)
                );
                if !same_path(&copy, &path) {
                    files.copy(&path, &copy)?;
                }
                copy
            }
        }
    };
    reg.write(BACKUP_KEY, BACKUP_WALLPAPER, &RegValue::Text(previous))
}

/// Ставит картинку обоями Windows. Прежний фон запоминается один раз.
pub fn apply_wallpaper(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    files: &dyn FilePort,
    path: &str,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    apply_wallpaper_as(reg, wallpaper, files, path, "custom", confirmed)
}

/// То же, но с запоминанием плитки, из которой взята картинка. Перед сменой выставляются
/// «Заполнить» (`WallpaperStyle=10`), без мозаики и `JPEGImportQuality=100`: иначе Windows
/// пережимает картинку в JPEG с качеством 85 и обои выглядят размытыми.
pub fn apply_wallpaper_as(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    files: &dyn FilePort,
    path: &str,
    tile: &str,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    validate_image_path(path)?;
    ensure_wallpaper_backup(reg, wallpaper, files)?;
    reg.write(DESKTOP, "WallpaperStyle", &RegValue::Text("10".into()))?;
    reg.write(DESKTOP, "TileWallpaper", &RegValue::Text("0".into()))?;
    reg.write(DESKTOP, "JPEGImportQuality", &RegValue::Dword(100))?;
    wallpaper.set(path)?;
    reg.write(BACKUP_KEY, APPLIED_WALLPAPER, &RegValue::Text(tile.into()))
}

/// Возвращает прежний фон: значения ветки рабочего стола и сохранённую копию файла.
/// Записи о нём удаляются: следующая смена запомнит новый. Если вернуть нечего, ничего не делает.
/// Нет копии (файл удалён) — ставятся обои Windows по умолчанию.
pub fn restore_wallpaper(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    files: &dyn FilePort,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    let Some(RegValue::Text(previous)) = reg.read(BACKUP_KEY, BACKUP_WALLPAPER)? else {
        return Ok(());
    };
    for name in DESKTOP_TARGETS {
        let id = format!("{DESKTOP_SNAPSHOT}{name}");
        match reg.read(BACKUP_KEY, &id)? {
            Some(value) => reg.write(DESKTOP, name, &value)?,
            None => reg.delete(DESKTOP, name)?,
        }
        reg.delete(BACKUP_KEY, &id)?;
        reg.delete(BACKUP_KEY, &format!("{id}.absent"))?;
    }
    let default = format!("{}\\Web\\Wallpaper\\Windows\\img0.jpg", files.windows_dir());
    let target = if previous.is_empty() {
        String::new()
    } else if previous != PREVIOUS_DEFAULT && files.exists(&previous) {
        previous
    } else if files.exists(&default) {
        default
    } else {
        String::new()
    };
    wallpaper.set(&target)?;
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

/// Папка копий обоев в данных приложения: туда плитка копирует оригинал из ресурсов установки.
pub fn wallpapers_dir(files: &dyn FilePort) -> Result<String, AppearanceError> {
    Ok(format!("{}\\Wallpapers", files.data_dir()?))
}

/// Размер сплошных обоев, если размер экрана неизвестен.
const FALLBACK_SCREEN: (u32, u32) = (3840, 2160);

/// Плитка обоев. `solid-*` рисуется в Rust в родном разрешении основного экрана (PNG);
/// остальные: оригинал (набор «Offline» владельца) лежит в ресурсах установки
/// (`<resource_dir>\wallpapers\<id>.jpg`), копируется в `%LOCALAPPDATA%\OCHISTKA\Wallpapers`.
/// Копия ставится через `WallpaperPort`; прежний фон запоминается один раз.
/// Возвращает путь к файлу.
pub fn apply_wallpaper_tile(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    files: &dyn FilePort,
    resource_dir: Option<&str>,
    id: &str,
    confirmed: bool,
) -> Result<String, AppearanceError> {
    need_confirm(confirmed)?;
    if !is_safe_id(id) {
        return Err(AppearanceError::Invalid("недопустимое имя обоев".into()));
    }
    if id.starts_with("solid-") {
        let (w, h) = wallpaper.screen_size().unwrap_or(FALLBACK_SCREEN);
        let png = crate::appearance_wallpaper::render_solid(id, w, h)?;
        let target = format!("{}\\{id}.png", wallpapers_dir(files)?);
        files.write(&target, &png)?;
        apply_wallpaper_as(reg, wallpaper, files, &target, id, true)?;
        return Ok(target);
    }
    let resource_dir = resource_dir
        .map(|d| d.trim_end_matches(['\\', '/']))
        .filter(|d| !d.is_empty())
        .ok_or_else(|| AppearanceError::Wallpaper("папка ресурсов приложения не найдена".into()))?;
    let source = format!("{resource_dir}\\wallpapers\\{id}.jpg");
    if !files.exists(&source) {
        return Err(AppearanceError::Wallpaper(
            "файла этих обоев нет в установке приложения".into(),
        ));
    }
    let target = format!("{}\\{id}.jpg", wallpapers_dir(files)?);
    files.copy(&source, &target)?;
    apply_wallpaper_as(reg, wallpaper, files, &target, id, true)?;
    Ok(target)
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

/// Ставит схему курсора Windows: рисует `.cur` (для «Фоновой работы» и «Занят» — анимированные
/// `.ani`), пишет `HKCU\Control Panel\Cursors` и просит Windows перечитать курсоры.
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
    // Сначала рисуем всё (параллельно: роли независимы): ошибка в данных не должна оставить
    // реестр наполовину изменённым.
    let rendered: Vec<
        Result<(crate::appearance_cursor::Role, &'static str, Vec<u8>), AppearanceError>,
    > = std::thread::scope(|scope| {
        let handles: Vec<_> = crate::appearance_cursor::Role::ALL
            .into_iter()
            .map(|role| {
                scope.spawn(move || {
                    let (ext, bytes) = if role.is_animated() {
                        (
                            "ani",
                            crate::appearance_cursor::build_ani(
                                role,
                                pattern,
                                spec.color,
                                spec.secondary,
                            )?,
                        )
                    } else {
                        (
                            "cur",
                            crate::appearance_cursor::build_cur(
                                role,
                                pattern,
                                spec.color,
                                spec.secondary,
                            )?,
                        )
                    };
                    Ok((role, ext, bytes))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join().unwrap_or_else(|_| {
                    Err(AppearanceError::Invalid(
                        "не удалось нарисовать курсор".into(),
                    ))
                })
            })
            .collect()
    });
    let mut cursors = Vec::new();
    for item in rendered {
        cursors.push(item?);
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
    // Имя схемы (значение по умолчанию ветки) и «Scheme Source» = 1: пользовательская схема.
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
        screen: Option<(u32, u32)>,
    }

    impl WallpaperPort for FakeWallpaper {
        fn current(&self) -> Result<Option<String>, AppearanceError> {
            Ok(self.path.lock().unwrap().clone())
        }
        fn set(&self, path: &str) -> Result<(), AppearanceError> {
            *self.path.lock().unwrap() = Some(path.into());
            Ok(())
        }
        fn screen_size(&self) -> Option<(u32, u32)> {
            self.screen
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
        let reg = FakeReg::with(&[(PERSONALIZE, "EnableTransparency", RegValue::Dword(0))]);
        set_transparency(&reg, true, true).unwrap();
        set_transparency(&reg, false, true).unwrap();
        set_transparency(&reg, true, true).unwrap();
        assert_eq!(
            reg.get(BACKUP_KEY, "transparency"),
            Some(RegValue::Dword(0))
        );
    }

    #[test]
    fn transparency_toggle_is_off_by_default_and_second_click_restores() {
        let reg = FakeReg::with(&[(PERSONALIZE, "EnableTransparency", RegValue::Dword(0))]);
        assert!(!status(&reg).unwrap().transparency_on);
        set_transparency(&reg, true, true).unwrap();
        assert!(status(&reg).unwrap().transparency_on);
        assert_eq!(
            reg.get(PERSONALIZE, "EnableTransparency"),
            Some(RegValue::Dword(1))
        );
        // Повторное включение не затирает снимок.
        set_transparency(&reg, true, true).unwrap();
        set_transparency(&reg, false, true).unwrap();
        assert!(!status(&reg).unwrap().transparency_on);
        assert_eq!(
            reg.get(PERSONALIZE, "EnableTransparency"),
            Some(RegValue::Dword(0))
        );
        // Выключение без включения ничего не пишет.
        let before = reg.writes.lock().unwrap().len();
        set_transparency(&reg, false, true).unwrap();
        assert_eq!(reg.writes.lock().unwrap().len(), before);
    }

    #[test]
    fn fill_toggle_sets_both_prevalence_values_and_reverts_absent_ones() {
        let reg = FakeReg::with(&[(DWM, "ColorPrevalence", RegValue::Dword(0))]);
        set_fill_taskbar(&reg, true, true).unwrap();
        assert!(status(&reg).unwrap().fill_on);
        assert_eq!(reg.get(DWM, "ColorPrevalence"), Some(RegValue::Dword(1)));
        assert_eq!(
            reg.get(PERSONALIZE, "ColorPrevalence"),
            Some(RegValue::Dword(1))
        );
        set_fill_taskbar(&reg, false, true).unwrap();
        assert!(!status(&reg).unwrap().fill_on);
        assert_eq!(reg.get(DWM, "ColorPrevalence"), Some(RegValue::Dword(0)));
        assert_eq!(
            reg.get(PERSONALIZE, "ColorPrevalence"),
            None,
            "значения не было: удалено"
        );
    }

    #[test]
    fn colour_pick_never_touches_prevalence_or_theme() {
        let reg = FakeReg::with(&[
            (DWM, "ColorPrevalence", RegValue::Dword(0)),
            (PERSONALIZE, "ColorPrevalence", RegValue::Dword(0)),
            (PERSONALIZE, "AppsUseLightTheme", RegValue::Dword(1)),
        ]);
        set_accent(&reg, "#087FFF", true).unwrap();
        set_accent(&reg, "#FF009D", true).unwrap();
        restore_accent(&reg, true).unwrap();
        let written = reg.writes.lock().unwrap().clone();
        for (key, name) in &written {
            if key != BACKUP_KEY {
                assert!(
                    [
                        "AccentColor",
                        "ColorizationColor",
                        "AccentColorMenu",
                        "AccentPalette"
                    ]
                    .contains(&name.as_str()),
                    "лишняя запись {key}\\{name}"
                );
            }
        }
        assert_eq!(reg.get(DWM, "ColorPrevalence"), Some(RegValue::Dword(0)));
        assert_eq!(
            reg.get(PERSONALIZE, "ColorPrevalence"),
            Some(RegValue::Dword(0))
        );
    }

    #[test]
    fn accent_restore_keeps_a_fill_toggle_set_in_between() {
        let reg = FakeReg::with(&[(PERSONALIZE, "ColorPrevalence", RegValue::Dword(0))]);
        set_accent(&reg, "#087FFF", true).unwrap();
        set_fill_taskbar(&reg, true, true).unwrap();
        restore_accent(&reg, true).unwrap();
        assert_eq!(
            reg.get(PERSONALIZE, "ColorPrevalence"),
            Some(RegValue::Dword(1))
        );
    }

    #[test]
    fn restore_returns_values_and_removes_added_ones() {
        let reg = FakeReg::with(&[
            (DWM, "AccentColor", RegValue::Dword(0xFF11_2233)),
            (PERSONALIZE, "EnableTransparency", RegValue::Dword(1)),
        ]);
        set_accent(&reg, "#FF8000", true).unwrap();
        set_transparency(&reg, true, true).unwrap();
        set_fill_taskbar(&reg, true, true).unwrap();
        restore(&reg, true).unwrap();
        let s = status(&reg).unwrap();
        assert!(!s.fill_on && !s.transparency_on && s.accent_tile.is_none());
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
        fn copy(&self, from: &str, to: &str) -> Result<(), AppearanceError> {
            let mut files = self.files.lock().unwrap();
            let bytes = files
                .get(from)
                .cloned()
                .ok_or_else(|| AppearanceError::Wallpaper("нет файла".into()))?;
            files.insert(to.into(), bytes);
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
        // «Занят» и «Фоновая работа» анимированные; пути абсолютные.
        for role in ["Wait", "AppStarting"] {
            let path = format!(r"C:\Data\OCHISTKA\Cursors\cursor-red\{role}.ani");
            assert_eq!(reg.get(CUR, role), Some(RegValue::Text(path.clone())));
            assert_eq!(&files.files.lock().unwrap()[&path][8..12], b"ACON");
        }
        assert_eq!(
            reg.get(CUR, ""),
            Some(RegValue::Text("OCHISTKA cursor-red".into()))
        );
        assert_eq!(reg.get(CUR, "Scheme Source"), Some(RegValue::Dword(1)));
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
    fn bad_cursor_data_leaves_registry_untouched() {
        let reg = FakeReg::default();
        let (files, refresh) = (FakeFiles::default(), FakeRefresh::default());
        let mut bad = spec("cursor-red");
        bad.color = "red";
        assert!(apply_cursor_scheme(&reg, &files, &refresh, &bad, true).is_err());
        assert!(apply_cursor_scheme(&reg, &files, &refresh, &spec("..\\x"), true).is_err());
        assert!(reg.writes.lock().unwrap().is_empty());
    }

    const RES: &str = r"C:\Program Files\OCHISTKA";
    const COPY: &str = r"C:\Data\OCHISTKA\Wallpapers\photo-1.jpg";
    const TRANSCODED: &str =
        r"C:\Users\u\AppData\Roaming\Microsoft\Windows\Themes\TranscodedWallpaper";
    const BACKUP_COPY: &str = r"C:\Data\OCHISTKA\Backup\wallpaper-previous.jpg";
    const DEFAULT_IMG: &str = r"C:\Windows\Web\Wallpaper\Windows\img0.jpg";

    fn bundled(files: &FakeFiles, id: &str) {
        files
            .write(&format!(r"{RES}\wallpapers\{id}.jpg"), b"jpeg")
            .unwrap();
    }

    /// Windows после установки: текущие обои лежат в `TranscodedWallpaper`.
    fn windows_default() -> (FakeReg, FakeWallpaper, FakeFiles) {
        let reg = FakeReg::with(&[
            (DESKTOP, "WallpaperStyle", RegValue::Text("2".into())),
            (DESKTOP, "TileWallpaper", RegValue::Text("0".into())),
        ]);
        let wp = FakeWallpaper::default();
        wp.set(TRANSCODED).unwrap();
        let files = FakeFiles::default();
        files.write(TRANSCODED, b"original").unwrap();
        files.write(DEFAULT_IMG, b"img0").unwrap();
        bundled(&files, "photo-1");
        bundled(&files, "photo-2");
        (reg, wp, files)
    }

    #[test]
    fn wallpaper_tile_copies_bundled_original_and_applies_the_copy() {
        let (reg, wp, files) = windows_default();
        // Без подтверждения ничего не копируется и не ставится.
        assert_eq!(
            apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", false),
            Err(AppearanceError::NotConfirmed)
        );
        assert!(!files.exists(COPY));
        let path = apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        assert_eq!(path, COPY);
        assert!(files.exists(COPY));
        assert_eq!(wp.current().unwrap().as_deref(), Some(COPY));
        assert_eq!(
            status(&reg).unwrap().wallpaper_tile.as_deref(),
            Some("photo-1")
        );
    }

    #[test]
    fn apply_sets_fill_style_and_lossless_jpeg_import() {
        let (reg, wp, files) = windows_default();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        assert_eq!(
            reg.get(DESKTOP, "WallpaperStyle"),
            Some(RegValue::Text("10".into()))
        );
        assert_eq!(
            reg.get(DESKTOP, "TileWallpaper"),
            Some(RegValue::Text("0".into()))
        );
        assert_eq!(
            reg.get(DESKTOP, "JPEGImportQuality"),
            Some(RegValue::Dword(100))
        );
    }

    #[test]
    fn previous_wallpaper_file_is_copied_before_windows_overwrites_it() {
        let (reg, wp, files) = windows_default();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        assert_eq!(
            reg.get(BACKUP_KEY, BACKUP_WALLPAPER),
            Some(RegValue::Text(BACKUP_COPY.into()))
        );
        assert_eq!(files.files.lock().unwrap()[BACKUP_COPY], b"original");
        assert!(status(&reg).unwrap().has_previous_wallpaper);
        // Windows перезаписывает TranscodedWallpaper новыми обоями.
        files.write(TRANSCODED, b"new wallpaper").unwrap();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-2", true).unwrap();
        assert_eq!(files.files.lock().unwrap()[BACKUP_COPY], b"original");
    }

    #[test]
    fn restore_brings_back_the_copy_and_the_desktop_values() {
        let (reg, wp, files) = windows_default();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        files.write(TRANSCODED, b"new wallpaper").unwrap();
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(BACKUP_COPY));
        assert_eq!(files.files.lock().unwrap()[BACKUP_COPY], b"original");
        assert_eq!(
            reg.get(DESKTOP, "WallpaperStyle"),
            Some(RegValue::Text("2".into()))
        );
        assert_eq!(
            reg.get(DESKTOP, "TileWallpaper"),
            Some(RegValue::Text("0".into()))
        );
        assert_eq!(
            reg.get(DESKTOP, "JPEGImportQuality"),
            None,
            "значения не было: убрано"
        );
        let s = status(&reg).unwrap();
        assert_eq!(s.wallpaper_tile, None);
        assert!(!s.has_previous_wallpaper);
        // Повторное возвращение ничего не меняет.
        wp.set(r"C:\other.png").unwrap();
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(r"C:\other.png"));
    }

    #[test]
    fn second_tile_keeps_the_very_first_previous_wallpaper() {
        let (reg, wp, files) = windows_default();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-2", true).unwrap();
        assert_eq!(
            status(&reg).unwrap().wallpaper_tile.as_deref(),
            Some("photo-2")
        );
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(BACKUP_COPY));
    }

    #[test]
    fn restoring_twice_in_a_row_backs_up_the_restored_copy_without_self_copy() {
        let (reg, wp, files) = windows_default();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        // Текущие обои теперь — сама копия: новая резервная копия не должна её затирать.
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-2", true).unwrap();
        assert_eq!(files.files.lock().unwrap()[BACKUP_COPY], b"original");
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(BACKUP_COPY));
    }

    #[test]
    fn lost_copy_falls_back_to_windows_default_and_no_wallpaper_stays_none() {
        let (reg, wp, files) = windows_default();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        files.files.lock().unwrap().remove(BACKUP_COPY);
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(DEFAULT_IMG));

        // Обоев не было (сплошной цвет): возвращается пустая строка.
        let reg = FakeReg::default();
        let wp = FakeWallpaper::default();
        let files = FakeFiles::default();
        bundled(&files, "photo-1");
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(""));
    }

    #[test]
    fn our_own_old_wallpaper_is_not_taken_for_the_previous_one() {
        let (reg, wp, files) = windows_default();
        let ours = r"C:\Data\OCHISTKA\Wallpapers\photo-9.jpg";
        files.write(ours, b"ours").unwrap();
        wp.set(ours).unwrap();
        apply_wallpaper_tile(&reg, &wp, &files, Some(RES), "photo-1", true).unwrap();
        assert_eq!(
            reg.get(BACKUP_KEY, BACKUP_WALLPAPER),
            Some(RegValue::Text(PREVIOUS_DEFAULT.into()))
        );
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(DEFAULT_IMG));
    }

    #[test]
    fn custom_picture_goes_through_the_same_backup_and_restore() {
        let (reg, wp, files) = windows_default();
        files.write(r"C:\img\mine.png", b"png").unwrap();
        apply_wallpaper(&reg, &wp, &files, r"C:\img\mine.png", true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(r"C:\img\mine.png"));
        assert_eq!(
            status(&reg).unwrap().wallpaper_tile.as_deref(),
            Some("custom")
        );
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(BACKUP_COPY));
    }

    #[test]
    fn solid_tile_is_generated_at_the_screen_size_and_applied() {
        let (reg, mut wp, files) = windows_default();
        wp.screen = Some((1280, 720));
        let path = apply_wallpaper_tile(&reg, &wp, &files, None, "solid-acid-01", true).unwrap();
        assert_eq!(path, r"C:\Data\OCHISTKA\Wallpapers\solid-acid-01.png");
        let bytes = files.files.lock().unwrap()[&path].clone();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        // IHDR: ширина и высота big-endian со смещения 16.
        assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 1280);
        assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 720);
        assert_eq!(wp.current().unwrap().as_deref(), Some(path.as_str()));
        assert_eq!(
            status(&reg).unwrap().wallpaper_tile.as_deref(),
            Some("solid-acid-01")
        );
        restore_wallpaper(&reg, &wp, &files, true).unwrap();
        assert_eq!(wp.current().unwrap().as_deref(), Some(BACKUP_COPY));
        assert!(apply_wallpaper_tile(&reg, &wp, &files, None, "solid-nope", true).is_err());
    }

    #[test]
    fn wallpaper_tile_refuses_missing_resource_and_bad_ids() {
        let reg = FakeReg::default();
        let wp = FakeWallpaper::default();
        let files = FakeFiles::default();
        bundled(&files, "photo-1");
        for (dir, id) in [
            (Some(RES), "photo-9"),
            (Some(RES), "..\\a"),
            (Some(RES), ""),
            (None, "photo-1"),
            (Some(""), "photo-1"),
        ] {
            assert!(apply_wallpaper_tile(&reg, &wp, &files, dir, id, true).is_err());
        }
        assert!(reg.writes.lock().unwrap().is_empty(), "реестр не тронут");
        assert_eq!(wp.current().unwrap(), None);
    }

    #[test]
    fn accent_palette_has_eight_rgba_entries_with_the_accent_fourth() {
        let palette = accent_palette(0x08, 0x7F, 0xFF);
        assert_eq!(palette.len(), 32);
        assert_eq!(&palette[12..16], &[0x08, 0x7F, 0xFF, 0xFF]);
        assert!(palette[0] > palette[12] && palette[24] < palette[12]);
    }

    #[test]
    fn colour_tile_sets_accent_and_palette_then_restores_all() {
        let old_palette = vec![1u8; 32];
        let reg = FakeReg::with(&[
            (DWM, "AccentColor", RegValue::Dword(0xFF11_2233)),
            (DWM, "ColorPrevalence", RegValue::Dword(0)),
            (
                ACCENT,
                "AccentPalette",
                RegValue::Binary(old_palette.clone()),
            ),
        ]);
        set_accent(&reg, "#087FFF", true).unwrap();
        assert_eq!(reg.get(DWM, "ColorPrevalence"), Some(RegValue::Dword(0)));
        assert_eq!(
            reg.get(ACCENT, "AccentColorMenu"),
            Some(RegValue::Dword(0xFFFF_7F08))
        );
        assert_eq!(
            reg.get(ACCENT, "AccentPalette"),
            Some(RegValue::Binary(accent_palette(0x08, 0x7F, 0xFF)))
        );
        // Второй цвет не затирает снимок: вернётся то, что было до первого.
        set_accent(&reg, "#FF009D", true).unwrap();
        restore_accent(&reg, true).unwrap();
        assert_eq!(
            reg.get(DWM, "AccentColor"),
            Some(RegValue::Dword(0xFF11_2233))
        );
        assert_eq!(
            reg.get(ACCENT, "AccentPalette"),
            Some(RegValue::Binary(old_palette))
        );
        assert_eq!(
            reg.get(DWM, "ColorizationColor"),
            None,
            "добавленное убрано"
        );
        assert_eq!(status(&reg).unwrap().accent_tile, None);
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
}
