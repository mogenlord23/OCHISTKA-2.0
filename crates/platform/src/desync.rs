//! Windows-адаптер «Обхода блокировок»: `winws2` + WinDivert за общим `DesyncAdapter` ядра.
//!
//! Что реализовано: проверка состава и SHA-256 файлов закреплённого выпуска, загрузка с проверкой
//! (общий механизм sing-box), запуск без окна через общий `spawn_logged`, проверка прав
//! администратора, остановка только СВОЕГО процесса (по номеру из файла и по пути к exe, а не по
//! имени), понятная реакция на блокировку файла защитой Windows (защиту не отключаем и
//! исключений не добавляем).
//!
//! Движок Windows из проверенного Release с закреплёнными хэшами доступен для установки.
//! Оригинальные Lua-скрипты из того же upstream-коммита встроены в приложение и
//! записываются рядом с движком с проверкой целостности. Кандидат аргументов взят из
//! документации Zapret 2, но не проверен на реальной сети. До такой проверки адаптер
//! сообщает NotImplemented и ничего не запускает. Подписанный драйвер используем без переименования;
//! защиту Windows не отключаем.

use std::path::{Path, PathBuf};

use ochistka_core::desync::{self, DesyncAdapter, Platform, Readiness};
use ochistka_core::vpn::engine::{self, PinnedEngine, PrepareError, PrepareProgress};
use ochistka_core::vpn::service::CoreProcess;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::vpn::spawn_logged;

mod merged_assets;
mod preset_assets;
pub mod probe;

use merged_assets::MERGED_EXTRA_RESOURCES;
use preset_assets::PINNED_RESOURCES;

/// Все встроенные ресурсы: 135 закреплённых файлов V13 Lite и данные объединённого пресета.
fn all_resources() -> impl Iterator<Item = &'static (&'static str, &'static [u8], &'static str)> {
    PINNED_RESOURCES.iter().chain(MERGED_EXTRA_RESOURCES.iter())
}

pub const ENGINE_EXE: &str = "winws2.exe";

/// Не модифицированные Lua-исходники из bol-van/zapret2@1eb4fd887b6091ccaf59feea7486162ac64c9319.
/// Upstream git blob SHA: lib=92fb6a01b78597f673cc20dd869a9d1bf7d41b27,
/// antidpi=9022555c59d7e986a7385c2ecae524130ae9cc7b. MIT (см. LICENSE-zapret2.txt).
const LUA_ASSETS: [(&str, &[u8]); 2] = [
    (
        "zapret-lib.lua",
        include_bytes!("../../../assets/desync/zapret-lib.lua"),
    ),
    (
        "zapret-antidpi.lua",
        include_bytes!("../../../assets/desync/zapret-antidpi.lua"),
    ),
];

fn lua_assets_valid(dir: &Path) -> bool {
    LUA_ASSETS.iter().all(|(name, data)| {
        std::fs::read(dir.join(name)).is_ok_and(|file| file.as_slice() == *data)
    })
}

#[cfg_attr(not(windows), allow(dead_code))]
fn lua_write_error(err: std::io::Error) -> PrepareError {
    PrepareError {
        reason: "не удалось сохранить Lua-скрипты".into(),
        detail: err.to_string(),
    }
}

/// Сохраняем оригинальные Lua-файлы только в каталог проверенного движка.
/// Повторная запись исправляет отсутствующие или изменённые Lua-файлы; проверка
/// целостности после подготовки обязательна, а запуск при подмене блокируется.
#[cfg_attr(not(windows), allow(dead_code))]
fn install_lua_assets(dir: &Path) -> Result<(), PrepareError> {
    std::fs::create_dir_all(dir).map_err(lua_write_error)?;
    for (name, data) in LUA_ASSETS {
        let path = dir.join(name);
        if std::fs::read(&path).is_ok_and(|file| file.as_slice() == data) {
            continue;
        }
        let tmp = dir.join(format!("{name}.part"));
        std::fs::write(&tmp, data).map_err(lua_write_error)?;
        // Удаляем только свой одноимённый ресурс, не чужие файлы и каталоги.
        if path.is_file() {
            std::fs::remove_file(&path).map_err(lua_write_error)?;
        }
        std::fs::rename(&tmp, &path).map_err(lua_write_error)?;
    }
    if !lua_assets_valid(dir) {
        return Err(PrepareError {
            reason: "Lua-скрипты не прошли проверку целостности".into(),
            detail: "Lua-скрипты отсутствуют или изменены".into(),
        });
    }
    Ok(())
}

/// Check all embedded preset dependencies without altering the network.
fn preset_assets_valid(dir: &Path) -> bool {
    if std::fs::symlink_metadata(dir).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return false;
    }
    let Ok(root) = dir.canonicalize() else {
        return false;
    };
    all_resources().all(|(relative, _, digest)| {
        let path = dir.join(relative);
        std::fs::symlink_metadata(&path)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
            && path
                .canonicalize()
                .is_ok_and(|resolved| resolved.starts_with(&root))
            && std::fs::read(&path).is_ok_and(|bytes| engine::sha256_hex(&bytes) == *digest)
    })
}

/// Хэши встроенных ресурсов проверяются один раз за запуск приложения.
fn embedded_resources_intact() -> bool {
    static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OK.get_or_init(|| {
        all_resources().all(|(_, bytes, digest)| engine::sha256_hex(bytes) == *digest)
    })
}

fn preset_asset_error(detail: impl Into<String>) -> PrepareError {
    PrepareError {
        reason: "не удалось подготовить проверенные файлы обхода".into(),
        detail: detail.into(),
    }
}

/// Explicit preparation only. Verify bundled hashes before writing anything.
/// A partial write can never create a ready adapter; locate/spawn recheck all 135.
fn install_preset_assets(dir: &Path) -> Result<(), PrepareError> {
    for (relative, bytes, digest) in all_resources() {
        if engine::sha256_hex(bytes) != *digest {
            return Err(preset_asset_error(format!(
                "нарушен SHA-256 встроенного ресурса: {relative}"
            )));
        }
    }
    std::fs::create_dir_all(dir).map_err(|e| preset_asset_error(e.to_string()))?;
    if std::fs::symlink_metadata(dir).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(preset_asset_error("каталог компонентов является ссылкой"));
    }
    let root = dir
        .canonicalize()
        .map_err(|e| preset_asset_error(e.to_string()))?;
    for (relative, embedded, digest) in all_resources() {
        let path = dir.join(relative);
        let parent = path
            .parent()
            .ok_or_else(|| preset_asset_error("нет каталога ресурса"))?;
        std::fs::create_dir_all(parent).map_err(|e| preset_asset_error(e.to_string()))?;
        if !parent
            .canonicalize()
            .is_ok_and(|resolved| resolved.starts_with(&root))
        {
            return Err(preset_asset_error(format!(
                "ресурс за пределами каталога компонентов: {relative}"
            )));
        }
        if let Ok(metadata) = std::fs::symlink_metadata(&path) {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(preset_asset_error(format!(
                    "ресурс является ссылкой или каталогом: {relative}"
                )));
            }
        }
        if std::fs::read(&path).is_ok_and(|bytes| engine::sha256_hex(&bytes) == *digest) {
            continue;
        }
        let part = path.with_extension("ochistka-part");
        if let Ok(metadata) = std::fs::symlink_metadata(&part) {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(preset_asset_error(format!(
                    "временный путь ресурса небезопасен: {relative}"
                )));
            }
            std::fs::remove_file(&part).map_err(|e| preset_asset_error(e.to_string()))?;
        }
        std::fs::write(&part, embedded).map_err(|e| preset_asset_error(e.to_string()))?;
        if !std::fs::read(&part).is_ok_and(|bytes| engine::sha256_hex(&bytes) == *digest) {
            return Err(preset_asset_error(format!(
                "ресурс после записи не прошёл SHA-256: {relative}"
            )));
        }
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| preset_asset_error(e.to_string()))?;
        }
        std::fs::rename(&part, &path).map_err(|e| preset_asset_error(e.to_string()))?;
    }
    if !preset_assets_valid(dir) {
        return Err(preset_asset_error(
            "не все ресурсы прошли проверку целостности",
        ));
    }
    Ok(())
}

pub struct WindowsDesyncAdapter {
    dir: PathBuf,
    spec: Option<&'static PinnedEngine>,
}

impl WindowsDesyncAdapter {
    /// `dir` — папка компонентов в данных приложения.
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            spec: desync::pinned(),
        }
    }

    #[cfg(test)]
    fn with_spec(dir: PathBuf, spec: Option<&'static PinnedEngine>) -> Self {
        Self { dir, spec }
    }

    fn exe_path(&self) -> PathBuf {
        self.dir.join(ENGINE_EXE)
    }
}

/// Какие сообщения системы значат «файл заблокирован защитой Windows».
#[cfg_attr(not(windows), allow(dead_code))]
fn blocked_by_protection(detail: &str) -> bool {
    let d = detail.to_lowercase();
    d.contains("virus")
        || d.contains("вирус")
        || d.contains("potentially unwanted")
        || d.contains("нежелательн")
        || d.contains("os error 225")
}

#[cfg_attr(not(windows), allow(dead_code))]
fn protection_error(detail: String) -> PrepareError {
    PrepareError {
        reason: "защита Windows заблокировала файл обхода. Защиту OCHISTKA не отключает и исключения не добавляет".into(),
        detail,
    }
}

#[cfg(windows)]
fn elevated() -> bool {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_WRITE};
    use winreg::RegKey;
    // Как у служб: открытие системного ключа на запись ничего не меняет, но без прав отклоняется.
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Services", KEY_WRITE)
        .is_ok()
}

#[cfg(not(windows))]
fn elevated() -> bool {
    false
}

impl DesyncAdapter for WindowsDesyncAdapter {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn readiness(&self) -> Readiness {
        if !cfg!(windows) {
            return Readiness::NotImplemented("Эта сборка не для Windows".into());
        }
        match self.spec {
            None => Readiness::NotImplemented(
                "Обход блокировок пока недоступен: подтверждённые файлы движка отсутствуют".into(),
            ),
            Some(_) if desync::site_discord_profile(Platform::Windows).is_none() => {
                Readiness::NotImplemented(
                    "Движок Zapret 2 закреплён, но рабочий профиль обхода ещё не проверен".into(),
                )
            }
            Some(_) => Readiness::Ready,
        }
    }

    /// Только для явного пробного запуска полного профиля: закреплённый движок известен и
    /// все встроенные ресурсы совпали со своими SHA-256. Установленные файлы и WinDivert
    /// перепроверяются при `locate`/`install`/`spawn`; права — при запуске. Обычную
    /// `readiness()` это не меняет.
    fn trial_readiness(&self) -> Readiness {
        if !cfg!(windows) {
            return Readiness::NotImplemented("Эта сборка не для Windows".into());
        }
        if self.spec.is_none() {
            return Readiness::NotImplemented(
                "Файлы движка не закреплены: пробный запуск недоступен".into(),
            );
        }
        if !embedded_resources_intact() {
            return Readiness::NotImplemented(
                "Встроенные файлы пресета не прошли проверку SHA-256".into(),
            );
        }
        Readiness::Ready
    }

    fn locate(&self) -> Result<PathBuf, String> {
        let spec = self
            .spec
            .ok_or("файлы движка не закреплены: нет подтверждённых версий и хэшей")?;
        if engine::is_installed(spec, &self.dir)
            && lua_assets_valid(&self.dir)
            && preset_assets_valid(&self.dir)
        {
            Ok(self.exe_path())
        } else {
            Err("компоненты обхода, Lua или файлы профиля отсутствуют либо изменены".into())
        }
    }

    fn install(&self, progress: &mut dyn FnMut(PrepareProgress)) -> Result<(), PrepareError> {
        let spec = self.spec.ok_or_else(|| {
            PrepareError::unsupported("загрузка запрещена: файлы движка не подтверждены")
        })?;
        #[cfg(windows)]
        {
            engine::install(
                spec,
                &self.dir,
                &crate::vpn::http::ReqwestDownloader,
                progress,
            )
            .map_err(|e| {
                if blocked_by_protection(&e.detail) {
                    protection_error(e.detail)
                } else {
                    e
                }
            })?;
            install_lua_assets(&self.dir)?;
            install_preset_assets(&self.dir)?;
            // Защита могла убрать файл сразу после записи.
            if engine::is_installed(spec, &self.dir)
                && lua_assets_valid(&self.dir)
                && preset_assets_valid(&self.dir)
            {
                Ok(())
            } else {
                Err(protection_error(
                    "после загрузки файлы отсутствуют или изменены".into(),
                ))
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (spec, progress);
            Err(PrepareError::unsupported("эта сборка не для Windows"))
        }
    }

    fn has_privileges(&self) -> bool {
        elevated()
    }

    fn spawn(
        &self,
        exe: &Path,
        args: &[String],
        log: &Path,
    ) -> Result<Box<dyn CoreProcess>, String> {
        // Только наш файл из папки компонентов: чужой путь не запускаем.
        if exe != self.exe_path() {
            return Err("запуск чужого файла отклонён".into());
        }
        let Some(spec) = self.spec else {
            return Err("движок не закреплён".into());
        };
        if !engine::is_installed(spec, &self.dir)
            || !lua_assets_valid(&self.dir)
            || !preset_assets_valid(&self.dir)
        {
            return Err(
                "запуск отклонён: целостность движка, Lua или файлов профиля нарушена".into(),
            );
        }
        let workdir = exe.parent().unwrap_or(Path::new("."));
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let process = spawn_logged(exe, &arg_refs, workdir, log)?;
        Ok(Box::new(process))
    }

    /// Останавливает процесс из прошлого запуска, только если номер принадлежит именно нашему
    /// `winws2.exe` (путь к файлу совпадает). По имени чужие процессы не трогаем.
    fn kill_stale(&self, pid: u32) {
        let target = [Pid::from_u32(pid)];
        let Ok(mut system) = System::new() else {
            return;
        };
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&target),
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
        );
        let ours = self.exe_path();
        if let Some(process) = system.process(target[0]) {
            if process.exe().is_some_and(|path| same_file(path, &ours)) {
                process.kill();
            }
        }
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Папка данных обхода: `%LOCALAPPDATA%\\OCHISTKA\\desync`, иначе временная.
pub fn desync_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OCHISTKA")
        .join("desync")
}

/// Адаптер для текущей ОС. Ядро (`ochistka-core`) об ОС ничего не знает.
pub fn adapter_for_current_os(dir: PathBuf) -> Box<dyn DesyncAdapter> {
    if cfg!(windows) {
        Box::new(WindowsDesyncAdapter::new(dir))
    } else {
        Box::new(desync::UnavailableAdapter::new(Platform::current()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochistka_core::vpn::engine::sha256_hex;

    #[test]
    fn merged_preset_resources_are_all_embedded_with_matching_hashes() {
        let merged = ochistka_core::desync::preset::ZapretPreset::parse(include_str!(
            "../../../assets/desync/presets/OCHISTKA Merged.txt"
        ))
        .unwrap();
        let names: std::collections::BTreeSet<&str> = all_resources().map(|(n, _, _)| *n).collect();
        assert_eq!(
            names.len(),
            PINNED_RESOURCES.len() + MERGED_EXTRA_RESOURCES.len(),
            "имена не повторяются"
        );
        for rel in merged.required_resources().unwrap() {
            assert!(names.contains(rel.as_str()), "{rel}");
        }
        for (name, bytes, digest) in MERGED_EXTRA_RESOURCES {
            assert_eq!(engine::sha256_hex(bytes), *digest, "{name}");
            assert!(name.starts_with("lists/") && !name.contains(".."), "{name}");
        }
        assert_eq!(
            PINNED_RESOURCES.len(),
            135,
            "135 закреплённых файлов не менялись"
        );
    }

    #[test]
    fn merged_resources_install_into_the_engine_folder_and_tampering_is_detected() {
        let dir = temp("merged");
        install_preset_assets(&dir).unwrap();
        assert!(preset_assets_valid(&dir));
        let (name, _, _) = MERGED_EXTRA_RESOURCES[0];
        std::fs::write(dir.join(name), b"tampered").unwrap();
        assert!(!preset_assets_valid(&dir));
        install_preset_assets(&dir).unwrap();
        assert!(preset_assets_valid(&dir));
    }

    #[test]
    fn trial_readiness_does_not_change_normal_readiness() {
        let adapter = WindowsDesyncAdapter::with_spec(temp("trial"), None);
        assert!(matches!(adapter.readiness(), Readiness::NotImplemented(_)));
        assert!(matches!(
            adapter.trial_readiness(),
            Readiness::NotImplemented(_)
        ));
        assert!(embedded_resources_intact());
        let pinned = WindowsDesyncAdapter::new(temp("trial2"));
        assert!(matches!(pinned.readiness(), Readiness::NotImplemented(_)));
        if cfg!(windows) && desync::pinned().is_some() {
            assert_eq!(pinned.trial_readiness(), Readiness::Ready);
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-wdesync-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn spec_for(files: &[(&str, &[u8])]) -> &'static PinnedEngine {
        let list: Vec<(&'static str, &'static str)> = files
            .iter()
            .map(|(n, d)| {
                (
                    &*Box::leak(n.to_string().into_boxed_str()),
                    &*Box::leak(sha256_hex(d).into_boxed_str()),
                )
            })
            .collect();
        Box::leak(Box::new(PinnedEngine {
            version: "t",
            url: "https://github.com/x/y/releases/download/t/a.zip",
            url_prefix: "/x/y/releases/download/",
            archive_sha256: "00",
            files: Box::leak(list.into_boxed_slice()),
        }))
    }

    #[test]
    fn adapter_stays_not_ready_without_verified_strategy() {
        let adapter = WindowsDesyncAdapter::new(temp("none"));
        assert!(matches!(adapter.readiness(), Readiness::NotImplemented(_)));
        assert!(adapter.locate().is_err());
        // Не запускаем реальную сетевую загрузку в тесте: Release закреплён, но профиль не готов.
    }

    #[test]
    fn five_required_runtime_files_are_checked_and_tampering_is_blocked() {
        let dir = temp("five-files");
        let files: [(&str, &[u8]); 5] = [
            ("winws2.exe", b"exe"),
            ("cygwin1.dll", b"cygwin"),
            ("cygz.dll", b"zlib"),
            ("WinDivert.dll", b"dll"),
            ("WinDivert64.sys", b"driver"),
        ];
        let spec = spec_for(&files);
        let adapter = WindowsDesyncAdapter::with_spec(dir.clone(), Some(spec));
        for (name, data) in files {
            std::fs::write(dir.join(name), data).unwrap();
        }
        install_lua_assets(&dir).unwrap();
        install_preset_assets(&dir).unwrap();
        assert!(adapter.locate().is_ok());
        for (name, data) in files {
            std::fs::write(dir.join(name), b"tampered").unwrap();
            assert!(
                adapter.locate().is_err(),
                "подмена {name} должна блокировать запуск"
            );
            std::fs::write(dir.join(name), data).unwrap();
            assert!(
                adapter.locate().is_ok(),
                "после восстановления {name} хэш совпадает"
            );
        }
        for (name, data) in files {
            std::fs::remove_file(dir.join(name)).unwrap();
            assert!(
                adapter.locate().is_err(),
                "отсутствие {name} должно блокировать запуск"
            );
            std::fs::write(dir.join(name), data).unwrap();
        }
    }

    #[test]
    fn components_are_found_only_when_every_hash_matches() {
        let dir = temp("hash");
        let spec = spec_for(&[(ENGINE_EXE, b"engine"), ("WinDivert.dll", b"dll")]);
        let adapter = WindowsDesyncAdapter::with_spec(dir.clone(), Some(spec));
        assert!(adapter.locate().is_err(), "файлов нет");
        std::fs::write(dir.join(ENGINE_EXE), b"engine").unwrap();
        std::fs::write(dir.join("WinDivert.dll"), b"dll").unwrap();
        install_lua_assets(&dir).unwrap();
        install_preset_assets(&dir).unwrap();
        assert_eq!(adapter.locate().unwrap(), dir.join(ENGINE_EXE));
        std::fs::write(dir.join("WinDivert.dll"), b"tampered").unwrap();
        assert!(adapter.locate().unwrap_err().contains("изменены"));
    }

    #[test]
    fn pinned_upstream_lua_assets_are_written_and_tampering_is_detected() {
        let dir = temp("lua-assets");
        assert!(!lua_assets_valid(&dir));
        install_lua_assets(&dir).unwrap();
        assert!(lua_assets_valid(&dir));
        for (name, original) in LUA_ASSETS {
            assert_eq!(std::fs::read(dir.join(name)).unwrap(), original);
            std::fs::write(dir.join(name), b"tampered").unwrap();
            assert!(!lua_assets_valid(&dir), "подмена {name} обнаружена");
            install_lua_assets(&dir).unwrap();
            assert!(lua_assets_valid(&dir));
            assert_eq!(std::fs::read(dir.join(name)).unwrap(), original);
        }
    }

    #[test]
    fn pinned_preset_dependencies_install_and_detect_tampering() {
        let dir = temp("preset-assets");
        assert_eq!(PINNED_RESOURCES.len(), 135);
        assert!(!preset_assets_valid(&dir));
        install_preset_assets(&dir).unwrap();
        assert!(preset_assets_valid(&dir));
        for (name, original, digest) in PINNED_RESOURCES {
            assert_eq!(engine::sha256_hex(original), *digest);
            assert_eq!(
                engine::sha256_hex(&std::fs::read(dir.join(name)).unwrap()),
                *digest
            );
        }
        let first = PINNED_RESOURCES[0].0;
        std::fs::write(dir.join(first), b"corrupted").unwrap();
        assert!(!preset_assets_valid(&dir));
        install_preset_assets(&dir).unwrap();
        assert!(preset_assets_valid(&dir));
        std::fs::remove_file(dir.join(first)).unwrap();
        assert!(!preset_assets_valid(&dir));
    }

    #[cfg(unix)]
    #[test]
    fn preset_runtime_symlink_is_rejected_without_following_it() {
        use std::os::unix::fs::symlink;
        let dir = temp("preset-symlink");
        let path = dir.join(PINNED_RESOURCES[0].0);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink("/etc/passwd", &path).unwrap();
        assert!(install_preset_assets(&dir).is_err());
        assert!(!preset_assets_valid(&dir));
    }

    #[test]
    fn only_our_own_engine_file_can_be_started() {
        let dir = temp("spawn");
        let adapter = WindowsDesyncAdapter::with_spec(dir.clone(), None);
        let err = adapter
            .spawn(Path::new("/bin/sh"), &[], &dir.join("l"))
            .map(|_| ())
            .unwrap_err();
        assert!(err.contains("чужого файла"));
    }

    #[test]
    fn stale_cleanup_stops_only_a_process_that_is_our_engine_file() {
        let dir = temp("stale");
        let sleeper = PathBuf::from("/bin/sleep");
        if !sleeper.exists() {
            return;
        }
        // Чужой процесс: путь не совпадает с нашим winws2.exe, имя ни при чём.
        let mut foreign = spawn_logged(&sleeper, &["30"], &dir, &dir.join("a.log")).unwrap();
        WindowsDesyncAdapter::with_spec(dir.clone(), None).kill_stale(foreign.pid());
        assert!(!foreign.has_exited(), "чужой процесс остался");
        // «Наш» процесс: экземпляр, чей exe равен пути адаптера.
        let ours_dir = temp("ours");
        std::fs::copy(&sleeper, ours_dir.join(ENGINE_EXE)).unwrap();
        let mut ours = spawn_logged(
            &ours_dir.join(ENGINE_EXE),
            &["30"],
            &ours_dir,
            &ours_dir.join("b.log"),
        )
        .unwrap();
        WindowsDesyncAdapter::with_spec(ours_dir.clone(), None).kill_stale(ours.pid());
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(ours.has_exited(), "свой процесс остановлен");
        foreign.kill();
    }

    #[test]
    fn protection_blocks_are_recognised_and_explained() {
        assert!(blocked_by_protection(
            "Operation did not complete successfully because the file contains a virus or potentially unwanted software."
        ));
        assert!(blocked_by_protection("os error 225"));
        assert!(!blocked_by_protection("timeout"));
        let e = protection_error("x".into());
        assert!(e.reason.contains("не отключает"));
    }

    #[test]
    fn privileges_and_os_selection_are_honest_off_windows() {
        let adapter = WindowsDesyncAdapter::new(temp("priv"));
        assert!(!adapter.has_privileges() || cfg!(windows));
        let chosen = adapter_for_current_os(temp("pick"));
        assert!(matches!(
            chosen.readiness(),
            Readiness::NotImplemented(_) | Readiness::Unsupported(_)
        ));
    }
}
