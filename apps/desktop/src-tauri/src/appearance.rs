//! Команда «Оформление»: цвета, прозрачность, тема и обои Windows. Один вход
//! `appearance_request` с полем `action`. Изменяющие действия требуют `confirmed: true`
//! (диалог в интерфейсе); реальные записи (реестр HKCU, обои) в тестах не выполняются.

use std::sync::Arc;

use ochistka_core::appearance::{
    self, AppearanceError, AppearanceStatus, CursorRefreshPort, CursorSpec, FilePort, RegistryPort,
    WallpaperPort, WallpaperSource,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct AppearanceErrorInfo {
    pub kind: String,
    pub message: String,
}

impl From<AppearanceError> for AppearanceErrorInfo {
    fn from(err: AppearanceError) -> Self {
        let kind = match err {
            AppearanceError::NotConfirmed => "notConfirmed",
            AppearanceError::Invalid(_) => "refused",
            AppearanceError::NoBackup => "noBackup",
            AppearanceError::Registry(_) => "registry",
            AppearanceError::Wallpaper(_) => "wallpaper",
            AppearanceError::Unsupported => "unsupported",
            AppearanceError::Cancelled => "cancelled",
        };
        Self {
            kind: kind.into(),
            message: err.to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AppearanceRequest {
    Status {
        /// Id любых обоев из галереи: по нему видно, есть ли оригиналы.
        #[serde(default)]
        sample_id: Option<String>,
    },
    SetAccent {
        color: String,
        confirmed: bool,
    },
    SetTransparency {
        enabled: bool,
        confirmed: bool,
    },
    SetFillTaskbar {
        enabled: bool,
        confirmed: bool,
    },
    SetTheme {
        dark: bool,
        also_system: bool,
        confirmed: bool,
    },
    /// Тёмная или светлая тема только Пуска и панели задач.
    SetSystemTheme {
        dark: bool,
        confirmed: bool,
    },
    /// Вернуть сохранённые прежние значения цветов, прозрачности и темы.
    RestoreAppearance {
        confirmed: bool,
    },
    /// Открывает окно выбора файла и ставит картинку обоями Windows.
    PickAndApplyWallpaper {
        confirmed: bool,
    },
    RestoreWallpaper {
        confirmed: bool,
    },
    /// Плитка обоев: оригинал из папки приложения или превью, если оригинала нет.
    ApplyWallpaperTile {
        id: String,
        solid_color: Option<String>,
        thumb_base64: Option<String>,
        confirmed: bool,
    },
    /// Повторный клик по выбранному цвету: вернуть прежний акцент.
    RestoreAccent {
        confirmed: bool,
    },
    /// Схема курсора Windows из таблицы плиток.
    ApplyCursorScheme {
        id: String,
        color: String,
        secondary: String,
        pattern: String,
        confirmed: bool,
    },
    RestoreCursorScheme {
        confirmed: bool,
    },
    /// Заглушка «Скачать оригиналы»: пишет причину, ничего не скачивает.
    DownloadOriginals {
        confirmed: bool,
    },
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppearanceResponse {
    Status { status: AppearanceStatus },
}

pub struct AppearanceState {
    registry: Arc<dyn RegistryPort>,
    wallpaper: Arc<dyn WallpaperPort>,
    files: Arc<dyn FilePort>,
    refresh: Arc<dyn CursorRefreshPort>,
}

impl AppearanceState {
    pub fn new(
        registry: Arc<dyn RegistryPort>,
        wallpaper: Arc<dyn WallpaperPort>,
        files: Arc<dyn FilePort>,
        refresh: Arc<dyn CursorRefreshPort>,
    ) -> Self {
        Self {
            registry,
            wallpaper,
            files,
            refresh,
        }
    }
}

/// Порты, с которыми работает `run`.
pub struct Ports<'a> {
    pub reg: &'a dyn RegistryPort,
    pub wallpaper: &'a dyn WallpaperPort,
    pub files: &'a dyn FilePort,
    pub refresh: &'a dyn CursorRefreshPort,
}

/// Окно выбора картинки. Вне Windows окна нет: `Unsupported`.
#[cfg(windows)]
fn pick_image() -> Result<String, AppearanceError> {
    rfd::FileDialog::new()
        .set_title("Выберите картинку для обоев")
        .add_filter("Картинки", &["png", "jpg", "jpeg", "bmp"])
        .pick_file()
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or(AppearanceError::Cancelled)
}

#[cfg(not(windows))]
fn pick_image() -> Result<String, AppearanceError> {
    Err(AppearanceError::Unsupported)
}

/// Выполняет запрос. Отдельная функция: проверяется без окна Tauri.
pub fn run(
    ports: &Ports,
    pick: &dyn Fn() -> Result<String, AppearanceError>,
    request: AppearanceRequest,
) -> Result<AppearanceResponse, AppearanceErrorInfo> {
    use AppearanceRequest as R;
    let Ports {
        reg,
        wallpaper,
        files,
        refresh,
    } = *ports;
    let mut sample = None;
    match request {
        R::Status { sample_id } => sample = sample_id,
        R::SetAccent { color, confirmed } => appearance::set_accent(reg, &color, confirmed)?,
        R::SetTransparency { enabled, confirmed } => {
            appearance::set_transparency(reg, enabled, confirmed)?
        }
        R::SetFillTaskbar { enabled, confirmed } => {
            appearance::set_fill_taskbar(reg, enabled, confirmed)?
        }
        R::SetTheme {
            dark,
            also_system,
            confirmed,
        } => appearance::set_theme(reg, dark, also_system, confirmed)?,
        R::SetSystemTheme { dark, confirmed } => {
            appearance::set_system_theme(reg, dark, confirmed)?
        }
        R::RestoreAppearance { confirmed } => appearance::restore(reg, confirmed)?,
        R::PickAndApplyWallpaper { confirmed } => {
            if !confirmed {
                return Err(AppearanceError::NotConfirmed.into());
            }
            let path = pick()?;
            appearance::apply_wallpaper(reg, wallpaper, &path, true)?;
        }
        R::RestoreWallpaper { confirmed } => {
            appearance::restore_wallpaper(reg, wallpaper, confirmed)?
        }
        R::ApplyWallpaperTile {
            id,
            solid_color,
            thumb_base64,
            confirmed,
        } => {
            let source = appearance::apply_wallpaper_tile(
                reg,
                wallpaper,
                files,
                &id,
                solid_color.as_deref(),
                thumb_base64.as_deref(),
                confirmed,
            )?;
            if let WallpaperSource::Thumbnail(path) = source {
                // Оригинала нет: для пользователя это не ошибка, только запись в журнал.
                eprintln!("обои {id}: оригинала нет, поставлено превью {path}");
            }
        }
        R::RestoreAccent { confirmed } => appearance::restore_accent(reg, confirmed)?,
        R::ApplyCursorScheme {
            id,
            color,
            secondary,
            pattern,
            confirmed,
        } => appearance::apply_cursor_scheme(
            reg,
            files,
            refresh,
            &CursorSpec {
                id: &id,
                color: &color,
                secondary: &secondary,
                pattern: &pattern,
            },
            confirmed,
        )?,
        R::RestoreCursorScheme { confirmed } => {
            appearance::restore_cursor_scheme(reg, refresh, confirmed)?
        }
        R::DownloadOriginals { confirmed } => {
            if !confirmed {
                return Err(AppearanceError::NotConfirmed.into());
            }
            let path = appearance::note_originals_download(files)?;
            eprintln!("скачивание оригиналов обоев не подключено, причина записана в {path}");
        }
    }
    let mut status = appearance::status(reg)?;
    if let Some(id) = sample {
        status.originals_missing = appearance::originals_missing(files, &id);
    }
    Ok(AppearanceResponse::Status { status })
}

/// Запросы раздела «Оформление». Работа с реестром и окном выбора идёт в фоновом потоке.
#[tauri::command]
pub async fn appearance_request(
    state: State<'_, AppearanceState>,
    request: AppearanceRequest,
) -> Result<AppearanceResponse, AppearanceErrorInfo> {
    let reg = Arc::clone(&state.registry);
    let wallpaper = Arc::clone(&state.wallpaper);
    let files = Arc::clone(&state.files);
    let refresh = Arc::clone(&state.refresh);
    tauri::async_runtime::spawn_blocking(move || {
        let ports = Ports {
            reg: &*reg,
            wallpaper: &*wallpaper,
            files: &*files,
            refresh: &*refresh,
        };
        run(&ports, &pick_image, request)
    })
    .await
    .map_err(|_| AppearanceErrorInfo {
        kind: "internal".into(),
        message: "внутренняя ошибка при работе с оформлением".into(),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::appearance::RegValue;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Reg(Mutex<HashMap<(String, String), RegValue>>);

    impl RegistryPort for Reg {
        fn read(&self, k: &str, n: &str) -> Result<Option<RegValue>, AppearanceError> {
            Ok(self.0.lock().unwrap().get(&(k.into(), n.into())).cloned())
        }
        fn write(&self, k: &str, n: &str, v: &RegValue) -> Result<(), AppearanceError> {
            self.0
                .lock()
                .unwrap()
                .insert((k.into(), n.into()), v.clone());
            Ok(())
        }
        fn delete(&self, k: &str, n: &str) -> Result<(), AppearanceError> {
            self.0.lock().unwrap().remove(&(k.into(), n.into()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct Wall(Mutex<Option<String>>);

    impl WallpaperPort for Wall {
        fn current(&self) -> Result<Option<String>, AppearanceError> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn set(&self, p: &str) -> Result<(), AppearanceError> {
            *self.0.lock().unwrap() = Some(p.into());
            Ok(())
        }
    }

    struct NoFiles;
    impl FilePort for NoFiles {
        fn data_dir(&self) -> Result<String, AppearanceError> {
            Ok(r"C:\D".into())
        }
        fn exists(&self, _: &str) -> bool {
            false
        }
        fn write(&self, _: &str, _: &[u8]) -> Result<(), AppearanceError> {
            Ok(())
        }
    }
    struct NoRefresh;
    impl CursorRefreshPort for NoRefresh {
        fn refresh(&self) -> Result<(), AppearanceError> {
            Ok(())
        }
    }

    fn go(
        reg: &Reg,
        wall: &Wall,
        pick: &dyn Fn() -> Result<String, AppearanceError>,
        request: AppearanceRequest,
    ) -> Result<AppearanceResponse, AppearanceErrorInfo> {
        let ports = Ports {
            reg,
            wallpaper: wall,
            files: &NoFiles,
            refresh: &NoRefresh,
        };
        run(&ports, pick, request)
    }

    fn no_pick() -> Result<String, AppearanceError> {
        panic!("окно выбора не должно открываться")
    }

    #[test]
    fn unconfirmed_requests_are_refused_and_do_not_open_dialogs() {
        let (reg, wall) = (Reg::default(), Wall::default());
        for request in [
            AppearanceRequest::SetAccent {
                color: "#FF0000".into(),
                confirmed: false,
            },
            AppearanceRequest::PickAndApplyWallpaper { confirmed: false },
            AppearanceRequest::RestoreAppearance { confirmed: false },
        ] {
            let err = go(&reg, &wall, &no_pick, request).unwrap_err();
            assert_eq!(err.kind, "notConfirmed");
        }
        assert!(reg.0.lock().unwrap().is_empty());
    }

    #[test]
    fn accent_then_status_reports_it() {
        let (reg, wall) = (Reg::default(), Wall::default());
        let req = AppearanceRequest::SetAccent {
            color: "#087FFF".into(),
            confirmed: true,
        };
        let AppearanceResponse::Status { status } = go(&reg, &wall, &no_pick, req).unwrap();
        assert_eq!(status.accent.as_deref(), Some("#087FFF"));
        assert!(status.has_backup);
    }

    #[test]
    fn picked_wallpaper_is_applied_and_cancel_changes_nothing() {
        let (reg, wall) = (Reg::default(), Wall::default());
        let pick_ok = || Ok(r"C:\img\a.png".to_string());
        let req = AppearanceRequest::PickAndApplyWallpaper { confirmed: true };
        go(&reg, &wall, &pick_ok, req.clone()).unwrap();
        assert_eq!(wall.current().unwrap().as_deref(), Some(r"C:\img\a.png"));
        let cancel = || Err(AppearanceError::Cancelled);
        let err = go(&reg, &wall, &cancel, req).unwrap_err();
        assert_eq!(err.kind, "cancelled");
    }

    #[test]
    fn status_request_works_without_fields_and_cursor_request_parses() {
        let r: AppearanceRequest = serde_json::from_str(r#"{"action":"status"}"#).unwrap();
        assert_eq!(r, AppearanceRequest::Status { sample_id: None });
        let r: AppearanceRequest = serde_json::from_str(
            r##"{"action":"applyCursorScheme","id":"cursor-red","color":"#FF0000","secondary":"#FF0000","pattern":"plain","confirmed":true}"##,
        )
        .unwrap();
        let (reg, wall) = (Reg::default(), Wall::default());
        let AppearanceResponse::Status { status } = go(&reg, &wall, &no_pick, r).unwrap();
        assert_eq!(status.cursor_scheme.as_deref(), Some("cursor-red"));
    }

    #[test]
    fn request_json_shape() {
        let r: AppearanceRequest = serde_json::from_str(
            r##"{"action":"setTheme","dark":true,"alsoSystem":false,"confirmed":true}"##,
        )
        .unwrap();
        assert_eq!(
            r,
            AppearanceRequest::SetTheme {
                dark: true,
                also_system: false,
                confirmed: true
            }
        );
    }
}
