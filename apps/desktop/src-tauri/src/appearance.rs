//! Команда «Оформление»: цвета, прозрачность, обои и курсоры Windows. Один вход
//! `appearance_request` с полем `action`. Изменяющие действия требуют `confirmed: true`
//! (интерфейс всегда шлёт его: клик по плитке и есть действие, всё обратимо); реальные
//! записи (реестр HKCU, обои) в тестах не выполняются.

use std::sync::Arc;

use ochistka_core::appearance::{
    self, AppearanceError, AppearanceStatus, CursorRefreshPort, CursorSpec, FilePort, RegistryPort,
    WallpaperPort,
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
    Status,
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
    /// Вернуть сохранённые прежние значения цветов и прозрачности.
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
    /// Плитка обоев: оригинал из ресурсов приложения копируется в данные и ставится обоями.
    ApplyWallpaperTile {
        id: String,
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
    /// Заранее рисует файлы схем курсоров (кэш по отпечатку). Реестр не трогает.
    PrepareCursorSchemes {
        schemes: Vec<PrepareScheme>,
    },
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PrepareScheme {
    pub id: String,
    pub color: String,
    pub secondary: String,
    pub pattern: String,
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
    /// Папка ресурсов установки: в ней `wallpapers\<id>.jpg`.
    resources: Option<String>,
}

impl AppearanceState {
    pub fn new(
        registry: Arc<dyn RegistryPort>,
        wallpaper: Arc<dyn WallpaperPort>,
        files: Arc<dyn FilePort>,
        refresh: Arc<dyn CursorRefreshPort>,
        resources: Option<String>,
    ) -> Self {
        Self {
            registry,
            wallpaper,
            files,
            refresh,
            resources,
        }
    }
}

/// Путь без префикса `\\?\`, который Tauri кладёт в пути ресурсов на Windows.
pub fn plain_path(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix("\\\\?\\").unwrap_or(&text).to_string()
}

/// Порты, с которыми работает `run`.
pub struct Ports<'a> {
    pub reg: &'a dyn RegistryPort,
    pub wallpaper: &'a dyn WallpaperPort,
    pub files: &'a dyn FilePort,
    pub refresh: &'a dyn CursorRefreshPort,
    pub resources: Option<&'a str>,
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
        resources,
    } = *ports;
    let colors_touched = matches!(
        request,
        R::SetAccent { .. }
            | R::SetTransparency { .. }
            | R::SetFillTaskbar { .. }
            | R::RestoreAppearance { .. }
            | R::RestoreAccent { .. }
    );
    match request {
        R::Status => {}
        R::SetAccent { color, confirmed } => appearance::set_accent(reg, &color, confirmed)?,
        R::SetTransparency { enabled, confirmed } => {
            appearance::set_transparency(reg, enabled, confirmed)?
        }
        R::SetFillTaskbar { enabled, confirmed } => {
            appearance::set_fill_taskbar(reg, enabled, confirmed)?
        }
        R::RestoreAppearance { confirmed } => appearance::restore(reg, confirmed)?,
        R::PickAndApplyWallpaper { confirmed } => {
            if !confirmed {
                return Err(AppearanceError::NotConfirmed.into());
            }
            let path = pick()?;
            appearance::apply_wallpaper(reg, wallpaper, files, &path, true)?;
        }
        R::RestoreWallpaper { confirmed } => {
            appearance::restore_wallpaper(reg, wallpaper, files, confirmed)?
        }
        R::ApplyWallpaperTile { id, confirmed } => {
            appearance::apply_wallpaper_tile(reg, wallpaper, files, resources, &id, confirmed)?;
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
        R::PrepareCursorSchemes { schemes } => {
            // Схема с ошибкой данных не мешает остальным: её просто пропускаем.
            for scheme in &schemes {
                let _ = appearance::prepare_cursor_scheme(
                    files,
                    &CursorSpec {
                        id: &scheme.id,
                        color: &scheme.color,
                        secondary: &scheme.secondary,
                        pattern: &scheme.pattern,
                    },
                );
            }
        }
    }
    if colors_touched {
        // Одна рассылка на запрос; сбой рассылки не отменяет уже записанное оформление.
        let _ = refresh.colors_changed();
    }
    let status = appearance::status(reg)?;
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
    let resources = state.resources.clone();
    if let AppearanceRequest::PrepareCursorSchemes { .. } = request {
        // Подготовка идёт в отдельном фоновом потоке: ответ сразу, очередь команд не занята.
        std::thread::spawn(move || {
            let ports = Ports {
                reg: &*reg,
                wallpaper: &*wallpaper,
                files: &*files,
                refresh: &*refresh,
                resources: resources.as_deref(),
            };
            let _ = run(&ports, &pick_image, request);
        });
        let status = appearance::status(&*state.registry)?;
        return Ok(AppearanceResponse::Status { status });
    }
    tauri::async_runtime::spawn_blocking(move || {
        let ports = Ports {
            reg: &*reg,
            wallpaper: &*wallpaper,
            files: &*files,
            refresh: &*refresh,
            resources: resources.as_deref(),
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
        fn copy(&self, _: &str, _: &str) -> Result<(), AppearanceError> {
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
            resources: None,
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
        assert_eq!(r, AppearanceRequest::Status);
        let r: AppearanceRequest = serde_json::from_str(
            r##"{"action":"applyCursorScheme","id":"cursor-red","color":"#FF0000","secondary":"#FF0000","pattern":"plain","confirmed":true}"##,
        )
        .unwrap();
        let (reg, wall) = (Reg::default(), Wall::default());
        let AppearanceResponse::Status { status } = go(&reg, &wall, &no_pick, r).unwrap();
        assert_eq!(status.cursor_scheme.as_deref(), Some("cursor-red"));
    }

    struct BundledFiles;
    impl FilePort for BundledFiles {
        fn data_dir(&self) -> Result<String, AppearanceError> {
            Ok(r"C:\D".into())
        }
        fn exists(&self, path: &str) -> bool {
            path.starts_with(r"C:\App\wallpapers\")
                || path == r"C:\old.jpg"
                || path.starts_with(r"C:\D\Backup\")
        }
        fn write(&self, _: &str, _: &[u8]) -> Result<(), AppearanceError> {
            Ok(())
        }
        fn copy(&self, _: &str, _: &str) -> Result<(), AppearanceError> {
            Ok(())
        }
    }

    #[test]
    fn wallpaper_tile_applies_then_second_request_restores_previous() {
        let (reg, wall) = (Reg::default(), Wall::default());
        wall.set(r"C:\old.jpg").unwrap();
        let ports = Ports {
            reg: &reg,
            wallpaper: &wall,
            files: &BundledFiles,
            refresh: &NoRefresh,
            resources: Some(r"C:\App"),
        };
        let apply = AppearanceRequest::ApplyWallpaperTile {
            id: "photo-1".into(),
            confirmed: true,
        };
        let AppearanceResponse::Status { status } = run(&ports, &no_pick, apply).unwrap();
        assert_eq!(status.wallpaper_tile.as_deref(), Some("photo-1"));
        assert_eq!(
            wall.current().unwrap().as_deref(),
            Some(r"C:\D\Wallpapers\photo-1.jpg")
        );
        let restore = AppearanceRequest::RestoreWallpaper { confirmed: true };
        let AppearanceResponse::Status { status } = run(&ports, &no_pick, restore).unwrap();
        assert_eq!(status.wallpaper_tile, None);
        assert_eq!(
            wall.current().unwrap().as_deref(),
            Some(r"C:\D\Backup\wallpaper-previous.jpg"),
            "возвращается копия прежнего файла, а не исходный путь"
        );
    }

    #[derive(Default)]
    struct RecordingFiles(Mutex<Vec<String>>);
    impl FilePort for RecordingFiles {
        fn data_dir(&self) -> Result<String, AppearanceError> {
            Ok(r"C:\D".into())
        }
        fn exists(&self, path: &str) -> bool {
            self.0.lock().unwrap().iter().any(|p| p == path)
        }
        fn write(&self, path: &str, _: &[u8]) -> Result<(), AppearanceError> {
            self.0.lock().unwrap().push(path.into());
            Ok(())
        }
        fn copy(&self, _: &str, _: &str) -> Result<(), AppearanceError> {
            Ok(())
        }
    }

    #[test]
    fn prepare_cursor_schemes_draws_files_but_leaves_the_registry_alone() {
        let (reg, wall) = (Reg::default(), Wall::default());
        let files = RecordingFiles::default();
        let ports = Ports {
            reg: &reg,
            wallpaper: &wall,
            files: &files,
            refresh: &NoRefresh,
            resources: None,
        };
        let request: AppearanceRequest = serde_json::from_str(
            r##"{"action":"prepareCursorSchemes","schemes":[{"id":"cursor-red","color":"#FF0000","secondary":"#FF0000","pattern":"plain"},{"id":"bad","color":"red","secondary":"x","pattern":"plain"}]}"##,
        )
        .unwrap();
        run(&ports, &no_pick, request).unwrap();
        assert!(files.exists(r"C:\D\Cursors\cursor-red\Arrow.cur"));
        assert!(reg.0.lock().unwrap().is_empty());
    }

    #[test]
    fn plain_path_drops_the_extended_prefix() {
        assert_eq!(plain_path(std::path::Path::new(r"\\?\C:\App")), r"C:\App");
        assert_eq!(plain_path(std::path::Path::new("/opt/app")), "/opt/app");
    }

    #[derive(Default)]
    struct CountingRefresh(Mutex<(u32, u32)>);
    impl CursorRefreshPort for CountingRefresh {
        fn refresh(&self) -> Result<(), AppearanceError> {
            self.0.lock().unwrap().0 += 1;
            Ok(())
        }
        fn colors_changed(&self) -> Result<(), AppearanceError> {
            self.0.lock().unwrap().1 += 1;
            Ok(())
        }
    }

    #[test]
    fn colour_requests_broadcast_exactly_once_and_others_never() {
        let (reg, wall) = (Reg::default(), Wall::default());
        let refresh = CountingRefresh::default();
        let ports = Ports {
            reg: &reg,
            wallpaper: &wall,
            files: &NoFiles,
            refresh: &refresh,
            resources: None,
        };
        for (request, broadcasts) in [
            (
                AppearanceRequest::SetAccent {
                    color: "#087FFF".into(),
                    confirmed: true,
                },
                1,
            ),
            (
                AppearanceRequest::SetTransparency {
                    enabled: true,
                    confirmed: true,
                },
                1,
            ),
            (
                AppearanceRequest::SetFillTaskbar {
                    enabled: true,
                    confirmed: true,
                },
                1,
            ),
            (AppearanceRequest::RestoreAppearance { confirmed: true }, 1),
            (AppearanceRequest::Status, 0),
        ] {
            let before = refresh.0.lock().unwrap().1;
            run(&ports, &no_pick, request).unwrap();
            assert_eq!(refresh.0.lock().unwrap().1 - before, broadcasts);
        }
    }

    #[test]
    fn theme_requests_are_gone() {
        assert!(serde_json::from_str::<AppearanceRequest>(
            r#"{"action":"setTheme","dark":true,"alsoSystem":false,"confirmed":true}"#
        )
        .is_err());
    }
}
