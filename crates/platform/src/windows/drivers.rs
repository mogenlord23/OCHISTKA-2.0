//! Установка драйверов на Windows: загрузка официального установщика по https, проверка
//! подписи Authenticode через PowerShell (абсолютный путь), запуск установщика через
//! `explorer.exe` (права администратора запрашивает сам установщик, UAC). Логика порядка и
//! проверок — в `ochistka_core::drivers`; здесь только системные действия (ADR 0011).

#[cfg(windows)]
pub use imp::WindowsDriverHost;

#[cfg(windows)]
mod imp {
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::Duration;

    use ochistka_core::drivers::{
        validate_page_url, validate_redirect, DriverError, DriverHost, MAX_REDIRECTS,
        SIGNATURE_SCRIPT,
    };
    use url::Url;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub struct WindowsDriverHost;

    impl WindowsDriverHost {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for WindowsDriverHost {
        fn default() -> Self {
            Self::new()
        }
    }

    fn system_root() -> PathBuf {
        std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
    }

    fn powershell_path() -> PathBuf {
        system_root().join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
    }

    fn explorer_path() -> PathBuf {
        system_root().join("explorer.exe")
    }

    impl DriverHost for WindowsDriverHost {
        fn downloads_dir(&self) -> Result<PathBuf, DriverError> {
            let base = std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .ok_or_else(|| DriverError::Download("не найдена папка LOCALAPPDATA".into()))?;
            let dir = base.join("OCHISTKA").join("Downloads");
            fs::create_dir_all(&dir)
                .map_err(|err| DriverError::Download(format!("папка загрузок: {err}")))?;
            Ok(dir)
        }

        fn download(
            &self,
            url: &Url,
            dest: &Path,
            max_bytes: u64,
            on_progress: &mut dyn FnMut(u64, Option<u64>),
        ) -> Result<(), DriverError> {
            // Каждая переадресация проходит ту же проверку, что и исходный адрес.
            let policy = reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= MAX_REDIRECTS {
                    return attempt.error("слишком много переадресаций");
                }
                match validate_redirect(attempt.url().as_str()) {
                    Ok(_) => attempt.follow(),
                    Err(err) => attempt.error(err.to_string()),
                }
            });
            let client = reqwest::blocking::Client::builder()
                .redirect(policy)
                .connect_timeout(Duration::from_secs(20))
                .timeout(Duration::from_secs(15 * 60))
                .user_agent("OCHISTKA")
                .build()
                .map_err(|err| DriverError::Download(err.to_string()))?;
            let mut response = client
                .get(url.as_str())
                .send()
                .map_err(|err| DriverError::Download(err.to_string()))?;
            if !response.status().is_success() {
                return Err(DriverError::Download(format!(
                    "сервер ответил {}",
                    response.status()
                )));
            }
            // Итоговый адрес тоже должен быть разрешённым.
            validate_redirect(response.url().as_str())?;
            let is_page = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/html"));
            if is_page {
                return Err(DriverError::Download(
                    "вместо установщика пришла веб-страница".into(),
                ));
            }
            let total = response.content_length();
            if total.is_some_and(|len| len > max_bytes) {
                return Err(DriverError::TooLarge);
            }
            let mut file = File::create(dest)
                .map_err(|err| DriverError::Download(format!("не создать файл: {err}")))?;
            let mut buffer = vec![0u8; 128 * 1024];
            let mut done: u64 = 0;
            loop {
                let count = response
                    .read(&mut buffer)
                    .map_err(|err| DriverError::Download(err.to_string()))?;
                if count == 0 {
                    break;
                }
                done += count as u64;
                if done > max_bytes {
                    return Err(DriverError::TooLarge);
                }
                file.write_all(&buffer[..count])
                    .map_err(|err| DriverError::Download(format!("запись файла: {err}")))?;
                on_progress(done, total);
            }
            file.sync_all()
                .map_err(|err| DriverError::Download(format!("запись файла: {err}")))?;
            if total.is_some_and(|len| len != done) {
                return Err(DriverError::Download("загрузка оборвалась".into()));
            }
            Ok(())
        }

        fn signature_output(&self, file: &Path) -> Result<String, DriverError> {
            let output = Command::new(powershell_path())
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    SIGNATURE_SCRIPT,
                ])
                .env("OCHISTKA_CHECK_FILE", file)
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|err| DriverError::SignatureInvalid(format!("PowerShell: {err}")))?;
            if !output.status.success() {
                return Err(DriverError::SignatureInvalid(
                    "проверка подписи не выполнилась".into(),
                ));
            }
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }

        fn rename(&self, from: &Path, to: &Path) -> Result<(), DriverError> {
            fs::rename(from, to).map_err(|err| DriverError::Download(format!("файл: {err}")))
        }

        fn remove(&self, file: &Path) {
            let _ = fs::remove_file(file);
        }

        fn launch(&self, file: &Path) -> Result<(), DriverError> {
            // Через оболочку Windows: если установщику нужны права администратора, система
            // сама покажет запрос UAC только для него.
            Command::new(explorer_path())
                .arg(file)
                .spawn()
                .map(|_| ())
                .map_err(|err| DriverError::Launch(err.to_string()))
        }

        fn open_url(&self, url: &str) -> Result<(), DriverError> {
            validate_page_url(url)?;
            Command::new(explorer_path())
                .arg(url)
                .spawn()
                .map(|_| ())
                .map_err(|err| DriverError::Launch(err.to_string()))
        }
    }
}
