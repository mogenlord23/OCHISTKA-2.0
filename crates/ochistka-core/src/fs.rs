//! Порт файловой системы для очистки и карантина, ошибки операций и обход дерева.
//! Чистый код: реальный диск подставляет адаптер платформы, тесты — дерево в памяти.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

/// Ошибки очистки и карантина. Текст — на русском, он уходит в интерфейс.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CleanupError {
    #[error("путь запрещён политикой безопасности: {0}")]
    Refused(String),
    #[error("не найдено: {0}")]
    NotFound(String),
    #[error("место занято: {0}")]
    Occupied(String),
    #[error("операция остановлена пользователем")]
    Cancelled,
    #[error("некорректный идентификатор записи карантина")]
    InvalidId,
    #[error("подтверждение не совпадает, окончательное удаление не выполнено")]
    ConfirmationMismatch,
    #[error("хэш файла не совпадает с манифестом: {0}")]
    HashMismatch(String),
    #[error("запись карантина не найдена")]
    EntryNotFound,
    #[error("манифест карантина повреждён")]
    ManifestCorrupt,
    #[error("сначала выполните анализ")]
    NoPlan,
    #[error("ошибка файловой операции: {0}")]
    Io(String),
    #[error("недоступно на этой ОС: {0}")]
    Unsupported(String),
}

/// Метаданные без перехода по ссылкам. `is_symlink` включает точки повторной обработки (junction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Meta {
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub meta: Meta,
}

/// Файловые операции, которые нужны очистке. Реализация на Windows — в `crates/platform`.
pub trait FsPort: Send + Sync {
    /// Метаданные пути; ссылки не разыменовываются.
    fn metadata(&self, path: &str) -> Result<Meta, CleanupError>;
    fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>, CleanupError>;
    /// Склеивает папку и имя по правилам ОС.
    fn child(&self, dir: &str, name: &str) -> String;
    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, CleanupError>;
    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), CleanupError>;
    fn create_dir_all(&self, path: &str) -> Result<(), CleanupError>;
    /// Перенос файла. Существующее назначение перезаписывается: вызывающий проверяет его заранее.
    fn rename(&self, from: &str, to: &str) -> Result<(), CleanupError>;
    fn remove_file(&self, path: &str) -> Result<(), CleanupError>;
    /// Удаляет только пустую папку.
    fn remove_dir(&self, path: &str) -> Result<(), CleanupError>;
}

/// Лимиты обхода: сколько записей и насколько глубоко.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_entries: u64,
    pub max_depth: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 300_000,
            max_depth: 24,
        }
    }
}

/// Общий счётчик записей для обхода. Когда он исчерпан, обход возвращает неполный результат.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub remaining: u64,
}

impl Budget {
    pub fn new(entries: u64) -> Self {
        Self { remaining: entries }
    }
}

/// Обходит дерево в глубину. Для каждой записи (кроме ссылок) вызывает `visit`.
/// Возвращает `true`, если обход полный. Недоступные вложенные папки и лимиты делают его неполным.
/// Ошибка чтения начальной папки возвращается как `Err`. Остановка по `cancel` даёт `Cancelled`.
pub fn walk(
    fs: &dyn FsPort,
    start: &str,
    cancel: &AtomicBool,
    budget: &mut Budget,
    max_depth: u32,
    visit: &mut dyn FnMut(&DirEntry),
) -> Result<bool, CleanupError> {
    let mut complete = true;
    let mut stack = vec![(start.to_string(), 0u32)];
    let mut first = true;
    while let Some((dir, depth)) = stack.pop() {
        if cancel.load(Ordering::SeqCst) {
            return Err(CleanupError::Cancelled);
        }
        let entries = match fs.read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if first => return Err(err),
            Err(_) => {
                complete = false;
                continue;
            }
        };
        first = false;
        for entry in entries {
            if cancel.load(Ordering::SeqCst) {
                return Err(CleanupError::Cancelled);
            }
            if budget.remaining == 0 {
                return Ok(false);
            }
            budget.remaining -= 1;
            if entry.meta.is_symlink {
                continue;
            }
            visit(&entry);
            if entry.meta.is_dir {
                if depth >= max_depth {
                    complete = false;
                } else {
                    stack.push((entry.path.clone(), depth + 1));
                }
            }
        }
    }
    Ok(complete)
}

/// Читает весь файл в память. Только для маленьких файлов: манифест карантина.
pub fn read_all(fs: &dyn FsPort, path: &str) -> Result<Vec<u8>, CleanupError> {
    let mut reader = fs.open(path)?;
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| CleanupError::Io(err.to_string()))?;
    Ok(bytes)
}

/// SHA-256 в виде шестнадцатеричной строки.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// SHA-256 файла, читается потоком (большие файлы не грузятся в память целиком).
pub fn sha256_file(fs: &dyn FsPort, path: &str) -> Result<String, CleanupError> {
    let mut reader = fs.open(path)?;
    sha256_reader(&mut reader)
}

/// Хэширует уже открытый объект, не открывая его повторно по пути.
pub fn sha256_reader(reader: &mut dyn Read) -> Result<String, CleanupError> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|err| CleanupError::Io(err.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
