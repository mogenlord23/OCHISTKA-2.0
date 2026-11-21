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

/// Задание Windows «завершить все процессы при закрытии дескриптора задания». Создаётся один раз и
/// не закрывается до конца работы программы: при принудительном закрытии или падении программы
/// система закрывает дескриптор сама и завершает движок обхода.
pub fn assign_to_kill_on_close_job(process: *mut core::ffi::c_void) -> Result<(), String> {
    use ::windows::Win32::Foundation::HANDLE;
    use ::windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use std::sync::OnceLock;

    static JOB: OnceLock<Result<usize, String>> = OnceLock::new();
    let job = JOB.get_or_init(|| {
        // SAFETY: создаётся безымянное задание без атрибутов безопасности; результат проверяется.
        let handle =
            unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|e| e.to_string())?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: `info` — корректная структура нужного типа на стеке, размер передан верно.
        unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(|e| e.to_string())?;
        Ok(handle.0 as usize)
    });
    let job = job.clone()?;
    // SAFETY: дескрипторы действительны: задание живёт до конца программы, процесс передан
    // вызывающим кодом, пока объект `Child` жив.
    unsafe { AssignProcessToJobObject(HANDLE(job as *mut core::ffi::c_void), HANDLE(process)) }
        .map_err(|e| e.to_string())
}

/// Ошибка запроса повышения прав: код Windows и текст. Код 1223 — пользователь отказал в запросе.
pub struct ShellError {
    pub code: u32,
    pub message: String,
}

/// Код Windows «операция отменена пользователем» (отказ в запросе UAC).
pub const ERROR_CANCELLED: u32 = 1223;

/// Запускает программу с повышением прав (`ShellExecuteExW`, глагол `runas`): Windows показывает
/// запрос UAC только для этой программы. Окно скрыто. Возвращает номер процесса (0, если система
/// его не вернула). `wait_ms` — подождать завершения процесса не дольше этого времени.
pub fn shell_execute_runas(
    file: &str,
    parameters: &str,
    directory: &str,
    wait_ms: Option<u32>,
) -> Result<u32, ShellError> {
    use ::windows::Win32::Foundation::CloseHandle;
    use ::windows::Win32::System::Threading::{GetProcessId, WaitForSingleObject};
    use ::windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use ::windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let verb = wide("runas");
    let file = wide(file);
    let parameters = wide(parameters);
    let directory = wide(directory);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        lpDirectory: PCWSTR(directory.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: `info` — корректная структура с верным `cbSize`; все строки заканчиваются нулём и
    // живут до конца вызова; вызов синхронный и не хранит указатели после возврата.
    if let Err(e) = unsafe { ShellExecuteExW(&mut info) } {
        let hr = e.code().0 as u32;
        // 0x800704C7 = HRESULT_FROM_WIN32(ERROR_CANCELLED).
        let code = if hr == 0x8007_04C7 {
            ERROR_CANCELLED
        } else {
            hr
        };
        return Err(ShellError {
            code,
            message: e.to_string(),
        });
    }
    let process = info.hProcess;
    if process.is_invalid() {
        return Ok(0);
    }
    // SAFETY: дескриптор получен от системы (SEE_MASK_NOCLOSEPROCESS) и закрывается ровно один раз.
    let pid = unsafe { GetProcessId(process) };
    if let Some(ms) = wait_ms {
        // SAFETY: тот же действительный дескриптор; ожидание ограничено по времени.
        unsafe { WaitForSingleObject(process, ms) };
    }
    // SAFETY: закрываем дескриптор, полученный выше, и больше не используем.
    let _ = unsafe { CloseHandle(process) };
    Ok(pid)
}
