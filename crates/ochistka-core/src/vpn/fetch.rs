//! Загрузка подписки. Адрес только `https://`; сам запрос делает адаптер платформы (порт
//! `HttpFetch`: таймаут, лимит размера, переходы только на https). Интерфейс сеть не трогает.

use super::mask::mask_secrets;
use super::parse::{parse_subscription, percent_decode, ImportReport, MAX_BODY};
use super::VpnError;

/// Ответ сервера подписки.
pub struct FetchResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Порт загрузки. Ошибка — текст причины на русском (нет сети, таймаут, слишком большой ответ).
pub trait HttpFetch: Send + Sync {
    fn get(&self, url: &str, user_agent: &str, max_bytes: usize) -> Result<FetchResponse, String>;
}

/// Имена клиента. Сначала общее имя приложения, затем известные клиенты по очереди (как в рабочей
/// версии 0c46358): часть провайдеров отдаёт список серверов только знакомым клиентам. Следующее
/// имя пробуется после отказа 403, 404 или 410 на любом шаге. Личных данных не отправляем.
pub const USER_AGENTS: [&str; 3] = [
    "OCHISTKA/1.0",
    "v2rayN/7.0 OCHISTKA",
    "v2rayTun/1.0 OCHISTKA",
];

pub const LINK_GONE: &str = "Ссылка больше не действует. Обновите её в личном кабинете сервиса";

/// Короткий текст по коду ответа сервиса подписки. «Ссылка больше не действует» — только для явных
/// 404 и 410; в остальных случаях называем номер ответа.
pub fn status_message(status: u16) -> String {
    match status {
        404 | 410 => LINK_GONE.into(),
        401 | 403 => format!("Провайдер не отдал подписку (код {status}). Попробуйте позже"),
        429 => format!("Провайдер просит подождать (код {status}). Повторите импорт чуть позже"),
        500..=599 => format!("Сервис подписки недоступен (код {status}). Повторите чуть позже"),
        _ => {
            format!("Провайдер ответил ошибкой (код {status}). Проверьте ссылку в личном кабинете")
        }
    }
}

/// Что ввёл пользователь.
#[derive(Debug, PartialEq, Eq)]
pub enum Input {
    Url(String),
    Text(String),
}

/// Проверяет, что строка — ссылка `https://` с хостом. `http://` и прочие схемы отклоняются.
pub fn validate_https_url(text: &str) -> Result<String, VpnError> {
    let t = text.trim();
    let lower = t.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("https://") else {
        return Err(VpnError::NotHttps);
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit('@').next().unwrap_or_default();
    if host.is_empty() || host.starts_with(':') || t.chars().any(char::is_whitespace) {
        return Err(VpnError::Fetch("Некорректная ссылка".into()));
    }
    Ok(t.to_string())
}

const WRAPPERS: &[&str] = &[
    "v2raytun://",
    "v2rayng://",
    "hiddify://",
    "happ://",
    "sing-box://",
];

/// Определяет, ссылка это или текст подписки. Обёртки приложений (`v2raytun://import/https://…`)
/// раскрываются до адреса https.
pub fn classify_input(input: &str) -> Result<Input, VpnError> {
    let t = input.trim().trim_matches('\u{feff}');
    if t.is_empty() {
        return Err(VpnError::EmptyInput);
    }
    let lower = t.to_ascii_lowercase();
    if WRAPPERS.iter().any(|w| lower.starts_with(w)) {
        return unwrap_link(t).map(Input::Url);
    }
    let single = !t.chars().any(char::is_whitespace);
    if single && (lower.starts_with("https://") || lower.starts_with("http://")) {
        return validate_https_url(t).map(Input::Url);
    }
    Ok(Input::Text(t.to_string()))
}

fn unwrap_link(link: &str) -> Result<String, VpnError> {
    let lower = link.to_ascii_lowercase();
    for key in ["url=", "config=", "subscription="] {
        if let Some(pos) = lower
            .find(&format!("?{key}"))
            .or_else(|| lower.find(&format!("&{key}")))
        {
            let value = link[pos + 1 + key.len()..]
                .split('&')
                .next()
                .unwrap_or_default();
            let decoded = percent_decode(value);
            if decoded.to_ascii_lowercase().starts_with("http") {
                return validate_https_url(&decoded);
            }
        }
    }
    for marker in ["https://", "https%3a%2f%2f"] {
        if let Some(pos) = lower.find(marker) {
            let tail = link[pos..].split(['#', ' ']).next().unwrap_or_default();
            return if marker == "https://" {
                validate_https_url(tail)
            } else {
                validate_https_url(&percent_decode(tail))
            };
        }
    }
    Err(VpnError::NotHttps)
}

/// Скачивает и разбирает подписку по ссылке https.
pub fn import_from_url(fetcher: &dyn HttpFetch, url: &str) -> Result<ImportReport, VpnError> {
    let url = validate_https_url(url)?;
    let mut last = VpnError::Fetch("Подписка не загрузилась".into());
    for agent in USER_AGENTS {
        match fetcher.get(&url, agent, MAX_BODY) {
            Err(why) => {
                last = VpnError::Fetch(format!(
                    "Не удалось связаться с сервисом подписки: {}",
                    mask_secrets(&why)
                ));
                // Нет сети или таймаут: второй запрос не поможет.
                return Err(last);
            }
            Ok(r) if (200..300).contains(&r.status) => {
                if r.body.len() > MAX_BODY {
                    return Err(VpnError::Parse("Подписка слишком большая".into()));
                }
                let text = String::from_utf8_lossy(&r.body);
                match parse_subscription(&text) {
                    Ok(report) => return Ok(report),
                    // Пустой ответ ссылки — не «пустой ввод» пользователя.
                    Err(VpnError::EmptyInput) => last = VpnError::Parse(
                        "Сервис вернул пустой ответ. Проверьте ссылку в личном кабинете сервиса"
                            .into(),
                    ),
                    Err(err) => last = err,
                }
            }
            Ok(r) => {
                last = VpnError::Fetch(status_message(r.status));
                // Запасное имя клиента — только после 403, 404 или 410; при прочих кодах сразу итог.
                if !matches!(r.status, 403 | 404 | 410) {
                    return Err(last);
                }
            }
        }
    }
    Err(last)
}

/// Общий вход импорта: ссылка (скачиваем) или вставленный текст/файл (разбираем).
pub fn import_input(fetcher: &dyn HttpFetch, input: &str) -> Result<ImportReport, VpnError> {
    match classify_input(input)? {
        Input::Url(url) => import_from_url(fetcher, &url),
        Input::Text(text) => parse_subscription(&text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Fake {
        replies: Mutex<Vec<Result<FetchResponse, String>>>,
        agents: Mutex<Vec<String>>,
    }

    impl Fake {
        fn new(replies: Vec<Result<FetchResponse, String>>) -> Self {
            Self {
                replies: Mutex::new(replies),
                agents: Mutex::new(vec![]),
            }
        }
    }

    impl HttpFetch for Fake {
        fn get(&self, _: &str, agent: &str, _: usize) -> Result<FetchResponse, String> {
            self.agents.lock().unwrap().push(agent.to_string());
            // Последний ответ повторяется: провайдер отвечает одинаково на все имена клиента.
            let mut replies = self.replies.lock().unwrap();
            if replies.len() > 1 {
                return replies.remove(0);
            }
            match &replies[0] {
                Ok(r) => Ok(FetchResponse {
                    status: r.status,
                    body: r.body.clone(),
                }),
                Err(e) => Err(e.clone()),
            }
        }
    }

    fn ok(body: &str) -> Result<FetchResponse, String> {
        Ok(FetchResponse {
            status: 200,
            body: body.as_bytes().to_vec(),
        })
    }

    const LINK: &str = "trojan://p@b.example.com:443#B";

    #[test]
    fn only_https_is_accepted() {
        assert!(validate_https_url("https://a.example/sub?token=1").is_ok());
        assert_eq!(
            validate_https_url("http://a.example/sub"),
            Err(VpnError::NotHttps)
        );
        assert_eq!(
            validate_https_url("ftp://a.example"),
            Err(VpnError::NotHttps)
        );
        assert_eq!(validate_https_url("a.example/sub"), Err(VpnError::NotHttps));
        assert!(validate_https_url("https://").is_err());
        assert!(validate_https_url("https:///path").is_err());
        assert!(validate_https_url("https://a b.example").is_err());
    }

    #[test]
    fn http_is_refused_before_any_request() {
        let fake = Fake::new(vec![]);
        assert_eq!(
            import_input(&fake, "http://a.example/sub").unwrap_err(),
            VpnError::NotHttps
        );
        assert!(fake.agents.lock().unwrap().is_empty());
    }

    #[test]
    fn classify_pasted_text_and_wrappers() {
        assert!(matches!(
            classify_input("vless://x@h:1#a vless://y@h:2#b"),
            Ok(Input::Text(_))
        ));
        let signed = "https://example.invalid/%2F?key=a%26b%3D";
        assert_eq!(
            classify_input(&format!("v2raytun://import/{signed}")).unwrap(),
            Input::Url(signed.into())
        );
        assert_eq!(
            classify_input("v2raytun://import/https://s.example/sub#Name").unwrap(),
            Input::Url("https://s.example/sub".into())
        );
        assert_eq!(
            classify_input("happ://add?url=https%3A%2F%2Fs.example%2Fsub%3Ft%3D1&x=2").unwrap(),
            Input::Url("https://s.example/sub?t=1".into())
        );
        assert_eq!(
            classify_input("hiddify://import/http://s.example/x"),
            Err(VpnError::NotHttps)
        );
        assert_eq!(classify_input("  \n"), Err(VpnError::EmptyInput));
    }

    #[test]
    fn fetch_parses_base64_body() {
        let fake = Fake::new(vec![ok("dHJvamFuOi8vcEBiLmV4YW1wbGUuY29tOjQ0MyNC")]);
        let r = import_input(&fake, "https://s.example/sub").unwrap();
        assert_eq!(r.servers.len(), 1);
        assert_eq!(fake.agents.lock().unwrap().as_slice(), ["OCHISTKA/1.0"]);
    }

    #[test]
    fn retries_with_fallback_agent_on_403() {
        let fake = Fake::new(vec![
            Ok(FetchResponse {
                status: 403,
                body: vec![],
            }),
            ok(LINK),
        ]);
        assert_eq!(
            import_input(&fake, "https://s.example/sub")
                .unwrap()
                .servers
                .len(),
            1
        );
        assert_eq!(fake.agents.lock().unwrap().len(), 2);
    }

    #[test]
    fn network_failure_is_reported_in_russian_without_retry() {
        let fake = Fake::new(vec![Err("время ожидания истекло".into())]);
        let err = import_input(&fake, "https://s.example/sub")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Не удалось связаться") && err.contains("время ожидания"));
        assert_eq!(fake.agents.lock().unwrap().len(), 1);
    }

    #[test]
    fn gone_link_gives_plain_message_without_code() {
        let gone = || {
            Ok(FetchResponse {
                status: 410,
                body: vec![],
            })
        };
        let fake = Fake::new(vec![gone(), gone()]);
        let text = import_input(&fake, "https://s.example/sub")
            .unwrap_err()
            .to_string();
        assert_eq!(text, LINK_GONE);
        assert!(!text.contains("410") && !text.contains("HTTP"));
        // Вся цепочка имён клиента пройдена.
        assert_eq!(fake.agents.lock().unwrap().len(), USER_AGENTS.len());
    }

    #[test]
    fn an_unknown_code_is_named_and_not_retried() {
        let fake = Fake::new(vec![Ok(FetchResponse {
            status: 418,
            body: vec![],
        })]);
        let text = import_input(&fake, "https://s.example/sub")
            .unwrap_err()
            .to_string();
        assert!(text.contains("код 418"), "{text}");
        assert!(!text.contains("больше не действует"));
        assert_eq!(fake.agents.lock().unwrap().len(), 1);
    }

    #[test]
    fn the_first_attempt_is_generic_and_the_fallback_is_the_known_client() {
        assert_eq!(USER_AGENTS[0], "OCHISTKA/1.0");
        assert!(USER_AGENTS[1].starts_with("v2rayN"));
    }

    #[test]
    fn a_403_404_or_410_on_the_first_attempt_is_rescued_by_the_known_client() {
        for status in [403u16, 404, 410] {
            let fake = Fake::new(vec![
                Ok(FetchResponse {
                    status,
                    body: vec![],
                }),
                ok(LINK),
            ]);
            let report = import_from_url(&fake, "https://s.example/sub/TOKEN123").unwrap();
            assert_eq!(report.servers.len(), 1, "{status}");
            let agents = fake.agents.lock().unwrap().clone();
            assert_eq!(agents, vec![USER_AGENTS[0], USER_AGENTS[1]], "{status}");
        }
    }

    #[test]
    fn both_attempts_failing_show_the_code_of_the_last_answer_without_secrets() {
        let reply = |status| {
            Ok(FetchResponse {
                status,
                body: vec![],
            })
        };
        let fake = Fake::new(vec![reply(403), reply(410)]);
        let text = import_from_url(&fake, "https://s.example/sub/TOKEN123?k=v")
            .unwrap_err()
            .to_string();
        assert!(text.contains("больше не действует"), "{text}");
        let fake = Fake::new(vec![reply(404), reply(403)]);
        let text = import_from_url(&fake, "https://s.example/sub/TOKEN123?k=v")
            .unwrap_err()
            .to_string();
        assert!(text.contains("код 403"), "{text}");
        for secret in ["TOKEN123", "k=v", "s.example"] {
            assert!(!text.contains(secret), "{text}");
        }
        // Прочие коды повтора не вызывают.
        let fake = Fake::new(vec![reply(429)]);
        assert!(import_from_url(&fake, "https://s.example/sub").is_err());
        assert_eq!(fake.agents.lock().unwrap().len(), 1);
    }

    #[test]
    fn the_chain_is_generic_then_v2rayn_then_v2raytun() {
        assert_eq!(
            USER_AGENTS,
            [
                "OCHISTKA/1.0",
                "v2rayN/7.0 OCHISTKA",
                "v2rayTun/1.0 OCHISTKA"
            ]
        );
    }

    #[test]
    fn a_404_on_the_first_two_names_is_rescued_by_the_third() {
        let not_found = || {
            Ok(FetchResponse {
                status: 404,
                body: vec![],
            })
        };
        let fake = Fake::new(vec![not_found(), not_found(), ok(LINK)]);
        let report = import_from_url(&fake, "https://s.example/sub/TOKEN123").unwrap();
        assert_eq!(report.servers.len(), 1);
        assert_eq!(
            *fake.agents.lock().unwrap(),
            vec![USER_AGENTS[0], USER_AGENTS[1], USER_AGENTS[2]]
        );
    }

    #[test]
    fn three_404_answers_say_the_link_is_gone_without_secrets() {
        let fake = Fake::new(vec![Ok(FetchResponse {
            status: 404,
            body: vec![],
        })]);
        let text = import_from_url(&fake, "https://sub.example.com/s/TOKEN123?k=v")
            .unwrap_err()
            .to_string();
        assert!(text.contains("больше не действует"), "{text}");
        assert_eq!(fake.agents.lock().unwrap().len(), 3);
        for secret in ["TOKEN123", "k=v", "sub.example.com"] {
            assert!(!text.contains(secret), "{text}");
        }
    }

    #[test]
    fn a_429_or_5xx_on_any_step_ends_the_chain_at_once() {
        let reply = |status| {
            Ok(FetchResponse {
                status,
                body: vec![],
            })
        };
        let fake = Fake::new(vec![reply(404), reply(503)]);
        let text = import_from_url(&fake, "https://s.example/sub")
            .unwrap_err()
            .to_string();
        assert!(text.contains("код 503"), "{text}");
        assert_eq!(fake.agents.lock().unwrap().len(), 2);
    }

    #[test]
    fn network_error_text_does_not_leak_the_subscription_token() {
        let fake = Fake::new(vec![Err(
            "error sending request for url (https://s.example/sub/TOKEN123?k=v)".into(),
        )]);
        let text = import_input(&fake, "https://s.example/sub/TOKEN123")
            .unwrap_err()
            .to_string();
        assert!(
            !text.contains("TOKEN123") && !text.contains("k=v"),
            "{text}"
        );
    }

    #[test]
    fn server_error_and_garbage_body() {
        let fake = Fake::new(vec![ok("<html>login</html>"), ok("<html>login</html>")]);
        assert!(import_input(&fake, "https://s.example/sub")
            .unwrap_err()
            .to_string()
            .contains("веб-страницу"));
    }

    #[test]
    fn every_code_has_its_own_text_and_only_404_and_410_say_the_link_is_gone() {
        for (status, needle) in [
            (404, "больше не действует"),
            (410, "больше не действует"),
            (401, "не отдал подписку (код 401)"),
            (403, "не отдал подписку (код 403)"),
            (429, "подождать (код 429)"),
            (500, "недоступен (код 500)"),
            (503, "недоступен (код 503)"),
            (418, "ошибкой (код 418)"),
        ] {
            let reply = || {
                Ok(FetchResponse {
                    status,
                    body: vec![],
                })
            };
            let fake = Fake::new(vec![reply(), reply()]);
            let err = import_from_url(&fake, "https://sub.example.com/s/TOKEN123?k=v")
                .unwrap_err()
                .to_string();
            assert!(err.contains(needle), "{status}: {err}");
            assert_eq!(
                err.contains("больше не действует"),
                matches!(status, 404 | 410),
                "{status}: {err}"
            );
            for secret in [
                "TOKEN123",
                "k=v",
                "sub.example.com",
                "HTTP",
                "токен",
                "агент",
            ] {
                assert!(!err.contains(secret), "{err}");
            }
        }
    }

    #[test]
    fn empty_answer_is_not_reported_as_empty_input() {
        let fake = Fake::new(vec![ok(""), ok("  \n")]);
        let err = import_from_url(&fake, "https://sub.example.com/s/T")
            .unwrap_err()
            .to_string();
        assert!(err.contains("пустой ответ"), "{err}");
    }

    #[test]
    fn second_client_name_rescues_a_format_the_first_one_could_not_use() {
        let clash = "proxies:\n  - name: a\n    type: vless\n";
        let fake = Fake::new(vec![ok(clash), ok(LINK)]);
        let report = import_from_url(&fake, "https://sub.example.com/s/T").unwrap();
        assert_eq!(report.servers.len(), 1);
        assert_eq!(fake.agents.lock().unwrap().len(), 2);
    }
}
