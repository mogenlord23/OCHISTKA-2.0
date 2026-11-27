//! Сайты пользователя для «Обхода блокировок»: список, добавление, удаление. Проверка ввода и
//! хранение — в `ochistka_core::desync::user_sites`; здесь только тонкие команды. Сейчас список
//! только хранится: движка обхода в приложении нет (ADR 0019).

use ochistka_core::desync::user_sites::{SiteError, UserSites};
use tauri::State;

use crate::commands::CommandError;

/// Хранилище списка сайтов (папка данных обхода).
pub struct BypassSitesState(pub UserSites);

fn err(e: SiteError) -> CommandError {
    CommandError {
        kind: "invalidTarget".into(),
        message: e.to_string(),
    }
}

#[tauri::command]
pub fn bypass_sites_list(state: State<'_, BypassSitesState>) -> Vec<String> {
    state.0.load()
}

/// Одна строка — один адрес. Ошибка в любой строке отклоняет весь ввод.
#[tauri::command]
pub fn bypass_sites_add(
    state: State<'_, BypassSitesState>,
    text: String,
) -> Result<Vec<String>, CommandError> {
    state.0.add(&text).map_err(err)
}

#[tauri::command]
pub fn bypass_sites_remove(
    state: State<'_, BypassSitesState>,
    domain: String,
) -> Result<Vec<String>, CommandError> {
    state.0.remove(&domain).map_err(err)
}
