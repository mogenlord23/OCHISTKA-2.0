//! Провайдер и расположение по внешнему IP. Это единственная функция, которой нужен внешний
//! сервис определения IP, поэтому запрос уходит только после явного согласия пользователя
//! (кнопка «Определить»). Согласие запоминается в настройках и отзывается. В запросе нет ни
//! идентификаторов, ни cookie: сервис видит только обычный внешний адрес. Если включён VPN,
//! сервис увидит адрес VPN-сервера, а не домашний (об этом сказано в интерфейсе).
//! Сам запрос делает адаптер платформы через порт `GeoTransport`; здесь логика и разбор ответа.

use serde::Serialize;
use ts_rs::TS;

/// Адрес сервиса (https, поля ограничены тем, что показываем).
pub const GEO_URL: &str = "https://ipwho.is/?fields=success,country,city,connection";

/// Самый длинный показываемый текст: защита от мусорного ответа.
const MAX_TEXT: usize = 80;

/// Что определили. `None` — не определено, интерфейс покажет «Не определено».
#[derive(Serialize, TS, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "GeoInfo.ts")]
pub struct GeoInfo {
    pub provider: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
}

/// Состояние согласия для интерфейса.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "GeoConsent.ts")]
pub struct GeoConsent {
    pub granted: bool,
}

/// Почему определить не вышло.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeoError {
    /// Нет согласия: запрос не отправлялся.
    ConsentRequired,
    /// Нет соединения с интернетом.
    Offline,
    /// Сервис не ответил или отказал.
    Server,
    /// Ответ не удалось прочитать.
    Unreadable,
    /// Не удалось сохранить настройку.
    Storage,
}

impl GeoError {
    pub fn kind(self) -> &'static str {
        match self {
            Self::ConsentRequired => "consentRequired",
            Self::Offline => "offline",
            Self::Server => "server",
            Self::Unreadable => "unreadable",
            Self::Storage => "storage",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::ConsentRequired => "Сначала нужно разрешение на запрос",
            Self::Offline => "Нет подключения к интернету",
            Self::Server => "Сервис не ответил. Попробуй позже",
            Self::Unreadable => "Не удалось разобрать ответ",
            Self::Storage => "Не удалось сохранить настройку",
        }
    }
}

/// Один https-запрос GET; возвращает тело ответа (ограниченное по размеру).
pub trait GeoTransport {
    fn get(&self, url: &str) -> Result<String, GeoError>;
}

/// Хранилище согласия (файл настроек).
pub trait ConsentStore {
    fn granted(&self) -> bool;
    fn set(&self, granted: bool) -> Result<(), GeoError>;
}

fn clean(value: Option<&serde_json::Value>) -> Option<String> {
    let text: String = value?
        .as_str()?
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.chars().take(MAX_TEXT).collect())
    }
}

/// Разбор ответа сервиса. Лишние поля игнорируются; чего нет, то «не определено».
pub fn parse_geo(body: &str) -> Result<GeoInfo, GeoError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|_| GeoError::Unreadable)?;
    if value.get("success").and_then(|v| v.as_bool()) == Some(false) {
        return Err(GeoError::Server);
    }
    if !value.is_object() {
        return Err(GeoError::Unreadable);
    }
    let connection = value.get("connection");
    let provider = clean(connection.and_then(|c| c.get("isp")))
        .or_else(|| clean(connection.and_then(|c| c.get("org"))));
    Ok(GeoInfo {
        provider,
        country: clean(value.get("country")),
        city: clean(value.get("city")),
    })
}

/// Определяет провайдера и расположение. Без согласия транспорт не вызывается вообще.
pub fn lookup(store: &dyn ConsentStore, transport: &dyn GeoTransport) -> Result<GeoInfo, GeoError> {
    if !store.granted() {
        return Err(GeoError::ConsentRequired);
    }
    parse_geo(&transport.get(GEO_URL)?)
}

/// Выдать или отозвать согласие.
pub fn set_consent(store: &dyn ConsentStore, granted: bool) -> Result<GeoConsent, GeoError> {
    store.set(granted)?;
    Ok(GeoConsent { granted })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct Fake {
        calls: RefCell<Vec<String>>,
        reply: Result<String, GeoError>,
    }

    impl Fake {
        fn ok(body: &str) -> Self {
            Self {
                calls: RefCell::default(),
                reply: Ok(body.into()),
            }
        }
    }

    impl GeoTransport for Fake {
        fn get(&self, url: &str) -> Result<String, GeoError> {
            self.calls.borrow_mut().push(url.into());
            self.reply.clone()
        }
    }

    #[derive(Default)]
    struct Store(Cell<bool>);

    impl ConsentStore for Store {
        fn granted(&self) -> bool {
            self.0.get()
        }
        fn set(&self, granted: bool) -> Result<(), GeoError> {
            self.0.set(granted);
            Ok(())
        }
    }

    const BODY: &str = r#"{"success":true,"country":"Latvia","city":"Riga","connection":{"isp":"Example ISP","org":"Example Org"},"ip":"203.0.113.9"}"#;

    #[test]
    fn without_consent_nothing_is_sent() {
        let transport = Fake::ok(BODY);
        let store = Store::default();
        assert_eq!(lookup(&store, &transport), Err(GeoError::ConsentRequired));
        assert!(transport.calls.borrow().is_empty());
    }

    #[test]
    fn with_consent_one_https_request_without_identifiers() {
        let transport = Fake::ok(BODY);
        let store = Store::default();
        set_consent(&store, true).unwrap();
        let info = lookup(&store, &transport).unwrap();
        assert_eq!(info.provider.as_deref(), Some("Example ISP"));
        assert_eq!(info.country.as_deref(), Some("Latvia"));
        assert_eq!(info.city.as_deref(), Some("Riga"));
        let calls = transport.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].starts_with("https://"));
        // Адрес запроса не содержит ничего, кроме списка нужных полей.
        assert_eq!(calls[0], GEO_URL);
        assert!(!calls[0].contains("id=") && !calls[0].contains("key="));
    }

    #[test]
    fn revoked_consent_stops_requests_again() {
        let transport = Fake::ok(BODY);
        let store = Store::default();
        set_consent(&store, true).unwrap();
        lookup(&store, &transport).unwrap();
        assert_eq!(
            set_consent(&store, false),
            Ok(GeoConsent { granted: false })
        );
        assert_eq!(lookup(&store, &transport), Err(GeoError::ConsentRequired));
        assert_eq!(transport.calls.borrow().len(), 1);
    }

    #[test]
    fn missing_fields_stay_undetermined_not_invented() {
        let info = parse_geo(r#"{"success":true,"country":"","city":null}"#).unwrap();
        assert_eq!(info, GeoInfo::default());
        let info = parse_geo(r#"{"success":true,"connection":{"org":"Only Org"}}"#).unwrap();
        assert_eq!(info.provider.as_deref(), Some("Only Org"));
        assert_eq!(info.country, None);
    }

    #[test]
    fn bad_replies_are_errors() {
        assert_eq!(parse_geo("not json"), Err(GeoError::Unreadable));
        assert_eq!(parse_geo("[1,2]"), Err(GeoError::Unreadable));
        assert_eq!(
            parse_geo(r#"{"success":false,"message":"limit"}"#),
            Err(GeoError::Server)
        );
    }

    #[test]
    fn transport_errors_pass_through() {
        let store = Store::default();
        store.0.set(true);
        for err in [GeoError::Offline, GeoError::Server] {
            let transport = Fake {
                calls: RefCell::default(),
                reply: Err(err),
            };
            assert_eq!(lookup(&store, &transport), Err(err));
        }
    }

    #[test]
    fn text_is_cleaned_and_limited() {
        let long = "я".repeat(200);
        let body = format!(r#"{{"country":"  Lat\u0000via\n ","city":"{long}"}}"#);
        let info = parse_geo(&body).unwrap();
        assert_eq!(info.country.as_deref(), Some("Latvia"));
        assert_eq!(
            info.city.as_ref().map(|c| c.chars().count()),
            Some(MAX_TEXT)
        );
    }

    #[test]
    fn service_url_is_https() {
        assert!(GEO_URL.starts_with("https://"));
    }
}
