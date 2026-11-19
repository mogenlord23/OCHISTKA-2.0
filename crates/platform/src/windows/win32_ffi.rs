//! Единственное место в `crates/platform`, где есть `unsafe` для вызовов Win32 (крейт `windows`):
//! обновление курсоров, рассылка о смене цветов, чтение и смена обоев, размер экрана.
//! Каждая функция самодостаточна и безопасна снаружи; в каждом `unsafe` есть `SAFETY`.

use ::windows::core::PCWSTR;
use ::windows::Win32::Foundation::{LPARAM, WPARAM};
use ::windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
use ::windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, SystemParametersInfoW, HWND_BROADCAST, SMTO_ABORTIFHUNG, SPIF_SENDCHANGE,
    SPIF_UPDATEINIFILE, SPI_GETDESKWALLPAPER, SPI_SETCURSORS, SPI_SETDESKWALLPAPER,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WM_SETTINGCHANGE,
};

/// Ошибка `windows` с кодом 0 («Операция успешно завершена») не ошибка: вызов отработал,
/// просто Windows не выставила код. Настоящий успех проверяется чтением реестра.
fn tolerate_code_zero(result: ::windows::core::Result<()>) -> Result<(), String> {
    match result {
        Ok(()) => Ok(()),
        Err(e) => crate::os_status::code_result(e.code().0, || e.to_string()),
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// `SystemParametersInfoW(SPI_SETCURSORS)`: система перечитывает схему курсоров из
/// `HKCU\Control Panel\Cursors`. Флаги 0: реестр уже записан нами, а SPIF_SENDCHANGE рассылает
/// сообщение всем окнам синхронно и ждёт каждое (одно зависшее окно давало многосекундную паузу).
pub fn reload_cursors() -> Result<(), String> {
    // SAFETY: SPI_SETCURSORS не использует pvParam, uiParam равен 0 по документации.
    tolerate_code_zero(unsafe {
        SystemParametersInfoW(
            SPI_SETCURSORS,
            0,
            None,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    })
}

/// Одно `WM_SETTINGCHANGE` с «ImmersiveColorSet» всем окнам; ждём не дольше 200 мс на окно,
/// зависшие окна пропускаются.
pub fn broadcast_color_change() -> Result<(), String> {
    let area = wide("ImmersiveColorSet");
    // SAFETY: `area` живёт до конца вызова; SendMessageTimeoutW синхронный и не хранит указатель.
    // Результат не важен: ноль — таймаут или зависшее окно, оформление уже записано в реестр.
    let _ = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(area.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            200,
            None,
        )
    };
    Ok(())
}

/// Полный путь к текущим обоям (`SPI_GETDESKWALLPAPER`); `None`, если обоев нет.
pub fn current_wallpaper() -> Option<String> {
    let mut buffer = vec![0u16; 1024];
    // SAFETY: буфер на 1024 u16; uiParam — его длина в символах; запись внутри буфера.
    unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Some(buffer.as_mut_ptr().cast()),
            Default::default(),
        )
    }
    .ok()?;
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let path = String::from_utf16_lossy(&buffer[..end]);
    (!path.is_empty()).then_some(path)
}

/// Ставит обои (`SPI_SETDESKWALLPAPER`, значение сохраняется в профиле). Пустая строка убирает
/// картинку. Положение («Заполнить») система берёт из реестра, он выставлен заранее.
pub fn set_wallpaper(path: &str) -> Result<(), String> {
    let mut text = wide(path);
    // SAFETY: `text` заканчивается нулём и живёт до конца вызова; система копирует строку.
    tolerate_code_zero(unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(text.as_mut_ptr().cast()),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    })
}

/// Физический размер основного экрана в пикселях (не зависит от масштаба интерфейса).
pub fn primary_screen_size() -> Option<(u32, u32)> {
    let mut mode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    // SAFETY: `mode` инициализирован с верным `dmSize`; пустое имя устройства — основной экран.
    let ok = unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut mode) };
    (ok.as_bool() && mode.dmPelsWidth > 0 && mode.dmPelsHeight > 0)
        .then_some((mode.dmPelsWidth, mode.dmPelsHeight))
}

// --- Прокси (VPN) ---
use std::ffi::c_void;
use std::ptr::null_mut;

#[link(name = "wininet")]
extern "system" {
    fn InternetSetOptionW(
        internet: *mut c_void,
        option: u32,
        buffer: *mut c_void,
        buffer_length: u32,
    ) -> i32;
}

const INTERNET_OPTION_REFRESH: u32 = 37;
const INTERNET_OPTION_SETTINGS_CHANGED: u32 = 39;

/// Сообщает WinINet (браузеры и программы с системным прокси), что настройки прокси
/// в реестре изменились. Без этого часть программ подхватывает их только после перезапуска.
pub fn notify_proxy_settings_changed() {
    // SAFETY: для этих двух глобальных параметров документация WinINet требует нулевой
    // дескриптор, нулевой буфер и нулевую длину; указатели не разыменовываются, память не
    // передаётся, результат только информационный.
    unsafe {
        InternetSetOptionW(null_mut(), INTERNET_OPTION_SETTINGS_CHANGED, null_mut(), 0);
        InternetSetOptionW(null_mut(), INTERNET_OPTION_REFRESH, null_mut(), 0);
    }
}
