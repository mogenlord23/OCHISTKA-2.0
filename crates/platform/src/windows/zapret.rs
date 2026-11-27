use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

const PROFILE: &str = "e883e1ad4009092e628355e57a8256880b45bbafe6a51dc502d8e8f41ceba620";
const BINARIES: &[(&str, &str)] = &[
    (
        "winws.exe",
        "31d68a175abebab19f7f39bf6d0845ec5c18ab653eeeea6ee830b22d799b7e55",
    ),
    (
        "cygwin1.dll",
        "103104a52e5293ce418944725df19e2bf81ad9269b9a120d71d39028e821499b",
    ),
    (
        "WinDivert.dll",
        "c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2",
    ),
    (
        "WinDivert64.sys",
        "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2",
    ),
    (
        "quic_initial_www_google_com.bin",
        "f4589c57749f956bb30538197a521d7005f8b0a8723b4707e72405e51ddac50a",
    ),
    (
        "tls_clienthello_4pda_to.bin",
        "eefeaf09dde8d69b1f176212541f63c68b314a33a335eced99a8a29f17254da8",
    ),
    (
        "tls_clienthello_www_google_com.bin",
        "936c2bee4cfb80aa3c426b2dcbcc834b3fbcd1adb17172959dc569c73a14275c",
    ),
];

fn checked(path: &Path, hash: &str) -> Result<Vec<u8>, String> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let meta = std::fs::symlink_metadata(path).map_err(|_| format!("Не найден файл {name}"))?;
    if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
        return Err(format!("Недопустимый файл {name}"));
    }
    let bytes = std::fs::read(path).map_err(|_| format!("Не удалось прочитать {name}"))?;
    if format!("{:x}", Sha256::digest(&bytes)) != hash {
        return Err(format!("Файл {name} отличается от релиза Flowseal 1.9.0b"));
    }
    Ok(bytes)
}

pub struct Profile {
    bin: PathBuf,
    lists: PathBuf,
    profile: String,
}

impl Profile {
    /// Lists remain local. Only the unmodified launch profile and executable payload are trusted.
    pub fn read(root: &Path) -> Result<Self, String> {
        let bin = root.join("bin");
        let lists = root.join("lists");
        let profile = String::from_utf8(checked(&root.join("general.bat"), PROFILE)?)
            .map_err(|_| "Не удалось прочитать профиль Zapret".to_owned())?;
        for (file, hash) in BINARIES {
            checked(&bin.join(file), hash)?;
        }
        for entry in std::fs::read_dir(&bin).map_err(|err| err.to_string())? {
            let name = entry.map_err(|err| err.to_string())?.file_name();
            let name = name.to_string_lossy();
            if name.to_ascii_lowercase().ends_with(".dll")
                && !BINARIES
                    .iter()
                    .any(|(file, _)| name.eq_ignore_ascii_case(file))
            {
                return Err(format!("Лишняя библиотека в комплекте Zapret: {name}"));
            }
        }
        for file in [
            "list-general.txt",
            "list-google.txt",
            "list-exclude.txt",
            "ipset-all.txt",
            "ipset-exclude.txt",
        ] {
            if !lists.join(file).is_file() {
                return Err(format!("Не найден список {file}"));
            }
        }
        Ok(Self {
            bin,
            lists,
            profile,
        })
    }

    fn arguments(&self, merged_list: Option<&Path>) -> Result<Vec<String>, String> {
        let start = self
            .profile
            .find("--wf-tcp=")
            .ok_or_else(|| "В профиле Zapret отсутствуют параметры запуска".to_owned())?;
        // Split the pinned profile before substituting paths, which can contain spaces.
        Ok(self.profile[start..]
            .split_ascii_whitespace()
            .filter(|word| *word != "^")
            .map(|word| {
                let arg = word.replace('"', "");
                if let Some(path) = merged_list {
                    if arg == "--hostlist=%LISTS%list-general.txt" {
                        return format!("--hostlist={}", path.display());
                    }
                }
                arg.replace("%GameFilter%", "12")
                    .replace("%BIN%", &format!("{}\\", self.bin.display()))
                    .replace("%LISTS%", &format!("{}\\", self.lists.display()))
            })
            .collect())
    }

    pub fn merge_list(&self, destination: &Path, extra: &[String]) -> Result<(), String> {
        let list = self.lists.join("list-general.txt");
        if std::fs::metadata(&list)
            .map_err(|err| err.to_string())?
            .len()
            > 4 * 1024 * 1024
        {
            return Err("Список list-general.txt слишком большой".into());
        }
        let mut bytes = std::fs::read(list).map_err(|err| err.to_string())?;
        bytes.push(b'\n');
        bytes.extend(extra.join("\n").as_bytes());
        bytes.push(b'\n');
        std::fs::write(destination, bytes).map_err(|err| err.to_string())
    }

    pub fn start(&self, merged_list: Option<&Path>) -> Result<Process, String> {
        let job = crate::windows::win32_ffi::ProcessJob::new()?;
        let mut child = self
            .command(merged_list)?
            .spawn()
            .map_err(|err| format!("Не удалось запустить Zapret: {err}"))?;
        if let Err(error) = job.attach(&child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        let mut process = Process { child, _job: job };
        std::thread::sleep(Duration::from_millis(350));
        if !process.is_running()? {
            return Err(
                "Zapret завершился при запуске. Проверьте права и состояние WinDivert.".into(),
            );
        }
        Ok(process)
    }

    fn command(&self, merged_list: Option<&Path>) -> Result<Command, String> {
        let mut command = Command::new(self.bin.join("winws.exe"));
        command
            .args(self.arguments(merged_list)?)
            .current_dir(&self.bin)
            .env_remove("PATH")
            .creation_flags(0x0800_0000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        Ok(command)
    }

    pub fn validate(&self, merged_list: Option<&Path>) -> Result<(), String> {
        let mut command = self.command(merged_list)?;
        // getopt accepts --dry-run anywhere in the argument list.
        let status = command
            .arg("--dry-run")
            .status()
            .map_err(|err| err.to_string())?;
        if !status.success() {
            return Err(
                "Zapret не принял профиль или один из списков. VPN остался в прежнем состоянии."
                    .into(),
            );
        }
        Ok(())
    }
}

pub struct Process {
    child: Child,
    _job: crate::windows::win32_ffi::ProcessJob,
}

impl Process {
    pub fn is_running(&mut self) -> Result<bool, String> {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|err| err.to_string())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if !self.is_running()? {
            return Ok(());
        }
        if let Err(err) = self.child.kill() {
            if self.is_running()? {
                return Err(format!("Не удалось остановить Zapret: {err}"));
            }
        }
        let until = Instant::now() + Duration::from_secs(3);
        while self.is_running()? {
            if Instant::now() >= until {
                return Err("Zapret не завершился вовремя".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        Ok(())
    }
}
