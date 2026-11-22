//! Обмен с диспетчером служб Windows. Единственное место в крейте с `unsafe`.

use std::ffi::c_void;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::NO_ERROR;
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatus,
    RegisterServiceCtrlHandlerExW, SetServiceStatus, StartServiceCtrlDispatcherW,
    SC_MANAGER_CONNECT, SERVICE_ACCEPT_SHUTDOWN, SERVICE_ACCEPT_STOP, SERVICE_CONTROL_INTERROGATE,
    SERVICE_CONTROL_SHUTDOWN, SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS, SERVICE_RUNNING,
    SERVICE_START_PENDING, SERVICE_STATUS, SERVICE_STATUS_HANDLE, SERVICE_STOP, SERVICE_STOPPED,
    SERVICE_STOP_PENDING, SERVICE_TABLE_ENTRYW, SERVICE_WIN32_OWN_PROCESS,
};

use crate::logic::{engine_path, DRIVER_SERVICE, ENGINE_ARGS, SERVICE_NAME};

const ERROR_CALL_NOT_IMPLEMENTED: u32 = 120;
const ERROR_SERVICE_SPECIFIC_ERROR: u32 = 1066;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

fn report(handle: SERVICE_STATUS_HANDLE, state: u32, accepted: u32, exit: u32, specific: u32) {
    let status = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: windows::Win32::System::Services::SERVICE_STATUS_CURRENT_STATE(state),
        dwControlsAccepted: accepted,
        dwWin32ExitCode: exit,
        dwServiceSpecificExitCode: specific,
        dwCheckPoint: 0,
        dwWaitHint: if state == SERVICE_START_PENDING.0 || state == SERVICE_STOP_PENDING.0 {
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

unsafe extern "system" fn service_main(_argc: u32, _argv: *mut PWSTR) {
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
    report(handle, SERVICE_START_PENDING.0, 0, 0, 0);

    let engine = std::env::current_exe()
        .ok()
        .and_then(|exe| engine_path(&exe));
    let Some(engine) = engine.filter(|p| p.is_file()) else {
        report(handle, SERVICE_STOPPED.0, 0, ERROR_FILE_NOT_FOUND, 0);
        return;
    };
    let driver_was_running = driver_running();
    let mut command = Command::new(&engine);
    command
        .args(ENGINE_ARGS)
        .current_dir(engine.parent().unwrap_or(std::path::Path::new(".")))
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
            report(
                handle,
                SERVICE_STOPPED.0,
                0,
                ERROR_SERVICE_SPECIFIC_ERROR,
                1,
            );
            return;
        }
    };
    let child = Arc::new(Mutex::new(child));
    report(
        handle,
        SERVICE_RUNNING.0,
        SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN,
        0,
        0,
    );

    // Ждём остановку службы или завершение движка (проверка раз в 250 мс только пока служба работает).
    let mut engine_exited = false;
    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Event::Stop) => break,
            Err(RecvTimeoutError::Timeout) => {
                let done = child
                    .lock()
                    .ok()
                    .and_then(|mut c| c.try_wait().ok())
                    .flatten()
                    .is_some();
                if done {
                    engine_exited = true;
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    report(handle, SERVICE_STOP_PENDING.0, 0, 0, 0);
    if let Ok(mut c) = child.lock() {
        let _ = c.kill();
        let _ = c.wait();
    }
    // Драйвер WinDivert останавливаем, только если он не работал до запуска службы.
    if !driver_was_running && driver_running() {
        stop_driver();
        for _ in 0..50 {
            if !driver_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    if engine_exited {
        report(
            handle,
            SERVICE_STOPPED.0,
            0,
            ERROR_SERVICE_SPECIFIC_ERROR,
            2,
        );
    } else {
        report(handle, SERVICE_STOPPED.0, 0, 0, 0);
    }
}

/// Запущена ли служба драйвера WinDivert.
fn driver_running() -> bool {
    with_driver(SERVICE_QUERY_STATUS, |service| {
        let mut status = SERVICE_STATUS::default();
        // SAFETY: `service` — открытый дескриптор службы, `status` — корректная структура.
        unsafe { QueryServiceStatus(service, &mut status) }.is_ok()
            && status.dwCurrentState != SERVICE_STOPPED
    })
    .unwrap_or(false)
}

fn stop_driver() {
    let _ = with_driver(SERVICE_STOP, |service| {
        let mut status = SERVICE_STATUS::default();
        // SAFETY: тот же открытый дескриптор; команда остановки без параметров.
        let _ = unsafe { ControlService(service, SERVICE_CONTROL_STOP, &mut status) };
    });
}

fn with_driver<T>(
    access: u32,
    job: impl FnOnce(windows::Win32::System::Services::SC_HANDLE) -> T,
) -> Option<T> {
    let name = wide(DRIVER_SERVICE);
    // SAFETY: открываем диспетчер и службу, оба дескриптора закрываются ниже ровно один раз.
    unsafe {
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT).ok()?;
        let service = OpenServiceW(scm, PCWSTR(name.as_ptr()), access);
        let result = service.as_ref().ok().map(|s| job(*s));
        if let Ok(s) = service {
            let _ = CloseServiceHandle(s);
        }
        let _ = CloseServiceHandle(scm);
        result
    }
}
