//! Сайты пользователя для «Обхода блокировок»: список, добавление, удаление. Проверка ввода и
//! хранение — в `ochistka_core::desync::user_sites`; здесь только тонкие команды.

use ochistka_core::desync::user_sites::SiteError;
use tauri::State;

use crate::commands::CommandError;
use crate::vpn::DesyncState;

fn err(e: SiteError) -> CommandError {
    CommandError {
        kind: "invalidTarget".into(),
        message: e.to_string(),
    }
}

#[tauri::command]
pub fn bypass_sites_list(state: State<'_, DesyncState>) -> Vec<String> {
    state.0.user_sites().load()
}

/// Одна строка — один адрес. Ошибка в любой строке отклоняет весь ввод.
#[tauri::command]
pub fn bypass_sites_add(
    state: State<'_, DesyncState>,
    text: String,
) -> Result<Vec<String>, CommandError> {
    state.0.user_sites().add(&text).map_err(err)
}

#[tauri::command]
pub fn bypass_sites_remove(
    state: State<'_, DesyncState>,
    domain: String,
) -> Result<Vec<String>, CommandError> {
    state.0.user_sites().remove(&domain).map_err(err)
}
