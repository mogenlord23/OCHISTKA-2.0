//! HTTPS-транспорт проверки доступности сервисов (Windows). Без системного и переменного
//! прокси, с проверкой TLS, коротким временем ожидания и ограничением размера ответа.
//! Сам по себе ничего не запускает: вызывается только явной командой.

use ochistka_core::desync::probe::{ProbeTransport, Reply};

const TIMEOUT_SECS: u64 = 8;
const MAX_BODY: u64 = 64 * 1024;

/// Определяет вид ошибки по признакам и тексту цепочки причин.
pub(crate) fn classify(is_timeout: bool, is_connect: bool, chain: &str) -> Reply {
    let text = chain.to_lowercase();
    if is_timeout || text.contains("timed out") {
        Reply::Timeout
    } else if text.contains("dns") || text.contains("lookup address") || text.contains("resolve") {
        Reply::Dns
    } else if text.contains("certificate")
        || text.contains("tls")
        || text.contains("ssl")
        || text.contains("handshake")
        || text.contains("schannel")
    {
        Reply::Tls
    } else if is_connect || text.contains("reset") || text.contains("refused") {
        Reply::Refused
    } else {
        Reply::Other("неопределённая ошибка запроса".into())
    }
}

#[cfg(windows)]
pub struct HttpsProbe;

#[cfg(windows)]
impl ProbeTransport for HttpsProbe {
    fn get(&self, url: &str) -> Reply {
        use std::error::Error;
        use std::io::Read;
        use std::time::Duration;
        if !url.starts_with("https://") {
            return Reply::Other("разрешён только https".into());
        }
        let client = match reqwest::blocking::Client::builder()
            .no_proxy()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .user_agent("OCHISTKA")
            .build()
        {
            Ok(c) => c,
            Err(_) => return Reply::Other("не удалось создать клиент".into()),
        };
        match client.get(url).send() {
            Ok(response) => {
                let status = response.status().as_u16();
                // Тело нужно не больше, чем чтобы убедиться, что соединение живое.
                let mut sink = Vec::new();
                let _ = response.take(MAX_BODY).read_to_end(&mut sink);
                Reply::Answered(status)
            }
            Err(err) => {
                let mut chain = err.to_string();
                let mut source = err.source();
                while let Some(s) = source {
                    chain.push(' ');
                    chain.push_str(&s.to_string());
                    source = s.source();
                }
                classify(err.is_timeout(), err.is_connect(), &chain)
            }
        }
    }
}

/// На ОС без реализации ничего не проверяется: всё остаётся «не проверено».
#[cfg(not(windows))]
pub struct HttpsProbe;

#[cfg(not(windows))]
impl ProbeTransport for HttpsProbe {
    fn get(&self, _url: &str) -> Reply {
        let _ = (TIMEOUT_SECS, MAX_BODY);
        Reply::Other("проверка доступности пока не поддерживается на этой ОС".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_classified_without_calling_a_block() {
        assert_eq!(classify(true, false, "x"), Reply::Timeout);
        assert_eq!(
            classify(false, true, "dns error: failed to lookup address"),
            Reply::Dns
        );
        assert_eq!(
            classify(false, true, "invalid peer certificate: UnknownIssuer"),
            Reply::Tls
        );
        assert_eq!(
            classify(false, true, "tcp connect error: connection refused"),
            Reply::Refused
        );
        assert!(matches!(classify(false, false, "weird"), Reply::Other(_)));
    }

    #[test]
    fn unsupported_os_never_answers() {
        #[cfg(not(windows))]
        assert!(matches!(
            HttpsProbe.get("https://example.com/"),
            Reply::Other(_)
        ));
    }
}
