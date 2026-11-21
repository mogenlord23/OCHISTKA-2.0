//! Сайты, которые пользователь добавил в «Обход блокировок». Только домены (по одному на строку,
//! можно https-ссылкой: берётся домен); IP-адреса, порты, пароли в ссылках и мусор отклоняются.
//! Список хранится в папке данных обхода (`user-sites.json`) без паролей и ссылок-подписок.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Не больше стольких адресов.
pub const MAX_SITES: usize = 200;
const FILE: &str = "user-sites.json";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SiteError {
    #[error("Введите адрес сайта, например example.com")]
    Invalid,
    #[error("Можно добавить не больше {MAX_SITES} сайтов")]
    TooMany,
    #[error("Не удалось сохранить список сайтов")]
    Save,
}

/// Приводит одну строку к домену или отказывает.
pub fn normalize(line: &str) -> Result<String, SiteError> {
    let line = line.trim();
    if line.is_empty() || line.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(SiteError::Invalid);
    }
    let host = match line.strip_prefix("https://") {
        Some(rest) => {
            // Из ссылки берётся только домен: без пути, запроса и якоря; логин и порт не принимаются.
            let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
            if authority.contains('@') || authority.contains(':') {
                return Err(SiteError::Invalid);
            }
            authority
        }
        None => {
            if line.contains("://") || line.contains(['/', '?', '#', ':', '@']) {
                return Err(SiteError::Invalid);
            }
            line
        }
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if !valid_domain(&host) {
        return Err(SiteError::Invalid);
    }
    Ok(host)
}

fn valid_domain(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 || !host.is_ascii() {
        return false;
    }
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let ok_label = |l: &&str| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    if !labels.iter().all(ok_label) {
        return false;
    }
    // Последняя часть — буквенная (так IP-адреса вроде 1.2.3.4 не проходят).
    let tld = labels[labels.len() - 1];
    tld.len() >= 2
        && tld.chars().any(|c| c.is_ascii_alphabetic())
        && !tld.chars().all(|c| c.is_ascii_digit())
}

/// Разбирает ввод: одна строка — один адрес. Любая плохая строка отклоняет весь ввод (ничего не
/// сохраняется). Повторы внутри ввода убираются.
pub fn parse_input(text: &str) -> Result<Vec<String>, SiteError> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let domain = normalize(line)?;
        if !out.contains(&domain) {
            out.push(domain);
        }
    }
    if out.is_empty() {
        return Err(SiteError::Invalid);
    }
    Ok(out)
}

/// Хранилище списка в папке данных обхода.
pub struct UserSites {
    path: PathBuf,
}

impl UserSites {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(FILE),
        }
    }

    /// Сохранённый список (при любой ошибке чтения или порче файла — пустой, строки проверяются заново).
    pub fn load(&self) -> Vec<String> {
        let Ok(meta) = std::fs::symlink_metadata(&self.path) else {
            return Vec::new();
        };
        if !meta.is_file() || meta.len() > 256 * 1024 {
            return Vec::new();
        }
        let sites: Vec<String> = std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("sites").cloned())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let mut out: Vec<String> = Vec::new();
        for s in sites {
            if let Ok(d) = normalize(&s) {
                if !out.contains(&d) && out.len() < MAX_SITES {
                    out.push(d);
                }
            }
        }
        out
    }

    fn save(&self, sites: &[String]) -> Result<(), SiteError> {
        if std::fs::symlink_metadata(&self.path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(SiteError::Save);
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|_| SiteError::Save)?;
        }
        let text = serde_json::json!({ "sites": sites }).to_string();
        let tmp = self.path.with_extension("json.part");
        std::fs::write(&tmp, text).map_err(|_| SiteError::Save)?;
        std::fs::rename(&tmp, &self.path).map_err(|_| {
            let _ = std::fs::remove_file(&tmp);
            SiteError::Save
        })
    }

    /// Добавляет адреса из ввода (повторы убираются). При ошибке ничего не меняется.
    pub fn add(&self, text: &str) -> Result<Vec<String>, SiteError> {
        let new = parse_input(text)?;
        let mut all = self.load();
        for d in new {
            if !all.contains(&d) {
                all.push(d);
            }
        }
        if all.len() > MAX_SITES {
            return Err(SiteError::TooMany);
        }
        self.save(&all)?;
        Ok(all)
    }

    pub fn remove(&self, domain: &str) -> Result<Vec<String>, SiteError> {
        let mut all = self.load();
        all.retain(|d| d != domain);
        self.save(&all)?;
        Ok(all)
    }
}

/// Секция для движка: те же правила, что у выбранных вручную сайтов в пресете (рецепт «Выбрано
/// вручную»), список — файл `lists/nz-user-sites.txt` в папке движка. Идёт последней: ловит только
/// трафик, который не поймали более ранние секции.
pub fn section_arguments() -> Vec<String> {
    [
        "--new",
        "--filter-tcp=80,443",
        "--hostlist=lists/nz-user-sites.txt",
        "--out-range=-d8",
        "--lua-desync=hostfakesplit_multi:hosts=google.com,vimeo.com:tcp_ts=-1000:tcp_md5:repeats=2",
    ]
    .iter()
    .map(|a| (*a).to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ochistka-sites-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn domains_and_https_links_are_accepted_and_normalized() {
        assert_eq!(normalize("example.com").unwrap(), "example.com");
        assert_eq!(normalize("  Sub.Example.COM ").unwrap(), "sub.example.com");
        assert_eq!(
            normalize("https://example.com/path?q=1#x").unwrap(),
            "example.com"
        );
        assert_eq!(
            normalize("https://www.example.com").unwrap(),
            "www.example.com"
        );
        assert_eq!(normalize("example.com.").unwrap(), "example.com");
        assert_eq!(
            normalize("xn--e1afmkfd.xn--p1ai").unwrap(),
            "xn--e1afmkfd.xn--p1ai"
        );
    }

    #[test]
    fn junk_ips_ports_logins_and_insecure_links_are_rejected_with_one_plain_text() {
        for bad in [
            "",
            "   ",
            "example",
            "exa mple.com",
            "http://example.com",
            "ftp://example.com",
            "1.2.3.4",
            "https://1.2.3.4/",
            "256.1.1.1",
            "[::1]",
            "example.com:8080",
            "https://example.com:8443/",
            "https://user:pass@example.com",
            "user@example.com",
            "*.example.com",
            "-bad.example.com",
            "bad-.example.com",
            "exa_mple.com",
            "пример.рф",
            "example.c",
            "example.123",
            "a..b.com",
            "example.com/path",
            "javascript:alert(1)",
            "https://",
            "https:///x",
        ] {
            assert_eq!(normalize(bad), Err(SiteError::Invalid), "{bad:?}");
        }
        assert_eq!(
            SiteError::Invalid.to_string(),
            "Введите адрес сайта, например example.com"
        );
        let long = format!("{}.com", "a".repeat(64));
        assert_eq!(normalize(&long), Err(SiteError::Invalid));
    }

    #[test]
    fn input_is_one_address_per_line_duplicates_are_dropped_and_one_bad_line_rejects_all() {
        assert_eq!(
            parse_input("example.com\n\nhttps://example.com/x\nother.org\r\n").unwrap(),
            vec!["example.com", "other.org"]
        );
        assert_eq!(parse_input("example.com\nмусор"), Err(SiteError::Invalid));
        assert_eq!(parse_input("  \n \n"), Err(SiteError::Invalid));
    }

    #[test]
    fn add_remove_save_and_load_round_trip_without_duplicates() {
        let dir = temp();
        let s = UserSites::new(&dir);
        assert!(s.load().is_empty());
        assert_eq!(
            s.add("example.com\nother.org").unwrap(),
            vec!["example.com", "other.org"]
        );
        assert_eq!(
            s.add("example.com\nthird.net").unwrap(),
            vec!["example.com", "other.org", "third.net"]
        );
        // Новый экземпляр читает то же (хранится в файле).
        assert_eq!(
            UserSites::new(&dir).load(),
            vec!["example.com", "other.org", "third.net"]
        );
        assert_eq!(
            s.remove("other.org").unwrap(),
            vec!["example.com", "third.net"]
        );
        assert_eq!(
            s.remove("absent.com").unwrap(),
            vec!["example.com", "third.net"]
        );
        // Ошибочный ввод ничего не меняет.
        assert!(s.add("example.com\nплохо").is_err());
        assert_eq!(s.load(), vec!["example.com", "third.net"]);
    }

    #[test]
    fn at_most_200_addresses() {
        let dir = temp();
        let s = UserSites::new(&dir);
        let many: String = (0..MAX_SITES)
            .map(|i| format!("site{i}.example.com\n"))
            .collect();
        assert_eq!(s.add(&many).unwrap().len(), MAX_SITES);
        assert_eq!(s.add("extra.example.com"), Err(SiteError::TooMany));
        assert_eq!(s.load().len(), MAX_SITES);
        // Повтор уже добавленного — не превышение.
        assert!(s.add("site0.example.com").is_ok());
        let over: String = (0..=MAX_SITES)
            .map(|i| format!("o{i}.example.org\n"))
            .collect();
        assert_eq!(UserSites::new(&temp()).add(&over), Err(SiteError::TooMany));
    }

    #[test]
    fn a_damaged_or_tampered_file_loads_as_valid_domains_only_and_never_panics() {
        let dir = temp();
        std::fs::write(
            dir.join(FILE),
            r#"{"sites":["good.com","bad domain","http://x.com","1.2.3.4","good.com"]}"#,
        )
        .unwrap();
        assert_eq!(UserSites::new(&dir).load(), vec!["good.com"]);
        std::fs::write(dir.join(FILE), "не json").unwrap();
        assert!(UserSites::new(&dir).load().is_empty());
    }

    #[test]
    fn the_file_holds_only_domains_nothing_secret() {
        let dir = temp();
        UserSites::new(&dir).add("example.com").unwrap();
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert_eq!(text, r#"{"sites":["example.com"]}"#);
    }

    #[test]
    fn the_engine_section_uses_the_manual_pick_recipe_and_the_user_list() {
        let a = section_arguments();
        assert_eq!(a[0], "--new");
        assert!(a.contains(&"--hostlist=lists/nz-user-sites.txt".to_string()));
        assert!(a
            .iter()
            .any(|x| x.starts_with("--lua-desync=hostfakesplit_multi")));
    }
}
