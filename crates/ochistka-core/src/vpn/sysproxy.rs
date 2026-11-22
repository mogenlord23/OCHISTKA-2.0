//! Системный прокси Windows (HKCU Internet Settings) с резервной копией прежних значений.
//! Перед изменением значения копируются в `HKCU\Software\OCHISTKA\Backup\VpnProxy`; откат
//! возвращает их (или удаляет значение, если его не было). Если приложение закрылось аварийно,
//! копия остаётся и откат делается при следующем запуске.

use std::sync::Arc;

use crate::tweaks::RegistryPort;

use super::config::PROXY_OVERRIDE;
use super::VpnError;

pub const INTERNET_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
pub const BACKUP_KEY: &str = r"Software\OCHISTKA\Backup\VpnProxy";

/// Сообщает программам, что настройки прокси изменились (WinINet). На других ОС — пусто.
pub trait ProxyRefresh: Send + Sync {
    fn refresh(&self);
}

/// Ничего не делает (тесты, не-Windows).
pub struct NoRefresh;
impl ProxyRefresh for NoRefresh {
    fn refresh(&self) {}
}

#[derive(Clone, Copy)]
enum Kind {
    Dword,
    Text,
}

/// Значения, которые мы меняем или временно убираем.
const VALUES: &[(&str, Kind)] = &[
    ("ProxyEnable", Kind::Dword),
    ("ProxyServer", Kind::Text),
    ("ProxyOverride", Kind::Text),
    ("AutoConfigURL", Kind::Text),
];

/// Состояние системного прокси относительно нашего адреса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyCheck {
    /// Включён на наш адрес.
    Ours,
    /// Наш адрес записан, но прокси выключен.
    Disabled,
    /// Адрес другой (или пустой): чужие настройки.
    Foreign,
}

/// Что сделал откат системного прокси.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restored {
    /// Копии не было: ничего не менялось.
    Nothing,
    /// Прежние значения возвращены.
    Restored,
    /// Адрес прокси уже сменила другая программа: её значения не тронуты, копия снята.
    ForeignKept,
}

pub struct SystemProxy {
    registry: Arc<dyn RegistryPort>,
    refresh: Arc<dyn ProxyRefresh>,
}

fn proxy_err(err: impl std::fmt::Display) -> VpnError {
    VpnError::Proxy(err.to_string())
}

impl SystemProxy {
    pub fn new(registry: Arc<dyn RegistryPort>, refresh: Arc<dyn ProxyRefresh>) -> Self {
        Self { registry, refresh }
    }

    /// Есть ли неотменённая копия (прокси выставлен нами).
    pub fn is_active(&self) -> bool {
        matches!(self.registry.read_dword(BACKUP_KEY, "Active"), Ok(Some(1)))
    }

    /// Чужие настройки прокси, которые мы не вправе молча менять: включённый прокси с адресом
    /// (другой VPN или программа) или сценарий автонастройки. Адреса не возвращаем.
    /// Вызывать после отката собственной устаревшей копии.
    pub fn foreign_conflict(&self) -> Result<Option<&'static str>, VpnError> {
        let enabled = matches!(
            self.registry
                .read_dword(INTERNET_KEY, "ProxyEnable")
                .map_err(proxy_err)?,
            Some(1)
        );
        let server = self
            .registry
            .read_string(INTERNET_KEY, "ProxyServer")
            .map_err(proxy_err)?
            .is_some_and(|s| !s.trim().is_empty());
        if enabled && server {
            return Ok(Some("системный прокси"));
        }
        let pac = self
            .registry
            .read_string(INTERNET_KEY, "AutoConfigURL")
            .map_err(proxy_err)?
            .is_some_and(|s| !s.trim().is_empty());
        Ok(pac.then_some("сценарий автонастройки прокси"))
    }

    /// Включает системный прокси на `127.0.0.1:port`. Повторный вызов копию не перезаписывает.
    pub fn enable(&self, port: u16) -> Result<(), VpnError> {
        let ours = format!("127.0.0.1:{port}");
        if !self.is_active() {
            for (name, kind) in VALUES {
                self.backup_value(name, *kind)?;
            }
            self.registry
                .write_dword(BACKUP_KEY, "Active", 1)
                .map_err(proxy_err)?;
        }
        self.registry
            .write_string(BACKUP_KEY, "Ours", &ours)
            .map_err(proxy_err)?;
        let apply = || -> Result<(), VpnError> {
            self.registry
                .write_string(INTERNET_KEY, "ProxyServer", &ours)
                .map_err(proxy_err)?;
            self.registry
                .write_string(INTERNET_KEY, "ProxyOverride", PROXY_OVERRIDE)
                .map_err(proxy_err)?;
            self.registry
                .delete_value(INTERNET_KEY, "AutoConfigURL")
                .map_err(proxy_err)?;
            self.registry
                .write_dword(INTERNET_KEY, "ProxyEnable", 1)
                .map_err(proxy_err)
        };
        if let Err(err) = apply() {
            let _ = self.restore();
            return Err(err);
        }
        self.refresh.refresh();
        Ok(())
    }

    /// Что сейчас в настройках: наши значения, выключено нами же сброшенное или чужое.
    pub fn check(&self, port: u16) -> Result<ProxyCheck, VpnError> {
        let ours = format!("127.0.0.1:{port}");
        let enabled = matches!(
            self.registry
                .read_dword(INTERNET_KEY, "ProxyEnable")
                .map_err(proxy_err)?,
            Some(1)
        );
        let server = self
            .registry
            .read_string(INTERNET_KEY, "ProxyServer")
            .map_err(proxy_err)?
            .unwrap_or_default();
        Ok(if enabled && server == ours {
            ProxyCheck::Ours
        } else if server == ours {
            // Адрес наш, но выключено: сбросила система или другая программа — можно включить снова.
            ProxyCheck::Disabled
        } else {
            ProxyCheck::Foreign
        })
    }

    /// Включает прокси и сразу проверяет чтением, что значения действительно записаны.
    pub fn enable_checked(&self, port: u16) -> Result<(), VpnError> {
        self.enable(port)?;
        match self.check(port)? {
            ProxyCheck::Ours => Ok(()),
            _ => {
                let _ = self.restore();
                Err(VpnError::Proxy(
                    "значения не записались, проверьте, не мешает ли другая программа".into(),
                ))
            }
        }
    }

    fn backup_value(&self, name: &str, kind: Kind) -> Result<(), VpnError> {
        let has = format!("Has{name}");
        let prev = format!("Prev{name}");
        match kind {
            Kind::Dword => match self
                .registry
                .read_dword(INTERNET_KEY, name)
                .map_err(proxy_err)?
            {
                Some(v) => {
                    self.registry
                        .write_dword(BACKUP_KEY, &prev, v)
                        .map_err(proxy_err)?;
                    self.registry
                        .write_dword(BACKUP_KEY, &has, 1)
                        .map_err(proxy_err)
                }
                None => self
                    .registry
                    .write_dword(BACKUP_KEY, &has, 0)
                    .map_err(proxy_err),
            },
            Kind::Text => match self
                .registry
                .read_string(INTERNET_KEY, name)
                .map_err(proxy_err)?
            {
                Some(v) => {
                    self.registry
                        .write_string(BACKUP_KEY, &prev, &v)
                        .map_err(proxy_err)?;
                    self.registry
                        .write_dword(BACKUP_KEY, &has, 1)
                        .map_err(proxy_err)
                }
                None => self
                    .registry
                    .write_dword(BACKUP_KEY, &has, 0)
                    .map_err(proxy_err),
            },
        }
    }

    /// Возвращает прежние настройки. Если пользователь или другая программа уже сменили
    /// адрес прокси, их значения не трогаем, копию просто снимаем. Без копии ничего не делает.
    pub fn restore(&self) -> Result<Restored, VpnError> {
        if !self.is_active() {
            return Ok(Restored::Nothing);
        }
        let ours = self
            .registry
            .read_string(BACKUP_KEY, "Ours")
            .map_err(proxy_err)?;
        let current = self
            .registry
            .read_string(INTERNET_KEY, "ProxyServer")
            .map_err(proxy_err)?;
        let untouched = ours.is_some() && current == ours;
        if untouched {
            for (name, kind) in VALUES {
                self.restore_value(name, *kind)?;
            }
        }
        self.registry.delete_key(BACKUP_KEY).map_err(proxy_err)?;
        self.refresh.refresh();
        Ok(if untouched {
            Restored::Restored
        } else {
            Restored::ForeignKept
        })
    }

    fn restore_value(&self, name: &str, kind: Kind) -> Result<(), VpnError> {
        let had = matches!(
            self.registry.read_dword(BACKUP_KEY, &format!("Has{name}")),
            Ok(Some(1))
        );
        let prev = format!("Prev{name}");
        if !had {
            return self
                .registry
                .delete_value(INTERNET_KEY, name)
                .map_err(proxy_err);
        }
        match kind {
            Kind::Dword => {
                if let Some(v) = self
                    .registry
                    .read_dword(BACKUP_KEY, &prev)
                    .map_err(proxy_err)?
                {
                    self.registry
                        .write_dword(INTERNET_KEY, name, v)
                        .map_err(proxy_err)?;
                }
            }
            Kind::Text => {
                if let Some(v) = self
                    .registry
                    .read_string(BACKUP_KEY, &prev)
                    .map_err(proxy_err)?
                {
                    self.registry
                        .write_string(INTERNET_KEY, name, &v)
                        .map_err(proxy_err)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::system::CoreError;
    use crate::tweaks::RegistryPort;

    #[derive(Clone, Debug, PartialEq)]
    pub enum V {
        D(u32),
        S(String),
    }

    /// Поддельный реестр: ключ + имя -> значение.
    #[derive(Default)]
    pub struct FakeRegistry {
        pub map: Mutex<HashMap<(String, String), V>>,
        /// Имитация отказа записи в настройки Windows (для проверки сообщений об ошибке отката).
        pub fail_internet_writes: std::sync::atomic::AtomicBool,
    }

    impl FakeRegistry {
        pub fn get(&self, key: &str, name: &str) -> Option<V> {
            self.map
                .lock()
                .unwrap()
                .get(&(key.into(), name.into()))
                .cloned()
        }
        pub fn set_s(&self, key: &str, name: &str, v: &str) {
            self.map
                .lock()
                .unwrap()
                .insert((key.into(), name.into()), V::S(v.into()));
        }
        pub fn set_d(&self, key: &str, name: &str, v: u32) {
            self.map
                .lock()
                .unwrap()
                .insert((key.into(), name.into()), V::D(v));
        }
    }

    impl FakeRegistry {
        fn check_fail(&self, key: &str) -> Result<(), CoreError> {
            if key == super::INTERNET_KEY
                && self
                    .fail_internet_writes
                    .load(std::sync::atomic::Ordering::SeqCst)
            {
                return Err(CoreError::Probe("запись отклонена".into()));
            }
            Ok(())
        }
    }

    impl RegistryPort for FakeRegistry {
        fn read_dword(&self, key: &str, name: &str) -> Result<Option<u32>, CoreError> {
            Ok(match self.get(key, name) {
                Some(V::D(v)) => Some(v),
                _ => None,
            })
        }
        fn write_dword(&self, key: &str, name: &str, value: u32) -> Result<(), CoreError> {
            self.check_fail(key)?;
            self.set_d(key, name, value);
            Ok(())
        }
        fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, CoreError> {
            Ok(match self.get(key, name) {
                Some(V::S(v)) => Some(v),
                _ => None,
            })
        }
        fn write_string(&self, key: &str, name: &str, value: &str) -> Result<(), CoreError> {
            self.check_fail(key)?;
            self.set_s(key, name, value);
            Ok(())
        }
        fn delete_value(&self, key: &str, name: &str) -> Result<(), CoreError> {
            self.check_fail(key)?;
            self.map.lock().unwrap().remove(&(key.into(), name.into()));
            Ok(())
        }
        fn delete_key(&self, key: &str) -> Result<(), CoreError> {
            self.map.lock().unwrap().retain(|(k, _), _| k != key);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{FakeRegistry, V};
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Counter(AtomicUsize);
    impl ProxyRefresh for Counter {
        fn refresh(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn make() -> (Arc<FakeRegistry>, Arc<Counter>, SystemProxy) {
        let reg = Arc::new(FakeRegistry::default());
        let counter = Arc::new(Counter(AtomicUsize::new(0)));
        let proxy = SystemProxy::new(reg.clone(), counter.clone());
        (reg, counter, proxy)
    }

    #[test]
    fn enable_sets_values_and_restore_returns_originals() {
        let (reg, counter, proxy) = make();
        reg.set_d(INTERNET_KEY, "ProxyEnable", 0);
        reg.set_s(INTERNET_KEY, "ProxyServer", "corp:3128");
        reg.set_s(INTERNET_KEY, "AutoConfigURL", "http://pac/x.pac");
        proxy.enable(20808).unwrap();
        assert_eq!(reg.get(INTERNET_KEY, "ProxyEnable"), Some(V::D(1)));
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("127.0.0.1:20808".into()))
        );
        assert!(reg.get(INTERNET_KEY, "AutoConfigURL").is_none());
        assert!(proxy.is_active());
        proxy.restore().unwrap();
        assert_eq!(reg.get(INTERNET_KEY, "ProxyEnable"), Some(V::D(0)));
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("corp:3128".into()))
        );
        assert_eq!(
            reg.get(INTERNET_KEY, "AutoConfigURL"),
            Some(V::S("http://pac/x.pac".into()))
        );
        assert!(
            reg.get(INTERNET_KEY, "ProxyOverride").is_none(),
            "значения не было — удалено"
        );
        assert!(!proxy.is_active());
        assert!(reg.map.lock().unwrap().keys().all(|(k, _)| k != BACKUP_KEY));
        assert_eq!(counter.0.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn missing_originals_are_deleted_on_restore() {
        let (reg, _, proxy) = make();
        proxy.enable(1080).unwrap();
        proxy.restore().unwrap();
        for name in [
            "ProxyEnable",
            "ProxyServer",
            "ProxyOverride",
            "AutoConfigURL",
        ] {
            assert!(reg.get(INTERNET_KEY, name).is_none(), "{name}");
        }
    }

    #[test]
    fn second_enable_keeps_the_first_backup() {
        let (reg, _, proxy) = make();
        reg.set_s(INTERNET_KEY, "ProxyServer", "orig:1");
        proxy.enable(1000).unwrap();
        proxy.enable(2000).unwrap();
        proxy.restore().unwrap();
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("orig:1".into()))
        );
    }

    #[test]
    fn crash_recovery_uses_the_leftover_backup() {
        let (reg, _, proxy) = make();
        reg.set_s(INTERNET_KEY, "ProxyServer", "orig:1");
        proxy.enable(1000).unwrap();
        // «Перезапуск»: новый объект над тем же реестром.
        let after_crash = SystemProxy::new(reg.clone(), Arc::new(NoRefresh));
        assert!(after_crash.is_active());
        after_crash.restore().unwrap();
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("orig:1".into()))
        );
    }

    #[test]
    fn restore_without_backup_does_nothing() {
        let (reg, counter, proxy) = make();
        reg.set_s(INTERNET_KEY, "ProxyServer", "user:1");
        proxy.restore().unwrap();
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("user:1".into()))
        );
        assert_eq!(counter.0.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn foreign_proxy_change_is_not_overwritten() {
        let (reg, _, proxy) = make();
        proxy.enable(1000).unwrap();
        reg.set_s(INTERNET_KEY, "ProxyServer", "other-vpn:9");
        proxy.restore().unwrap();
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("other-vpn:9".into()))
        );
        assert!(!proxy.is_active());
    }

    #[test]
    fn foreign_proxy_and_pac_are_reported_without_addresses() {
        let reg = Arc::new(FakeRegistry::default());
        let proxy = SystemProxy::new(reg.clone(), Arc::new(NoRefresh));
        assert_eq!(proxy.foreign_conflict().unwrap(), None);
        reg.set_d(INTERNET_KEY, "ProxyEnable", 1);
        reg.set_s(INTERNET_KEY, "ProxyServer", "10.1.2.3:8080");
        assert_eq!(proxy.foreign_conflict().unwrap(), Some("системный прокси"));
        reg.set_d(INTERNET_KEY, "ProxyEnable", 0);
        assert_eq!(
            proxy.foreign_conflict().unwrap(),
            None,
            "выключенный прокси не мешает"
        );
        reg.set_s(INTERNET_KEY, "AutoConfigURL", "http://pac.example/x.pac");
        assert_eq!(
            proxy.foreign_conflict().unwrap(),
            Some("сценарий автонастройки прокси")
        );
    }

    #[test]
    fn restore_reports_what_happened() {
        let reg = Arc::new(FakeRegistry::default());
        let proxy = SystemProxy::new(reg.clone(), Arc::new(NoRefresh));
        assert_eq!(proxy.restore().unwrap(), Restored::Nothing);
        proxy.enable(1111).unwrap();
        assert_eq!(proxy.restore().unwrap(), Restored::Restored);
        proxy.enable(2222).unwrap();
        reg.set_s(INTERNET_KEY, "ProxyServer", "10.0.0.9:3128");
        assert_eq!(proxy.restore().unwrap(), Restored::ForeignKept);
        assert_eq!(
            reg.get(INTERNET_KEY, "ProxyServer"),
            Some(V::S("10.0.0.9:3128".into())),
            "чужой адрес не тронут"
        );
    }

    #[test]
    fn failed_restore_keeps_the_backup_for_a_retry() {
        let reg = Arc::new(FakeRegistry::default());
        let proxy = SystemProxy::new(reg.clone(), Arc::new(NoRefresh));
        proxy.enable(3333).unwrap();
        reg.fail_internet_writes
            .store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(proxy.restore().is_err());
        assert!(proxy.is_active(), "копия осталась, откат можно повторить");
        reg.fail_internet_writes
            .store(false, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(proxy.restore().unwrap(), Restored::Restored);
    }
}
