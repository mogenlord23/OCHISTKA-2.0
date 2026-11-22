//! Конфигурация sing-box: локальный вход `mixed` на 127.0.0.1, выбранный сервер как `proxy`, правила
//! маршрутов пользователя; для области «весь компьютер» ещё и туннель TUN. Системный прокси Windows
//! выставляет само приложение (с резервной копией), а не ядро.

use serde_json::{json, Map, Value};

use super::parse::Server;
use super::routes::{parse_domain_entry, preset_suffixes, DomainRule, RouteMode, RoutingRules};

/// Строит конфигурацию. `rules` ожидаются очищенными (`RoutingRules::sanitized`); `sites` — список
/// из панели «Сайты» (используется только в области «только нужные сайты»).
///
/// - `All` («весь компьютер»): туннель TUN, всё через VPN, кроме локальных адресов и исключений
///   пользователя. Локальный вход `mixed` остаётся для проверки связи, системный прокси не ставится.
/// - `ListOnly` («только нужные сайты»): через VPN только сайты из списка (и явные «через VPN»
///   пользователя), остальное напрямую; ядро слушает локальный вход, на него смотрит системный прокси.
pub fn build_config(server: &Server, port: u16, rules: &RoutingRules, sites: &[String]) -> Value {
    let mut proxy = server.outbound.clone();
    if let Some(map) = proxy.as_object_mut() {
        map.insert("tag".into(), json!("proxy"));
    }
    // Обе области идут через туннель. Сначала DNS-запросы туннеля, адрес самого сервера напрямую
    // (иначе соединение с ним попало бы в туннель), затем локальные адреса напрямую.
    // `sniff` первым: без него ядро не видит ни DNS-запросов (для hijack-dns), ни имён сайтов в
    // соединениях по IP-адресу, и правила по доменам не срабатывают.
    let mut route_rules: Vec<Value> = vec![
        json!({"action": "sniff"}),
        json!({"protocol": "dns", "action": "hijack-dns"}),
        server_direct_rule(&server.host),
        json!({"ip_is_private": true, "outbound": "direct"}),
    ];
    if rules.mode == RouteMode::ListOnly {
        // Только сайты: через VPN сайты из списка (с поддоменами) и явные «через VPN», остальное
        // напрямую.
        if !rules.proxy_apps.is_empty() {
            route_rules.push(json!({"process_name": rules.proxy_apps, "outbound": "proxy"}));
        }
        route_rules.extend(domain_rules(&rules.proxy_domains, "proxy"));
        route_rules.extend(domain_rules(sites, "proxy"));
        let mut route = Map::new();
        route.insert("rules".into(), Value::Array(route_rules));
        route.insert("final".into(), json!("direct"));
        route.insert("default_domain_resolver".into(), json!("local"));
        route.insert("auto_detect_interface".into(), json!(true));
        if !rules.proxy_apps.is_empty() {
            route.insert("find_process".into(), json!(true));
        }
        return json!({
            "log": {"level": "warn", "timestamp": true},
            "dns": tunnel_dns(server, rules, sites),
            "inbounds": tunnel_inbounds(port),
            "outbounds": [proxy, {"type": "direct", "tag": "direct"}],
            "route": Value::Object(route),
        });
    }
    // Порядок важен: приложения, затем явные домены пользователя, затем наборы.
    if !rules.direct_apps.is_empty() {
        route_rules.push(json!({"process_name": rules.direct_apps, "outbound": "direct"}));
    }
    if !rules.proxy_apps.is_empty() {
        route_rules.push(json!({"process_name": rules.proxy_apps, "outbound": "proxy"}));
    }
    route_rules.extend(domain_rules(&rules.proxy_domains, "proxy"));
    // Российские зоны мимо VPN: сайты вроде vk.ru видят обычный адрес, как без VPN.
    route_rules.push(json!({"domain_suffix": RUSSIAN_ZONES, "outbound": "direct"}));
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
        "dns": tunnel_dns(server, rules, sites),
        "inbounds": tunnel_inbounds(port),
        "outbounds": [proxy, {"type": "direct", "tag": "direct"}],
        "route": Value::Object(route),
    })
}

fn tunnel_inbounds(port: u16) -> Value {
    json!([
        {"type": "mixed", "tag": "mixed-in", "listen": "127.0.0.1", "listen_port": port},
        {"type": "tun", "tag": "tun-in", "interface_name": TUN_NAME, "address": [TUN_ADDRESS, TUN_ADDRESS_V6], "mtu": TUN_MTU, "auto_route": true, "strict_route": true, "stack": "mixed"},
    ])
}

/// Адрес туннельного адаптера (частная сеть, не пересекается с домашними 192.168.*).
/// Размер пакета туннеля: с запасом под заголовки VPN, без дробления пакетов (стандартные 9000
/// у адаптера дают потери и медленную загрузку при передаче через прокси-канал).
pub const TUN_MTU: u32 = 1400;
/// Имя адаптера туннеля: по нему приложение находит и снимает только свои остатки.
pub const TUN_NAME: &str = "OCHISTKA-VPN";
/// Российские зоны: имена в них разрешаются локально и идут напрямую, мимо VPN. `xn--p1ai` — «рф».
pub const RUSSIAN_ZONES: [&str; 2] = ["ru", "xn--p1ai"];
pub const TUN_ADDRESS: &str = "172.19.0.1/30";
/// Тот же адаптер для IPv6: без него IPv6-трафик пошёл бы мимо туннеля.
pub const TUN_ADDRESS_V6: &str = "fdfe:dcba:9876::1/126";

fn is_ip(host: &str) -> bool {
    host.parse::<std::net::IpAddr>().is_ok()
}

/// Адрес сервера VPN напрямую: по адресу, если это IP, иначе по имени.
fn server_direct_rule(host: &str) -> Value {
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(_)) => {
            json!({"ip_cidr": [format!("{host}/32")], "outbound": "direct"})
        }
        Ok(std::net::IpAddr::V6(_)) => {
            json!({"ip_cidr": [format!("{host}/128")], "outbound": "direct"})
        }
        Err(_) => json!({"domain": [host], "outbound": "direct"}),
    }
}

/// Правила имён из правил маршрута: те же домены, но с указанием сервера имён вместо выхода.
fn to_dns_rules(list: Vec<Value>, server_tag: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for mut rule in list {
        if let Some(map) = rule.as_object_mut() {
            if map.contains_key("ip_cidr") {
                continue;
            }
            map.remove("outbound");
            map.insert("server".into(), json!(server_tag));
            out.push(rule);
        }
    }
    out
}

/// Запросы имён в туннеле. Весь компьютер: обычные идут к публичному серверу ЧЕРЕЗ VPN, а имя самого
/// сервера VPN и имена из исключений «напрямую» разрешаются локально. Только сайты: через VPN
/// разрешаются имена из списка, остальные — локально, как без VPN.
fn tunnel_dns(server: &Server, rules: &RoutingRules, sites: &[String]) -> Value {
    let mut dns_rules: Vec<Value> = Vec::new();
    if !is_ip(&server.host) {
        dns_rules.push(json!({"domain": [server.host], "server": "local"}));
    }
    let (via_vpn, final_server) = if rules.mode == RouteMode::ListOnly {
        let mut list = domain_rules(&rules.proxy_domains, "proxy");
        list.extend(domain_rules(sites, "proxy"));
        (to_dns_rules(list, "remote"), "local")
    } else {
        let mut direct: Vec<Value> = domain_rules(&rules.direct_domains, "direct");
        direct.push(json!({"domain_suffix": RUSSIAN_ZONES, "outbound": "direct"}));
        for id in &rules.presets {
            if let Some(list) = preset_suffixes(id) {
                direct.push(json!({"domain_suffix": list, "outbound": "direct"}));
            }
        }
        (to_dns_rules(direct, "local"), "remote")
    };
    dns_rules.extend(via_vpn);
    json!({
        "servers": [
            {"type": "local", "tag": "local"},
            {"type": "https", "tag": "remote", "server": "1.1.1.1", "detour": "proxy"},
        ],
        "rules": dns_rules,
        "final": final_server,
        // Запоминает, какому имени принадлежит IP-адрес из ответа: соединение по адресу получает имя
        // сайта и попадает под правило по домену.
        "reverse_mapping": true,
        // Ответы кэшируются: повторные запросы одного имени не идут в туннель.
        "cache_capacity": 4096,
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
        let all = RoutingRules {
            mode: RouteMode::All,
            ..RoutingRules::default()
        };
        let cfg = build_config(&server(), 20808, &all, &[]);
        let expected = json!({
          "log": {"level": "warn", "timestamp": true},
          "dns": {
            "servers": [
              {"type": "local", "tag": "local"},
              {"type": "https", "tag": "remote", "server": "1.1.1.1", "detour": "proxy"}
            ],
            "rules": [
              {"domain": ["tr.example.com"], "server": "local"},
              {"domain_suffix": ["ru", "xn--p1ai"], "server": "local"}
            ],
            "final": "remote",
            "reverse_mapping": true,
            "cache_capacity": 4096
          },
          "inbounds": [
            {"type": "mixed", "tag": "mixed-in", "listen": "127.0.0.1", "listen_port": 20808},
            {"type": "tun", "tag": "tun-in", "interface_name": "OCHISTKA-VPN", "address": ["172.19.0.1/30", "fdfe:dcba:9876::1/126"], "mtu": 1400, "auto_route": true, "strict_route": true, "stack": "mixed"}
          ],
          "outbounds": [
            {"type": "trojan", "server": "tr.example.com", "server_port": 443, "password": "pw",
             "tls": {"enabled": true, "server_name": "tr.example.com"}, "tag": "proxy"},
            {"type": "direct", "tag": "direct"}
          ],
          "route": {
            "rules": [
              {"action": "sniff"},
              {"protocol": "dns", "action": "hijack-dns"},
              {"domain": ["tr.example.com"], "outbound": "direct"},
              {"ip_is_private": true, "outbound": "direct"},
              {"domain_suffix": ["ru", "xn--p1ai"], "outbound": "direct"}
            ],
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
            mode: RouteMode::All,
        }
        .sanitized()
        .unwrap();
        let cfg = build_config(&server(), 1080, &rules, &[]);
        let list = rules_of(&cfg);
        assert_eq!(list[0], json!({"protocol": "dns", "action": "hijack-dns"}));
        assert_eq!(list[2]["ip_is_private"], true);
        assert_eq!(
            list[3],
            json!({"process_name": ["steam.exe"], "outbound": "direct"})
        );
        assert_eq!(
            list[4],
            json!({"process_name": ["chrome.exe"], "outbound": "proxy"})
        );
        assert_eq!(
            list[5],
            json!({"domain_keyword": ["news"], "outbound": "proxy"})
        );
        assert_eq!(
            list[6],
            json!({"domain_suffix": ["ru", "xn--p1ai"], "outbound": "direct"})
        );
        assert_eq!(
            list[7],
            json!({"domain_suffix": ["example.com"], "outbound": "direct"})
        );
        assert_eq!(list[8], json!({"domain": ["a.b.c"], "outbound": "direct"}));
        assert_eq!(
            list[9],
            json!({"ip_cidr": ["10.0.0.0/8"], "outbound": "direct"})
        );
        assert!(list[10]["domain_suffix"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d == "ru"));
        assert_eq!(list.len(), 11);
        assert_eq!(cfg["route"]["find_process"], true);
        assert_eq!(cfg["route"]["final"], "proxy");
    }

    #[test]
    fn listens_only_on_loopback() {
        let cfg = build_config(&server(), 5, &RoutingRules::default(), &[]);
        assert_eq!(cfg["inbounds"][0]["listen"], "127.0.0.1");
        assert_eq!(cfg["inbounds"][0]["type"], "mixed");
    }

    /// Правила маршрута без первого `sniff` (он обязателен и проверяется отдельно).
    fn rules_of(cfg: &Value) -> &[Value] {
        let all = cfg["route"]["rules"].as_array().unwrap();
        assert_eq!(all[0], json!({"action": "sniff"}));
        &all[1..]
    }

    fn rules(mode: RouteMode) -> RoutingRules {
        RoutingRules {
            mode,
            ..RoutingRules::default()
        }
    }

    #[test]
    fn list_only_mode_sends_the_listed_sites_and_nothing_else_through_the_vpn() {
        let sites = vec!["youtube.com".to_string(), "discord.gg".to_string()];
        let mut r = rules(RouteMode::ListOnly);
        // Правила «напрямую» и наборы в этом режиме не нужны: остальное и так напрямую.
        r.direct_domains = vec!["example.org".into()];
        r.presets = vec!["ru-direct".into()];
        let r = r.sanitized().unwrap();
        let cfg = build_config(&server(), 1, &r, &sites);
        assert_eq!(cfg["route"]["final"], "direct");
        let list = rules_of(&cfg);
        assert_eq!(list[0], json!({"protocol": "dns", "action": "hijack-dns"}));
        assert_eq!(list[1]["outbound"], "direct", "адрес сервера напрямую");
        assert_eq!(list[2]["ip_is_private"], true);
        // Домен из списка и его поддомены (domain_suffix) идут через VPN.
        assert_eq!(
            list[3],
            json!({"domain_suffix": ["youtube.com", "discord.gg"], "outbound": "proxy"})
        );
        assert_eq!(list.len(), 4);
        // Всё остальное напрямую: итоговый выход direct, других правил «через VPN» нет.
        assert!(!cfg["route"]["rules"].to_string().contains("example.org"));
    }

    #[test]
    fn only_sites_has_rules_exactly_for_the_listed_domains_and_a_direct_final() {
        let sites: Vec<String> = ["youtube.com", "googlevideo.com", "ytimg.com"]
            .iter()
            .map(|d| d.to_string())
            .collect();
        let cfg = build_config(&server(), 1, &rules(RouteMode::ListOnly), &sites);
        assert_eq!(cfg["route"]["final"], "direct");
        let list = rules_of(&cfg);
        let proxied: Vec<&Value> = list.iter().filter(|r| r["outbound"] == "proxy").collect();
        assert_eq!(proxied.len(), 1);
        assert_eq!(
            proxied[0]["domain_suffix"],
            json!(["youtube.com", "googlevideo.com", "ytimg.com"]),
            "ровно домены списка, с поддоменами"
        );
        // Кроме списка через VPN не уходит ничего: ни по адресам, ни по ключевым словам.
        for key in ["ip_cidr", "domain_keyword", "domain", "geoip"] {
            assert!(proxied[0].get(key).is_none(), "{key}");
        }
        // Адрес сервера и локальные адреса напрямую, и они раньше правила списка.
        let first_proxy = list.iter().position(|r| r["outbound"] == "proxy").unwrap();
        for (i, r) in list.iter().enumerate().take(first_proxy) {
            assert!(
                r["outbound"] == "direct" || r["action"] == "hijack-dns",
                "{i}: {r}"
            );
        }
        assert!(list.iter().any(|r| r["ip_is_private"] == true));
        assert!(list
            .iter()
            .any(|r| r["domain"] == json!(["tr.example.com"]) && r["outbound"] == "direct"));
    }

    /// Какой сервер имён ответит на `name`: первое подходящее правило, иначе `final`.
    fn dns_server_for(cfg: &Value, name: &str) -> String {
        let matches = |rule: &Value| {
            let hit = |key: &str, f: &dyn Fn(&str) -> bool| {
                rule.get(key)
                    .and_then(Value::as_array)
                    .is_some_and(|l| l.iter().filter_map(Value::as_str).any(f))
            };
            hit("domain", &|d| d == name)
                || hit("domain_suffix", &|d| {
                    name == d || name.ends_with(&format!(".{d}"))
                })
        };
        let rule = cfg["dns"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| matches(r));
        let tag = rule.map_or(&cfg["dns"]["final"], |r| &r["server"]);
        tag.as_str().unwrap().to_string()
    }

    #[test]
    fn only_sites_always_has_a_name_server_for_listed_and_other_domains() {
        let sites = vec!["youtube.com".to_string()];
        let cfg = build_config(&server(), 1, &rules(RouteMode::ListOnly), &sites);
        let servers: Vec<&str> = cfg["dns"]["servers"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| s["tag"].as_str())
            .collect();
        for (name, expected) in [
            ("youtube.com", "remote"),
            ("www.youtube.com", "remote"),
            ("example.org", "local"),
            ("tr.example.com", "local"),
        ] {
            let server = dns_server_for(&cfg, name);
            assert_eq!(server, expected, "{name}");
            assert!(
                servers.contains(&server.as_str()),
                "{name}: нет сервера {server}"
            );
        }
        // Правила не исключают друг друга: у каждого правила есть существующий сервер.
        for rule in cfg["dns"]["rules"].as_array().unwrap() {
            assert!(
                servers.contains(&rule["server"].as_str().unwrap()),
                "{rule}"
            );
        }
    }

    #[test]
    fn the_tunnel_can_see_names_in_connections_by_address_and_dns_queries() {
        for mode in [RouteMode::ListOnly, RouteMode::All] {
            let cfg = build_config(&server(), 1, &rules(mode), &["youtube.com".to_string()]);
            let all = cfg["route"]["rules"].as_array().unwrap();
            // sniff самым первым, затем перехват DNS: иначе правила по доменам не видят имён.
            assert_eq!(all[0], json!({"action": "sniff"}), "{mode:?}");
            assert_eq!(
                all[1],
                json!({"protocol": "dns", "action": "hijack-dns"}),
                "{mode:?}"
            );
            assert_eq!(cfg["dns"]["reverse_mapping"], true, "{mode:?}");
        }
    }

    #[test]
    fn list_only_resolves_listed_names_through_the_vpn_and_the_rest_locally() {
        let sites = vec!["youtube.com".to_string()];
        let cfg = build_config(&server(), 1, &rules(RouteMode::ListOnly), &sites);
        assert_eq!(cfg["dns"]["final"], "local");
        assert_eq!(cfg["dns"]["rules"][0]["server"], "local");
        assert_eq!(
            cfg["dns"]["rules"][1],
            json!({"domain_suffix": ["youtube.com"], "server": "remote"})
        );
    }

    #[test]
    fn list_only_never_sends_the_vpn_server_itself_through_the_vpn() {
        let sites = vec!["tr.example.com".to_string()];
        let cfg = build_config(&server(), 1, &rules(RouteMode::ListOnly), &sites);
        let list = rules_of(&cfg);
        let direct = list
            .iter()
            .position(|r| r["outbound"] == "direct" && r.get("domain").is_some())
            .expect("правило адреса сервера");
        let listed = list
            .iter()
            .position(|r| r["outbound"] == "proxy")
            .expect("правило списка");
        assert!(direct < listed, "адрес сервера раньше списка сайтов");
    }

    #[test]
    fn list_only_with_an_empty_list_sends_nothing_through_the_vpn() {
        let cfg = build_config(&server(), 1, &rules(RouteMode::ListOnly), &[]);
        assert_eq!(cfg["route"]["final"], "direct");
        // Только DNS, адрес сервера и локальные адреса: через VPN ничего.
        assert_eq!(cfg["route"]["rules"].as_array().unwrap().len(), 4);
        assert!(!cfg["route"]["rules"].to_string().contains("\"proxy\""));
    }

    #[test]
    fn all_mode_sends_everything_through_the_vpn_except_local_addresses_and_exceptions() {
        let mut r = rules(RouteMode::All);
        r.direct_domains = vec!["example.org".into()];
        let r = r.sanitized().unwrap();
        // Список сайтов в этом режиме не используется.
        let cfg = build_config(&server(), 1, &r, &["youtube.com".to_string()]);
        assert_eq!(cfg["route"]["final"], "proxy");
        let text = cfg["route"]["rules"].to_string();
        assert!(text.contains("example.org") && !text.contains("youtube.com"));
        assert_eq!(cfg["route"]["rules"][3]["ip_is_private"], true);
    }

    #[test]
    fn the_only_area_is_the_whole_computer_and_old_saved_values_read_as_that() {
        assert_eq!(RoutingRules::default().mode, RouteMode::All);
        for old in [
            r#"{"mode":"direct"}"#,
            r#"{"mode":"listOnly"}"#,
            r#"{"mode":"all"}"#,
        ] {
            let rules: RoutingRules = serde_json::from_str(old).unwrap();
            assert_eq!(rules.mode, RouteMode::All, "{old}");
        }
    }

    #[test]
    fn whole_computer_sends_russian_zones_direct_for_routes_and_names() {
        let mut r = rules(RouteMode::All);
        r.proxy_domains = vec!["example.org".into()];
        let cfg = build_config(&server(), 1, &r, &[]);
        let route = cfg["route"]["rules"].to_string();
        assert!(
            route.contains(r#""domain_suffix":["ru","xn--p1ai"]"#),
            "{route}"
        );
        // Явное «через VPN» для домена сильнее зоны: его правило стоит раньше.
        let proxy_at = route.find("example.org").unwrap();
        let zone_at = route.find("xn--p1ai").unwrap();
        assert!(proxy_at < zone_at);
        let dns = cfg["dns"]["rules"].to_string();
        assert!(
            dns.contains(r#""domain_suffix":["ru","xn--p1ai"],"server":"local""#),
            "{dns}"
        );
    }

    #[test]
    fn both_areas_use_the_tunnel() {
        let all = build_config(&server(), 7, &rules(RouteMode::All), &[]);
        let inbounds = all["inbounds"].as_array().unwrap();
        assert_eq!(inbounds.len(), 2);
        assert_eq!(inbounds[0]["type"], "mixed");
        assert_eq!(inbounds[0]["listen"], "127.0.0.1");
        assert_eq!(inbounds[1]["type"], "tun");
        assert_eq!(inbounds[1]["auto_route"], true);
        assert_eq!(inbounds[1]["address"], json!([TUN_ADDRESS, TUN_ADDRESS_V6]));
        assert_eq!(
            all["route"]["rules"][1],
            json!({"protocol": "dns", "action": "hijack-dns"})
        );
        let sites = build_config(&server(), 7, &rules(RouteMode::ListOnly), &[]);
        assert_eq!(sites["inbounds"].as_array().unwrap().len(), 2);
        assert_eq!(sites["inbounds"][1]["type"], "tun");
        assert_eq!(sites["inbounds"][1]["auto_route"], true);
    }

    #[test]
    fn the_whole_computer_routes_everything_through_the_tunnel_except_the_server_and_local_addresses(
    ) {
        let all = build_config(&server(), 7, &rules(RouteMode::All), &[]);
        // Маршрут по умолчанию через туннель: автомаршрут и конечный выход через VPN.
        assert_eq!(all["inbounds"][1]["auto_route"], true);
        assert_eq!(all["inbounds"][1]["strict_route"], true);
        assert_eq!(all["route"]["final"], "proxy");
        assert_eq!(all["route"]["auto_detect_interface"], true);
        let list = rules_of(&all);
        // Имена из туннеля обрабатывает ядро; адрес сервера и локальные адреса напрямую.
        assert_eq!(list[0], json!({"protocol": "dns", "action": "hijack-dns"}));
        assert_eq!(
            list[1],
            json!({"domain": ["tr.example.com"], "outbound": "direct"})
        );
        assert_eq!(list[2]["ip_is_private"], true);
        // Запросы имён идут к публичному серверу через VPN; имя сервера VPN — локально.
        assert_eq!(all["dns"]["final"], "remote");
        assert_eq!(all["dns"]["servers"][1]["detour"], "proxy");
        assert_eq!(
            all["dns"]["rules"][0],
            json!({"domain": ["tr.example.com"], "server": "local"})
        );
    }

    #[test]
    fn a_server_given_by_address_is_routed_directly_by_address_and_needs_no_name_lookup_rule() {
        let mut ip_server = server();
        ip_server.host = "203.0.113.7".into();
        let all = build_config(&ip_server, 7, &rules(RouteMode::All), &[]);
        assert_eq!(
            all["route"]["rules"][2],
            json!({"ip_cidr": ["203.0.113.7/32"], "outbound": "direct"})
        );
        // В DNS остаётся только правило российских зон (локальные имена).
        assert_eq!(all["dns"]["rules"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn exceptions_stay_local_for_names_too() {
        let mut r = rules(RouteMode::All);
        r.direct_domains = vec!["example.org".into(), "10.0.0.0/8".into()];
        r.presets = vec!["ru-direct".into()];
        let r = r.sanitized().unwrap();
        let all = build_config(&server(), 7, &r, &[]);
        let text = all["dns"]["rules"].to_string();
        assert!(text.contains("example.org") && text.contains("\"server\":\"local\""));
        assert!(
            !text.contains("10.0.0.0"),
            "адресные исключения в правила имён не попадают"
        );
    }
}
