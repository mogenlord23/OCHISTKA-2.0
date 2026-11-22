//! Команды раздела VPN: подписка, серверы, задержка, подключение, маршруты.
//! Сеть (загрузка подписки) и процессы выполняет Rust; интерфейс только вызывает команды.

use std::path::PathBuf;
use std::sync::Arc;

use ochistka_core::vpn::engine::PrepareProgress;
use ochistka_core::vpn::routes::RoutingRules;
use ochistka_core::vpn::service::{ConnectionStatus, ImportSummary, VpnService, VpnSnapshot};
use ochistka_core::vpn::sysproxy::SystemProxy;
use ochistka_core::vpn::VpnError;
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::CommandError;

/// Общий сервис VPN. Создаётся при запуске (см. `build_service`).
pub struct VpnState(pub Arc<VpnService>);

impl From<VpnError> for CommandError {
    fn from(err: VpnError) -> Self {
        let kind = match err {
            VpnError::CoreMissing(_) => "engineMissing",
            VpnError::Prepare(_) => "prepare",
            VpnError::NotHttps | VpnError::EmptyInput | VpnError::Rules(_) => "invalidTarget",
            _ => "probe",
        };
        CommandError {
            kind: kind.into(),
            message: err.to_string(),
        }
    }
}

/// Папки, где ищется ядро: ресурсы приложения, рядом с exe и (в разработке) папка исходников.
pub fn core_dirs(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let plain = |p: PathBuf| PathBuf::from(crate::appearance::plain_path(&p));
    if let Some(res) = resource_dir {
        dirs.push(plain(res.join("vpn-core")));
        dirs.push(plain(res.join("resources").join("vpn-core")));
    }
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
    {
        dirs.push(exe_dir.join("vpn-core"));
        dirs.push(exe_dir.join("resources").join("vpn-core"));
    }
    #[cfg(debug_assertions)]
    dirs.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("vpn-core"),
    );
    dirs
}

/// Собирает сервис и сразу убирает последствия аварийного завершения прошлого запуска
/// (оставшееся ядро и системный прокси).
pub fn build_service(
    resource_dir: Option<PathBuf>,
    coord: Arc<ochistka_core::desync::coordinator::NetCoordinator>,
) -> VpnState {
    let service = VpnService::new(
        ochistka_platform::vpn::vpn_data_dir(),
        ochistka_platform::vpn::current_vpn_host(core_dirs(resource_dir)),
        SystemProxy::new(
            ochistka_platform::vpn::current_registry_port(),
            ochistka_platform::vpn::current_proxy_refresh(),
        ),
        ochistka_platform::vpn::current_http_fetch(),
        ochistka_platform::vpn::current_dialer(),
    )
    .with_coordinator(coord);
    service.recover_on_start();
    VpnState(Arc::new(service))
}

async fn blocking<T: Send + 'static>(
    state: &State<'_, VpnState>,
    job: impl FnOnce(&VpnService) -> Result<T, VpnError> + Send + 'static,
) -> Result<T, CommandError> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || job(&service).map_err(CommandError::from))
        .await
        .map_err(|err| CommandError {
            kind: "probe".into(),
            message: err.to_string(),
        })?
}

#[tauri::command]
pub async fn vpn_snapshot(state: State<'_, VpnState>) -> Result<VpnSnapshot, CommandError> {
    blocking(&state, |s| Ok(s.snapshot())).await
}

#[tauri::command]
pub async fn vpn_status(state: State<'_, VpnState>) -> Result<ConnectionStatus, CommandError> {
    blocking(&state, |s| Ok(s.status())).await
}

/// Ссылка https:// подписки, вставленный список ссылок или текст файла.
#[tauri::command]
pub async fn vpn_import(
    state: State<'_, VpnState>,
    input: String,
) -> Result<ImportSummary, CommandError> {
    blocking(&state, move |s| s.import(&input)).await
}

#[tauri::command]
pub async fn vpn_select(state: State<'_, VpnState>, id: String) -> Result<(), CommandError> {
    blocking(&state, move |s| s.select(&id)).await
}

#[tauri::command]
pub async fn vpn_set_rules(
    state: State<'_, VpnState>,
    rules: RoutingRules,
) -> Result<RoutingRules, CommandError> {
    blocking(&state, move |s| s.set_rules(&rules)).await
}

#[tauri::command]
pub async fn vpn_measure_latency(state: State<'_, VpnState>) -> Result<VpnSnapshot, CommandError> {
    blocking(&state, |s| {
        s.measure_latency();
        Ok(s.snapshot())
    })
    .await
}

#[tauri::command]
pub async fn vpn_connect(
    state: State<'_, VpnState>,
    on_progress: Channel<PrepareProgress>,
) -> Result<ConnectionStatus, CommandError> {
    blocking(&state, move |s| {
        s.connect_with(&mut |event| {
            let _ = on_progress.send(event);
        })
    })
    .await
}

#[tauri::command]
pub async fn vpn_disconnect(state: State<'_, VpnState>) -> Result<ConnectionStatus, CommandError> {
    blocking(&state, |s| Ok(s.disconnect())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_map_to_ui_kinds_with_russian_text() {
        let e: CommandError = VpnError::NotHttps.into();
        assert_eq!(e.kind, "invalidTarget");
        assert_eq!(e.message, "Разрешены только ссылки https://");
        let e: CommandError = VpnError::CoreMissing("x".into()).into();
        assert_eq!(e.kind, "engineMissing");
    }

    #[test]
    fn preparation_failure_maps_to_its_own_kind() {
        let e: CommandError = VpnError::Prepare("проверьте интернет".into()).into();
        assert_eq!(e.kind, "prepare");
        assert_eq!(e.message, "Не удалось подготовить VPN: проверьте интернет");
    }

    #[test]
    fn core_dirs_include_resources_and_exe_neighbours() {
        let dirs = core_dirs(Some(PathBuf::from("/app/res")));
        assert_eq!(dirs[0], PathBuf::from("/app/res/vpn-core"));
        assert!(dirs.iter().any(|d| d.ends_with("resources/vpn-core")));
    }
}
