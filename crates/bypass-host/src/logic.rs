//! Что запускает служба. Путь к движку НЕ берётся из командной строки службы: служба работает
//! от имени системы, поэтому запускается только файл рядом с ней (папка программы, права на запись
//! только у администратора).

use std::path::{Path, PathBuf};

pub const SERVICE_NAME: &str = "OchistkaBypass";
pub const ENGINE_EXE: &str = "ochistkadpi.exe";
/// Аргументы из README SonicDPI для запуска движка (то же, что пишет его `install` в службу).
pub const ENGINE_ARGS: [&str; 3] = ["run", "--profile", "youtube-discord"];
/// Служба драйвера WinDivert (создаётся самим движком).
pub const DRIVER_SERVICE: &str = "WinDivert";

/// Путь к движку: рядом с файлом службы.
pub fn engine_path(host_exe: &Path) -> Option<PathBuf> {
    Some(host_exe.parent()?.join(ENGINE_EXE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_is_always_the_file_next_to_the_service() {
        let dir = std::env::temp_dir().join("ochistkadpi");
        let host = dir.join("ochistka-bypass-host.exe");
        assert_eq!(engine_path(&host).unwrap(), dir.join("ochistkadpi.exe"));
        assert_eq!(engine_path(Path::new("")), None);
    }

    #[test]
    fn the_arguments_are_the_readme_profile_and_fixed() {
        assert_eq!(ENGINE_ARGS, ["run", "--profile", "youtube-discord"]);
        assert_eq!(SERVICE_NAME, "OchistkaBypass");
    }
}
