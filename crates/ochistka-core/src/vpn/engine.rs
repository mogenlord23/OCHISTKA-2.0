//! Подготовка VPN-движка (sing-box): закреплённая версия, проверка SHA-256, распаковка.
//! Приложение само скачивает движок при первом подключении, поэтому пользователь ничего
//! не ставит руками. Сеть и файловые права берутся из порта `Downloader`; здесь только логика.
//! Значения совпадают с `tools/vpn-core/fetch.sh` и `fetch.ps1` (менять только вместе).

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};
use ts_rs::TS;
use url::Url;

/// Закреплённый выпуск: адрес, хэш архива и хэши файлов внутри.
pub struct PinnedEngine {
    pub version: &'static str,
    pub url: &'static str,
    pub archive_sha256: &'static str,
    pub files: &'static [(&'static str, &'static str)],
}

pub const PINNED: PinnedEngine = PinnedEngine {
    version: "1.14.4",
    url: "https://github.com/SagerNet/sing-box/releases/download/v1.14.4/sing-box-1.14.4-windows-amd64.zip",
    archive_sha256: "99185d75d314a07dbba4a130c8ad986cbeb629a07927975636b341ca61f991df",
    files: &[
        (
            "sing-box.exe",
            "8f30682ee198855440d2a7fe070d5d3653793b2d01bb8c85eea88f93bbe7c8b9",
        ),
        (
            "libcronet.dll",
            "3217c6260fbca5f16072e0b79735742f40109a63bb0ff88fd6b96dd6b54a2928",
        ),
        (
            "LICENSE",
            "bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c",
        ),
    ],
};

/// Больше этого архив не принимаем (настоящий около 30 МБ).
pub const MAX_ARCHIVE_BYTES: usize = 150 * 1024 * 1024;

/// Первый адрес: только https и только выпуски SagerNet/sing-box на github.com.
pub fn download_url_allowed(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    parsed.scheme() == "https"
        && parsed.host_str() == Some("github.com")
        && parsed.port().is_none()
        && parsed.username().is_empty()
        && parsed
            .path()
            .starts_with("/SagerNet/sing-box/releases/download/")
}

/// Переходы при загрузке: GitHub отдаёт файл с собственных адресов `*.githubusercontent.com`.
pub fn redirect_allowed(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    parsed.scheme() == "https"
        && (host == "github.com" || host.ends_with(".githubusercontent.com"))
        && parsed.username().is_empty()
}

/// Ход подготовки для кнопки: `percent` от 0 до 100.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnPrepareProgress.ts")]
pub struct PrepareProgress {
    pub percent: u8,
}

/// Причина неудачи, понятная человеку. Подробности (для журнала) отдельно.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrepareError {
    pub reason: String,
    pub detail: String,
}

impl PrepareError {
    pub fn network(detail: impl Into<String>) -> Self {
        Self {
            reason: "проверьте интернет".into(),
            detail: detail.into(),
        }
    }

    fn integrity(detail: impl Into<String>) -> Self {
        Self {
            reason: "загруженный файл не прошёл проверку, попробуйте позже".into(),
            detail: detail.into(),
        }
    }

    fn disk(detail: impl Into<String>) -> Self {
        Self {
            reason: "не удалось сохранить файлы, проверьте свободное место".into(),
            detail: detail.into(),
        }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self {
            detail: reason.clone(),
            reason,
        }
    }
}

/// Загрузка файла по https. Реализация в `crates/platform`.
pub trait Downloader {
    /// Скачивает `url` целиком (не больше `max_bytes`). `on_progress(получено, всего)`.
    fn download(
        &self,
        url: &str,
        max_bytes: usize,
        on_progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>, PrepareError>;
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Все файлы на месте и их хэши совпадают.
pub fn is_installed(spec: &PinnedEngine, dir: &Path) -> bool {
    spec.files.iter().all(|(name, hash)| {
        std::fs::read(dir.join(name))
            .map(|bytes| sha256_hex(&bytes) == *hash)
            .unwrap_or(false)
    })
}

/// Скачивает архив, проверяет хэши и кладёт файлы в `dir`. Ничего не пишет, пока всё не проверено.
pub fn install(
    spec: &PinnedEngine,
    dir: &Path,
    downloader: &dyn Downloader,
    progress: &mut dyn FnMut(PrepareProgress),
) -> Result<(), PrepareError> {
    if is_installed(spec, dir) {
        progress(PrepareProgress { percent: 100 });
        return Ok(());
    }
    if !download_url_allowed(spec.url) {
        return Err(PrepareError::integrity("адрес загрузки не разрешён"));
    }
    let mut last = None;
    // Загрузка занимает 0-90 %, проверка и запись остальное.
    let mut report = |received: u64, total: Option<u64>| {
        let percent = match total {
            Some(t) if t > 0 => ((received.min(t) * 90) / t) as u8,
            _ => 0,
        };
        if last != Some(percent) {
            last = Some(percent);
            progress(PrepareProgress { percent });
        }
    };
    report(0, None);
    let archive = downloader.download(spec.url, MAX_ARCHIVE_BYTES, &mut report)?;
    if sha256_hex(&archive) != spec.archive_sha256 {
        return Err(PrepareError::integrity("SHA-256 архива не совпал"));
    }
    let files = unpack(spec, &archive)?;
    std::fs::create_dir_all(dir).map_err(|e| PrepareError::disk(e.to_string()))?;
    for (name, bytes) in &files {
        let tmp = dir.join(format!("{name}.part"));
        std::fs::write(&tmp, bytes).map_err(|e| PrepareError::disk(e.to_string()))?;
        std::fs::rename(&tmp, dir.join(name)).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            PrepareError::disk(e.to_string())
        })?;
    }
    progress(PrepareProgress { percent: 100 });
    Ok(())
}

/// Достаёт нужные файлы из архива по имени (папки внутри игнорируем) и сверяет хэши.
fn unpack(spec: &PinnedEngine, archive: &[u8]) -> Result<HashMap<String, Vec<u8>>, PrepareError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))
        .map_err(|e| PrepareError::integrity(format!("архив не открывается: {e}")))?;
    let mut found: HashMap<String, Vec<u8>> = HashMap::new();
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|e| PrepareError::integrity(e.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let Some(base) = entry.name().rsplit(['/', '\\']).next().map(str::to_owned) else {
            continue;
        };
        let Some((name, _)) = spec.files.iter().find(|(n, _)| *n == base) else {
            continue;
        };
        if entry.size() > MAX_ARCHIVE_BYTES as u64 {
            return Err(PrepareError::integrity("файл в архиве слишком большой"));
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| PrepareError::integrity(e.to_string()))?;
        found.insert((*name).to_owned(), bytes);
    }
    for (name, hash) in spec.files {
        let bytes = found
            .get(*name)
            .ok_or_else(|| PrepareError::integrity(format!("в архиве нет {name}")))?;
        if sha256_hex(bytes) != *hash {
            return Err(PrepareError::integrity(format!("SHA-256 {name} не совпал")));
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for (name, data) in files {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }
            writer.finish().unwrap();
        }
        out
    }

    struct Fake {
        body: Result<Vec<u8>, PrepareError>,
    }

    impl Downloader for Fake {
        fn download(
            &self,
            url: &str,
            _: usize,
            on_progress: &mut dyn FnMut(u64, Option<u64>),
        ) -> Result<Vec<u8>, PrepareError> {
            assert!(download_url_allowed(url));
            let body = self.body.clone()?;
            let total = body.len() as u64;
            on_progress(total / 2, Some(total));
            on_progress(total, Some(total));
            Ok(body)
        }
    }

    fn spec_for(files: &[(&str, &[u8])], archive: &[u8]) -> PinnedEngine {
        let list: Vec<(&'static str, &'static str)> = files
            .iter()
            .map(|(n, d)| {
                (
                    &*Box::leak(n.to_string().into_boxed_str()),
                    &*Box::leak(sha256_hex(d).into_boxed_str()),
                )
            })
            .collect();
        PinnedEngine {
            version: "t",
            url: PINNED.url,
            archive_sha256: Box::leak(sha256_hex(archive).into_boxed_str()),
            files: Box::leak(list.into_boxed_slice()),
        }
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ochistka-engine-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn pinned_values_match_the_fetch_scripts() {
        let script = include_str!("../../../../tools/vpn-core/fetch.sh");
        assert!(script.contains(&format!("VERSION=\"{}\"", PINNED.version)));
        assert!(script.contains(PINNED.archive_sha256));
        for (_, hash) in PINNED.files {
            assert!(script.contains(hash));
        }
        assert!(download_url_allowed(PINNED.url));
        assert!(PINNED.url.contains(PINNED.version));
    }

    #[test]
    fn only_github_release_urls_are_accepted() {
        assert!(download_url_allowed(
            "https://github.com/SagerNet/sing-box/releases/download/v1/x.zip"
        ));
        for bad in [
            "http://github.com/SagerNet/sing-box/releases/download/v1/x.zip",
            "https://evil.com/SagerNet/sing-box/releases/download/v1/x.zip",
            "https://github.com.evil.com/SagerNet/sing-box/releases/download/v1/x.zip",
            "https://github.com/other/repo/releases/download/v1/x.zip",
            "https://user@github.com/SagerNet/sing-box/releases/download/v1/x.zip",
            "https://github.com:444/SagerNet/sing-box/releases/download/v1/x.zip",
            "not a url",
        ] {
            assert!(!download_url_allowed(bad), "{bad}");
        }
    }

    #[test]
    fn redirects_stay_on_github_over_https() {
        assert!(redirect_allowed(
            "https://release-assets.githubusercontent.com/a"
        ));
        assert!(redirect_allowed("https://objects.githubusercontent.com/a"));
        assert!(redirect_allowed("https://github.com/a"));
        assert!(!redirect_allowed("http://objects.githubusercontent.com/a"));
        assert!(!redirect_allowed(
            "https://githubusercontent.com.evil.com/a"
        ));
        assert!(!redirect_allowed("https://evil.com/a"));
    }

    #[test]
    fn installs_verified_files_and_reports_progress() {
        let files: [(&str, &[u8]); 2] = [("sing-box.exe", b"exe"), ("libcronet.dll", b"dll")];
        let archive = make_zip(&[
            ("sing-box-1/sing-box.exe", b"exe"),
            ("sing-box-1/libcronet.dll", b"dll"),
            ("sing-box-1/readme.txt", b"ignored"),
        ]);
        let spec = spec_for(&files, &archive);
        let dir = temp("ok");
        let mut seen = Vec::new();
        install(&spec, &dir, &Fake { body: Ok(archive) }, &mut |p| {
            seen.push(p.percent)
        })
        .unwrap();
        assert_eq!(std::fs::read(dir.join("sing-box.exe")).unwrap(), b"exe");
        assert!(!dir.join("readme.txt").exists());
        assert!(is_installed(&spec, &dir));
        assert_eq!(seen.last(), Some(&100));
        assert!(seen.windows(2).all(|w| w[0] <= w[1]));
        // Повторный вызов ничего не качает.
        install(
            &spec,
            &dir,
            &Fake {
                body: Err(PrepareError::network("не должно вызываться")),
            },
            &mut |_| {},
        )
        .unwrap();
    }

    #[test]
    fn wrong_archive_hash_writes_nothing() {
        let files: [(&str, &[u8]); 1] = [("sing-box.exe", b"exe")];
        let archive = make_zip(&[("sing-box.exe", b"exe")]);
        let spec = spec_for(&files, &archive);
        let dir = temp("badhash");
        let tampered = make_zip(&[("sing-box.exe", b"evil")]);
        let err = install(&spec, &dir, &Fake { body: Ok(tampered) }, &mut |_| {}).unwrap_err();
        assert!(err.reason.contains("не прошёл проверку"));
        assert!(!dir.exists());
    }

    #[test]
    fn wrong_file_hash_inside_archive_writes_nothing() {
        let files: [(&str, &[u8]); 1] = [("sing-box.exe", b"exe")];
        let archive = make_zip(&[("sing-box.exe", b"evil")]);
        let mut spec = spec_for(&files, &archive);
        spec.archive_sha256 = Box::leak(sha256_hex(&archive).into_boxed_str());
        let dir = temp("badfile");
        let err = install(&spec, &dir, &Fake { body: Ok(archive) }, &mut |_| {}).unwrap_err();
        assert!(err.detail.contains("sing-box.exe"));
        assert!(!dir.exists());
    }

    #[test]
    fn missing_file_and_network_error_are_reported() {
        let files: [(&str, &[u8]); 1] = [("sing-box.exe", b"exe")];
        let archive = make_zip(&[("other.exe", b"exe")]);
        let spec = spec_for(&files, &archive);
        let dir = temp("missing");
        let err = install(&spec, &dir, &Fake { body: Ok(archive) }, &mut |_| {}).unwrap_err();
        assert!(err.detail.contains("нет sing-box.exe"));
        let err = install(
            &spec,
            &dir,
            &Fake {
                body: Err(PrepareError::network("offline")),
            },
            &mut |_| {},
        )
        .unwrap_err();
        assert_eq!(err.reason, "проверьте интернет");
    }
}
