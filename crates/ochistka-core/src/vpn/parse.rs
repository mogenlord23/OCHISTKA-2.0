//! Разбор подписок и ссылок серверов в исходящие подключения sing-box.
//! Поддержано: `vless://`, `vmess://`, `trojan://`, `ss://`, `hysteria2://` (`hy2://`), `tuic://`;
//! тело подписки: список ссылок (обычный или base64), JSON sing-box. Код ничего не запускает и
//! не обращается к сети.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use super::country::country_code;
use super::VpnError;

/// Сервер подписки. `outbound` — готовое описание для sing-box (без `tag`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    /// `vless`, `vmess`, `trojan`, `shadowsocks`, `hysteria2`, `tuic`.
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub country: Option<String>,
    pub outbound: Value,
}

/// Итог разбора: найденные серверы и причины пропуска остальных строк.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportReport {
    pub servers: Vec<Server>,
    pub issues: Vec<String>,
}

/// Допустимый размер текста подписки (байты).
pub const MAX_BODY: usize = 5 * 1024 * 1024;

const PROXY_TYPES: &[&str] = &[
    "vless",
    "vmess",
    "trojan",
    "shadowsocks",
    "hysteria2",
    "tuic",
];

/// Разбирает текст подписки. Ошибка, если ни одного сервера не найдено.
pub fn parse_subscription(text: &str) -> Result<ImportReport, VpnError> {
    if text.len() > MAX_BODY {
        return Err(VpnError::Parse("Подписка слишком большая".into()));
    }
    let body = text.trim_matches(|c: char| c == '\u{feff}' || c.is_whitespace());
    if body.is_empty() {
        return Err(VpnError::EmptyInput);
    }
    let mut report = parse_body(body, true)?;
    dedupe(&mut report.servers);
    if report.servers.is_empty() {
        return Err(VpnError::Parse(explain_empty(&report.issues)));
    }
    Ok(report)
}

/// Понятное объяснение, почему серверов нет: формат Clash, все записи пропущены или формат неизвестен.
/// Причины группируются (`причина ×N`) и не содержат значений ссылок: только короткие пояснения.
fn explain_empty(issues: &[String]) -> String {
    if issues.iter().any(|i| i.starts_with("Clash")) {
        return "Сервис отдал подписку в формате Clash. Этот формат пока не поддерживается: нужен список ссылок или JSON (sing-box, Xray)".into();
    }
    if issues.is_empty() {
        return "В подписке нет поддерживаемых серверов: ответ получен, но формат не распознан (ждём ссылки vless, vmess, trojan, ss, hysteria2, tuic или JSON sing-box, Xray)".into();
    }
    let mut groups: Vec<(String, usize)> = Vec::new();
    for issue in issues {
        let clean = super::mask::mask_secrets(issue);
        match groups.iter_mut().find(|(t, _)| *t == clean) {
            Some((_, n)) => *n += 1,
            None => groups.push((clean, 1)),
        }
    }
    let list = groups
        .iter()
        .take(3)
        .map(|(t, n)| {
            if *n > 1 {
                format!("{t} ×{n}")
            } else {
                t.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("Серверы найдены, но ни один не подошёл. Причины: {list}")
}

fn parse_body(body: &str, allow_base64: bool) -> Result<ImportReport, VpnError> {
    if body.starts_with('{') || body.starts_with('[') {
        return Ok(parse_json(body));
    }
    if looks_like_link_list(body) {
        return Ok(parse_lines(body));
    }
    if body.starts_with("proxies:") || body.contains("\nproxies:") {
        return Ok(ImportReport {
            servers: vec![],
            issues: vec!["Clash YAML не поддерживается: нужна подписка со ссылками".into()],
        });
    }
    if allow_base64 {
        if let Some(bytes) = decode_base64(body) {
            if let Ok(text) = String::from_utf8(bytes) {
                let inner = text.trim_matches(|c: char| c == '\u{feff}' || c.is_whitespace());
                if !inner.is_empty() {
                    return parse_body(inner, false);
                }
            }
        }
    }
    if body.contains('<') && body.to_ascii_lowercase().contains("<html") {
        return Err(VpnError::Parse(
            "Ссылка открывает веб-страницу, а не подписку. Скопируй ссылку подписки из личного кабинета".into(),
        ));
    }
    Ok(ImportReport::default())
}

fn looks_like_link_list(body: &str) -> bool {
    body.lines().any(|l| {
        let l = l.trim();
        !l.starts_with('#') && l.contains("://") && l.find("://").is_some_and(|i| i < 16)
    })
}

fn parse_lines(body: &str) -> ImportReport {
    let mut report = ImportReport::default();
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains("://") {
            continue;
        }
        match parse_link(line) {
            Ok(server) => report.servers.push(server),
            Err(why) => report.issues.push(why),
        }
    }
    report
}

/// Разбирает одну ссылку сервера.
pub fn parse_link(link: &str) -> Result<Server, String> {
    let link = link.trim();
    let scheme = link
        .split("://")
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let label = |why: &str| format!("{scheme}: {why}");
    let (name, outbound) = match scheme.as_str() {
        "vless" => parse_userinfo_link(link, "vless"),
        "trojan" => parse_userinfo_link(link, "trojan"),
        "hysteria2" | "hy2" => parse_userinfo_link(link, "hysteria2"),
        "tuic" => parse_userinfo_link(link, "tuic"),
        "vmess" => parse_vmess(link),
        "ss" => parse_ss(link),
        other => Err(format!("протокол {other} не поддерживается")),
    }
    .map_err(|why| label(&why))?;
    finish_server(name, outbound).map_err(|why| label(&why))
}

fn finish_server(name: String, outbound: Value) -> Result<Server, String> {
    let host = outbound["server"].as_str().unwrap_or_default().to_string();
    let port = outbound["server_port"].as_u64().unwrap_or(0);
    if host.is_empty() {
        return Err("нет адреса сервера".into());
    }
    if port == 0 || port > 65535 {
        return Err("неверный порт".into());
    }
    let protocol = outbound["type"].as_str().unwrap_or_default().to_string();
    let name = if name.trim().is_empty() {
        host.clone()
    } else {
        name.trim().to_string()
    };
    let mut hasher = Sha256::new();
    hasher.update(outbound.to_string().as_bytes());
    let id: String = hasher
        .finalize()
        .iter()
        .take(6)
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(Server {
        id,
        country: country_code(&name),
        name,
        protocol,
        host,
        port: port as u16,
        outbound,
    })
}

fn dedupe(servers: &mut Vec<Server>) {
    let mut seen = HashSet::new();
    servers.retain(|s| seen.insert(s.id.clone()));
}

// ---------- ссылки вида scheme://user@host:port?query#name ----------

struct Link {
    userinfo: String,
    host: String,
    port: Option<u16>,
    query: Vec<(String, String)>,
    fragment: String,
}

impl Link {
    fn q(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.is_empty())
    }
}

fn split_link(link: &str) -> Result<Link, String> {
    let rest = link.split_once("://").ok_or("нет схемы")?.1;
    let (rest, fragment) = match rest.split_once('#') {
        Some((a, b)) => (a, percent_decode(b)),
        None => (rest, String::new()),
    };
    let (authority_path, query_text) = match rest.split_once('?') {
        Some((a, b)) => (a, b),
        None => (rest, ""),
    };
    let authority = authority_path.split('/').next().unwrap_or_default();
    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((u, h)) => (percent_decode(u), h),
        None => (String::new(), authority),
    };
    let (host, port) = split_host_port(hostport)?;
    let query = query_text
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| match p.split_once('=') {
            Some((k, v)) => (percent_decode(k), percent_decode(v)),
            None => (percent_decode(p), String::new()),
        })
        .collect();
    Ok(Link {
        userinfo,
        host,
        port,
        query,
        fragment,
    })
}

fn split_host_port(text: &str) -> Result<(String, Option<u16>), String> {
    if let Some(rest) = text.strip_prefix('[') {
        let (host, tail) = rest.split_once(']').ok_or("неверный IPv6-адрес")?;
        if host.is_empty() {
            return Err("пустой адрес".into());
        }
        let port = match tail.strip_prefix(':') {
            Some(p) => Some(parse_port(p)?),
            None if tail.is_empty() => None,
            None => return Err("неверный адрес".into()),
        };
        return Ok((host.to_string(), port));
    }
    match text.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => {
            check_host(host)?;
            Ok((host.to_string(), Some(parse_port(port)?)))
        }
        Some(_) => Err("IPv6-адрес должен быть в квадратных скобках".into()),
        None => {
            check_host(text)?;
            Ok((text.to_string(), None))
        }
    }
}

fn check_host(host: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err("пустой адрес".into());
    }
    if host
        .chars()
        .any(|c| c.is_whitespace() || c == '"' || c == '\\')
    {
        return Err("неверный адрес".into());
    }
    Ok(())
}

fn parse_port(text: &str) -> Result<u16, String> {
    text.trim()
        .parse::<u16>()
        .ok()
        .filter(|p| *p != 0)
        .ok_or_else(|| "неверный порт".to_string())
}

fn parse_userinfo_link(link: &str, kind: &str) -> Result<(String, Value), String> {
    let l = split_link(link)?;
    let port = l.port.ok_or("не указан порт")?;
    if l.userinfo.is_empty() {
        return Err("нет ключа доступа".into());
    }
    let mut out = Map::new();
    out.insert("type".into(), json!(kind));
    out.insert("server".into(), json!(l.host));
    out.insert("server_port".into(), json!(port));
    match kind {
        "vless" => {
            if !is_uuid(&l.userinfo) {
                return Err("неверный UUID".into());
            }
            out.insert("uuid".into(), json!(l.userinfo));
            if let Some(flow) = l.q("flow") {
                out.insert("flow".into(), json!(flow));
                out.insert("packet_encoding".into(), json!("xudp"));
            }
            if matches!(l.q("encryption"), Some(e) if e != "none") {
                return Err("шифрование VLESS (encryption) не поддерживается".into());
            }
            apply_tls(&mut out, &l, false)?;
            apply_transport(&mut out, &l, "type")?;
        }
        "trojan" => {
            out.insert("password".into(), json!(l.userinfo));
            apply_tls(&mut out, &l, true)?;
            apply_transport(&mut out, &l, "type")?;
        }
        "hysteria2" => {
            out.insert("password".into(), json!(l.userinfo));
            if let Some(obfs) = l.q("obfs") {
                if obfs != "salamander" {
                    return Err(format!("obfs {obfs} не поддерживается"));
                }
                out.insert(
                    "obfs".into(),
                    json!({"type":"salamander","password": l.q("obfs-password").unwrap_or_default()}),
                );
            }
            apply_tls(&mut out, &l, true)?;
        }
        "tuic" => {
            let (uuid, password) = l.userinfo.split_once(':').ok_or("нужны uuid:пароль")?;
            if !is_uuid(uuid) {
                return Err("неверный UUID".into());
            }
            out.insert("uuid".into(), json!(uuid));
            out.insert("password".into(), json!(password));
            if let Some(cc) = l.q("congestion_control") {
                out.insert("congestion_control".into(), json!(cc));
            }
            apply_tls(&mut out, &l, true)?;
        }
        _ => return Err("протокол не поддерживается".into()),
    }
    Ok((l.fragment, Value::Object(out)))
}

pub(super) fn is_uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&groups)
            .all(|(n, g)| g.len() == *n && g.chars().all(|c| c.is_ascii_hexdigit()))
}

fn truthy(value: Option<&str>) -> bool {
    matches!(value, Some("1" | "true" | "True" | "TRUE"))
}

/// TLS/REALITY из параметров ссылки. `default_tls` — включён ли TLS без параметра `security`.
fn apply_tls(out: &mut Map<String, Value>, l: &Link, default_tls: bool) -> Result<(), String> {
    let security = l
        .q("security")
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| {
            if default_tls {
                "tls".into()
            } else {
                "none".into()
            }
        });
    match security.as_str() {
        "none" | "" => return Ok(()),
        "tls" | "reality" | "xtls" => {}
        other => return Err(format!("security={other} не поддерживается")),
    }
    let mut tls = Map::new();
    tls.insert("enabled".into(), json!(true));
    let sni = l.q("sni").or_else(|| l.q("peer")).or_else(|| l.q("host"));
    let server_name = sni
        .map(str::to_string)
        .or_else(|| (l.host.parse::<std::net::IpAddr>().is_err()).then(|| l.host.clone()));
    if let Some(name) = server_name {
        tls.insert("server_name".into(), json!(name));
    }
    if truthy(l.q("allowInsecure")) || truthy(l.q("insecure")) {
        tls.insert("insecure".into(), json!(true));
    }
    if let Some(alpn) = l.q("alpn") {
        let list: Vec<&str> = alpn
            .split(',')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .collect();
        if !list.is_empty() {
            tls.insert("alpn".into(), json!(list));
        }
    }
    let fingerprint = l.q("fp").or(if security == "reality" {
        Some("chrome")
    } else {
        None
    });
    if let Some(fp) = fingerprint {
        tls.insert("utls".into(), json!({"enabled": true, "fingerprint": fp}));
    }
    if security == "reality" {
        let key = l.q("pbk").ok_or("REALITY без публичного ключа (pbk)")?;
        tls.insert(
            "reality".into(),
            json!({"enabled": true, "public_key": key, "short_id": l.q("sid").unwrap_or_default()}),
        );
    }
    out.insert("tls".into(), Value::Object(tls));
    Ok(())
}

/// Транспорт (ws, grpc, http, httpupgrade) из параметра `type`.
fn apply_transport(out: &mut Map<String, Value>, l: &Link, key: &str) -> Result<(), String> {
    let kind = l.q(key).unwrap_or("tcp").to_ascii_lowercase();
    let transport = transport_value(
        &kind,
        l.q("path"),
        l.q("host"),
        l.q("serviceName").or_else(|| l.q("service_name")),
    )?;
    if let Some(t) = transport {
        out.insert("transport".into(), t);
    }
    Ok(())
}

pub(super) fn transport_value(
    kind: &str,
    path: Option<&str>,
    host: Option<&str>,
    service: Option<&str>,
) -> Result<Option<Value>, String> {
    Ok(match kind {
        "tcp" | "raw" | "" | "none" => None,
        "ws" | "websocket" => {
            let mut t = json!({"type": "ws", "path": path.unwrap_or("/")});
            if let Some(h) = host {
                t["headers"] = json!({ "Host": h });
            }
            Some(t)
        }
        "grpc" => Some(json!({"type": "grpc", "service_name": service.unwrap_or_default()})),
        "http" | "h2" => {
            let mut t = json!({"type": "http", "path": path.unwrap_or("/")});
            if let Some(h) = host {
                t["host"] = json!(h.split(',').map(str::trim).collect::<Vec<_>>());
            }
            Some(t)
        }
        "httpupgrade" => {
            let mut t = json!({"type": "httpupgrade", "path": path.unwrap_or("/")});
            if let Some(h) = host {
                t["host"] = json!(h);
            }
            Some(t)
        }
        other => {
            return Err(format!(
                "транспорт {other} не поддерживается ядром sing-box"
            ))
        }
    })
}

// ---------- vmess ----------

fn parse_vmess(link: &str) -> Result<(String, Value), String> {
    let payload = link
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or_default()
        .split('#')
        .next()
        .unwrap_or_default();
    let bytes = decode_base64(payload).ok_or("ссылка не раскодируется (base64)")?;
    let v: Value = serde_json::from_slice(&bytes).map_err(|_| "внутри нет корректного JSON")?;
    let text = |k: &str| match &v[k] {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    };
    let host = text("add").ok_or("нет адреса сервера")?;
    let port = text("port")
        .and_then(|p| p.parse::<u16>().ok())
        .filter(|p| *p != 0)
        .ok_or("неверный порт")?;
    let uuid = text("id").ok_or("нет UUID")?;
    if !is_uuid(&uuid) {
        return Err("неверный UUID".into());
    }
    let mut out = Map::new();
    out.insert("type".into(), json!("vmess"));
    out.insert("server".into(), json!(host));
    out.insert("server_port".into(), json!(port));
    out.insert("uuid".into(), json!(uuid));
    out.insert(
        "security".into(),
        json!(text("scy").unwrap_or_else(|| "auto".into())),
    );
    out.insert(
        "alter_id".into(),
        json!(text("aid").and_then(|a| a.parse::<u32>().ok()).unwrap_or(0)),
    );
    let tls_on = matches!(text("tls").as_deref(), Some("tls"));
    if tls_on {
        let mut tls = json!({"enabled": true});
        if let Some(sni) = text("sni").or_else(|| text("host")) {
            tls["server_name"] = json!(sni);
        } else if host.parse::<std::net::IpAddr>().is_err() {
            tls["server_name"] = json!(host);
        }
        if let Some(alpn) = text("alpn") {
            tls["alpn"] = json!(alpn.split(',').map(str::trim).collect::<Vec<_>>());
        }
        if let Some(fp) = text("fp") {
            tls["utls"] = json!({"enabled": true, "fingerprint": fp});
        }
        out.insert("tls".into(), tls);
    }
    let net = text("net")
        .unwrap_or_else(|| "tcp".into())
        .to_ascii_lowercase();
    if let Some(t) = transport_value(
        &net,
        text("path").as_deref(),
        text("host").as_deref(),
        text("path").as_deref(),
    )? {
        out.insert("transport".into(), t);
    }
    Ok((text("ps").unwrap_or_default(), Value::Object(out)))
}

// ---------- shadowsocks ----------

fn parse_ss(link: &str) -> Result<(String, Value), String> {
    let rest = link.split_once("://").map(|(_, r)| r).unwrap_or_default();
    let (main, fragment) = match rest.split_once('#') {
        Some((a, b)) => (a, percent_decode(b)),
        None => (rest, String::new()),
    };
    let (main, query) = match main.split_once('?') {
        Some((a, b)) => (a, b),
        None => (main, ""),
    };
    if query.split('&').any(|p| p.starts_with("plugin=")) {
        return Err("Shadowsocks с плагином не поддерживается".into());
    }
    let main = main.trim_end_matches('/');
    // Вид 1: base64(method:password@host:port). Вид 2 (SIP002): userinfo@host:port.
    let (userinfo, hostport) = if let Some((u, h)) = main.rsplit_once('@') {
        let user = match decode_base64(u).and_then(|b| String::from_utf8(b).ok()) {
            Some(decoded) if decoded.contains(':') && !u.contains(':') => decoded,
            _ => percent_decode(u),
        };
        (user, h.to_string())
    } else {
        let decoded = decode_base64(main)
            .and_then(|b| String::from_utf8(b).ok())
            .ok_or("ссылка не раскодируется (base64)")?;
        let (u, h) = decoded.rsplit_once('@').ok_or("неверный формат")?;
        (u.to_string(), h.to_string())
    };
    let (method, password) = userinfo.split_once(':').ok_or("нет метода и пароля")?;
    let (host, port) = split_host_port(&hostport)?;
    let port = port.ok_or("не указан порт")?;
    if method.is_empty() {
        return Err("нет метода шифрования".into());
    }
    Ok((
        fragment,
        json!({"type": "shadowsocks", "server": host, "server_port": port, "method": method, "password": password}),
    ))
}

// ---------- JSON sing-box ----------

fn parse_json(body: &str) -> ImportReport {
    let mut report = ImportReport::default();
    let Ok(root) = serde_json::from_str::<Value>(body) else {
        report.issues.push("JSON не читается".into());
        return report;
    };
    let configs: Vec<&Value> = match &root {
        Value::Array(items) => items.iter().collect(),
        other => vec![other],
    };
    for config in configs {
        let list: Vec<&Value> = match config.get("outbounds") {
            Some(Value::Array(items)) => items.iter().collect(),
            _ if config.get("type").is_some() => vec![config],
            _ => Vec::new(),
        };
        for item in list {
            if item.get("protocol").is_some() && item.get("type").is_none() {
                // Xray JSON: служебные исходящие (direct, block, dns) пропускаем молча.
                let protocol = item["protocol"]
                    .as_str()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if !matches!(
                    protocol.as_str(),
                    "vless" | "vmess" | "trojan" | "shadowsocks"
                ) {
                    if !matches!(
                        protocol.as_str(),
                        "freedom" | "blackhole" | "dns" | "loopback"
                    ) {
                        report
                            .issues
                            .push(format!("xray: протокол {protocol} не поддерживается"));
                    }
                    continue;
                }
                let name = config["remarks"]
                    .as_str()
                    .or_else(|| item["tag"].as_str())
                    .unwrap_or_default();
                let converted =
                    super::xray::convert(item, name).and_then(|(n, o)| finish_server(n, o));
                match converted {
                    Ok(s) => report.servers.push(s),
                    Err(why) => report.issues.push(format!("xray {protocol}: {why}")),
                }
                // В одном конфиге Xray обычно один рабочий сервер: остальные исходящие служебные.
                continue;
            }
            let kind = item["type"].as_str().unwrap_or_default();
            if !PROXY_TYPES.contains(&kind) {
                continue;
            }
            let mut outbound = item.clone();
            let name = outbound
                .as_object_mut()
                .and_then(|m| m.remove("tag"))
                .and_then(|t| t.as_str().map(str::to_string))
                .unwrap_or_default();
            match finish_server(name, outbound) {
                Ok(s) => report.servers.push(s),
                Err(why) => report.issues.push(format!("json: {why}")),
            }
        }
    }
    report
}

// ---------- помощники ----------

/// Процентное декодирование UTF-8; неверные последовательности остаются как есть.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(b) = hex {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Base64: обычный и url-safe алфавит, с паддингом и без, пробелы игнорируются.
pub fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut buffer: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut count = 0usize;
    for c in text.chars() {
        let v = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            '=' => break,
            c if c.is_whitespace() => continue,
            _ => return None,
        };
        count += 1;
        buffer = (buffer << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    (count % 4 != 1 && count > 0).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "b831381d-6324-4d53-ad4f-8cda48b30811";

    fn b64(text: &str) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in text.as_bytes().chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |a, (i, b)| a | (*b as u32) << (16 - 8 * i));
            for i in 0..=chunk.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            }
            for _ in chunk.len()..3 {
                out.push('=');
            }
        }
        out
    }

    #[test]
    fn base64_variants_and_garbage() {
        assert_eq!(decode_base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode_base64("aGVsbG8").unwrap(), b"hello");
        assert_eq!(decode_base64("_-8").unwrap(), vec![0xff, 0xef]);
        assert_eq!(decode_base64("aGVs\nbG8=").unwrap(), b"hello");
        assert!(decode_base64("a").is_none());
        assert!(decode_base64("не base64!").is_none());
    }

    #[test]
    fn percent_decoding_handles_unicode_and_broken_sequences() {
        assert_eq!(
            percent_decode("%F0%9F%87%A9%F0%9F%87%AA%20Berlin"),
            "🇩🇪 Berlin"
        );
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz%41"), "%zzA");
        assert_eq!(percent_decode("%D0%BC%D0%B8%D1%80"), "мир");
    }

    #[test]
    fn vless_reality_with_flow() {
        let link = format!(
            "vless://{UUID}@203.0.113.7:443?type=tcp&security=reality&sni=www.microsoft.com&fp=chrome&pbk=KEY123&sid=ab12&flow=xtls-rprx-vision#%F0%9F%87%A9%F0%9F%87%AA%20Germany%201"
        );
        let s = parse_link(&link).unwrap();
        assert_eq!(s.protocol, "vless");
        assert_eq!(s.name, "🇩🇪 Germany 1");
        assert_eq!(s.country.as_deref(), Some("DE"));
        assert_eq!(s.host, "203.0.113.7");
        assert_eq!(s.outbound["flow"], "xtls-rprx-vision");
        assert_eq!(s.outbound["tls"]["reality"]["public_key"], "KEY123");
        assert_eq!(s.outbound["tls"]["reality"]["short_id"], "ab12");
        assert_eq!(s.outbound["tls"]["server_name"], "www.microsoft.com");
        assert_eq!(s.outbound["tls"]["utls"]["fingerprint"], "chrome");
        assert!(s.outbound.get("transport").is_none());
    }

    #[test]
    fn vless_ws_tls_and_ipv6_host() {
        let link = format!(
            "vless://{UUID}@[2001:db8::1]:8443?security=tls&sni=cdn.example.com&type=ws&path=%2Fws%3Fed%3D2048&host=cdn.example.com&alpn=h2,http/1.1#v6"
        );
        let s = parse_link(&link).unwrap();
        assert_eq!(s.host, "2001:db8::1");
        assert_eq!(s.port, 8443);
        assert_eq!(s.outbound["transport"]["type"], "ws");
        assert_eq!(s.outbound["transport"]["path"], "/ws?ed=2048");
        assert_eq!(
            s.outbound["transport"]["headers"]["Host"],
            "cdn.example.com"
        );
        assert_eq!(s.outbound["tls"]["alpn"], json!(["h2", "http/1.1"]));
    }

    #[test]
    fn vless_missing_params_are_reported() {
        assert!(parse_link(&format!("vless://{UUID}@host.example#x"))
            .unwrap_err()
            .contains("порт"));
        assert!(parse_link("vless://not-a-uuid@h.example:443#x")
            .unwrap_err()
            .contains("UUID"));
        assert!(parse_link("vless://@h.example:443#x")
            .unwrap_err()
            .contains("ключа"));
        assert!(parse_link(&format!("vless://{UUID}@h.example:99999#x")).is_err());
        assert!(parse_link(&format!("vless://{UUID}@::1:443#x"))
            .unwrap_err()
            .contains("скобках"));
        let reality = format!("vless://{UUID}@h.example:443?security=reality#x");
        assert!(parse_link(&reality).unwrap_err().contains("pbk"));
        let xhttp = format!("vless://{UUID}@h.example:443?type=xhttp&security=tls#x");
        assert!(parse_link(&xhttp).unwrap_err().contains("xhttp"));
    }

    #[test]
    fn name_falls_back_to_host() {
        let s = parse_link(&format!("vless://{UUID}@h.example:443")).unwrap();
        assert_eq!(s.name, "h.example");
    }

    #[test]
    fn vmess_json_link_with_unicode_name() {
        let json = format!(
            r#"{{"v":"2","ps":"Москва ✈ 1","add":"vm.example.com","port":"443","id":"{UUID}","aid":"0","scy":"auto","net":"ws","type":"none","host":"vm.example.com","path":"/v","tls":"tls","sni":"vm.example.com"}}"#
        );
        let s = parse_link(&format!("vmess://{}", b64(&json))).unwrap();
        assert_eq!(s.name, "Москва ✈ 1");
        assert_eq!(s.protocol, "vmess");
        assert_eq!(s.port, 443);
        assert_eq!(s.outbound["transport"]["type"], "ws");
        assert_eq!(s.outbound["tls"]["server_name"], "vm.example.com");
        assert_eq!(s.outbound["security"], "auto");
    }

    #[test]
    fn vmess_numeric_port_and_bad_payload() {
        let json = format!(r#"{{"ps":"n","add":"1.2.3.4","port":8080,"id":"{UUID}"}}"#);
        let s = parse_link(&format!("vmess://{}", b64(&json))).unwrap();
        assert_eq!(s.port, 8080);
        assert!(s.outbound.get("tls").is_none());
        assert!(parse_link("vmess://%%%").unwrap_err().contains("base64"));
        assert!(parse_link(&format!("vmess://{}", b64("not json")))
            .unwrap_err()
            .contains("JSON"));
    }

    #[test]
    fn trojan_defaults_to_tls() {
        let s = parse_link("trojan://p%40ss@tr.example.com:443?sni=tr.example.com#FI%20Helsinki")
            .unwrap();
        assert_eq!(s.outbound["password"], "p@ss");
        assert_eq!(s.outbound["tls"]["enabled"], true);
        assert_eq!(s.country.as_deref(), Some("FI"));
    }

    #[test]
    fn shadowsocks_both_forms_and_plugin() {
        let old = format!(
            "ss://{}#SS%20old",
            b64("aes-256-gcm:secret@198.51.100.2:8388")
        );
        let s = parse_link(&old).unwrap();
        assert_eq!(s.outbound["method"], "aes-256-gcm");
        assert_eq!(s.outbound["server"], "198.51.100.2");
        assert_eq!(s.name, "SS old");
        let sip = format!(
            "ss://{}@198.51.100.2:8388#new",
            b64("chacha20-ietf-poly1305:pw")
        );
        let s = parse_link(&sip).unwrap();
        assert_eq!(s.outbound["password"], "pw");
        let plain = "ss://2022-blake3-aes-128-gcm:abc@[2001:db8::2]:1234#six";
        assert_eq!(parse_link(plain).unwrap().host, "2001:db8::2");
        let plugin = format!(
            "ss://{}@h.example:1?plugin=obfs-local#p",
            b64("aes-128-gcm:x")
        );
        assert!(parse_link(&plugin).unwrap_err().contains("плагин"));
        assert!(parse_link("ss://@@@").is_err());
    }

    #[test]
    fn hysteria2_and_tuic() {
        let h = parse_link("hysteria2://secret@hy.example.com:443?sni=hy.example.com&obfs=salamander&obfs-password=zz&insecure=1#HY").unwrap();
        assert_eq!(h.protocol, "hysteria2");
        assert_eq!(h.outbound["obfs"]["password"], "zz");
        assert_eq!(h.outbound["tls"]["insecure"], true);
        let alias = parse_link("hy2://secret@hy.example.com:8443#A").unwrap();
        assert_eq!(alias.protocol, "hysteria2");
        let t = parse_link(&format!(
            "tuic://{UUID}:pw@t.example.com:443?congestion_control=bbr&alpn=h3#T"
        ))
        .unwrap();
        assert_eq!(t.outbound["congestion_control"], "bbr");
        assert!(parse_link("hysteria2://s@h.example:1?obfs=other#x").is_err());
    }

    #[test]
    fn unsupported_scheme_is_named() {
        assert!(parse_link("wireguard://x@h:1")
            .unwrap_err()
            .contains("wireguard"));
    }

    #[test]
    fn plain_list_with_comments_and_junk() {
        let body = format!(
            "#profile-title: Test\nSTATUS=ok\nvless://{UUID}@a.example.com:443?security=tls#A\n\ntrojan://p@b.example.com:443#B\nvless://broken\n"
        );
        let r = parse_subscription(&body).unwrap();
        assert_eq!(r.servers.len(), 2);
        assert_eq!(r.issues.len(), 1);
    }

    #[test]
    fn base64_list_standard_and_urlsafe_without_padding() {
        let list = format!("vless://{UUID}@a.example.com:443#Ä\ntrojan://p@b.example.com:443#B");
        let std = b64(&list);
        assert_eq!(parse_subscription(&std).unwrap().servers.len(), 2);
        let url_safe = std
            .trim_end_matches('=')
            .replace('+', "-")
            .replace('/', "_");
        assert_eq!(parse_subscription(&url_safe).unwrap().servers.len(), 2);
        let crlf = format!("{}\r\n", std);
        assert_eq!(parse_subscription(&crlf).unwrap().servers.len(), 2);
        assert_eq!(
            parse_subscription(&format!("\u{feff}{std}"))
                .unwrap()
                .servers
                .len(),
            2
        );
    }

    #[test]
    fn bad_base64_and_empty_inputs_fail_clearly() {
        assert!(matches!(
            parse_subscription("   "),
            Err(VpnError::EmptyInput)
        ));
        let err = parse_subscription("%%% definitely not a subscription").unwrap_err();
        assert!(err.to_string().contains("нет поддерживаемых серверов"));
        let html =
            parse_subscription("<!DOCTYPE html><html><body>Login</body></html>").unwrap_err();
        assert!(html.to_string().contains("веб-страницу"));
        let valid_b64_junk = b64("hello world, nothing here");
        assert!(parse_subscription(&valid_b64_junk).is_err());
    }

    #[test]
    fn duplicates_collapse() {
        let one = format!("vless://{UUID}@a.example.com:443#A");
        let r = parse_subscription(&format!("{one}\n{one}")).unwrap();
        assert_eq!(r.servers.len(), 1);
    }

    #[test]
    fn singbox_json_outbounds() {
        let body = r#"{"outbounds":[
            {"type":"selector","tag":"sel","outbounds":["a"]},
            {"type":"vless","tag":"🇳🇱 NL-1","server":"n.example.com","server_port":443,"uuid":"b831381d-6324-4d53-ad4f-8cda48b30811"},
            {"type":"direct","tag":"direct"}]}"#;
        let r = parse_subscription(body).unwrap();
        assert_eq!(r.servers.len(), 1);
        assert_eq!(r.servers[0].name, "🇳🇱 NL-1");
        assert!(r.servers[0].outbound.get("tag").is_none());
    }

    #[test]
    fn xray_json_and_clash_are_reported_not_crashing() {
        let xray = r#"{"outbounds":[{"protocol":"vless","settings":{}}]}"#;
        assert!(parse_subscription(xray)
            .unwrap_err()
            .to_string()
            .contains("ни один не подошёл"));
        let clash = "proxies:\n  - name: a\n    type: vless\n";
        assert!(parse_subscription(clash)
            .unwrap_err()
            .to_string()
            .contains("Clash"));
        assert!(parse_subscription("{ broken").is_err());
    }

    #[test]
    fn xray_json_array_with_several_servers_and_a_broken_one() {
        let body = r#"[
          {"remarks":"🇳🇱 NL","outbounds":[
            {"protocol":"vless","tag":"proxy","settings":{"vnext":[{"address":"n.example.com","port":443,
              "users":[{"id":"b831381d-6324-4d53-ad4f-8cda48b30811","flow":"xtls-rprx-vision","encryption":"none"}]}]},
             "streamSettings":{"network":"tcp","security":"reality","realitySettings":{"serverName":"c.example.com","publicKey":"PK","shortId":"01"}}},
            {"protocol":"freedom","tag":"direct"},{"protocol":"blackhole","tag":"block"}]},
          {"remarks":"DE","outbounds":[
            {"protocol":"trojan","tag":"proxy","settings":{"servers":[{"address":"d.example.com","port":443,"password":"SECRETPW"}]}}]},
          {"remarks":"XH","outbounds":[
            {"protocol":"vless","tag":"proxy","settings":{"vnext":[{"address":"x.example.com","port":443,
              "users":[{"id":"b831381d-6324-4d53-ad4f-8cda48b30811"}]}]},"streamSettings":{"network":"xhttp","security":"tls"}}]}
        ]"#;
        let r = parse_subscription(body).unwrap();
        assert_eq!(r.servers.len(), 2);
        assert_eq!(r.servers[0].name, "🇳🇱 NL");
        assert_eq!(r.servers[0].protocol, "vless");
        assert_eq!(r.servers[1].protocol, "trojan");
        assert_eq!(r.issues.len(), 1);
        assert!(r.issues[0].contains("xhttp"));
        assert!(!r.issues.join(" ").contains("b831381d"));
    }

    #[test]
    fn all_entries_skipped_gives_grouped_reasons_without_secrets() {
        let list = "vless://b831381d-6324-4d53-ad4f-8cda48b30811@a.example.com:443?type=xhttp&security=tls#A\n\
                    vless://b831381d-6324-4d53-ad4f-8cda48b30811@b.example.com:443?type=xhttp&security=tls#B\n";
        let msg = parse_subscription(list).unwrap_err().to_string();
        assert!(msg.contains("ни один не подошёл"), "{msg}");
        assert!(msg.contains("×2"), "{msg}");
        assert!(msg.contains("xhttp"), "{msg}");
        assert!(
            !msg.contains("b831381d") && !msg.contains("a.example.com"),
            "{msg}"
        );
    }

    #[test]
    fn unknown_text_and_clash_get_distinct_messages() {
        let unknown = parse_subscription("hello world, this is not a subscription")
            .unwrap_err()
            .to_string();
        assert!(unknown.contains("формат не распознан"), "{unknown}");
        let clash = parse_subscription("proxies:\n  - name: a\n    type: vless\n")
            .unwrap_err()
            .to_string();
        assert!(
            clash.contains("Clash") && clash.contains("не поддерживается"),
            "{clash}"
        );
    }

    #[test]
    fn every_supported_form_yields_the_same_server_list() {
        let links = "vless://b831381d-6324-4d53-ad4f-8cda48b30811@a.example.com:443?security=tls&type=ws&path=%2Fp#A\ntrojan://pw@b.example.com:443#B\n";
        let plain = parse_subscription(links).unwrap();
        let b64 = parse_subscription(&b64(links)).unwrap();
        assert_eq!(plain.servers.len(), 2);
        assert_eq!(plain.servers, b64.servers);
    }

    #[test]
    fn size_limit() {
        let huge = "a".repeat(MAX_BODY + 1);
        assert!(parse_subscription(&huge)
            .unwrap_err()
            .to_string()
            .contains("большая"));
    }
}
