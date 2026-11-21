//! Движок обхода — переименованный SonicDPI v0.2.2 (`ochistkadpi.exe`, код движка не изменён):
//! адаптер запуска и остановки. Движку нужны права администратора: если приложение их не имеет,
//! повышение запрашивается ТОЛЬКО для запуска движка (UAC), всё приложение от администратора не
//! запускается.
//! Команды взяты из README SonicDPI (см. `docs/status/sonicdpi-integration.md`).
//!
//! Правила: запуск по абсолютному пути из папки ресурсов (без поиска в PATH), без оболочки;
//! остановка только по PID с проверкой имени и пути файла (чужой процесс не трогается).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use ochistka_core::desync::ELEVATION_DENIED;
use ochistka_core::vpn::service::CoreProcess;

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

/// Результат запроса повышения прав.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElevationError {
    /// Пользователь отказал в запросе.
    Denied,
    Failed(String),
}

/// Всё, что зависит от Windows при запуске с повышением прав (в тестах подменяется).
pub trait ElevationHost: Send + Sync {
    /// Приложение уже работает с правами администратора.
    fn is_elevated(&self) -> bool;
    /// Запускает `exe` с повышением прав (запрос UAC только для него); возвращает номер процесса
    /// (0, если система его не сообщила).
    fn run_elevated(&self, exe: &Path, args: &[String], dir: &Path) -> Result<u32, ElevationError>;
    /// Завершает процесс с повышенными правами (отдельный запрос UAC). `true` — команда выполнена.
    fn kill_elevated(&self, pid: u32) -> bool;
    /// Журнал, который пишет сам движок (`%LOCALAPPDATA%\SonicDPI\sonicdpi.log`).
    fn engine_log(&self) -> Option<PathBuf>;
}

/// Настоящая реализация: Windows (`ShellExecuteExW` с глаголом `runas`).
pub struct SystemElevation;

impl ElevationHost for SystemElevation {
    fn is_elevated(&self) -> bool {
        crate::desync::elevated()
    }

    fn run_elevated(&self, exe: &Path, args: &[String], dir: &Path) -> Result<u32, ElevationError> {
        #[cfg(windows)]
        {
            if args.iter().any(|a| a.contains(['"', ' ', '\t'])) {
                return Err(ElevationError::Failed("недопустимые аргументы".into()));
            }
            crate::windows::win32_ffi::shell_execute_runas(
                &exe.to_string_lossy(),
                &args.join(" "),
                &dir.to_string_lossy(),
                None,
            )
            .map_err(|e| {
                if e.code == crate::windows::win32_ffi::ERROR_CANCELLED {
                    ElevationError::Denied
                } else {
                    ElevationError::Failed(e.message)
                }
            })
        }
        #[cfg(not(windows))]
        {
            let _ = (exe, args, dir);
            Err(ElevationError::Failed("только для Windows".into()))
        }
    }

    fn kill_elevated(&self, pid: u32) -> bool {
        #[cfg(windows)]
        {
            // Абсолютный путь к системной программе, без поиска в PATH.
            crate::windows::win32_ffi::shell_execute_runas(
                r"C:\Windows\System32\taskkill.exe",
                &format!("/PID {pid} /F"),
                r"C:\Windows\System32",
                Some(10_000),
            )
            .is_ok()
        }
        #[cfg(not(windows))]
        {
            let _ = pid;
            false
        }
    }

    fn engine_log(&self) -> Option<PathBuf> {
        engine_log_path()
    }
}

/// `%LOCALAPPDATA%\SonicDPI\sonicdpi.log`: имя пишет сам движок, оно не меняется.
pub fn engine_log_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|dir| PathBuf::from(dir).join("SonicDPI").join("sonicdpi.log"))
}

/// Движок появился после запроса прав: процесс отслеживается по номеру, а если права
/// повышенные, то и завершается с отдельным запросом, когда обычное завершение не сработало.
pub struct ElevatedProcess {
    pid: u32,
    host: Arc<dyn ElevationHost>,
}

impl ElevatedProcess {
    pub fn new(pid: u32, host: Arc<dyn ElevationHost>) -> Self {
        Self { pid, host }
    }
}

impl CoreProcess for ElevatedProcess {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn has_exited(&mut self) -> bool {
        !process_guard::is_alive_named(self.pid, SONICDPI_EXE)
    }

    /// Сначала обычное завершение (по номеру, с проверкой имени); если процесс с повышенными
    /// правами не поддался, — завершение с запросом прав. Чужой процесс с тем же номером не
    /// трогается: проверяется имя.
    fn kill(&mut self) {
        if process_guard::terminate_named(self.pid, SONICDPI_EXE, None) {
            return;
        }
        if self.host.kill_elevated(self.pid) {
            for _ in 0..30 {
                if self.has_exited() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

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

    /// Запуск для кнопки: если приложение уже администратор — обычный запуск с журналом, иначе
    /// запрос повышения прав только для движка. Отказ даёт `ELEVATION_DENIED`. Если номер
    /// процесса после повышения неизвестен, процесс ищется по имени и пути файла.
    pub fn spawn_auto(
        &self,
        profile: &str,
        log: &Path,
        host: &Arc<dyn ElevationHost>,
    ) -> Result<Box<dyn CoreProcess>, String> {
        if host.is_elevated() {
            return Ok(Box::new(self.spawn(profile, log)?));
        }
        let args = self.launch_arguments(profile)?;
        let exe = self.exe_path();
        if !exe.is_absolute() {
            return Err("путь к движку должен быть абсолютным".into());
        }
        match std::fs::symlink_metadata(&exe) {
            Ok(meta) if meta.is_file() => {}
            _ => return Err("файл ochistkadpi.exe отсутствует или не обычный файл".into()),
        }
        let mut pid = match host.run_elevated(&exe, &args, &self.dir) {
            Ok(pid) => pid,
            Err(ElevationError::Denied) => return Err(ELEVATION_DENIED.into()),
            Err(ElevationError::Failed(why)) => return Err(why),
        };
        if pid == 0 {
            for _ in 0..30 {
                if let Some(found) = process_guard::find_by_name_and_path(SONICDPI_EXE, &exe) {
                    pid = found;
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        if pid == 0 {
            return Err("процесс движка не найден после запуска".into());
        }
        Ok(Box::new(ElevatedProcess::new(pid, host.clone())))
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

    // ----- запуск с повышением прав (фейковый хозяин, настоящий процесс sleep) -----

    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    struct FakeHost {
        elevated: bool,
        deny: bool,
        /// Сообщать номер процесса (иначе 0: ищем по имени и пути).
        report_pid: bool,
        runs: AtomicU32,
        kills: AtomicU32,
        args: Mutex<Vec<String>>,
        log: Option<PathBuf>,
    }

    impl FakeHost {
        fn new(elevated: bool) -> Self {
            Self {
                elevated,
                deny: false,
                report_pid: true,
                runs: AtomicU32::new(0),
                kills: AtomicU32::new(0),
                args: Mutex::new(vec![]),
                log: None,
            }
        }
    }

    impl ElevationHost for FakeHost {
        fn is_elevated(&self) -> bool {
            self.elevated
        }
        fn run_elevated(
            &self,
            exe: &Path,
            args: &[String],
            dir: &Path,
        ) -> Result<u32, ElevationError> {
            self.runs.fetch_add(1, Ordering::SeqCst);
            *self.args.lock().unwrap() = args.to_vec();
            if self.deny {
                return Err(ElevationError::Denied);
            }
            let mut child = std::process::Command::new(exe)
                .arg("30")
                .current_dir(dir)
                .stdin(std::process::Stdio::null())
                .spawn()
                .map_err(|e| ElevationError::Failed(e.to_string()))?;
            let pid = child.id();
            // Завершённый процесс принимаем (иначе на Linux он остаётся «зомби»).
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            Ok(if self.report_pid { pid } else { 0 })
        }
        fn kill_elevated(&self, _pid: u32) -> bool {
            self.kills.fetch_add(1, Ordering::SeqCst);
            true
        }
        fn engine_log(&self) -> Option<PathBuf> {
            self.log.clone()
        }
    }

    fn engine_with_fake_exe(name: &str) -> Option<SonicDpiEngine> {
        let sleeper = PathBuf::from("/bin/sleep");
        if !sleeper.exists() {
            return None;
        }
        let dir = temp(name);
        std::fs::copy(&sleeper, dir.join(SONICDPI_EXE)).unwrap();
        Some(SonicDpiEngine::new(dir))
    }

    #[test]
    fn without_admin_rights_the_elevated_launch_is_requested_once_with_the_profile_arguments() {
        let Some(engine) = engine_with_fake_exe("el-ask") else {
            return;
        };
        let fake = Arc::new(FakeHost::new(false));
        let host: Arc<dyn ElevationHost> = fake.clone();
        let mut process = engine
            .spawn_auto(DEFAULT_PROFILE, &engine.dir.join("a.log"), &host)
            .unwrap();
        assert_eq!(fake.runs.load(Ordering::SeqCst), 1);
        assert_eq!(
            *fake.args.lock().unwrap(),
            ["run", "--profile", "youtube-discord"]
        );
        assert!(!process.has_exited());
        process.kill();
        assert!(process.has_exited());
        assert_eq!(
            fake.kills.load(Ordering::SeqCst),
            0,
            "обычного завершения хватило"
        );
    }

    #[test]
    fn with_admin_rights_there_is_no_request() {
        let Some(engine) = engine_with_fake_exe("el-none") else {
            return;
        };
        let fake = Arc::new(FakeHost::new(true));
        let host: Arc<dyn ElevationHost> = fake.clone();
        // Фейковый exe — `sleep`: обычный запуск с «run …» завершится сразу, это не важно.
        let _ = engine.spawn_auto(DEFAULT_PROFILE, &engine.dir.join("a.log"), &host);
        assert_eq!(fake.runs.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_refused_request_gives_the_denied_marker_and_starts_nothing() {
        let Some(engine) = engine_with_fake_exe("el-deny") else {
            return;
        };
        let mut fake = FakeHost::new(false);
        fake.deny = true;
        let fake = Arc::new(fake);
        let host: Arc<dyn ElevationHost> = fake.clone();
        let err = engine
            .spawn_auto(DEFAULT_PROFILE, &engine.dir.join("a.log"), &host)
            .err()
            .unwrap();
        assert_eq!(err, ELEVATION_DENIED);
        assert_eq!(fake.runs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn an_unknown_pid_is_found_by_name_and_path_only() {
        let Some(engine) = engine_with_fake_exe("el-find") else {
            return;
        };
        let mut fake = FakeHost::new(false);
        fake.report_pid = false;
        let host: Arc<dyn ElevationHost> = Arc::new(fake);
        let mut process = engine
            .spawn_auto(DEFAULT_PROFILE, &engine.dir.join("a.log"), &host)
            .unwrap();
        assert!(process.pid() > 0);
        assert!(!process.has_exited());
        // Одноимённый процесс из ДРУГОЙ папки не находится по этому пути.
        let other = engine_with_fake_exe("el-find-other").unwrap();
        assert_eq!(
            process_guard::find_by_name_and_path(SONICDPI_EXE, &other.exe_path()),
            None
        );
        process.kill();
    }

    #[test]
    fn the_elevated_process_object_never_touches_a_foreign_process() {
        let sleeper = PathBuf::from("/bin/sleep");
        if !sleeper.exists() {
            return;
        }
        let dir = temp("el-foreign");
        let mut foreign = spawn_logged(&sleeper, &["30"], &dir, &dir.join("f.log")).unwrap();
        let fake = Arc::new(FakeHost::new(false));
        let host: Arc<dyn ElevationHost> = fake.clone();
        // Номер чужого процесса (имя sleep, не ochistkadpi.exe): считается «нашего нет».
        let mut ours_by_mistake = ElevatedProcess::new(foreign.pid(), host);
        assert!(ours_by_mistake.has_exited());
        ours_by_mistake.kill();
        assert!(!foreign.has_exited(), "чужой процесс жив");
        foreign.kill();
    }
}
