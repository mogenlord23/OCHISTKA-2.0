//! Единственное место с `unsafe` для вызовов Win32, у которых нет безопасной обёртки.
//! Каждая функция самодостаточна и снабжена комментарием `SAFETY`.

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
