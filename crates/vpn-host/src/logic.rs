//! Что запускает служба и какую конфигурацию принимает. Служба работает от имени системы, а запустить
//! её может обычный пользователь (права выдаёт установщик), поэтому:
//! - ядро берётся ТОЛЬКО из папки программы (`..\vpn-core\sing-box.exe` рядом со службой);
//! - конфигурация приходит в аргументе запуска и проверяется: допустимы только знакомые разделы,
//!   входы `mixed` на 127.0.0.1 и `tun`, без записи журнала в файл, внешних наборов правил и
//!   служебных интерфейсов ядра.

use std::path::{Path, PathBuf};

use serde_json::Value;

pub const SERVICE_NAME: &str = "OchistkaVpn";
pub const CORE_EXE: &str = "sing-box.exe";
const MAX_CONFIG_BYTES: usize = 256 * 1024;
const TOP_LEVEL: [&str; 5] = ["log", "dns", "inbounds", "outbounds", "route"];
const ROUTE_KEYS: [&str; 5] = [
    "rules",
    "final",
    "default_domain_resolver",
    "auto_detect_interface",
    "find_process",
];
const INBOUND_TYPES: [&str; 2] = ["mixed", "tun"];

/// Ядро: `<папка программы>\vpn-core\sing-box.exe`; служба лежит в `<папка программы>\vpn-service\`.
pub fn core_path(host_exe: &Path) -> Option<PathBuf> {
    let app_dir = host_exe.parent()?.parent()?;
    Some(app_dir.join("vpn-core").join(CORE_EXE))
}

/// Куда писать принятую конфигурацию (папка службы, права на запись только у администратора).
pub fn config_path(host_exe: &Path) -> Option<PathBuf> {
    Some(host_exe.parent()?.join("run").join("config.json"))
}

/// Проверяет конфигурацию из аргумента запуска.
pub fn validate_config(text: &str) -> Result<(), String> {
    if text.len() > MAX_CONFIG_BYTES {
        return Err("конфигурация слишком большая".into());
    }
    let value: Value = serde_json::from_str(text).map_err(|_| "конфигурация не разобрана")?;
    let root = value.as_object().ok_or("конфигурация не объект")?;
    if let Some(key) = root.keys().find(|k| !TOP_LEVEL.contains(&k.as_str())) {
        return Err(format!("лишний раздел: {key}"));
    }
    if let Some(log) = root.get("log") {
        if log.get("output").is_some() {
            return Err("запись журнала в файл не разрешена".into());
        }
    }
    let inbounds = root
        .get("inbounds")
        .and_then(Value::as_array)
        .ok_or("нет входов")?;
    for inbound in inbounds {
        let kind = inbound.get("type").and_then(Value::as_str).unwrap_or("");
        if !INBOUND_TYPES.contains(&kind) {
            return Err(format!("вход не разрешён: {kind}"));
        }
        if kind == "mixed" && inbound.get("listen").and_then(Value::as_str) != Some("127.0.0.1") {
            return Err("локальный вход должен слушать только 127.0.0.1".into());
        }
    }
    if !root.get("outbounds").is_some_and(Value::is_array) {
        return Err("нет выходов".into());
    }
    if let Some(route) = root.get("route").and_then(Value::as_object) {
        if let Some(key) = route.keys().find(|k| !ROUTE_KEYS.contains(&k.as_str())) {
            return Err(format!("лишний параметр правил: {key}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn good() -> Value {
        json!({
            "log": {"level": "warn", "timestamp": true},
            "dns": {"servers": [{"type": "local", "tag": "local"}], "final": "local"},
            "inbounds": [
                {"type": "mixed", "tag": "m", "listen": "127.0.0.1", "listen_port": 1080},
                {"type": "tun", "tag": "t", "address": ["172.19.0.1/30"], "auto_route": true}
            ],
            "outbounds": [{"type": "direct", "tag": "direct"}],
            "route": {"rules": [{"protocol": "dns", "action": "hijack-dns"}], "final": "direct",
                      "auto_detect_interface": true}
        })
    }

    #[test]
    fn the_core_is_always_the_file_in_the_program_folder() {
        let app = std::env::temp_dir().join("OCHISTKA");
        let host = app.join("vpn-service").join("ochistka-vpn-host.exe");
        assert_eq!(
            core_path(&host).unwrap(),
            app.join("vpn-core").join("sing-box.exe")
        );
        assert_eq!(
            config_path(&host).unwrap(),
            app.join("vpn-service").join("run").join("config.json")
        );
        assert_eq!(core_path(Path::new("")), None);
    }

    #[test]
    fn the_apps_own_configuration_is_accepted() {
        assert!(validate_config(&good().to_string()).is_ok());
    }

    type Mutation = Box<dyn Fn(&mut Value)>;

    #[test]
    fn anything_unfamiliar_is_refused() {
        let cases: Vec<(&str, Mutation)> = vec![
            (
                "раздел",
                Box::new(|v| v["experimental"] = json!({"clash_api": {}})),
            ),
            (
                "журнал в файл",
                Box::new(|v| v["log"]["output"] = json!("C:/x.log")),
            ),
            (
                "вход http",
                Box::new(|v| v["inbounds"][0]["type"] = json!("http")),
            ),
            (
                "чужой адрес",
                Box::new(|v| v["inbounds"][0]["listen"] = json!("0.0.0.0")),
            ),
            (
                "наборы правил",
                Box::new(|v| v["route"]["rule_set"] = json!([])),
            ),
            (
                "нет выходов",
                Box::new(|v| {
                    v.as_object_mut().unwrap().remove("outbounds");
                }),
            ),
        ];
        for (name, mutate) in cases {
            let mut v = good();
            mutate(&mut v);
            assert!(validate_config(&v.to_string()).is_err(), "{name}");
        }
        assert!(validate_config("не json").is_err());
        assert!(validate_config("[]").is_err());
        assert!(validate_config(&"x".repeat(300_000)).is_err());
    }
}
