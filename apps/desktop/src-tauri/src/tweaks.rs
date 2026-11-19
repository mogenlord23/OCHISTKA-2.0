//! Команды «Настройки» и «Boost»: чтение состояния и изменения реестра HKCU и схемы питания.
//! Включение опасной настройки (отключение VBS/HVCI) требует `confirmed: true` из окна подтверждения;
//! остальные настройки применяются сразу. В тестах системные изменения не выполняются.

use std::sync::OnceLock;

use ochistka_core::system::CoreError;
use ochistka_core::tweaks::{self, ActivationInfo, Group, TweaksOverview, TweaksPort};

use crate::commands::CommandError;

fn port() -> &'static dyn TweaksPort {
    static PORT: OnceLock<Box<dyn TweaksPort>> = OnceLock::new();
    PORT.get_or_init(ochistka_platform::current_tweaks_port)
        .as_ref()
}

fn refused(message: &str) -> CommandError {
    CommandError {
        kind: "invalidTarget".into(),
        message: message.into(),
    }
}

fn require_confirmation(confirmed: bool) -> Result<(), CommandError> {
    if confirmed {
        Ok(())
    } else {
        Err(refused("Изменение нужно подтвердить в окне программы"))
    }
}

/// Запуск в фоне: powercfg и реестр могут отвечать не мгновенно.
async fn run<T: Send + 'static>(
    job: impl FnOnce(&'static dyn TweaksPort) -> Result<T, CoreError> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(move || job(port()).map_err(CommandError::from))
        .await
        .map_err(|err| CommandError {
            kind: "probe".into(),
            message: err.to_string(),
        })?
}

#[tauri::command]
pub async fn tweaks_overview() -> Result<TweaksOverview, CommandError> {
    run(tweaks::overview).await
}

#[tauri::command]
pub async fn tweak_set(
    id: String,
    enabled: bool,
    confirmed: bool,
) -> Result<TweaksOverview, CommandError> {
    if enabled && tweaks::is_dangerous(&id) {
        require_confirmation(confirmed)?;
    }
    run(move |p| {
        tweaks::set_tweak(p, &id, enabled)?;
        tweaks::overview(p)
    })
    .await
}

#[tauri::command]
pub async fn tweaks_revert(group: String) -> Result<TweaksOverview, CommandError> {
    let group = Group::parse(&group).ok_or_else(|| refused("Неизвестный раздел настроек"))?;
    run(move |p| {
        tweaks::revert_group(p, group)?;
        tweaks::overview(p)
    })
    .await
}

#[tauri::command]
pub async fn boost_set_profile(profile: Option<String>) -> Result<TweaksOverview, CommandError> {
    run(move |p| {
        tweaks::set_boost_profile(p, profile.as_deref())?;
        tweaks::overview(p)
    })
    .await
}

/// Статус активации Windows (запрос лицензии идёт в фоне и может занять пару секунд).
#[tauri::command]
pub async fn windows_activation() -> Result<ActivationInfo, CommandError> {
    run(|p| p.activation()).await
}

/// Открывает «Параметры → Активация»; ничего не меняет.
#[tauri::command]
pub async fn open_activation_settings() -> Result<(), CommandError> {
    run(|p| p.open_activation_settings()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_dangerous_tweak_needs_confirmation() {
        assert!(tweaks::is_dangerous("vbs_off"));
        assert!(!tweaks::is_dangerous("game_dvr"));
        assert!(!tweaks::is_dangerous("transparency"));
    }

    #[test]
    fn changes_without_confirmation_are_refused() {
        assert_eq!(
            require_confirmation(false).unwrap_err().kind,
            "invalidTarget"
        );
        assert!(require_confirmation(true).is_ok());
    }
}
