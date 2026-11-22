//! Список сайтов: встроенный файл и пользовательский список. Совпадает сам домен и поддомены.

const BUILTIN: &str = include_str!("../../../assets/sites/builtin.txt");

/// Домены из встроенного файла `assets/sites/builtin.txt` (комментарии `#` и пустые строки пропускаются).
pub fn builtin_sites() -> Vec<String> {
    parse_list(BUILTIN)
}

fn parse_list(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.trim_end_matches('.').to_ascii_lowercase())
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteList {
    domains: Vec<String>,
}

impl SiteList {
    pub fn new<I, S>(domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut out: Vec<String> = Vec::new();
        for d in domains {
            let d = d.as_ref().trim().trim_end_matches('.').to_ascii_lowercase();
            if !d.is_empty() && !out.contains(&d) {
                out.push(d);
            }
        }
        Self { domains: out }
    }

    /// Встроенный список плюс пользовательские сайты.
    pub fn builtin_with(user: &[String]) -> Self {
        Self::new(builtin_sites().into_iter().chain(user.iter().cloned()))
    }

    pub fn len(&self) -> usize {
        self.domains.len()
    }

    pub fn is_empty(&self) -> bool {
        self.domains.is_empty()
    }

    /// Точное совпадение или поддомен (`www.youtube.com` совпадает с `youtube.com`;
    /// `notyoutube.com` — нет).
    pub fn matches(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        self.domains.iter().any(|d| {
            host == *d
                || host
                    .strip_suffix(d.as_str())
                    .is_some_and(|rest| rest.ends_with('.'))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_subdomain_and_foreign_hosts() {
        let list = SiteList::new(["youtube.com", "Discord.GG"]);
        assert!(list.matches("youtube.com"));
        assert!(list.matches("www.youtube.com"));
        assert!(list.matches("a.b.youtube.com."));
        assert!(list.matches("DISCORD.gg"));
        assert!(!list.matches("notyoutube.com"));
        assert!(!list.matches("youtube.com.evil.net"));
        assert!(!list.matches("example.org"));
        assert!(!list.matches(""));
    }

    #[test]
    fn the_builtin_file_has_the_expected_sites_and_no_comments() {
        let list = SiteList::builtin_with(&[]);
        for host in [
            "youtube.com",
            "r1---sn.googlevideo.com",
            "i.ytimg.com",
            "youtu.be",
            "discord.com",
            "discord.gg",
            "x.com",
            "twitter.com",
        ] {
            assert!(list.matches(host), "{host}");
        }
        assert!(builtin_sites().iter().all(|d| !d.starts_with('#')));
    }

    #[test]
    fn user_sites_are_added_without_duplicates() {
        let list = SiteList::builtin_with(&["example.com".into(), "youtube.com".into()]);
        assert!(list.matches("cdn.example.com"));
        let once = list.len();
        assert_eq!(
            SiteList::builtin_with(&["example.com".into(), "example.com".into()]).len(),
            once
        );
    }
}
