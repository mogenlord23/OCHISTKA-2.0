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
    write_dword(reg, "accentStart", abgr(r, g, b))
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

/// Ставит картинку обоями Windows. Прежний фон запоминается один раз.
pub fn apply_wallpaper(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    path: &str,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    validate_image_path(path)?;
    if reg.read(BACKUP_KEY, BACKUP_WALLPAPER)?.is_none() {
        let previous = wallpaper.current()?.unwrap_or_default();
        reg.write(BACKUP_KEY, BACKUP_WALLPAPER, &RegValue::Text(previous))?;
    }
    wallpaper.set(path)
}

/// Возвращает прежний фон. Запись о нём удаляется: следующая смена запомнит новый.
pub fn restore_wallpaper(
    reg: &dyn RegistryPort,
    wallpaper: &dyn WallpaperPort,
    confirmed: bool,
) -> Result<(), AppearanceError> {
    need_confirm(confirmed)?;
    let Some(RegValue::Text(previous)) = reg.read(BACKUP_KEY, BACKUP_WALLPAPER)? else {
        return Err(AppearanceError::NoBackup);
    };
    if !previous.is_empty() {
        wallpaper.set(&previous)?;
    }
    reg.delete(BACKUP_KEY, BACKUP_WALLPAPER)
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
        assert_eq!(
            restore_wallpaper(&reg, &wp, true),
            Err(AppearanceError::NoBackup)
        );
    }
}
