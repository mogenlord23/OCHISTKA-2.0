use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ochistka_core::desync::coordinator::{Mode, NetCoordinator, Owner};
use ochistka_core::desync::user_sites::UserSites;
use ochistka_core::vpn::service::{ConnState, VpnService};
use ochistka_platform::zapret::{Process, Profile};
use serde::Serialize;
use tauri::State;

use crate::commands::CommandError;
use crate::vpn::VpnState;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/zapret_embedded.rs"));
}

struct Running {
    process: Process,
    _embedded: Option<tempfile::TempDir>,
}

fn unpack_embedded() -> Result<tempfile::TempDir, String> {
    if embedded::FILES.is_empty() {
        return Err("В этой сборке нет встроенного Zapret".into());
    }
    let dir = tempfile::Builder::new()
        .prefix("ochistka-zapret-")
        .tempdir()
        .map_err(|err| err.to_string())?;
    for (file, bytes) in embedded::FILES {
        let path = dir.path().join(file);
        std::fs::create_dir_all(path.parent().expect("embedded path parent"))
            .map_err(|err| err.to_string())?;
        std::fs::write(path, bytes).map_err(|err| err.to_string())?;
    }
    Ok(dir)
}

pub fn check_embedded() -> Result<(), String> {
    let assets = unpack_embedded()?;
    Profile::read(assets.path())?.validate(None)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    running: bool,
    configured: bool,
    elevated: bool,
    embedded: bool,
}

pub struct Zapret {
    coord: Arc<NetCoordinator>,
    process: Mutex<Option<Running>>,
    data: PathBuf,
}

fn error(message: String) -> CommandError {
    CommandError {
        kind: "zapret".into(),
        message,
    }
}

impl Zapret {
    pub fn new(coord: Arc<NetCoordinator>) -> Self {
        Self {
            coord,
            process: Mutex::new(None),
            data: ochistka_platform::desync::desync_data_dir().join("zapret"),
        }
    }

    fn folder(&self) -> Option<PathBuf> {
        std::fs::read_to_string(self.data.join("folder.txt"))
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("OCHISTKA_ZAPRET_DIR").map(PathBuf::from))
            .filter(|path| path.is_absolute())
    }

    fn status(&self) -> Result<Status, String> {
        let mut process = self.process.lock().map_err(|err| err.to_string())?;
        let running = match process.as_mut() {
            Some(child) => child.process.is_running()?,
            None => false,
        };
        if !running && process.take().is_some() {
            self.coord.release(Owner::Desync);
        }
        let embedded = !embedded::FILES.is_empty();
        Ok(Status {
            running,
            configured: embedded || self.folder().is_some(),
            elevated: ochistka_platform::vpn::is_elevated(),
            embedded,
        })
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut process = self.process.lock().map_err(|err| err.to_string())?;
        if let Some(child) = process.as_mut() {
            self.coord.begin_stop(Owner::Desync);
            if let Err(err) = child.process.stop() {
                self.coord.mark_unclean(Owner::Desync);
                return Err(err);
            }
            *process = None;
            self.coord.release(Owner::Desync);
        }
        Ok(())
    }

    fn start(&self, vpn: &VpnService) -> Result<(), String> {
        let mut process = self.process.lock().map_err(|err| err.to_string())?;
        if let Some(child) = process.as_mut() {
            if child.process.is_running()? {
                return Ok(());
            }
            *process = None;
            self.coord.release(Owner::Desync);
        }
        if !ochistka_platform::vpn::is_elevated() {
            return Err(
                "Для запуска WinDivert откройте эту сборку ОЧИСТКИ от имени администратора.".into(),
            );
        }
        let (folder, extracted) = if embedded::FILES.is_empty() {
            (
                self.folder()
                    .ok_or("Сначала выберите папку Zapret 1.9.0b")?,
                None,
            )
        } else {
            let assets = unpack_embedded()?;
            (assets.path().to_path_buf(), Some(assets))
        };
        let profile = Profile::read(&folder)?;
        let extra = UserSites::new(&self.data).load();
        let merged = self.data.join("list-general-merged.txt");
        let merged_list = if extra.is_empty() {
            None
        } else {
            profile.merge_list(&merged, &extra)?;
            Some(merged.as_path())
        };
        profile.validate(merged_list)?;
        if matches!(
            self.coord.mode(),
            Some(Mode::Active(Owner::Vpn) | Mode::Unclean(Owner::Vpn))
        ) {
            if vpn.disconnect().state != ConnState::Disconnected {
                return Err("VPN не завершил отключение. Zapret не запущен.".into());
            }
        }
        self.coord
            .begin(Owner::Desync)
            .map_err(|err| err.to_string())?;
        match profile.start(merged_list) {
            Ok(child) => {
                *process = Some(Running {
                    process: child,
                    _embedded: extracted,
                });
                self.coord.activate(Owner::Desync);
                Ok(())
            }
            Err(err) => {
                self.coord.release(Owner::Desync);
                Err(err)
            }
        }
    }
}

#[tauri::command]
pub fn zapret_status(state: State<'_, Arc<Zapret>>) -> Result<Status, CommandError> {
    state.status().map_err(error)
}

#[tauri::command]
pub async fn zapret_select_folder(state: State<'_, Arc<Zapret>>) -> Result<Status, CommandError> {
    if !embedded::FILES.is_empty() {
        return state.status().map_err(error);
    }
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Папка Zapret 1.9.0b")
            .pick_folder()
        {
            Profile::read(&folder).map_err(error)?;
            std::fs::create_dir_all(&state.data).map_err(|err| error(err.to_string()))?;
            std::fs::write(
                state.data.join("folder.txt"),
                folder.to_string_lossy().as_bytes(),
            )
            .map_err(|err| error(err.to_string()))?;
        }
        state.status().map_err(error)
    })
    .await
    .map_err(|err| error(err.to_string()))?
}

#[tauri::command]
pub async fn zapret_set(
    state: State<'_, Arc<Zapret>>,
    vpn: State<'_, VpnState>,
    enabled: bool,
) -> Result<Status, CommandError> {
    let state = Arc::clone(state.inner());
    let vpn = Arc::clone(&vpn.0);
    tauri::async_runtime::spawn_blocking(move || {
        if enabled {
            state.start(&vpn)
        } else {
            state.stop()
        }
        .map_err(error)?;
        state.status().map_err(error)
    })
    .await
    .map_err(|err| error(err.to_string()))?
}
