//! Маскирование ссылок в тексте, который попадает в журналы и сообщения. Ссылка подписки
//! содержит личный токен, ссылки серверов содержат пароли: оставляем только схему и хост.

fn is_end(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | ']' | '}' | ',')
}

/// Заменяет в тексте каждую ссылку `схема://…`: у http(s) остаётся хост (без логина и порта
/// не трогаем), всё остальное становится `***`; у прочих схем (vless, trojan…) остаётся схема.
pub fn mask_secrets(text: &str) -> String {
    mask_ids(&mask_links(text))
}

fn mask_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find("://") {
        // Начало схемы: буквы, цифры, + - . перед «://».
        let before = &rest[..pos];
        let scheme_start = before
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
            .last()
            .map_or(pos, |(i, _)| i);
        let scheme = &rest[scheme_start..pos];
        out.push_str(&rest[..scheme_start]);
        let after = &rest[pos + 3..];
        let end = after.find(is_end).unwrap_or(after.len());
        let body = &after[..end];
        let lower = scheme.to_ascii_lowercase();
        if scheme.is_empty() {
            out.push_str("://");
            out.push_str(body);
        } else if lower == "http" || lower == "https" {
            let authority = body.split(['/', '?', '#']).next().unwrap_or_default();
            let host = authority.rsplit('@').next().unwrap_or_default();
            out.push_str(scheme);
            out.push_str("://");
            out.push_str(host);
            if body.len() > authority.len() {
                out.push_str("/***");
            }
        } else {
            out.push_str(scheme);
            out.push_str("://***");
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Скрывает UUID (8-4-4-4-12) и длинные ключеподобные строки (24+ знака из букв, цифр, `+/_=-`,
/// с буквами и цифрами сразу): ключи REALITY, пароли и токены, которые ядро могло напечатать.
fn mask_ids(text: &str) -> String {
    fn is_uuid(w: &str) -> bool {
        let g: Vec<&str> = w.split('-').collect();
        g.len() == 5
            && [8, 4, 4, 4, 12]
                .iter()
                .zip(&g)
                .all(|(n, p)| p.len() == *n && p.chars().all(|c| c.is_ascii_hexdigit()))
    }
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            let letters = word.chars().any(|c| c.is_ascii_alphabetic());
            let digits = word.chars().any(|c| c.is_ascii_digit());
            if is_uuid(word) || (word.len() >= 24 && letters && digits) {
                out.push_str("***");
            } else {
                out.push_str(word);
            }
            word.clear();
        }
    };
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '_' | '=' | '-') {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::mask_secrets;

    #[test]
    fn subscription_token_is_hidden() {
        let m = mask_secrets("error sending request for url (https://sub.example.com/api/sub/SECRETTOKEN?x=1): timeout");
        assert!(!m.contains("SECRETTOKEN") && !m.contains("x=1"));
        assert!(m.contains("https://sub.example.com/***"));
        assert!(m.ends_with("timeout"));
    }

    #[test]
    fn userinfo_and_proxy_links_are_hidden() {
        let m = mask_secrets("a https://user:pw@h.example/p b vless://uuid-123@srv:443?x=y#N c");
        assert!(!m.contains("pw") && !m.contains("uuid-123") && !m.contains("srv"));
        assert!(m.contains("https://h.example/***") && m.contains("vless://***"));
        assert!(m.starts_with("a ") && m.ends_with(" c"));
    }

    #[test]
    fn plain_text_and_host_only_urls_are_kept() {
        assert_eq!(mask_secrets("без ссылок"), "без ссылок");
        assert_eq!(mask_secrets("https://github.com"), "https://github.com");
        assert_eq!(mask_secrets("x ://y"), "x ://y");
    }

    #[test]
    fn uuids_and_long_keys_in_core_output_are_hidden() {
        let m = mask_secrets(
            "decode b831381d-6324-4d53-ad4f-8cda48b30811 key Zq7aB3kN9xW2pR5tY8uI1oP4sD6fG0hJ done",
        );
        assert!(
            !m.contains("b831381d") && !m.contains("Zq7aB3kN9xW2pR5tY8uI1oP4sD6fG0hJ"),
            "{m}"
        );
        assert!(m.starts_with("decode ***") && m.ends_with("done"), "{m}");
        assert_eq!(
            mask_secrets("обычная ошибка 12345 timeout"),
            "обычная ошибка 12345 timeout"
        );
        assert_eq!(
            mask_secrets("dial tcp 127.0.0.1:443: refused"),
            "dial tcp 127.0.0.1:443: refused"
        );
    }
}
