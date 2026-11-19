//! Страна сервера по названию: флаг-эмодзи, код в начале имени или название страны.

/// Код страны (ISO 3166-1 alpha-2, верхний регистр) или `None`.
pub fn country_code(name: &str) -> Option<String> {
    flag_code(name)
        .or_else(|| bracket_code(name))
        .or_else(|| keyword_code(name))
}

/// Флаг-эмодзи (два региональных индикатора) в любом месте имени.
fn flag_code(name: &str) -> Option<String> {
    let mut chars = name.chars().peekable();
    while let Some(first) = chars.next() {
        if let (Some(a), Some(&second)) = (regional(first), chars.peek()) {
            if let Some(b) = regional(second) {
                return Some(format!("{a}{b}"));
            }
        }
    }
    None
}

fn regional(c: char) -> Option<char> {
    let p = c as u32;
    (0x1F1E6..=0x1F1FF)
        .contains(&p)
        .then(|| char::from(b'A' + (p - 0x1F1E6) as u8))
}

/// `[DE]`, `(DE)` или `DE ` / `DE-` / `DE_` в начале имени.
fn bracket_code(name: &str) -> Option<String> {
    let t = name.trim_start();
    let inner = t
        .strip_prefix('[')
        .or_else(|| t.strip_prefix('('))
        .unwrap_or(t);
    let letters: String = inner
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let rest = &inner[letters.len()..];
    let bracketed = inner.len() != t.len();
    let ok_sep = rest.is_empty()
        || rest.starts_with([' ', '-', '_', '|', ']', ')'])
        || (bracketed && rest.starts_with([']', ')']));
    if letters.len() == 2 && ok_sep {
        let up = letters.to_ascii_uppercase();
        if KNOWN.contains(&up.as_str()) {
            return Some(up);
        }
    }
    None
}

const KNOWN: &[&str] = &[
    "AE", "AM", "AR", "AT", "AU", "AZ", "BE", "BG", "BR", "BY", "CA", "CH", "CL", "CN", "CY", "CZ",
    "DE", "DK", "EE", "ES", "FI", "FR", "GB", "GE", "GR", "HK", "HR", "HU", "ID", "IE", "IL", "IN",
    "IS", "IT", "JP", "KG", "KR", "KZ", "LT", "LU", "LV", "MD", "MX", "NL", "NO", "NZ", "PL", "PT",
    "RO", "RS", "RU", "SE", "SG", "SI", "SK", "TH", "TR", "TW", "UA", "UK", "US", "UZ", "VN", "ZA",
];

const WORDS: &[(&str, &str)] = &[
    ("germany", "DE"),
    ("германия", "DE"),
    ("netherlands", "NL"),
    ("нидерланды", "NL"),
    ("голландия", "NL"),
    ("finland", "FI"),
    ("финляндия", "FI"),
    ("sweden", "SE"),
    ("швеция", "SE"),
    ("norway", "NO"),
    ("норвегия", "NO"),
    ("poland", "PL"),
    ("польша", "PL"),
    ("france", "FR"),
    ("франция", "FR"),
    ("united kingdom", "GB"),
    ("великобритания", "GB"),
    ("england", "GB"),
    ("united states", "US"),
    ("сша", "US"),
    ("америка", "US"),
    ("usa", "US"),
    ("canada", "CA"),
    ("канада", "CA"),
    ("latvia", "LV"),
    ("латвия", "LV"),
    ("lithuania", "LT"),
    ("литва", "LT"),
    ("estonia", "EE"),
    ("эстония", "EE"),
    ("russia", "RU"),
    ("россия", "RU"),
    ("ukraine", "UA"),
    ("украина", "UA"),
    ("kazakhstan", "KZ"),
    ("казахстан", "KZ"),
    ("turkey", "TR"),
    ("турция", "TR"),
    ("japan", "JP"),
    ("япония", "JP"),
    ("singapore", "SG"),
    ("сингапур", "SG"),
    ("hong kong", "HK"),
    ("гонконг", "HK"),
    ("switzerland", "CH"),
    ("швейцария", "CH"),
    ("austria", "AT"),
    ("австрия", "AT"),
    ("italy", "IT"),
    ("италия", "IT"),
    ("spain", "ES"),
    ("испания", "ES"),
    ("armenia", "AM"),
    ("армения", "AM"),
    ("georgia", "GE"),
    ("грузия", "GE"),
    ("moldova", "MD"),
    ("молдова", "MD"),
    ("bulgaria", "BG"),
    ("болгария", "BG"),
    ("romania", "RO"),
    ("румыния", "RO"),
    ("czech", "CZ"),
    ("чехия", "CZ"),
    ("israel", "IL"),
    ("израиль", "IL"),
    ("india", "IN"),
    ("индия", "IN"),
    ("brazil", "BR"),
    ("бразилия", "BR"),
];

fn keyword_code(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    WORDS
        .iter()
        .find(|(word, _)| lower.contains(word))
        .map(|(_, code)| (*code).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_emoji_anywhere() {
        assert_eq!(country_code("🇱🇻 Латвия #1").as_deref(), Some("LV"));
        assert_eq!(country_code("Fast 🇩🇪 node").as_deref(), Some("DE"));
    }

    #[test]
    fn bracket_and_prefix_codes() {
        assert_eq!(country_code("[NL] server").as_deref(), Some("NL"));
        assert_eq!(country_code("FI-Helsinki").as_deref(), Some("FI"));
        assert_eq!(country_code("(us) west").as_deref(), Some("US"));
        assert_eq!(country_code("XY test"), None);
    }

    #[test]
    fn country_words_in_both_languages() {
        assert_eq!(country_code("Германия премиум").as_deref(), Some("DE"));
        assert_eq!(country_code("Sweden 02").as_deref(), Some("SE"));
        assert_eq!(country_code("Безымянный"), None);
    }
}
