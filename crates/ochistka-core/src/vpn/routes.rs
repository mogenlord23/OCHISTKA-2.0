//! Маршруты и исключения: что идёт напрямую, а что через VPN. Правила по доменам и по
//! приложениям (имя процесса) плюс готовые наборы. Хранятся в папке данных приложения и
//! применяются при создании конфигурации ядра.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::VpnError;

/// Область VPN: что идёт через него. Выбирается и меняется в любой момент; «выключено» — это сам
/// VPN выключен (всё напрямую), отдельного режима «напрямую» нет.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "VpnRouteMode.ts")]
pub enum RouteMode {
    /// «Только нужные сайты»: через VPN сайты из панели «Сайты», остальное напрямую. Работает через
    /// локальный вход и системный прокси (права администратора не нужны). Старое сохранённое
    /// значение `direct` читается как это.
    #[default]
    ListOnly,
    /// «Весь компьютер»: всё через VPN (туннель TUN), кроме локальных адресов и исключений
    /// пользователя. Системный прокси не используется.
    All,
}

impl<'de> Deserialize<'de> for RouteMode {
    /// Читает `listOnly` и `all`; старое значение `direct` (режима «напрямую» больше нет) — как
    /// `listOnly`, неизвестное — тоже (область по умолчанию).
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Ok(if text == "all" {
            RouteMode::All
        } else {
            RouteMode::ListOnly
        })
    }
}

/// Правила пользователя. Пустые списки допустимы.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "VpnRules.ts")]
pub struct RoutingRules {
    /// Домены и адреса, которые идут напрямую.
    pub direct_domains: Vec<String>,
    /// Домены и адреса, которые идут через VPN (сильнее наборов).
    pub proxy_domains: Vec<String>,
    /// Приложения (имя exe), которые идут напрямую.
    pub direct_apps: Vec<String>,
    /// Приложения (имя exe), которые идут через VPN.
    pub proxy_apps: Vec<String>,
    /// Включённые наборы (`id` из `PRESETS`).
    pub presets: Vec<String>,
    /// Режим маршрутов (меняется только через `VpnService::set_mode`).
    pub mode: RouteMode,
}

/// Готовый набор доменов, которые идут напрямую.
pub struct Preset {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub domain_suffixes: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "ru-direct",
        title: "Российские сайты напрямую",
        description: "Зоны .ru, .su, .рф, Яндекс, VK, Mail.ru, Одноклассники",
        domain_suffixes: &[
            "ru",
            "su",
            "xn--p1ai",
            "yandex.com",
            "yandex.net",
            "yastatic.net",
            "ya.ru",
            "vk.com",
            "vk.me",
            "vkuser.net",
            "userapi.com",
            "mycdn.me",
            "mail.ru",
            "ok.ru",
            "mts.ru",
        ],
    },
    Preset {
        id: "windows-update-direct",
        title: "Обновления Windows напрямую",
        description: "Windows Update и Microsoft Store не гоняют через VPN",
        domain_suffixes: &[
            "windowsupdate.com",
            "update.microsoft.com",
            "delivery.mp.microsoft.com",
            "download.windowsupdate.com",
            "dl.delivery.mp.microsoft.com",
        ],
    },
    Preset {
        id: "local-direct",
        title: "Локальная сеть напрямую",
        description: "Имена .local, .lan, .home.arpa и localhost",
        domain_suffixes: &["local", "lan", "home.arpa", "localhost"],
    },
];

/// Домены набора по `id`.
pub fn preset_suffixes(id: &str) -> Option<&'static [&'static str]> {
    PRESETS
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.domain_suffixes)
}

/// Тип записи списка доменов.
#[derive(Debug, PartialEq, Eq)]
pub enum DomainRule {
    Suffix(String),
    Full(String),
    Keyword(String),
    Cidr(String),
}

/// Разбирает запись: `example.com`, `*.example.com`, `full:host.example.com`,
/// `keyword:word`, IP или сеть (`10.0.0.0/8`, `2001:db8::/32`).
pub fn parse_domain_entry(text: &str) -> Result<DomainRule, String> {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return Err("пустая запись".into());
    }
    if t.contains("://") || t.contains('/') && !is_cidr(&t) || t.chars().any(char::is_whitespace) {
        return Err(format!("«{text}»: нужен домен без https:// и пути"));
    }
    if let Some(rest) = t.strip_prefix("full:") {
        return Ok(DomainRule::Full(domain_name(rest, text)?));
    }
    if let Some(rest) = t.strip_prefix("keyword:") {
        if rest.is_empty() || !rest.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(format!("«{text}»: неверное ключевое слово"));
        }
        return Ok(DomainRule::Keyword(rest.to_string()));
    }
    if is_cidr(&t) {
        return Ok(DomainRule::Cidr(if t.contains('/') {
            t
        } else if t.contains(':') {
            format!("{t}/128")
        } else {
            format!("{t}/32")
        }));
    }
    let rest = t
        .strip_prefix("*.")
        .or_else(|| t.strip_prefix('.'))
        .unwrap_or(&t);
    Ok(DomainRule::Suffix(domain_name(rest, text)?))
}

fn is_cidr(t: &str) -> bool {
    let (addr, bits) = match t.split_once('/') {
        Some((a, b)) => (a, Some(b)),
        None => (t, None),
    };
    let Ok(ip) = addr.parse::<std::net::IpAddr>() else {
        return false;
    };
    match bits {
        None => true,
        Some(b) => b
            .parse::<u8>()
            .is_ok_and(|n| n <= if ip.is_ipv4() { 32 } else { 128 }),
    }
}

/// Имя домена в виде, годном для sing-box: ASCII, цифры, дефис, точки; юникодные метки
/// (например, `сайт.рф`) переводятся в punycode.
fn domain_name(name: &str, original: &str) -> Result<String, String> {
    let name = name.trim_matches('.');
    if name.is_empty() {
        return Err(format!("«{original}»: пустое имя"));
    }
    let mut labels = Vec::new();
    for label in name.split('.') {
        let ascii = if label.is_ascii() {
            label.to_string()
        } else {
            punycode_label(label).ok_or_else(|| format!("«{original}»: недопустимое имя домена"))?
        };
        labels.push(ascii);
    }
    let name = labels.join(".");
    if name.len() > 253
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_')
        || name.split('.').any(|l| l.is_empty() || l.len() > 63)
    {
        return Err(format!("«{original}»: недопустимое имя домена"));
    }
    Ok(name)
}

/// Punycode (RFC 3492) для одной метки с символами не из ASCII: `xn--…`.
fn punycode_label(label: &str) -> Option<String> {
    const BASE: u32 = 36;
    const T_MIN: u32 = 1;
    const T_MAX: u32 = 26;
    const SKEW: u32 = 38;
    const DAMP: u32 = 700;
    fn digit(d: u32) -> char {
        char::from(if d < 26 {
            b'a' + d as u8
        } else {
            b'0' + (d - 26) as u8
        })
    }
    fn adapt(mut delta: u32, points: u32, first: bool) -> u32 {
        delta = if first { delta / DAMP } else { delta / 2 };
        delta += delta / points;
        let mut k = 0;
        while delta > ((BASE - T_MIN) * T_MAX) / 2 {
            delta /= BASE - T_MIN;
            k += BASE;
        }
        k + (BASE - T_MIN + 1) * delta / (delta + SKEW)
    }
    let chars: Vec<u32> = label.to_lowercase().chars().map(|c| c as u32).collect();
    let mut out: String = chars
        .iter()
        .filter(|c| **c < 128)
        .map(|c| char::from(*c as u8))
        .collect();
    let basic = out.len() as u32;
    let mut handled = basic;
    if basic > 0 {
        out.push('-');
    }
    let (mut n, mut delta, mut bias) = (128u32, 0u32, 72u32);
    while (handled as usize) < chars.len() {
        let m = *chars.iter().filter(|c| **c >= n).min()?;
        delta = delta.checked_add((m - n).checked_mul(handled + 1)?)?;
        n = m;
        for &c in &chars {
            if c < n {
                delta = delta.checked_add(1)?;
            }
            if c == n {
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias {
                        T_MIN
                    } else if k >= bias + T_MAX {
                        T_MAX
                    } else {
                        k - bias
                    };
                    if q < t {
                        break;
                    }
                    out.push(digit(t + (q - t) % (BASE - t)));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                out.push(digit(q));
                bias = adapt(delta, handled + 1, handled == basic);
                delta = 0;
                handled += 1;
            }
        }
        delta += 1;
        n += 1;
    }
    Some(format!("xn--{out}"))
}

/// Имя процесса: только файл, без путей и кавычек.
pub fn parse_app_entry(text: &str) -> Result<String, String> {
    let t = text.trim().trim_matches('"');
    if t.is_empty() {
        return Err("пустое имя приложения".into());
    }
    if t.contains(['/', '\\', ':', '*', '?', '<', '>', '|', '"']) {
        return Err(format!("«{text}»: нужно имя файла, например chrome.exe"));
    }
    let name = if t.contains('.') {
        t.to_string()
    } else {
        format!("{t}.exe")
    };
    Ok(name)
}

fn clean_list<F: Fn(&str) -> Result<String, String>>(
    items: &[String],
    check: F,
    errors: &mut Vec<String>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        if item.trim().is_empty() {
            continue;
        }
        match check(item) {
            Ok(v) if !out.iter().any(|o| o.eq_ignore_ascii_case(&v)) => out.push(v),
            Ok(_) => {}
            Err(e) => errors.push(e),
        }
    }
    out
}

impl RoutingRules {
    /// Приводит записи к каноническому виду. Ошибка перечисляет все неверные строки.
    pub fn sanitized(&self) -> Result<RoutingRules, VpnError> {
        let mut errors = Vec::new();
        let domain = |s: &str| {
            parse_domain_entry(s).map(|r| match r {
                DomainRule::Suffix(v) => v,
                DomainRule::Full(v) => format!("full:{v}"),
                DomainRule::Keyword(v) => format!("keyword:{v}"),
                DomainRule::Cidr(v) => v,
            })
        };
        let rules = RoutingRules {
            direct_domains: clean_list(&self.direct_domains, domain, &mut errors),
            proxy_domains: clean_list(&self.proxy_domains, domain, &mut errors),
            direct_apps: clean_list(&self.direct_apps, parse_app_entry, &mut errors),
            proxy_apps: clean_list(&self.proxy_apps, parse_app_entry, &mut errors),
            presets: self
                .presets
                .iter()
                .filter(|p| preset_suffixes(p).is_some())
                .cloned()
                .collect(),
            mode: self.mode,
        };
        if errors.is_empty() {
            Ok(rules)
        } else {
            Err(VpnError::Rules(errors.join("; ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_entries() {
        assert_eq!(
            parse_domain_entry("Example.COM"),
            Ok(DomainRule::Suffix("example.com".into()))
        );
        assert_eq!(
            parse_domain_entry("*.a.org"),
            Ok(DomainRule::Suffix("a.org".into()))
        );
        assert_eq!(
            parse_domain_entry(".a.org"),
            Ok(DomainRule::Suffix("a.org".into()))
        );
        assert_eq!(
            parse_domain_entry("full:Host.a.org"),
            Ok(DomainRule::Full("host.a.org".into()))
        );
        assert_eq!(
            parse_domain_entry("keyword:bank"),
            Ok(DomainRule::Keyword("bank".into()))
        );
        assert_eq!(
            parse_domain_entry("10.0.0.0/8"),
            Ok(DomainRule::Cidr("10.0.0.0/8".into()))
        );
        assert_eq!(
            parse_domain_entry("8.8.8.8"),
            Ok(DomainRule::Cidr("8.8.8.8/32".into()))
        );
        assert_eq!(
            parse_domain_entry("2001:db8::1"),
            Ok(DomainRule::Cidr("2001:db8::1/128".into()))
        );
        assert_eq!(
            parse_domain_entry("сайт.рф"),
            Ok(DomainRule::Suffix("xn--80aswg.xn--p1ai".into()))
        );
        assert_eq!(
            parse_domain_entry("рф"),
            Ok(DomainRule::Suffix("xn--p1ai".into()))
        );
    }

    #[test]
    fn bad_domain_entries() {
        for bad in [
            "https://a.com",
            "a.com/path",
            "a b.com",
            "",
            "a..com",
            "10.0.0.0/40",
        ] {
            assert!(parse_domain_entry(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn app_entries() {
        assert_eq!(parse_app_entry("chrome"), Ok("chrome.exe".into()));
        assert_eq!(parse_app_entry(" \"Steam.exe\" "), Ok("Steam.exe".into()));
        assert!(parse_app_entry(r"C:\Windows\x.exe").is_err());
        assert!(parse_app_entry("").is_err());
    }

    #[test]
    fn sanitize_dedupes_and_reports_all_errors() {
        let rules = RoutingRules {
            direct_domains: vec!["A.com".into(), "a.com".into(), " ".into()],
            direct_apps: vec!["steam".into()],
            presets: vec!["ru-direct".into(), "unknown".into()],
            ..Default::default()
        };
        let clean = rules.sanitized().unwrap();
        assert_eq!(clean.direct_domains, vec!["a.com"]);
        assert_eq!(clean.direct_apps, vec!["steam.exe"]);
        assert_eq!(clean.presets, vec!["ru-direct"]);
        let bad = RoutingRules {
            direct_domains: vec!["http://x".into(), "ok.com".into()],
            proxy_apps: vec!["a\\b".into()],
            ..Default::default()
        };
        let err = bad.sanitized().unwrap_err().to_string();
        assert!(err.contains("http://x") && err.contains("a\\b"));
    }

    #[test]
    fn presets_exist_with_russian_titles() {
        assert!(preset_suffixes("ru-direct").unwrap().contains(&"ru"));
        assert_eq!(PRESETS[0].title, "Российские сайты напрямую");
        assert!(preset_suffixes("nope").is_none());
    }
}
