//! «Показать в Проводнике»: `explorer.exe` по абсолютному пути в папке Windows, без PATH.
//! Окно Проводника не ждём (он сам возвращает ненулевой код даже при успехе), так что зависаний нет.

use ochistka_core::reveal::{explorer_args, normalize_path, RevealError};

/// Открывает файл (с выделением) или папку. Проверка существования — до запуска.
pub fn reveal(path: &str) -> Result<(), RevealError> {
    let path = normalize_path(path)?;
    let metadata = std::fs::metadata(&path).map_err(|_| RevealError::Missing)?;
    launch(&explorer_args(&path, metadata.is_dir()))
}

#[cfg(windows)]
fn launch(args: &str) -> Result<(), RevealError> {
    use std::os::windows::process::CommandExt;
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let explorer = std::path::PathBuf::from(root).join("explorer.exe");
    let mut child = std::process::Command::new(explorer)
        .raw_arg(args)
        .spawn()
        .map_err(|err| RevealError::Launch(err.to_string()))?;
    // Забираем код завершения в фоне, чтобы не копились зомби-процессы.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(not(windows))]
fn launch(_args: &str) -> Result<(), RevealError> {
    Err(RevealError::Unsupported {
        feature: "Проводник".into(),
        os: std::env::consts::OS.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_and_missing_paths_are_refused_before_launch() {
        assert_eq!(reveal(""), Err(RevealError::Empty));
        assert_eq!(reveal("notepad.exe"), Err(RevealError::NotAbsolute));
        assert_eq!(
            reveal(r"C:\definitely\not\here\x.exe"),
            Err(RevealError::Missing)
        );
    }
}
