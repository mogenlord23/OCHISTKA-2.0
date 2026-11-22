//! Минимальная совместимость с подписками в формате Xray JSON (массив конфигов с `outbounds`,
//! `protocol`, `settings`, `streamSettings`): серверы преобразуются в описание sing-box.
//! Поддержано: vless, vmess, trojan, shadowsocks; транспорт tcp, ws, grpc, httpupgrade;
//! защита none, tls, reality. Остальное пропускается с причиной (без значений секретов).

use serde_json::{json, Map, Value};

use super::parse::{is_uuid, transport_value};

fn text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn port_of(v: &Value) -> Option<u16> {
    text(v)?.parse::<u16>().ok().filter(|p| *p != 0)
}

/// Берёт `settings.vnext[0]` (vless, vmess) или `settings.servers[0]` (trojan, shadowsocks).
fn first<'a>(settings: &'a Value, key: &str) -> Option<&'a Value> {
    settings.get(key)?.as_array()?.first()
}

/// Преобразует один исходящий Xray. Имя приходит снаружи (`remarks` конфига или `tag`).
pub fn convert(item: &Value, name: &str) -> Result<(String, Value), String> {
    let protocol = item["protocol"]
        .as_str()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let settings = &item["settings"];
    let mut out = Map::new();
    let (host, port): (String, u16);
    match protocol.as_str() {
        "vless" | "vmess" => {
            let node = first(settings, "vnext").ok_or("нет описания сервера")?;
            host = text(&node["address"]).ok_or("нет адреса сервера")?;
            port = port_of(&node["port"]).ok_or("неверный порт")?;
            let user = node["users"]
                .as_array()
                .and_then(|u| u.first())
                .ok_or("нет ключа доступа")?;
            let uuid = text(&user["id"]).ok_or("нет UUID")?;
            if !is_uuid(&uuid) {
                return Err("неверный UUID".into());
            }
            out.insert("type".into(), json!(protocol));
            out.insert("uuid".into(), json!(uuid));
            if protocol == "vless" {
                if let Some(flow) = text(&user["flow"]) {
                    out.insert("flow".into(), json!(flow));
                    out.insert("packet_encoding".into(), json!("xudp"));
                }
                if matches!(text(&user["encryption"]).as_deref(), Some(e) if e != "none") {
                    return Err("шифрование VLESS (encryption) не поддерживается".into());
                }
            } else {
                out.insert(
                    "security".into(),
                    json!(text(&user["security"]).unwrap_or_else(|| "auto".into())),
                );
                out.insert(
                    "alter_id".into(),
                    json!(text(&user["alterId"])
                        .and_then(|a| a.parse::<u32>().ok())
                        .unwrap_or(0)),
                );
            }
        }
        "trojan" | "shadowsocks" => {
            let node = first(settings, "servers").ok_or("нет описания сервера")?;
            host = text(&node["address"]).ok_or("нет адреса сервера")?;
            port = port_of(&node["port"]).ok_or("неверный порт")?;
            let password = text(&node["password"]).ok_or("нет пароля")?;
            out.insert("type".into(), json!(protocol));
            out.insert("password".into(), json!(password));
            if protocol == "shadowsocks" {
                let method = text(&node["method"]).ok_or("нет метода шифрования")?;
                out.insert("method".into(), json!(method));
            }
        }
        "hysteria" | "hysteria2" => {
            // Xray: адрес в settings, пароль — в streamSettings.hysteriaSettings.auth.
            // Версию не угадываем: Hysteria 2 берём только если она явно указана в конфигурации.
            let version = text(&settings["version"])
                .or_else(|| text(&item["streamSettings"]["hysteriaSettings"]["version"]));
            match version.as_deref() {
                Some("2") => {}
                Some(other) => {
                    return Err(format!(
                        "Hysteria версии {other} не поддерживается (нужна 2)"
                    ))
                }
                None if protocol == "hysteria2" => {}
                None => return Err("версия Hysteria не указана, определить протокол нельзя".into()),
            }
            host = text(&settings["address"]).ok_or("нет адреса сервера")?;
            port = port_of(&settings["port"]).ok_or("неверный порт")?;
            let password = text(&item["streamSettings"]["hysteriaSettings"]["auth"])
                .ok_or("нет пароля (auth)")?;
            out.insert("type".into(), json!("hysteria2"));
            out.insert("password".into(), json!(password));
        }
        other => return Err(format!("протокол {other} не поддерживается")),
    }
    out.insert("server".into(), json!(host));
    out.insert("server_port".into(), json!(port));
    apply_stream(
        &mut out,
        &item["streamSettings"],
        &host,
        protocol == "trojan",
    )?;
    Ok((name.to_string(), Value::Object(out)))
}

fn apply_stream(
    out: &mut Map<String, Value>,
    stream: &Value,
    host: &str,
    tls_by_default: bool,
) -> Result<(), String> {
    let empty = json!({});
    let stream = if stream.is_object() { stream } else { &empty };
    let security = text(&stream["security"])
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| {
            if tls_by_default {
                "tls".into()
            } else {
                "none".into()
            }
        });
    match security.as_str() {
        "none" | "" => {}
        "tls" | "reality" => {
            let src = if security == "reality" {
                &stream["realitySettings"]
            } else {
                &stream["tlsSettings"]
            };
            let mut tls = json!({"enabled": true});
            if let Some(sni) = text(&src["serverName"]) {
                tls["server_name"] = json!(sni);
            } else if host.parse::<std::net::IpAddr>().is_err() {
                tls["server_name"] = json!(host);
            }
            if src["allowInsecure"].as_bool() == Some(true) {
                tls["insecure"] = json!(true);
            }
            if let Some(alpn) = src["alpn"].as_array() {
                let list: Vec<&str> = alpn.iter().filter_map(Value::as_str).collect();
                if !list.is_empty() {
                    tls["alpn"] = json!(list);
                }
            }
            let fp = text(&src["fingerprint"]).or(if security == "reality" {
                Some("chrome".into())
            } else {
                None
            });
            if let Some(fp) = fp {
                tls["utls"] = json!({"enabled": true, "fingerprint": fp});
            }
            if security == "reality" {
                let key = text(&src["publicKey"]).ok_or("REALITY без публичного ключа")?;
                tls["reality"] = json!({
                    "enabled": true,
                    "public_key": key,
                    "short_id": text(&src["shortId"]).unwrap_or_default(),
                });
            }
            out.insert("tls".into(), tls);
        }
        other => return Err(format!("security={other} не поддерживается")),
    }
    let network = text(&stream["network"])
        .unwrap_or_else(|| "tcp".into())
        .to_ascii_lowercase();
    if matches!(network.as_str(), "hysteria" | "hysteria2") {
        return Ok(());
    }
    let (path, host_header, service) = match network.as_str() {
        "ws" | "websocket" => {
            let s = &stream["wsSettings"];
            (
                text(&s["path"]),
                text(&s["host"]).or_else(|| text(&s["headers"]["Host"])),
                None,
            )
        }
        "httpupgrade" => {
            let s = &stream["httpupgradeSettings"];
            (text(&s["path"]), text(&s["host"]), None)
        }
        "grpc" => (None, None, text(&stream["grpcSettings"]["serviceName"])),
        _ => (None, None, None),
    };
    if let Some(t) = transport_value(
        &network,
        path.as_deref(),
        host_header.as_deref(),
        service.as_deref(),
    )? {
        out.insert("transport".into(), t);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn vless_reality_grpc_and_ws() {
        let item = json!({"protocol":"vless","settings":{"vnext":[{"address":"h.example.com","port":443,
            "users":[{"id":"b831381d-6324-4d53-ad4f-8cda48b30811","flow":"xtls-rprx-vision","encryption":"none"}]}]},
            "streamSettings":{"network":"tcp","security":"reality","realitySettings":{"serverName":"cdn.example.com","fingerprint":"chrome","publicKey":"PUBKEY","shortId":"ab"}}});
        let (name, o) = convert(&item, "NL").unwrap();
        assert_eq!(name, "NL");
        assert_eq!(o["type"], "vless");
        assert_eq!(o["flow"], "xtls-rprx-vision");
        assert_eq!(o["tls"]["reality"]["public_key"], "PUBKEY");
        assert_eq!(o["tls"]["server_name"], "cdn.example.com");
        assert!(o.get("transport").is_none());

        let ws = json!({"protocol":"vless","settings":{"vnext":[{"address":"h.example.com","port":"443",
            "users":[{"id":"b831381d-6324-4d53-ad4f-8cda48b30811"}]}]},
            "streamSettings":{"network":"ws","security":"tls","wsSettings":{"path":"/p","headers":{"Host":"w.example.com"}}}});
        let (_, o) = convert(&ws, "").unwrap();
        assert_eq!(o["transport"]["type"], "ws");
        assert_eq!(o["transport"]["headers"]["Host"], "w.example.com");
        assert_eq!(o["server_port"], 443);
    }

    #[test]
    fn trojan_ss_and_vmess() {
        let t = json!({"protocol":"trojan","settings":{"servers":[{"address":"t.example.com","port":443,"password":"pw"}]}});
        let (_, o) = convert(&t, "T").unwrap();
        assert_eq!(o["type"], "trojan");
        assert_eq!(o["tls"]["server_name"], "t.example.com");
        let s = json!({"protocol":"shadowsocks","settings":{"servers":[{"address":"s.example.com","port":8388,"method":"aes-256-gcm","password":"pw"}]}});
        assert_eq!(convert(&s, "S").unwrap().1["method"], "aes-256-gcm");
        let v = json!({"protocol":"vmess","settings":{"vnext":[{"address":"v.example.com","port":443,
            "users":[{"id":"b831381d-6324-4d53-ad4f-8cda48b30811","alterId":0,"security":"auto"}]}]}});
        assert_eq!(convert(&v, "V").unwrap().1["type"], "vmess");
    }

    #[test]
    fn unsupported_things_are_reported_without_secrets() {
        let xhttp = json!({"protocol":"vless","settings":{"vnext":[{"address":"h.example.com","port":443,
            "users":[{"id":"b831381d-6324-4d53-ad4f-8cda48b30811"}]}]},"streamSettings":{"network":"xhttp","security":"tls"}});
        let err = convert(&xhttp, "").unwrap_err();
        assert!(err.contains("xhttp"));
        assert!(!err.contains("b831381d"));
        assert!(convert(&json!({"protocol":"wireguard"}), "").is_err());
        let bad = json!({"protocol":"vless","settings":{"vnext":[{"address":"h","port":443,"users":[{"id":"nope"}]}]}});
        assert_eq!(convert(&bad, "").unwrap_err(), "неверный UUID");
    }

    /// Вымышленная подписка той же структуры, что у реального сервиса: 10 конфигов Xray JSON,
    /// 9 VLESS (6 gRPC + REALITY, 3 TCP) и 1 Hysteria; в каждом ещё freedom и blackhole.
    pub(crate) fn fictional_subscription() -> String {
        let mut configs = Vec::new();
        for i in 0..9 {
            let grpc = i < 6;
            let stream = if grpc {
                json!({"network":"grpc","security":"reality",
                    "grpcSettings":{"serviceName":"svc","multiMode":true},
                    "realitySettings":{"serverName":"cdn.example.com","fingerprint":"chrome",
                        "publicKey":"FAKEPUBLICKEY","shortId":"ab"}})
            } else {
                json!({"network":"tcp","security":"tls","tlsSettings":{"serverName":"t.example.com"}})
            };
            configs.push(json!({"remarks": format!("Сервер {i}"), "outbounds":[
                {"protocol":"vless","tag":"proxy","settings":{"vnext":[{"address":format!("h{i}.example.com"),
                    "port":443,"users":[{"id":"b831381d-6324-4d53-ad4f-8cda48b30811","encryption":"none"}]}]},
                 "streamSettings": stream},
                {"protocol":"freedom","tag":"direct"},
                {"protocol":"blackhole","tag":"block"}]}));
        }
        configs.push(json!({"remarks":"Hy","outbounds":[
            {"protocol":"hysteria","tag":"proxy","settings":{"version":2,"address":"hy.example.com","port":8443},
             "streamSettings":{"network":"hysteria","security":"tls","tlsSettings":{"serverName":"hy.example.com","alpn":["h3"]},
                "hysteriaSettings":{"version":2,"auth":"FAKEAUTH"}}},
            {"protocol":"freedom","tag":"direct"},
            {"protocol":"blackhole","tag":"block"}]}));
        Value::Array(configs).to_string()
    }

    #[test]
    fn realistic_ten_config_subscription_gives_ten_servers_and_no_service_outbounds() {
        let report = crate::vpn::parse::parse_subscription(&fictional_subscription()).unwrap();
        assert_eq!(report.servers.len(), 10);
        assert!(report.issues.is_empty(), "{:?}", report.issues);
        let kinds: Vec<&str> = report.servers.iter().map(|s| s.protocol.as_str()).collect();
        assert_eq!(kinds.iter().filter(|k| **k == "vless").count(), 9);
        assert_eq!(kinds.iter().filter(|k| **k == "hysteria2").count(), 1);
        assert!(report.servers.iter().all(|s| !matches!(
            s.protocol.as_str(),
            "direct" | "block" | "freedom" | "blackhole"
        )));
        let grpc = &report.servers[0].outbound;
        assert_eq!(grpc["transport"]["type"], "grpc");
        assert_eq!(grpc["transport"]["service_name"], "svc");
        assert_eq!(grpc["tls"]["reality"]["public_key"], "FAKEPUBLICKEY");
        assert_eq!(grpc["tls"]["utls"]["fingerprint"], "chrome");
        let tcp = &report.servers[6].outbound;
        assert!(tcp.get("transport").is_none());
        assert_eq!(tcp["tls"]["server_name"], "t.example.com");
        let hy = &report.servers[9].outbound;
        assert_eq!(hy["type"], "hysteria2");
        assert_eq!(hy["password"], "FAKEAUTH");
        assert_eq!(hy["server_port"], 8443);
        assert_eq!(hy["tls"]["alpn"][0], "h3");
        assert_eq!(report.servers[9].name, "Hy");
    }

    #[test]
    fn imported_servers_build_a_core_config_with_the_chosen_outbound() {
        let report = crate::vpn::parse::parse_subscription(&fictional_subscription()).unwrap();
        for server in [&report.servers[0], &report.servers[6], &report.servers[9]] {
            let cfg = crate::vpn::config::build_config(server, 17890, &Default::default(), &[]);
            let text = cfg.to_string();
            assert!(text.contains(&server.host));
            let proxy = cfg["outbounds"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["server"] == server.host.as_str())
                .expect("outbound сервера в конфиге");
            assert_eq!(proxy["type"], server.outbound["type"]);
        }
    }

    #[test]
    fn hysteria_version_one_and_missing_auth_are_reported() {
        let v1 = json!({"protocol":"hysteria","settings":{"version":1,"address":"h","port":443}});
        assert!(convert(&v1, "").unwrap_err().contains("версии 1"));
        // Версия не указана: не считаем её второй.
        let unknown = json!({"protocol":"hysteria","settings":{"address":"h","port":443},
            "streamSettings":{"hysteriaSettings":{"auth":"X"}}});
        assert!(convert(&unknown, "")
            .unwrap_err()
            .contains("версия Hysteria не указана"));
        // Версия 2 подтверждена в streamSettings.
        let in_stream = json!({"protocol":"hysteria","settings":{"address":"h.example.com","port":443},
            "streamSettings":{"hysteriaSettings":{"version":2,"auth":"X"}}});
        assert_eq!(convert(&in_stream, "").unwrap().1["type"], "hysteria2");
        // Явный protocol hysteria2 — версия 2 по определению.
        let explicit = json!({"protocol":"hysteria2","settings":{"address":"h.example.com","port":443},
            "streamSettings":{"hysteriaSettings":{"auth":"X"}}});
        assert_eq!(convert(&explicit, "").unwrap().1["type"], "hysteria2");
        // VLESS не затронут.
        let vless = json!({"protocol":"vless","settings":{"vnext":[{"address":"v.example.com","port":443,
            "users":[{"id":"b831381d-6324-4d53-ad4f-8cda48b30811"}]}]}});
        assert_eq!(convert(&vless, "").unwrap().1["type"], "vless");
        let none = json!({"protocol":"hysteria","settings":{"version":2,"address":"h","port":443}});
        assert!(convert(&none, "").unwrap_err().contains("нет пароля"));
    }
}
