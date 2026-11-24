//! Граница с libclamav. Единственный модуль крейта с `unsafe`; компилируется только когда
//! build.rs нашёл официальную сборку и выставил `clamav_linked` (см. docs/status/clamav-build.md).
//!
//! Объявления сверены с `libclamav/clamav.h` ClamAV 1.4.6. Сборка с библиотекой и
//! проверка сигнатур остаются отдельными этапами для каждой целевой платформы.

use std::ffi::{c_char, c_int, c_uint, c_ulong, CStr, CString};
use std::ptr::NonNull;

use ochistka_core::scan::FileVerdict;

/// Непрозрачный движок (`struct cl_engine` в C).
#[repr(C)]
pub struct ClEngine {
    _private: [u8; 0],
}

/// Опции сканирования (`struct cl_scan_options` в ClamAV 1.4.6).
#[repr(C)]
pub struct ClScanOptions {
    pub general: u32,
    pub parse: u32,
    pub heuristic: u32,
    pub mail: u32,
    pub dev: u32,
}

impl Default for ClScanOptions {
    fn default() -> Self {
        // ClamAV 1.4 clamscan: встроенные парсеры от ARCHIVE до IMAGE.
        // Fuzzy-hash изображений и эвристические вердикты отдельно не включаем.
        Self {
            general: 0,
            parse: 0x0fff,
            heuristic: 0,
            mail: 0,
            dev: 0,
        }
    }
}

/// Коды результата из `clamav.h`.
const CL_SUCCESS: c_int = 0;
const CL_CLEAN: c_int = 0;
const CL_VIRUS: c_int = 1;
/// Рекомендуемые опции инициализации и загрузки базы из `clamav.h`.
const CL_INIT_DEFAULT: c_uint = 0;
const CL_DB_STDOPT: c_uint = 0x2 | 0x8 | 0x2000;

fn clamav_path(path: &str) -> Result<CString, String> {
    #[cfg(windows)]
    let path = if path.contains('/') {
        std::borrow::Cow::Owned(path.replace('/', "\\"))
    } else {
        std::borrow::Cow::Borrowed(path)
    };
    CString::new(path.as_bytes()).map_err(|_| "путь содержит нулевой байт".to_string())
}

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
    #[cfg(windows)]
    fn cl_scanfile(
        filename: *const c_char,
        virname: *mut *const c_char,
        scanned: *mut c_ulong,
        engine: *const ClEngine,
        scanoptions: *mut ClScanOptions,
    ) -> c_int;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn cl_scandesc(
        desc: c_int,
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

// SAFETY: после компиляции используется только потокобезопасный API сканирования libclamav.
// clamd так же разделяет скомпилированный движок. Владелец в clamav.rs защищает его время
// жизни RwLock. Каждый вызов передаёт собственные опции и копирует имя сигнатуры до возврата.
unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}

impl Engine {
    /// Загружает и компилирует сигнатуры из папки. Возвращает движок и число сигнатур.
    /// Ошибка содержит код библиотеки.
    pub fn load(dir: &str) -> Result<(Self, u32), String> {
        static INIT: std::sync::OnceLock<c_int> = std::sync::OnceLock::new();
        // SAFETY: cl_init не принимает указателей. OnceLock вызывает его до сканирования один раз.
        if *INIT.get_or_init(|| unsafe { cl_init(CL_INIT_DEFAULT) }) != CL_SUCCESS {
            return Err("cl_init завершился с ошибкой".into());
        }
        // SAFETY: `cl_engine_new` не принимает аргументов; результат проверяется на NULL ниже.
        let raw = NonNull::new(unsafe { cl_engine_new() })
            .ok_or_else(|| "cl_engine_new вернул NULL".to_string())?;
        let engine = Self { raw };

        let path = clamav_path(dir)?;
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
        if signatures == 0 {
            return Err("в базе нет загруженных сигнатур".into());
        }
        // SAFETY: движок загружен и ещё не скомпилирован, указатель валиден.
        let code = unsafe { cl_engine_compile(engine.raw.as_ptr()) };
        if code != CL_SUCCESS {
            return Err(format!("cl_engine_compile: код {code}"));
        }
        Ok((engine, signatures))
    }

    /// Проверяет один файл. Возвращает вердикт или текст ошибки библиотеки.
    #[cfg(windows)]
    pub fn scan(&self, path: &str) -> Result<FileVerdict, String> {
        let file = clamav_path(path)?;
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
        Self::verdict(code, virus)
    }

    /// Проверяет открытый файловый дескриптор без повторного открытия пути.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn scan_open_file(&self, file: &std::fs::File) -> Result<FileVerdict, String> {
        use std::os::fd::AsRawFd;
        let mut virus: *const c_char = std::ptr::null();
        let mut scanned: c_ulong = 0;
        let mut options = ClScanOptions::default();
        // SAFETY: дескриптор остаётся открытым на протяжении вызова, указатели живут
        // достаточно долго. Владелец удерживает движок до возврата.
        let code = unsafe {
            cl_scandesc(
                file.as_raw_fd(),
                std::ptr::null(),
                &mut virus,
                &mut scanned,
                self.raw.as_ptr(),
                &mut options,
            )
        };
        Self::verdict(code, virus)
    }

    fn verdict(code: c_int, virus: *const c_char) -> Result<FileVerdict, String> {
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
