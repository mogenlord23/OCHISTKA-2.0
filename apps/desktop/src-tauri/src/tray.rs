use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::Duration;

use ochistka_core::vpn::service::ConnState;
use serde::{Deserialize, Serialize};
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Rect, WindowEvent};

use crate::commands::{AppState, CommandError};
use crate::vpn::VpnState;

const ICON: &str = "ochistka-tray";
const PANEL: &str = "tray-panel";
const MAIN: &str = "main";

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrayAction {
    Open,
    ToggleVpn,
    Vpn,
    Viruses,
    Processes,
    Programs,
    Disks,
    Computer,
    Design,
    Exit,
}

const ACTIONS: [(&str, &str, TrayAction); 10] = [
    ("tray-open", "Открыть OCHISTKA", TrayAction::Open),
    ("tray-toggle-vpn", "Переключить VPN", TrayAction::ToggleVpn),
    ("tray-vpn", "VPN", TrayAction::Vpn),
    ("tray-viruses", "Вирусы", TrayAction::Viruses),
    ("tray-processes", "Процессы", TrayAction::Processes),
    ("tray-programs", "Программы", TrayAction::Programs),
    ("tray-disks", "Диски", TrayAction::Disks),
    ("tray-computer", "Компьютер", TrayAction::Computer),
    ("tray-design", "Оформление", TrayAction::Design),
    ("tray-exit", "Выйти", TrayAction::Exit),
];

#[derive(Default)]
struct Hover {
    icon: bool,
    panel: bool,
    version: u64,
}

type HoverState = Mutex<Hover>;

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    app.manage(HoverState::default());
    let mut menu = MenuBuilder::new(app);
    for (id, text, _) in ACTIONS {
        menu = menu.text(id, text);
    }
    TrayIconBuilder::with_id(ICON)
        .icon(tauri::include_image!("icons/icon.png"))
        .menu(&menu.build()?)
        .tooltip("OCHISTKA")
        .show_menu_on_left_click(cfg!(target_os = "linux"))
        .on_menu_event(|app, event| {
            if let Some((_, _, action)) = ACTIONS.iter().find(|(id, _, _)| *id == event.id().0) {
                let (app, action) = (app.clone(), *action);
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = tray_action(app.clone(), action).await {
                        show_main(&app);
                        let _ = app.emit_to(MAIN, "tray-action-error", error.message);
                    }
                });
            }
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Enter { position, rect, .. } => {
                show_panel(tray.app_handle(), position, rect)
            }
            TrayIconEvent::Leave { .. } => update_hover(tray.app_handle(), false, false),
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => {
                hide_panel(tray.app_handle());
                show_main(tray.app_handle());
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

pub fn window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == MAIN
            && window.app_handle().tray_by_id(ICON).is_some()
            && window.hide().is_ok()
        {
            api.prevent_close();
        }
    }
}

fn show_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window(MAIN) {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
}

fn hide_panel(app: &AppHandle) {
    if let Some(panel) = app.get_webview_window(PANEL) {
        let _ = panel.hide();
        let _ = app.emit_to(PANEL, "tray-closed", ());
    }
    if let Some(state) = app.try_state::<HoverState>() {
        if let Ok(mut hover) = state.lock() {
            hover.panel = false;
        }
    }
}

fn show_panel(app: &AppHandle, cursor: PhysicalPosition<f64>, icon: Rect) {
    let Some(panel) = app.get_webview_window(PANEL) else {
        return;
    };
    update_hover(app, false, true);
    if panel.is_visible().unwrap_or(false) {
        return;
    }
    let Ok(Some(monitor)) = app.monitor_from_point(cursor.x, cursor.y) else {
        return;
    };
    let area = monitor.work_area();
    let origin = icon.position.to_physical::<i32>(monitor.scale_factor());
    let icon_size = icon.size.to_physical::<i32>(monitor.scale_factor());
    let Ok(size) = panel.outer_size() else { return };
    let x = origin.x + icon_size.width - size.width as i32;
    let above = origin.y - size.height as i32 - 5;
    let y = if above >= area.position.y {
        above
    } else {
        origin.y + icon_size.height + 5
    };
    let limit_x = area.position.x + area.size.width.saturating_sub(size.width) as i32;
    let limit_y = area.position.y + area.size.height.saturating_sub(size.height) as i32;
    let position = PhysicalPosition::new(
        x.clamp(area.position.x, limit_x),
        y.clamp(area.position.y, limit_y),
    );
    if panel
        .set_position(position)
        .and_then(|_| panel.show())
        .is_ok()
    {
        let _ = app.emit_to(PANEL, "tray-open", ());
    }
}

fn update_hover(app: &AppHandle, panel: bool, inside: bool) {
    let state = app.state::<HoverState>();
    let Ok(mut hover) = state.lock() else { return };
    if panel {
        hover.panel = inside
    } else {
        hover.icon = inside
    }
    hover.version = hover.version.wrapping_add(1);
    if hover.panel || hover.icon {
        return;
    }
    let version = hover.version;
    drop(hover);
    let app = app.clone();
    // В этот промежуток указатель может пересечь зазор между значком и окном.
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(380));
        let main_thread = app.clone();
        let _ = app.run_on_main_thread(move || {
            let should_hide = main_thread
                .state::<HoverState>()
                .lock()
                .is_ok_and(|hover| hover.version == version && !hover.panel && !hover.icon);
            if should_hide {
                hide_panel(&main_thread);
            }
        });
    });
}

#[tauri::command]
pub fn tray_hover(window: tauri::Window, inside: bool) {
    if window.label() == PANEL {
        update_hover(window.app_handle(), true, inside);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayStatus {
    vpn_connected: bool,
}

#[tauri::command]
pub fn tray_status(app: AppHandle) -> TrayStatus {
    TrayStatus {
        vpn_connected: app.state::<VpnState>().0.status().state == ConnState::Connected,
    }
}

#[tauri::command]
pub async fn tray_action(app: AppHandle, action: TrayAction) -> Result<(), CommandError> {
    let result = match action {
        TrayAction::Exit => {
            app.state::<AppState>()
                .scan_cancel
                .store(true, Ordering::SeqCst);
            app.exit(0);
            return Ok(());
        }
        TrayAction::ToggleVpn => {
            let service = app.state::<VpnState>().0.clone();
            let result = tauri::async_runtime::spawn_blocking(move || {
                let snapshot = service.snapshot();
                if snapshot.connection.state == ConnState::Connected {
                    service.disconnect();
                } else if !snapshot.servers.is_empty() {
                    service.connect()?;
                } else {
                    return Ok::<_, CommandError>(false);
                }
                Ok(true)
            })
            .await
            .map_err(|err| CommandError {
                kind: "probe".into(),
                message: err.to_string(),
            })??;
            if result {
                app.emit_to(MAIN, "tray-vpn-changed", ())
            } else {
                hide_panel(&app);
                show_main(&app);
                app.emit_to(MAIN, "tray-action", TrayAction::Vpn)
            }
        }
        _ => {
            hide_panel(&app);
            show_main(&app);
            app.emit_to(MAIN, "tray-action", action)
        }
    };
    result.map_err(|err| CommandError {
        kind: "probe".into(),
        message: err.to_string(),
    })
}
