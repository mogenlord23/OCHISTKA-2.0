//! Граница с libclamav. Единственный модуль крейта с `unsafe`; компилируется только с фичей
//! `libclamav` (по умолчанию выключена, см. docs/status/clamav-build.md).
//!
//! НЕ ПРОВЕРЕНО по заголовкам ClamAV: исходники и заголовки в этой среде получить не удалось
//! (GitHub отдаёт 403). Сигнатуры функций, значения констант и состав `ClScanOptions`
//! записаны по памяти API ClamAV 1.x и требуют сверки с `clamav.h` при первой сборке.
//! Остальной код приложения к этим деталям не обращается: он видит только `Engine`.

use std::ffi::{c_char, c_int, c_uint, c_ulong, CStr, CString};
use std::ptr::NonNull;

use ochistka_core::scan::FileVerdict;

/// Непрозрачный движок (`struct cl_engine` в C).
#[repr(C)]
pub struct ClEngine {
    _private: [u8; 0],
}

/// Опции сканирования (`struct cl_scan_options`). НЕ ПРОВЕРЕНО: набор полей и порядок взяты по памяти.
#[repr(C)]
#[derive(Default)]
pub struct ClScanOptions {
    pub parse: u32,
    pub heuristic: u32,
    pub mail: u32,
    pub dev: u32,
}

/// `CL_SUCCESS` и `CL_CLEAN` равны 0, `CL_VIRUS` равен 1. НЕ ПРОВЕРЕНО по заголовкам.
const CL_SUCCESS: c_int = 0;
const CL_CLEAN: c_int = 0;
const CL_VIRUS: c_int = 1;
/// Значения опций init и загрузки БД. НЕ ПРОВЕРЕНО: заменить на константы из заголовка.
const CL_INIT_DEFAULT: c_uint = 0;
const CL_DB_STDOPT: c_uint = 0;

#[link(name = "clamav")]
extern "C" {
    fn cl_init(options: c_uint) -> c_int;
    fn cl_engine_new() -> *mut ClEngine;
    fn cl_load(
        path: *const c_char,
        engine: *mut ClEngine,
        signo: *mut c_uint,
        dboptions: c_uint,
    ) -> c_int;
    fn cl_engine_compile(engine: *mut ClEngine) -> c_int;
    fn cl_scanfile(
        filename: *const c_char,
        virname: *mut *const c_char,
        scanned: *mut c_ulong,
        engine: *const ClEngine,
        scanoptions: *mut ClScanOptions,
    ) -> c_int;
    fn cl_engine_free(engine: *mut ClEngine) -> c_int;
}

/// Скомпилированный движок с загруженными сигнатурами. Освобождается при удалении.
pub struct Engine {
    raw: NonNull<ClEngine>,
}

// SAFETY: после `cl_engine_compile` движок используется только на чтение в `cl_scanfile`;
// доступ из нескольких потоков сериализует `Mutex` в `clamav.rs`, поэтому `Send` достаточно.
unsafe impl Send for Engine {}

impl Engine {
    /// Загружает и компилирует сигнатуры из папки. Ошибка содержит код библиотеки.
    pub fn load(dir: &str) -> Result<Self, String> {
        // SAFETY: `cl_init` не принимает указателей; повторный вызов безопасен (возвращает успех).
        if unsafe { cl_init(CL_INIT_DEFAULT) } != CL_SUCCESS {
            return Err("cl_init завершился с ошибкой".into());
        }
        // SAFETY: `cl_engine_new` не принимает аргументов; результат проверяется на NULL ниже.
        let raw = NonNull::new(unsafe { cl_engine_new() })
            .ok_or_else(|| "cl_engine_new вернул NULL".to_string())?;
        let engine = Self { raw };

        let path = CString::new(dir).map_err(|_| "путь содержит нулевой байт".to_string())?;
        let mut signatures: c_uint = 0;
        // SAFETY: `path` живёт до конца вызова; `engine.raw` валиден, пока жив `engine`
        // (его освободит Drop, даже если загрузка ниже вернёт ошибку).
        let code = unsafe {
            cl_load(
                path.as_ptr(),
                engine.raw.as_ptr(),
                &mut signatures,
                CL_DB_STDOPT,
            )
        };
        if code != CL_SUCCESS {
            return Err(format!("cl_load: код {code}"));
        }
        // SAFETY: движок загружен и ещё не скомпилирован, указатель валиден.
        let code = unsafe { cl_engine_compile(engine.raw.as_ptr()) };
        if code != CL_SUCCESS {
            return Err(format!("cl_engine_compile: код {code}"));
        }
        Ok(engine)
    }

    /// Проверяет один файл. Возвращает вердикт или текст ошибки библиотеки.
    pub fn scan(&self, path: &str) -> Result<FileVerdict, String> {
        let file = CString::new(path).map_err(|_| "путь содержит нулевой байт".to_string())?;
        let mut virus: *const c_char = std::ptr::null();
        let mut scanned: c_ulong = 0;
        let mut options = ClScanOptions::default();
        // SAFETY: все указатели указывают на живые локальные данные; `engine` скомпилирован
        // и не освобождается во время вызова (владеет им `self`).
        let code = unsafe {
            cl_scanfile(
                file.as_ptr(),
                &mut virus,
                &mut scanned,
                self.raw.as_ptr(),
                &mut options,
            )
        };
        match code {
            CL_CLEAN => Ok(FileVerdict::Clean),
            CL_VIRUS => {
                let signature = if virus.is_null() {
                    "неизвестная сигнатура".to_string()
                } else {
                    // SAFETY: при CL_VIRUS библиотека отдаёт указатель на строку с именем
                    // сигнатуры, которая живёт, пока жив движок; копируем её сразу.
                    unsafe { CStr::from_ptr(virus) }
                        .to_string_lossy()
                        .into_owned()
                };
                Ok(FileVerdict::Found { signature })
            }
            other => Err(format!("cl_scanfile: код {other}")),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // SAFETY: `raw` получен от `cl_engine_new` и освобождается ровно один раз.
        unsafe {
            cl_engine_free(self.raw.as_ptr());
        }
    }
}
