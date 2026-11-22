//! Остатки туннеля VPN в Windows: адаптер и его маршруты. Ищем и снимаем только адаптер с нашим
//! именем (`OCHISTKA-VPN`); чужие адаптеры, маршруты и настройки не читаем как свои и не трогаем.
//! Команды запускаются по абсолютным путям (netsh.exe), без поиска по PATH.

use std::sync::Arc;

use ochistka_core::vpn::config::TUN_NAME;

pub const NETSH: &str = r"C:\Windows\System32\netsh.exe";

/// Запуск системной команды; в тестах подменяется.
pub trait CommandRun: Send + Sync {
    /// Возвращает вывод команды; `Err` — команду не удалось запустить или она завершилась ошибкой.
    fn run(&self, exe: &str, args: &[&str]) -> Result<String, String>;
}

pub struct TunnelCleaner {
    runner: Arc<dyn CommandRun>,
}

impl TunnelCleaner {
    pub fn new(runner: Arc<dyn CommandRun>) -> Self {
        Self { runner }
    }

    /// Строка вывода netsh относится к нашему адаптеру: имя — последний столбец и совпадает точно.
    fn is_ours(line: &str) -> bool {
        line.split_whitespace().next_back() == Some(TUN_NAME)
    }

    fn adapter_present(&self) -> bool {
        self.runner
            .run(NETSH, &["interface", "show", "interface"])
            .map(|out| out.lines().any(Self::is_ours))
            .unwrap_or(false)
    }

    fn routes_present(&self) -> bool {
        ["ipv4", "ipv6"].iter().any(|family| {
            self.runner
                .run(NETSH, &["interface", family, "show", "route"])
                .map(|out| out.lines().any(Self::is_ours))
                .unwrap_or(false)
        })
    }

    /// Остался наш адаптер или маршруты через него.
    pub fn leftovers(&self) -> bool {
        self.adapter_present() || self.routes_present()
    }

    /// Снимает маршруты нашего адаптера и выключает его. Возвращает, было ли что снимать.
    pub fn remove(&self) -> Result<bool, String> {
        let had = self.leftovers();
        if !had {
            return Ok(false);
        }
        let name = format!("interface={TUN_NAME}");
        for (family, prefixes) in [
            ("ipv4", ["0.0.0.0/0", "0.0.0.0/1", "128.0.0.0/1"]),
            ("ipv6", ["::/0", "::/1", "8000::/1"]),
        ] {
            for prefix in prefixes {
                // Ошибка «маршрута нет» — не причина прерывать снятие остальных.
                let _ = self.runner.run(
                    NETSH,
                    &[
                        "interface",
                        family,
                        "delete",
                        "route",
                        &format!("prefix={prefix}"),
                        &name,
                    ],
                );
            }
        }
        let _ = self.runner.run(
            NETSH,
            &[
                "interface",
                "set",
                "interface",
                &format!("name={TUN_NAME}"),
                "admin=disabled",
            ],
        );
        Ok(true)
    }
}

/// Запуск команд Windows без окна.
#[cfg(windows)]
pub struct WindowsRunner;

#[cfg(windows)]
impl CommandRun for WindowsRunner {
    fn run(&self, exe: &str, args: &[&str]) -> Result<String, String> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let output = std::process::Command::new(exe)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Поддельная система: есть ли наш адаптер, чужой адаптер, какие команды были.
    #[derive(Default)]
    struct Fake {
        ours: Mutex<bool>,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl CommandRun for Fake {
        fn run(&self, exe: &str, args: &[&str]) -> Result<String, String> {
            assert_eq!(exe, NETSH, "только абсолютный путь к netsh");
            self.calls
                .lock()
                .unwrap()
                .push(args.iter().map(|a| a.to_string()).collect());
            let ours = *self.ours.lock().unwrap();
            if args == ["interface", "show", "interface"] {
                let mut text = String::from(
                    "Состояние адм.  Состояние      Тип           Имя интерфейса\n\
                     Разрешен       Подключен      Выделенный    Ethernet\n\
                     Разрешен       Подключен      Выделенный    OCHISTKA-VPN-2\n\
                     Разрешен       Подключен      Выделенный    Other VPN\n",
                );
                if ours {
                    text.push_str("Разрешен       Подключен      Выделенный    OCHISTKA-VPN\n");
                }
                return Ok(text);
            }
            if args.len() == 4 && args[2] == "show" && args[3] == "route" {
                let mut text = String::from("No Manual 0 0.0.0.0/0 12 Ethernet\n");
                if ours {
                    text.push_str("No Manual 0 0.0.0.0/1 14 OCHISTKA-VPN\n");
                }
                return Ok(text);
            }
            if args.contains(&"admin=disabled") {
                *self.ours.lock().unwrap() = false;
            }
            Ok(String::new())
        }
    }

    #[test]
    fn nothing_is_found_or_touched_without_our_adapter_even_with_similar_names() {
        let fake = Arc::new(Fake::default());
        let cleaner = TunnelCleaner::new(fake.clone());
        assert!(!cleaner.leftovers(), "OCHISTKA-VPN-2 и Other VPN — не наши");
        assert_eq!(cleaner.remove(), Ok(false));
        // Только чтение: ни одной команды изменения.
        let calls = fake.calls.lock().unwrap();
        assert!(
            calls.iter().all(|c| c.iter().any(|a| a == "show")),
            "{calls:?}"
        );
    }

    #[test]
    fn our_leftovers_are_found_and_removed_only_for_our_name() {
        let fake = Arc::new(Fake::default());
        *fake.ours.lock().unwrap() = true;
        let cleaner = TunnelCleaner::new(fake.clone());
        assert!(cleaner.leftovers());
        assert_eq!(cleaner.remove(), Ok(true));
        assert!(!cleaner.leftovers(), "после снятия остатков нет");
        let calls = fake.calls.lock().unwrap();
        for call in calls.iter().filter(|c| !c.iter().any(|a| a == "show")) {
            let text = call.join(" ");
            assert!(
                text.contains(TUN_NAME),
                "команда изменения без нашего имени: {text}"
            );
            assert!(
                !text.contains("Ethernet") && !text.contains("Other"),
                "{text}"
            );
        }
        assert!(calls
            .iter()
            .any(|c| c.iter().any(|a| a == "admin=disabled")));
    }
}
