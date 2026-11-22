//! Системный прокси по ОС (для «Обхода сайтов»): Windows — реестр HKCU (ProxyEnable, ProxyServer) и
//! обновление через InternetSetOption; macOS — `networksetup` (может запросить пароль); Linux — только
//! GNOME через `gsettings`, остальное «не поддерживается». Прежние значения возвращаются как есть.
//! Логику выбора и хранения файла ведёт `proxy-bypass`; здесь только чтение и запись настроек.

use std::sync::Arc;

use proxy_bypass::{Snapshot, SystemProxyPort};

pub const UNSUPPORTED: &str = "На этой системе обход сайтов пока не поддерживается";

/// Запуск системной программы по абсолютному пути (без поиска в PATH); в тестах подменяется.
pub trait Runner: Send + Sync {
    fn run(&self, exe: &str, args: &[&str]) -> Result<String, String>;
}

#[cfg(not(windows))]
struct RealRunner;

#[cfg(not(windows))]
impl Runner for RealRunner {
    fn run(&self, exe: &str, args: &[&str]) -> Result<String, String> {
        let out = std::process::Command::new(exe)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("не удалось запустить {exe}: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "{exe} завершился с ошибкой: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

// ---------------------------------------------------------------- GNOME (Linux)

const GSETTINGS: &str = "/usr/bin/gsettings";
/// (схема, ключ) в порядке чтения и записи.
const GNOME_KEYS: [(&str, &str); 5] = [
    ("org.gnome.system.proxy", "mode"),
    ("org.gnome.system.proxy.http", "host"),
    ("org.gnome.system.proxy.http", "port"),
    ("org.gnome.system.proxy.https", "host"),
    ("org.gnome.system.proxy.https", "port"),
];

pub struct GnomeProxy {
    runner: Arc<dyn Runner>,
    desktop: String,
    tool_present: bool,
}

impl GnomeProxy {
    pub fn new(runner: Arc<dyn Runner>, desktop: String, tool_present: bool) -> Self {
        Self {
            runner,
            desktop,
            tool_present,
        }
    }

    fn get(&self, schema: &str, key: &str) -> Result<String, String> {
        let out = self.runner.run(GSETTINGS, &["get", schema, key])?;
        let v = out.trim();
        // «'manual'» → manual; «uint32 8080» → 8080 (разные версии gsettings печатают по-разному).
        let v = v.strip_prefix("uint32 ").unwrap_or(v);
        Ok(v.trim_matches('\'').to_string())
    }

    fn put(&self, schema: &str, key: &str, value: &str) -> Result<(), String> {
        let arg = if key == "port" {
            value.to_string()
        } else {
            format!("'{}'", value.replace('\'', ""))
        };
        self.runner
            .run(GSETTINGS, &["set", schema, key, &arg])
            .map(|_| ())
    }
}

impl SystemProxyPort for GnomeProxy {
    fn unsupported(&self) -> Option<String> {
        let gnome = self
            .desktop
            .split(':')
            .any(|d| d.eq_ignore_ascii_case("gnome") || d.eq_ignore_ascii_case("ubuntu"));
        (!(gnome && self.tool_present)).then(|| UNSUPPORTED.to_string())
    }

    fn snapshot(&self) -> Result<Snapshot, String> {
        let mut pairs = Vec::new();
        for (schema, key) in GNOME_KEYS {
            pairs.push((format!("{schema} {key}"), self.get(schema, key)?));
        }
        Ok(Snapshot(pairs))
    }

    fn set_local(&self, port: u16) -> Result<(), String> {
        for schema in [
            "org.gnome.system.proxy.http",
            "org.gnome.system.proxy.https",
        ] {
            self.put(schema, "host", "127.0.0.1")?;
            self.put(schema, "port", &port.to_string())?;
        }
        self.put("org.gnome.system.proxy", "mode", "manual")
    }

    fn restore(&self, snapshot: &Snapshot) -> Result<(), String> {
        // Режим возвращается последним: пока он не включён, значения хоста и порта ни на что не влияют.
        let mut first_error = None;
        let mut mode = None;
        for (name, value) in &snapshot.0 {
            let Some((schema, key)) = name.split_once(' ') else {
                continue;
            };
            if !GNOME_KEYS.iter().any(|(s, k)| *s == schema && *k == key) {
                continue; // чужой ключ из файла не записываем
            }
            if key == "mode" {
                mode = Some((schema, key, value));
                continue;
            }
            if let Err(e) = self.put(schema, key, value) {
                first_error.get_or_insert(e);
            }
        }
        if let Some((schema, key, value)) = mode {
            if let Err(e) = self.put(schema, key, value) {
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

// ---------------------------------------------------------------- macOS

const NETWORKSETUP: &str = "/usr/sbin/networksetup";

pub struct MacProxy {
    runner: Arc<dyn Runner>,
}

impl MacProxy {
    pub fn new(runner: Arc<dyn Runner>) -> Self {
        Self { runner }
    }

    fn services(&self) -> Result<Vec<String>, String> {
        let out = self
            .runner
            .run(NETWORKSETUP, &["-listallnetworkservices"])?;
        // Первая строка — пояснение; службы со звёздочкой отключены.
        Ok(out
            .lines()
            .skip(1)
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('*'))
            .map(str::to_string)
            .collect())
    }

    /// «Enabled: Yes / Server: host / Port: 80» → «1|host|80».
    fn read(&self, getter: &str, service: &str) -> Result<String, String> {
        let out = self.runner.run(NETWORKSETUP, &[getter, service])?;
        let field = |name: &str| {
            out.lines()
                .find_map(|l| l.strip_prefix(name))
                .map(|v| v.trim().to_string())
                .unwrap_or_default()
        };
        let enabled = field("Enabled:").eq_ignore_ascii_case("yes");
        Ok(format!(
            "{}|{}|{}",
            u8::from(enabled),
            field("Server:"),
            field("Port:")
        ))
    }

    fn write(&self, kind: &str, service: &str, value: &str) -> Result<(), String> {
        let mut parts = value.splitn(3, '|');
        let (enabled, host, port) = (
            parts.next().unwrap_or("0"),
            parts.next().unwrap_or(""),
            parts.next().unwrap_or("0"),
        );
        let (set, state) = if kind == "https" {
            ("-setsecurewebproxy", "-setsecurewebproxystate")
        } else {
            ("-setwebproxy", "-setwebproxystate")
        };
        if !host.is_empty() && port != "0" {
            self.runner.run(NETWORKSETUP, &[set, service, host, port])?;
        }
        self.runner
            .run(
                NETWORKSETUP,
                &[state, service, if enabled == "1" { "on" } else { "off" }],
            )
            .map(|_| ())
    }
}

impl SystemProxyPort for MacProxy {
    fn unsupported(&self) -> Option<String> {
        None
    }

    fn snapshot(&self) -> Result<Snapshot, String> {
        let mut pairs = Vec::new();
        for service in self.services()? {
            pairs.push((
                format!("https:{service}"),
                self.read("-getsecurewebproxy", &service)?,
            ));
            pairs.push((
                format!("http:{service}"),
                self.read("-getwebproxy", &service)?,
            ));
        }
        Ok(Snapshot(pairs))
    }

    fn set_local(&self, port: u16) -> Result<(), String> {
        let value = format!("1|127.0.0.1|{port}");
        for service in self.services()? {
            self.write("https", &service, &value)?;
            self.write("http", &service, &value)?;
        }
        Ok(())
    }

    fn restore(&self, snapshot: &Snapshot) -> Result<(), String> {
        let mut first_error = None;
        for (name, value) in &snapshot.0 {
            let Some((kind, service)) = name.split_once(':') else {
                continue;
            };
            if kind != "http" && kind != "https" {
                continue;
            }
            if let Err(e) = self.write(kind, service, value) {
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

// ---------------------------------------------------------------- Windows

/// Реестр для настроек прокси (в тестах подменяется).
pub trait ProxyRegistry: Send + Sync {
    fn get_dword(&self, name: &str) -> Result<Option<u32>, String>;
    fn get_string(&self, name: &str) -> Result<Option<String>, String>;
    fn set_dword(&self, name: &str, value: u32) -> Result<(), String>;
    fn set_string(&self, name: &str, value: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
    /// Просит систему перечитать настройки (InternetSetOption).
    fn refresh(&self);
}

pub struct WindowsProxy {
    registry: Arc<dyn ProxyRegistry>,
    available: bool,
}

const ABSENT: &str = "\u{1}absent";

impl WindowsProxy {
    pub fn new(registry: Arc<dyn ProxyRegistry>, available: bool) -> Self {
        Self {
            registry,
            available,
        }
    }
}

impl SystemProxyPort for WindowsProxy {
    fn unsupported(&self) -> Option<String> {
        (!self.available).then(|| UNSUPPORTED.to_string())
    }

    fn snapshot(&self) -> Result<Snapshot, String> {
        let enable = self
            .registry
            .get_dword("ProxyEnable")?
            .map_or(ABSENT.to_string(), |v| v.to_string());
        let server = self
            .registry
            .get_string("ProxyServer")?
            .unwrap_or_else(|| ABSENT.to_string());
        Ok(Snapshot(vec![
            ("ProxyEnable".into(), enable),
            ("ProxyServer".into(), server),
        ]))
    }

    fn set_local(&self, port: u16) -> Result<(), String> {
        self.registry
            .set_string("ProxyServer", &format!("127.0.0.1:{port}"))?;
        self.registry.set_dword("ProxyEnable", 1)?;
        self.registry.refresh();
        Ok(())
    }

    fn restore(&self, snapshot: &Snapshot) -> Result<(), String> {
        let mut first_error = None;
        let mut note = |r: Result<(), String>| {
            if let Err(e) = r {
                first_error.get_or_insert(e);
            }
        };
        // Сначала адрес, потом флаг: так система не остаётся «включённой» с нашим адресом.
        let mut ordered: Vec<&(String, String)> = snapshot.0.iter().collect();
        ordered.sort_by_key(|(name, _)| name != "ProxyServer");
        for (name, value) in ordered {
            match name.as_str() {
                "ProxyServer" if value == ABSENT => note(self.registry.delete("ProxyServer")),
                "ProxyServer" => note(self.registry.set_string("ProxyServer", value)),
                "ProxyEnable" if value == ABSENT => note(self.registry.delete("ProxyEnable")),
                "ProxyEnable" => match value.parse::<u32>() {
                    Ok(v) => note(self.registry.set_dword("ProxyEnable", v)),
                    Err(_) => note(Err("повреждено значение ProxyEnable".into())),
                },
                _ => {}
            }
        }
        self.registry.refresh();
        first_error.map_or(Ok(()), Err)
    }
}

#[cfg(windows)]
struct HkcuProxyRegistry;

#[cfg(windows)]
const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[cfg(windows)]
impl HkcuProxyRegistry {
    fn key(&self) -> Result<winreg::RegKey, String> {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
        winreg::RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ | KEY_WRITE)
            .map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
fn not_found(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::NotFound
}

#[cfg(windows)]
impl ProxyRegistry for HkcuProxyRegistry {
    fn get_dword(&self, name: &str) -> Result<Option<u32>, String> {
        match self.key()?.get_value::<u32, _>(name) {
            Ok(v) => Ok(Some(v)),
            Err(e) if not_found(&e) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn get_string(&self, name: &str) -> Result<Option<String>, String> {
        match self.key()?.get_value::<String, _>(name) {
            Ok(v) => Ok(Some(v)),
            Err(e) if not_found(&e) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set_dword(&self, name: &str, value: u32) -> Result<(), String> {
        self.key()?
            .set_value(name, &value)
            .map_err(|e| e.to_string())
    }
    fn set_string(&self, name: &str, value: &str) -> Result<(), String> {
        self.key()?
            .set_value(name, &value.to_string())
            .map_err(|e| e.to_string())
    }
    fn delete(&self, name: &str) -> Result<(), String> {
        match self.key()?.delete_value(name) {
            Ok(()) => Ok(()),
            Err(e) if not_found(&e) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn refresh(&self) {
        crate::windows::win32_ffi::notify_proxy_settings_changed();
    }
}

/// Системный прокси текущей ОС.
pub fn current_system_proxy() -> Arc<dyn SystemProxyPort> {
    #[cfg(windows)]
    {
        Arc::new(WindowsProxy::new(Arc::new(HkcuProxyRegistry), true))
    }
    #[cfg(target_os = "macos")]
    {
        Arc::new(MacProxy::new(Arc::new(RealRunner)))
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        let tool = std::path::Path::new(GSETTINGS).is_file();
        Arc::new(GnomeProxy::new(Arc::new(RealRunner), desktop, tool))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    /// Фейковый gsettings/networksetup: хранит значения и записывает команды.
    #[derive(Default)]
    struct FakeRunner {
        store: Mutex<BTreeMap<String, String>>,
        log: Mutex<Vec<String>>,
        listing: String,
    }

    impl Runner for FakeRunner {
        fn run(&self, exe: &str, args: &[&str]) -> Result<String, String> {
            self.log
                .lock()
                .unwrap()
                .push(format!("{exe} {}", args.join(" ")));
            let mut store = self.store.lock().unwrap();
            match (exe, args) {
                (GSETTINGS, ["get", s, k]) => Ok(format!(
                    "{}\n",
                    store.get(&format!("{s} {k}")).cloned().unwrap_or_default()
                )),
                (GSETTINGS, ["set", s, k, v]) => {
                    store.insert(format!("{s} {k}"), v.to_string());
                    Ok(String::new())
                }
                (NETWORKSETUP, ["-listallnetworkservices"]) => Ok(self.listing.clone()),
                (NETWORKSETUP, [get, svc]) if get.starts_with("-get") => {
                    let kind = if *get == "-getsecurewebproxy" {
                        "https"
                    } else {
                        "http"
                    };
                    let v = store
                        .get(&format!("{kind}:{svc}"))
                        .cloned()
                        .unwrap_or("0||0".into());
                    let mut p = v.splitn(3, '|');
                    let (e, h, po) = (p.next().unwrap(), p.next().unwrap(), p.next().unwrap());
                    Ok(format!(
                        "Enabled: {}\nServer: {h}\nPort: {po}\nAuthenticated Proxy Enabled: 0\n",
                        if e == "1" { "Yes" } else { "No" }
                    ))
                }
                (NETWORKSETUP, [set, svc, host, port]) => {
                    let kind = if *set == "-setsecurewebproxy" {
                        "https"
                    } else {
                        "http"
                    };
                    store.insert(format!("{kind}:{svc}"), format!("1|{host}|{port}"));
                    Ok(String::new())
                }
                (NETWORKSETUP, [state, svc, on]) => {
                    let kind = if *state == "-setsecurewebproxystate" {
                        "https"
                    } else {
                        "http"
                    };
                    let key = format!("{kind}:{svc}");
                    let cur = store.get(&key).cloned().unwrap_or("0||0".into());
                    let rest = cur.split_once('|').map(|x| x.1).unwrap_or("|0").to_string();
                    store.insert(key, format!("{}|{rest}", if *on == "on" { 1 } else { 0 }));
                    Ok(String::new())
                }
                _ => Err(format!("неожиданная команда {exe} {args:?}")),
            }
        }
    }

    fn gnome(runner: &Arc<FakeRunner>, desktop: &str, tool: bool) -> GnomeProxy {
        GnomeProxy::new(runner.clone(), desktop.into(), tool)
    }

    #[test]
    fn gnome_snapshot_set_and_restore_return_the_previous_values() {
        let r = Arc::new(FakeRunner::default());
        {
            let mut s = r.store.lock().unwrap();
            s.insert("org.gnome.system.proxy mode".into(), "'none'".into());
            s.insert("org.gnome.system.proxy.http host".into(), "''".into());
            s.insert("org.gnome.system.proxy.http port".into(), "0".into());
            s.insert("org.gnome.system.proxy.https host".into(), "'corp'".into());
            s.insert(
                "org.gnome.system.proxy.https port".into(),
                "uint32 3128".into(),
            );
        }
        let g = gnome(&r, "ubuntu:GNOME", true);
        let snap = g.snapshot().unwrap();
        assert_eq!(
            snap.0[0],
            ("org.gnome.system.proxy mode".into(), "none".into())
        );
        assert_eq!(snap.0[4].1, "3128");
        g.set_local(4321).unwrap();
        {
            let s = r.store.lock().unwrap();
            assert_eq!(s["org.gnome.system.proxy mode"], "'manual'");
            assert_eq!(s["org.gnome.system.proxy.http port"], "4321");
            assert_eq!(s["org.gnome.system.proxy.https host"], "'127.0.0.1'");
        }
        g.restore(&snap).unwrap();
        let again = g.snapshot().unwrap();
        assert_eq!(again, snap, "прежние значения возвращены");
        // Режим возвращается последним.
        let log = r.log.lock().unwrap();
        let last_set = log.iter().rev().find(|l| l.contains(" set ")).unwrap();
        assert!(last_set.contains("mode"), "{last_set}");
    }

    #[test]
    fn gnome_is_the_only_supported_linux_desktop() {
        let r = Arc::new(FakeRunner::default());
        assert!(gnome(&r, "GNOME", true).unsupported().is_none());
        assert!(gnome(&r, "KDE", true).unsupported().is_some());
        assert!(gnome(&r, "", true).unsupported().is_some());
        assert!(gnome(&r, "GNOME", false).unsupported().is_some());
    }

    #[test]
    fn gnome_does_not_write_foreign_keys_from_a_tampered_file() {
        let r = Arc::new(FakeRunner::default());
        let g = gnome(&r, "GNOME", true);
        let snap = Snapshot(vec![("org.evil key; rm -rf".into(), "x".into())]);
        g.restore(&snap).unwrap();
        assert!(r.log.lock().unwrap().is_empty());
    }

    #[test]
    fn mac_snapshot_set_and_restore_per_network_service() {
        let r = Arc::new(FakeRunner {
            listing: "An asterisk (*) denotes that a network service is disabled.\nWi-Fi\n*Old\nEthernet\n"
                .into(),
            ..Default::default()
        });
        {
            let mut s = r.store.lock().unwrap();
            s.insert("https:Wi-Fi".into(), "1|corp|3128".into());
        }
        let m = MacProxy::new(r.clone());
        let snap = m.snapshot().unwrap();
        assert_eq!(snap.0.len(), 4, "две службы по два вида: {snap:?}");
        assert!(!snap.0.iter().any(|(k, _)| k.contains("Old")));
        m.set_local(5555).unwrap();
        assert_eq!(r.store.lock().unwrap()["https:Wi-Fi"], "1|127.0.0.1|5555");
        m.restore(&snap).unwrap();
        assert_eq!(r.store.lock().unwrap()["https:Wi-Fi"], "1|corp|3128");
        assert!(r.store.lock().unwrap()["http:Ethernet"].starts_with("0|"));
    }

    #[derive(Default)]
    struct FakeReg {
        dwords: Mutex<BTreeMap<String, u32>>,
        strings: Mutex<BTreeMap<String, String>>,
        refreshes: Mutex<u32>,
    }

    impl ProxyRegistry for FakeReg {
        fn get_dword(&self, n: &str) -> Result<Option<u32>, String> {
            Ok(self.dwords.lock().unwrap().get(n).copied())
        }
        fn get_string(&self, n: &str) -> Result<Option<String>, String> {
            Ok(self.strings.lock().unwrap().get(n).cloned())
        }
        fn set_dword(&self, n: &str, v: u32) -> Result<(), String> {
            self.dwords.lock().unwrap().insert(n.into(), v);
            Ok(())
        }
        fn set_string(&self, n: &str, v: &str) -> Result<(), String> {
            self.strings.lock().unwrap().insert(n.into(), v.into());
            Ok(())
        }
        fn delete(&self, n: &str) -> Result<(), String> {
            self.dwords.lock().unwrap().remove(n);
            self.strings.lock().unwrap().remove(n);
            Ok(())
        }
        fn refresh(&self) {
            *self.refreshes.lock().unwrap() += 1;
        }
    }

    #[test]
    fn windows_set_and_restore_including_values_that_did_not_exist() {
        // Было: прокси включён, сервер задан.
        let reg = Arc::new(FakeReg::default());
        reg.set_dword("ProxyEnable", 1).unwrap();
        reg.set_string("ProxyServer", "corp:3128").unwrap();
        let w = WindowsProxy::new(reg.clone(), true);
        let snap = w.snapshot().unwrap();
        w.set_local(7000).unwrap();
        assert_eq!(
            reg.get_string("ProxyServer").unwrap().unwrap(),
            "127.0.0.1:7000"
        );
        assert_eq!(reg.get_dword("ProxyEnable").unwrap(), Some(1));
        w.restore(&snap).unwrap();
        assert_eq!(reg.get_string("ProxyServer").unwrap().unwrap(), "corp:3128");
        assert_eq!(reg.get_dword("ProxyEnable").unwrap(), Some(1));
        assert!(*reg.refreshes.lock().unwrap() >= 2, "система уведомлена");
        // Было: значений нет вообще — после выключения их снова нет.
        let empty = Arc::new(FakeReg::default());
        let w = WindowsProxy::new(empty.clone(), true);
        let snap = w.snapshot().unwrap();
        w.set_local(7001).unwrap();
        w.restore(&snap).unwrap();
        assert_eq!(empty.get_dword("ProxyEnable").unwrap(), None);
        assert_eq!(empty.get_string("ProxyServer").unwrap(), None);
    }
}
