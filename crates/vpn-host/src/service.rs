//! Обмен с диспетчером служб Windows. Единственное место в крейте с `unsafe`.

use std::ffi::c_void;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::NO_ERROR;
use windows::Win32::System::Services::{
    RegisterServiceCtrlHandlerExW, SetServiceStatus, StartServiceCtrlDispatcherW,
    SERVICE_ACCEPT_SHUTDOWN, SERVICE_ACCEPT_STOP, SERVICE_CONTROL_INTERROGATE,
    SERVICE_CONTROL_SHUTDOWN, SERVICE_CONTROL_STOP, SERVICE_RUNNING, SERVICE_START_PENDING,
    SERVICE_STATUS, SERVICE_STATUS_CURRENT_STATE, SERVICE_STATUS_HANDLE, SERVICE_STOPPED,
    SERVICE_STOP_PENDING, SERVICE_TABLE_ENTRYW, SERVICE_WIN32_OWN_PROCESS,
};

use crate::logic::{config_path, core_path, validate_config, SERVICE_NAME};

const ERROR_CALL_NOT_IMPLEMENTED: u32 = 120;
const ERROR_SERVICE_SPECIFIC_ERROR: u32 = 1066;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_INVALID_PARAMETER: u32 = 87;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Через столько после запуска ядра файл конфигурации (в нём ключи сервера) стирается.
const CONFIG_LIFETIME: Duration = Duration::from_secs(3);

enum Event {
    Stop,
}

static EVENTS: OnceLock<Mutex<Option<Sender<Event>>>> = OnceLock::new();

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn run() {
    let name = wide(SERVICE_NAME);
    let table = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: PWSTR(name.as_ptr() as *mut u16),
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYW::default(),
    ];
    // SAFETY: таблица заканчивается нулевой записью; `name` живёт до конца вызова, который
    // блокируется до остановки службы.
    if let Err(e) = unsafe { StartServiceCtrlDispatcherW(table.as_ptr()) } {
        eprintln!("не удалось подключиться к диспетчеру служб: {e}");
    }
}

fn report(
    handle: SERVICE_STATUS_HANDLE,
    state: SERVICE_STATUS_CURRENT_STATE,
    accepted: u32,
    exit: u32,
    specific: u32,
) {
    let status = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: state,
        dwControlsAccepted: accepted,
        dwWin32ExitCode: exit,
        dwServiceSpecificExitCode: specific,
        dwCheckPoint: 0,
        dwWaitHint: if state == SERVICE_START_PENDING || state == SERVICE_STOP_PENDING {
            15_000
        } else {
            0
        },
    };
    // SAFETY: `handle` получен от системы для этой службы, `status` — корректная структура.
    let _ = unsafe { SetServiceStatus(handle, &status) };
}

unsafe extern "system" fn handler(
    control: u32,
    _event_type: u32,
    _event_data: *mut c_void,
    _context: *mut c_void,
) -> u32 {
    match control {
        SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
            if let Some(slot) = EVENTS.get() {
                if let Ok(guard) = slot.lock() {
                    if let Some(tx) = guard.as_ref() {
                        let _ = tx.send(Event::Stop);
                    }
                }
            }
            NO_ERROR.0
        }
        SERVICE_CONTROL_INTERROGATE => NO_ERROR.0,
        _ => ERROR_CALL_NOT_IMPLEMENTED,
    }
}

/// Аргумент запуска №1 (после имени службы): конфигурация ядра.
///
/// # Safety
/// `argv` — массив из `argc` указателей на строки с нулём в конце, как его передаёт диспетчер служб.
unsafe fn config_argument(argc: u32, argv: *mut PWSTR) -> Option<String> {
    if argc < 2 || argv.is_null() {
        return None;
    }
    // SAFETY: по контракту функции элемент 1 существует и заканчивается нулём.
    let arg = unsafe { *argv.add(1) };
    unsafe { arg.to_string().ok() }
}

unsafe extern "system" fn service_main(argc: u32, argv: *mut PWSTR) {
    let (tx, rx) = channel();
    let slot = EVENTS.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(tx);
    }
    let name = wide(SERVICE_NAME);
    // SAFETY: имя заканчивается нулём и живёт до конца вызова; обработчик — функция с нужной
    // сигнатурой, контекст не используется.
    let handle = match unsafe {
        RegisterServiceCtrlHandlerExW(PCWSTR(name.as_ptr()), Some(handler), None)
    } {
        Ok(h) => h,
        Err(_) => return,
    };
    report(handle, SERVICE_START_PENDING, 0, 0, 0);

    // SAFETY: значения приходят из диспетчера служб прямо в эту функцию.
    let config = unsafe { config_argument(argc, argv) };
    let Some(config) = config.filter(|c| validate_config(c).is_ok()) else {
        report(handle, SERVICE_STOPPED, 0, ERROR_INVALID_PARAMETER, 0);
        return;
    };
    let exe = std::env::current_exe().ok();
    let core = exe.as_deref().and_then(core_path).filter(|p| p.is_file());
    let config_file = exe.as_deref().and_then(config_path);
    let (Some(core), Some(config_file)) = (core, config_file) else {
        report(handle, SERVICE_STOPPED, 0, ERROR_FILE_NOT_FOUND, 0);
        return;
    };
    if let Some(dir) = config_file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::write(&config_file, config).is_err() {
        report(handle, SERVICE_STOPPED, 0, ERROR_SERVICE_SPECIFIC_ERROR, 1);
        return;
    }
    let mut command = Command::new(&core);
    command
        .args(["run", "-c"])
        .arg(&config_file)
        .current_dir(core.parent().unwrap_or(std::path::Path::new(".")))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let child: Child = match command.spawn() {
        Ok(c) => c,
        Err(_) => {
            let _ = std::fs::remove_file(&config_file);
            report(handle, SERVICE_STOPPED, 0, ERROR_SERVICE_SPECIFIC_ERROR, 2);
            return;
        }
    };
    let child = Arc::new(Mutex::new(child));
    let started = Instant::now();
    report(
        handle,
        SERVICE_RUNNING,
        SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN,
        0,
        0,
    );

    // Ждём остановку службы или завершение ядра (проверка раз в 250 мс только пока служба работает).
    let mut config_removed = false;
    let mut core_exited = false;
    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Event::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                if !config_removed && started.elapsed() >= CONFIG_LIFETIME {
                    let _ = std::fs::remove_file(&config_file);
                    config_removed = true;
                }
                let done = child
                    .lock()
                    .ok()
                    .and_then(|mut c| c.try_wait().ok())
                    .flatten()
                    .is_some();
                if done {
                    core_exited = true;
                    break;
                }
            }
        }
    }
    report(handle, SERVICE_STOP_PENDING, 0, 0, 0);
    if let Ok(mut c) = child.lock() {
        let _ = c.kill();
        let _ = c.wait();
    }
    let _ = std::fs::remove_file(&config_file);
    if core_exited {
        report(handle, SERVICE_STOPPED, 0, ERROR_SERVICE_SPECIFIC_ERROR, 3);
    } else {
        report(handle, SERVICE_STOPPED, 0, 0, 0);
    }
}
