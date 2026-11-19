//! Конфигурация sing-box для режима системного прокси: локальный вход `mixed` на 127.0.0.1,
//! выбранный сервер как `proxy`, правила маршрутов пользователя. Системный прокси Windows
//! выставляет само приложение (с резервной копией), а не ядро.

use serde_json::{json, Map, Value};

use super::parse::Server;
use super::routes::{parse_domain_entry, preset_suffixes, DomainRule, RoutingRules};

/// Строит конфигурацию. `rules` ожидаются очищенными (`RoutingRules::sanitized`).
pub fn build_config(server: &Server, port: u16, rules: &RoutingRules) -> Value {
    let mut proxy = server.outbound.clone();
    if let Some(map) = proxy.as_object_mut() {
        map.insert("tag".into(), json!("proxy"));
    }
    let mut route_rules: Vec<Value> = vec![json!({"ip_is_private": true, "outbound": "direct"})];
    // Порядок важен: приложения, затем явные домены пользователя, затем наборы.
    if !rules.direct_apps.is_empty() {
        route_rules.push(json!({"process_name": rules.direct_apps, "outbound": "direct"}));
    }
    if !rules.proxy_apps.is_empty() {
        route_rules.push(json!({"process_name": rules.proxy_apps, "outbound": "proxy"}));
    }
    route_rules.extend(domain_rules(&rules.proxy_domains, "proxy"));
    route_rules.extend(domain_rules(&rules.direct_domains, "direct"));
    for id in &rules.presets {
        if let Some(list) = preset_suffixes(id) {
            route_rules.push(json!({"domain_suffix": list, "outbound": "direct"}));
        }
    }
    let mut route = Map::new();
    route.insert("rules".into(), Value::Array(route_rules));
    route.insert("final".into(), json!("proxy"));
    route.insert("default_domain_resolver".into(), json!("local"));
    route.insert("auto_detect_interface".into(), json!(true));
    if !rules.direct_apps.is_empty() || !rules.proxy_apps.is_empty() {
        route.insert("find_process".into(), json!(true));
    }
    json!({
        "log": {"level": "warn", "timestamp": true},
        "dns": {"servers": [{"type": "local", "tag": "local"}], "final": "local"},
        "inbounds": [{"type": "mixed", "tag": "mixed-in", "listen": "127.0.0.1", "listen_port": port}],
        "outbounds": [proxy, {"type": "direct", "tag": "direct"}],
        "route": Value::Object(route),
    })
}

fn domain_rules(entries: &[String], outbound: &str) -> Vec<Value> {
    let (mut suffix, mut full, mut keyword, mut cidr) = (vec![], vec![], vec![], vec![]);
    for entry in entries {
        match parse_domain_entry(entry) {
            Ok(DomainRule::Suffix(v)) => suffix.push(v),
            Ok(DomainRule::Full(v)) => full.push(v),
            Ok(DomainRule::Keyword(v)) => keyword.push(v),
            Ok(DomainRule::Cidr(v)) => cidr.push(v),
            Err(_) => {}
        }
    }
    let mut out = Vec::new();
    for (key, list) in [
        ("domain_suffix", suffix),
        ("domain", full),
        ("domain_keyword", keyword),
        ("ip_cidr", cidr),
    ] {
        if !list.is_empty() {
            out.push(json!({ key: list, "outbound": outbound }));
        }
    }
    out
}

/// Значение `ProxyOverride` для Windows: локальные адреса и частные сети не идут через прокси.
pub const PROXY_OVERRIDE: &str =
    "localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.2*;172.30.*;172.31.*;192.168.*;<local>";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vpn::parse::parse_link;

    fn server() -> Server {
        parse_link("trojan://pw@tr.example.com:443?sni=tr.example.com#Test").unwrap()
    }

    #[test]
    fn minimal_config_snapshot() {
        let cfg = build_config(&server(), 20808, &RoutingRules::default());
        let expected = json!({
          "log": {"level": "warn", "timestamp": true},
          "dns": {"servers": [{"type": "local", "tag": "local"}], "final": "local"},
          "inbounds": [{"type": "mixed", "tag": "mixed-in", "listen": "127.0.0.1", "listen_port": 20808}],
          "outbounds": [
            {"type": "trojan", "server": "tr.example.com", "server_port": 443, "password": "pw",
             "tls": {"enabled": true, "server_name": "tr.example.com"}, "tag": "proxy"},
            {"type": "direct", "tag": "direct"}
          ],
          "route": {
            "rules": [{"ip_is_private": true, "outbound": "direct"}],
            "final": "proxy",
            "default_domain_resolver": "local",
            "auto_detect_interface": true
          }
        });
        assert_eq!(cfg, expected);
    }

    #[test]
    fn rules_are_ordered_and_grouped() {
        let rules = RoutingRules {
            direct_domains: vec![
                "example.com".into(),
                "full:a.b.c".into(),
                "10.0.0.0/8".into(),
            ],
            proxy_domains: vec!["keyword:news".into()],
            direct_apps: vec!["steam.exe".into()],
            proxy_apps: vec!["chrome.exe".into()],
            presets: vec!["ru-direct".into()],
        }
        .sanitized()
        .unwrap();
        let cfg = build_config(&server(), 1080, &rules);
        let list = cfg["route"]["rules"].as_array().unwrap();
        assert_eq!(list[0]["ip_is_private"], true);
        assert_eq!(
            list[1],
            json!({"process_name": ["steam.exe"], "outbound": "direct"})
        );
        assert_eq!(
            list[2],
            json!({"process_name": ["chrome.exe"], "outbound": "proxy"})
        );
        assert_eq!(
            list[3],
            json!({"domain_keyword": ["news"], "outbound": "proxy"})
        );
        assert_eq!(
            list[4],
            json!({"domain_suffix": ["example.com"], "outbound": "direct"})
        );
        assert_eq!(list[5], json!({"domain": ["a.b.c"], "outbound": "direct"}));
        assert_eq!(
            list[6],
            json!({"ip_cidr": ["10.0.0.0/8"], "outbound": "direct"})
        );
        assert!(list[7]["domain_suffix"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d == "ru"));
        assert_eq!(list.len(), 8);
        assert_eq!(cfg["route"]["find_process"], true);
        assert_eq!(cfg["route"]["final"], "proxy");
    }

    #[test]
    fn listens_only_on_loopback() {
        let cfg = build_config(&server(), 5, &RoutingRules::default());
        assert_eq!(cfg["inbounds"][0]["listen"], "127.0.0.1");
        assert!(cfg["inbounds"].as_array().unwrap().len() == 1);
    }
}
