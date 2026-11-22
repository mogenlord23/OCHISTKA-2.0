//! Список сайтов области «Только сайты»: встроенный список (assets/sites/builtin.txt) плюс сайты
//! пользователя из панели «Мои сайты», без повторов.

use std::collections::HashSet;

/// Встроенный список: по одному домену в строке, `#` — комментарий.
const BUILTIN: &str = include_str!("../../../../assets/sites/builtin.txt");

/// Домены встроенного списка без комментариев и пустых строк.
pub fn builtin_sites() -> Vec<String> {
    parse_lines(BUILTIN)
}

fn parse_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Встроенные сайты и сайты пользователя вместе; повтор (без учёта регистра) остаётся один раз.
pub fn merged_sites(user: &[String]) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for site in builtin_sites().into_iter().chain(user.iter().cloned()) {
        let site = site.trim().to_string();
        if !site.is_empty() && seen.insert(site.to_lowercase()) {
            out.push(site);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vpn::config::build_config;
    use crate::vpn::parse::{parse_link, Server};
    use crate::vpn::routes::{RouteMode, RoutingRules};
    use serde_json::json;

    fn server() -> Server {
        parse_link("trojan://pw@tr.example.com:443?sni=tr.example.com#Test").unwrap()
    }

    fn rules(mode: RouteMode) -> RoutingRules {
        RoutingRules {
            mode,
            ..RoutingRules::default()
        }
    }

    #[test]
    fn the_builtin_list_has_no_comments_or_blank_lines_and_contains_youtube() {
        let list = builtin_sites();
        assert!(list.iter().all(|l| !l.starts_with('#') && !l.is_empty()));
        for d in ["youtube.com", "googlevideo.com", "ytimg.com"] {
            assert!(list.iter().any(|x| x == d), "{d}");
        }
    }

    #[test]
    fn merging_adds_the_user_sites_once() {
        let user = vec![
            "my-site.example".to_string(),
            "YouTube.com".to_string(),
            "my-site.example".to_string(),
        ];
        let all = merged_sites(&user);
        assert_eq!(all.iter().filter(|d| *d == "my-site.example").count(), 1);
        assert_eq!(
            all.iter()
                .filter(|d| d.eq_ignore_ascii_case("youtube.com"))
                .count(),
            1,
            "дубль встроенного не повторяется"
        );
    }

    #[test]
    fn only_sites_config_has_builtin_and_user_sites_and_the_whole_computer_ignores_them() {
        let sites = merged_sites(&["my-site.example".to_string()]);
        let cfg = build_config(&server(), 1, &rules(RouteMode::ListOnly), &sites);
        let text = cfg["route"]["rules"].to_string();
        for d in [
            "youtube.com",
            "googlevideo.com",
            "ytimg.com",
            "my-site.example",
        ] {
            assert!(text.contains(d), "{d}");
        }
        assert_eq!(cfg["route"]["final"], json!("direct"));
        let all = build_config(&server(), 1, &rules(RouteMode::All), &sites);
        let text = all["route"]["rules"].to_string();
        assert!(!text.contains("youtube.com") && !text.contains("my-site.example"));
        assert_eq!(all["route"]["final"], json!("proxy"));
    }
}
