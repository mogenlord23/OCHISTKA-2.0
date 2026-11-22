//! «Обход сайтов»: три команды (статус, включить, выключить). Вся логика — в
//! `ochistka_core::site_bypass`; здесь только связка с VPN и системным прокси ОС.

use std::sync::Arc;

use ochistka_core::desync::coordinator::NetCoordinator;
use ochistka_core::desync::user_sites::UserSites;
use ochistka_core::site_bypass::{
    vpn_conflict_guard, SiteBypassService, SiteBypassState as State_, SiteBypassStatus,
};
use ochistka_core::vpn::service::ConnState;
#[cfg(not(windows))]
use proxy_bypass::BypassController;
use serde::Serialize;
use tauri::State;

use crate::commands::CommandError;
use crate::vpn::VpnState;

pub struct SiteBypassState(pub Arc<SiteBypassService>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteBypassDto {
    pub active: bool,
    pub error: bool,
    pub message: String,
}

fn to_dto(status: SiteBypassStatus) -> SiteBypassDto {
    SiteBypassDto {
        active: status.state == State_::On,
        error: status.state == State_::Error,
        message: status.message,
    }
}

/// Создаёт сервис, связывает его с VPN в обе стороны (через `Weak`) и возвращает прокси после аварии.
pub fn build(vpn: &VpnState, coord: Arc<NetCoordinator>) -> SiteBypassState {
    let dir = ochistka_platform::desync::desync_data_dir();
    let sites_dir = dir.clone();
    let user_sites: ochistka_core::site_bypass::UserSitesLoader =
        Box::new(move || UserSites::new(&sites_dir).load());
    // Windows: служба обхода (права один раз при установке, без запросов). Остальные ОС: локальный
    // прокси и системный прокси.
    #[cfg(windows)]
    let service = Arc::new(SiteBypassService::with_engine(
        ochistka_platform::windows::bypass_service::current_engine(),
        coord,
        user_sites,
    ));
    #[cfg(not(windows))]
    let service = {
        let controller = Arc::new(BypassController::new(
            ochistka_platform::sys_proxy::current_system_proxy(),
            dir.join("proxy-backup.txt"),
        ));
        Arc::new(SiteBypassService::new(controller, coord, user_sites))
    };
    // VPN → включает VPN: сначала выключается обход, иначе VPN не включается.
    vpn.0
        .set_conflict_guard(vpn_conflict_guard(Arc::downgrade(&service)));
    // Обход → включает обход: сначала выключается работающий VPN.
    let weak_vpn = Arc::downgrade(&vpn.0);
    service.set_vpn_stopper(Box::new(move || {
        let Some(vpn) = weak_vpn.upgrade() else {
            return Err("состояние VPN недоступно".into());
        };
        if matches!(
            vpn.status().state,
            ConnState::Disconnected | ConnState::Error
        ) {
            return Ok(());
        }
        let status = vpn.disconnect();
        if matches!(status.state, ConnState::Disconnected | ConnState::Error) {
            Ok(())
        } else {
            Err(status.message)
        }
    }));
    service.recover_on_start();
    SiteBypassState(service)
}

async fn run(
    state: &State<'_, SiteBypassState>,
    job: impl FnOnce(&SiteBypassService) -> SiteBypassStatus + Send + 'static,
) -> Result<SiteBypassDto, CommandError> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || to_dto(job(&service)))
        .await
        .map_err(|err| CommandError {
            kind: "siteBypass".into(),
            message: err.to_string(),
        })
}

#[tauri::command]
pub async fn site_bypass_status(
    state: State<'_, SiteBypassState>,
) -> Result<SiteBypassDto, CommandError> {
    run(&state, |s| s.status()).await
}

#[tauri::command]
pub async fn site_bypass_start(
    state: State<'_, SiteBypassState>,
) -> Result<SiteBypassDto, CommandError> {
    run(&state, |s| s.start()).await
}

#[tauri::command]
pub async fn site_bypass_stop(
    state: State<'_, SiteBypassState>,
) -> Result<SiteBypassDto, CommandError> {
    run(&state, |s| s.stop()).await
}
