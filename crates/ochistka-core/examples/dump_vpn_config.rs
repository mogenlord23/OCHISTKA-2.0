//! Записывает конфигурации ядра для обеих областей (вымышленный сервер, не подписка владельца), чтобы
//! в CI проверить их командой `sing-box check` без запуска ядра. Запуск:
//! `cargo run -p ochistka-core --example dump_vpn_config -- <папка>`.

use ochistka_core::vpn::config::build_config;
use ochistka_core::vpn::parse::parse_link;
use ochistka_core::vpn::routes::{RouteMode, RoutingRules};

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("укажите папку для файлов конфигурации");
    std::fs::create_dir_all(&dir).expect("папка недоступна");
    let links = [
        ("trojan", "trojan://pw@tr.example.com:443?sni=tr.example.com#Test"),
        (
            "vless-ip",
            "vless://11111111-2222-3333-4444-555555555555@203.0.113.7:443?security=tls&sni=cdn.example.com&type=tcp#Ip",
        ),
    ];
    let sites = vec!["youtube.com".to_string(), "discord.gg".to_string()];
    for (name, link) in links {
        let server = parse_link(link).expect("вымышленная ссылка разобрана");
        for (area, mode) in [("all", RouteMode::All), ("sites", RouteMode::ListOnly)] {
            let rules = RoutingRules {
                direct_domains: vec!["example.org".into()],
                mode,
                ..RoutingRules::default()
            }
            .sanitized()
            .expect("правила");
            let config = build_config(&server, 20808, &rules, &sites);
            let path = std::path::Path::new(&dir).join(format!("{name}-{area}.json"));
            std::fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
            println!("{}", path.display());
        }
    }
}
