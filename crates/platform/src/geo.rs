//! Определение провайдера и расположения: запрос к внешнему сервису и файл с согласием.
//! Запрос только https, без cookie и идентификаторов, без переходов по редиректам; тело ответа
//! читается с ограничением по размеру. Windows: reqwest; на других ОС запрос не выполняется.

use std::path::{Path, PathBuf};

use ochistka_core::netinfo::{ConsentStore, GeoError, GeoTransport};

/// Файл настроек: `%LOCALAPPDATA%\OCHISTKA\settings.json`, иначе временная папка.
pub fn settings_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OCHISTKA")
        .join("settings.json")
}

/// Согласие на внешний запрос хранится ключом `geoConsent` в файле настроек; другие ключи
/// сохраняются как есть.
pub struct FileConsent {
    path: PathBuf,
}

impl FileConsent {
    pub fn new(path: &Path) -> Self {
        Self { path: path.into() }
    }

    fn read(&self) -> serde_json::Map<String, serde_json::Value> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    }
}

impl ConsentStore for FileConsent {
    fn granted(&self) -> bool {
        self.read().get("geoConsent").and_then(|v| v.as_bool()) == Some(true)
    }

    fn set(&self, granted: bool) -> Result<(), GeoError> {
        let mut map = self.read();
        map.insert("geoConsent".into(), serde_json::Value::Bool(granted));
        let text = serde_json::to_string_pretty(&map).map_err(|_| GeoError::Storage)?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|_| GeoError::Storage)?;
        }
        // Пишем во временный файл и переименовываем: настройки не остаются наполовину записанными.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|_| GeoError::Storage)?;
        std::fs::rename(&tmp, &self.path).map_err(|_| GeoError::Storage)
    }
}

/// Транспорт текущей ОС.
pub fn current_geo_transport() -> Box<dyn GeoTransport> {
    #[cfg(windows)]
    {
        Box::new(http::ReqwestGeo::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(Unsupported)
    }
}

#[cfg(not(windows))]
struct Unsupported;

#[cfg(not(windows))]
impl GeoTransport for Unsupported {
    fn get(&self, _: &str) -> Result<String, GeoError> {
        Err(GeoError::Server)
    }
}

#[cfg(windows)]
mod http {
    use std::io::Read;
    use std::time::Duration;

    use ochistka_core::netinfo::{GeoError, GeoTransport};
    use reqwest::blocking::Client;

    /// Ответ сервиса меньше килобайта; всё сверх 16 КБ отбрасываем.
    const MAX_BODY: u64 = 16 * 1024;

    pub struct ReqwestGeo {
        client: Option<Client>,
    }

    impl ReqwestGeo {
        pub fn new() -> Self {
            let client = Client::builder()
                .user_agent("OCHISTKA")
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(6))
                .timeout(Duration::from_secs(10))
                .build()
                .ok();
            Self { client }
        }
    }

    impl GeoTransport for ReqwestGeo {
        fn get(&self, url: &str) -> Result<String, GeoError> {
            let response = self
                .client
                .as_ref()
                .ok_or(GeoError::Server)?
                .get(url)
                .send()
                .map_err(|e| {
                    if e.is_connect() {
                        GeoError::Offline
                    } else {
                        GeoError::Server
                    }
                })?;
            if !response.status().is_success() {
                return Err(GeoError::Server);
            }
            let mut body = String::new();
            response
                .take(MAX_BODY)
                .read_to_string(&mut body)
                .map_err(|_| GeoError::Unreadable)?;
            Ok(body)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ochistka-geo-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("settings.json")
    }

    #[test]
    fn consent_is_off_by_default_remembered_and_revocable() {
        let path = temp("a");
        let store = FileConsent::new(&path);
        assert!(!store.granted());
        store.set(true).unwrap();
        assert!(
            FileConsent::new(&path).granted(),
            "запоминается между запусками"
        );
        store.set(false).unwrap();
        assert!(!FileConsent::new(&path).granted());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn other_settings_survive_and_broken_file_means_no_consent() {
        let path = temp("b");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"other":1}"#).unwrap();
        let store = FileConsent::new(&path);
        store.set(true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"other\"") && text.contains("geoConsent"));
        std::fs::write(&path, "{ broken").unwrap();
        assert!(!store.granted());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
