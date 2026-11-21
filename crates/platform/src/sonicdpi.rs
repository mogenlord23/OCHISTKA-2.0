//! Движок обхода — переименованный SonicDPI v0.2.2 (`ochistkadpi.exe`, код движка не изменён). Модуль НЕ подключён к кнопке
//! «Обход блокировок» и не вызывается приложением: это адаптер запуска и остановки с тестами.
//! Команды взяты из README SonicDPI (см. `docs/status/sonicdpi-integration.md`).
//!
//! Правила: запуск по абсолютному пути из папки ресурсов (без поиска в PATH), без оболочки;
//! остановка только по PID с проверкой имени и пути файла (чужой процесс не трогается).

use std::path::{Path, PathBuf};

use crate::vpn::spawn_logged;
use crate::windows::process_guard::{self, GuardedProcess};

/// Файл движка внутри папки ресурсов.
pub const SONICDPI_EXE: &str = "ochistkadpi.exe";
/// Релиз, для которого проверена контрольная сумма архива (этап 3).
pub const RELEASE_TAG: &str = "v0.2.2";
pub const RELEASE_ZIP: &str = "SonicDPI-windows-x64.zip";
pub const RELEASE_ZIP_SHA256: &str =
    "ec1e9f32315bdab1989d8e78bb3d3ad6975700fe83546139eb6f21e132991fc0";
/// Профиль по умолчанию (README: «По умолчанию»).
pub const DEFAULT_PROFILE: &str = "youtube-discord";
/// Встроенные профили из таблицы README.
pub const BUILTIN_PROFILES: [&str; 5] = [
    "youtube-discord",
    "youtube-discord-aggressive",
    "youtube-discord-multidisorder",
    "youtube-discord-seqovl",
    "youtube-discord-hostfakesplit",
];

pub struct SonicDpiEngine {
    dir: PathBuf,
}

impl SonicDpiEngine {
    /// `dir` — абсолютная папка, где лежат `ochistkadpi.exe` и `WinDivert.dll`/`WinDivert64.sys`.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn exe_path(&self) -> PathBuf {
        self.dir.join(SONICDPI_EXE)
    }

    /// Аргументы запуска: `run --profile <профиль>` (README: `.\sonicdpi.exe run --profile
    /// youtube-discord -vv`; `-vv` — отладка, не добавляем). Профиль — встроенное имя или
    /// абсолютный `.toml` внутри папки движка. Любое другое значение отклоняется.
    pub fn launch_arguments(&self, profile: &str) -> Result<Vec<String>, String> {
        Ok(vec![
            "run".into(),
            "--profile".into(),
            self.profile_argument(profile)?,
        ])
    }

    fn profile_argument(&self, profile: &str) -> Result<String, String> {
        if BUILTIN_PROFILES.contains(&profile) {
            return Ok(profile.to_string());
        }
        let path = Path::new(profile);
        let is_toml = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("toml"));
        let inside = path.is_absolute()
            && path.starts_with(&self.dir)
            && !path.components().any(|c| c.as_os_str() == "..");
        if is_toml && inside {
            return Ok(profile.to_string());
        }
        Err("неизвестный профиль SonicDPI".into())
    }

    /// Запускает движок без окна, вывод пишет в `log`. Процесс привязывается к заданию «завершить
    /// при закрытии» (ошибка привязки не мешает запуску; остаётся остановка по PID).
    pub fn spawn(&self, profile: &str, log: &Path) -> Result<GuardedProcess, String> {
        let args = self.launch_arguments(profile)?;
        let exe = self.exe_path();
        if !exe.is_absolute() {
            return Err("путь к движку должен быть абсолютным".into());
        }
        match std::fs::symlink_metadata(&exe) {
            Ok(meta) if meta.is_file() => {}
            _ => return Err("файл ochistkadpi.exe отсутствует или не обычный файл".into()),
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let process = spawn_logged(&exe, &refs, &self.dir, log)?;
        let _ = process_guard::attach_kill_on_close(&process);
        Ok(GuardedProcess::new(process, SONICDPI_EXE))
    }

    /// Останавливает процесс по PID, только если имя `ochistkadpi.exe` и файл из папки движка.
    /// `true` — процесса с таким PID и именем больше нет (чужой процесс не трогается).
    pub fn stop_by_pid(&self, pid: u32) -> bool {
        process_guard::terminate_named(pid, SONICDPI_EXE, Some(&self.exe_path()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::vpn::service::CoreProcess;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-sonic-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_profile_makes_run_profile_arguments() {
        let engine = SonicDpiEngine::new(temp("args"));
        assert_eq!(
            engine.launch_arguments(DEFAULT_PROFILE).unwrap(),
            ["run", "--profile", "youtube-discord"]
        );
        for name in BUILTIN_PROFILES {
            assert_eq!(engine.launch_arguments(name).unwrap()[2], name);
        }
    }

    #[test]
    fn custom_toml_is_allowed_only_inside_the_engine_folder() {
        let dir = temp("toml");
        let engine = SonicDpiEngine::new(dir.clone());
        let inside = dir.join("my.toml");
        let inside = inside.to_string_lossy().to_string();
        assert_eq!(engine.launch_arguments(&inside).unwrap()[2], inside);
        for bad in [
            "my.toml",
            "/etc/other.toml",
            "youtube-discord; calc",
            "--profile",
            "unknown",
            "",
        ] {
            assert!(engine.launch_arguments(bad).is_err(), "{bad}");
        }
        let escape = dir.join("..").join("x.toml");
        assert!(engine.launch_arguments(&escape.to_string_lossy()).is_err());
    }

    #[test]
    fn spawn_refuses_missing_or_relative_executable() {
        let dir = temp("nofile");
        let log = dir.join("a.log");
        assert!(SonicDpiEngine::new(dir.clone())
            .spawn(DEFAULT_PROFILE, &log)
            .is_err());
        assert!(SonicDpiEngine::new(PathBuf::from("relative-dir"))
            .spawn(DEFAULT_PROFILE, &log)
            .is_err());
        assert!(SonicDpiEngine::new(dir).spawn("bad profile", &log).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn spawn_passes_the_arguments_and_stop_checks_the_name() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp("fake");
        // Фейковый «ochistkadpi.exe»: записывает свои аргументы и ждёт.
        let script = "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/args.txt\"\nsleep 30 &\nwait\n";
        let exe = dir.join(SONICDPI_EXE);
        std::fs::write(&exe, script).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        let engine = SonicDpiEngine::new(dir.clone());
        let mut process = engine.spawn(DEFAULT_PROFILE, &dir.join("e.log")).unwrap();
        let pid = process.pid();
        let args_file = dir.join("args.txt");
        for _ in 0..50 {
            if args_file.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert_eq!(
            std::fs::read_to_string(&args_file).unwrap().trim(),
            "run --profile youtube-discord"
        );
        process.kill();
        assert!(process.has_exited());
        assert!(!process_guard::is_alive_named(pid, SONICDPI_EXE));
    }

    #[test]
    fn stop_by_pid_leaves_a_foreign_process_alone() {
        let sleeper = PathBuf::from("/bin/sleep");
        if !sleeper.exists() {
            return;
        }
        let dir = temp("foreign");
        let mut foreign = spawn_logged(&sleeper, &["30"], &dir, &dir.join("f.log")).unwrap();
        // Имя «sleep» не ochistkadpi.exe: не наш, не трогаем.
        assert!(SonicDpiEngine::new(dir.clone()).stop_by_pid(foreign.pid()));
        assert!(!foreign.has_exited(), "чужой процесс жив");
        // Одноимённый ochistkadpi.exe из ДРУГОЙ папки тоже не наш.
        let other = temp("other");
        std::fs::copy(&sleeper, other.join(SONICDPI_EXE)).unwrap();
        let mut same_name = spawn_logged(
            &other.join(SONICDPI_EXE),
            &["30"],
            &other,
            &other.join("s.log"),
        )
        .unwrap();
        assert!(SonicDpiEngine::new(dir).stop_by_pid(same_name.pid()));
        assert!(!same_name.has_exited(), "из другой папки: не наш");
        // Из «нашей» папки — останавливается (завершённый процесс принимаем, как это делает
        // родитель в приложении; иначе на Linux он остаётся «зомби»).
        let pid = same_name.pid();
        let stopper = std::thread::spawn(move || SonicDpiEngine::new(other).stop_by_pid(pid));
        for _ in 0..50 {
            if same_name.has_exited() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(same_name.has_exited());
        assert!(stopper.join().unwrap());
        foreign.kill();
    }
}
