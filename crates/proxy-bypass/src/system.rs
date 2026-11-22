//! Системный прокси: описание «порта» для адаптеров ОС и сохранённых прежних значений.

/// Прежние значения настроек прокси в виде пар «ключ — значение». Смысл ключей знает только
/// адаптер ОС, который их записал; здесь они хранятся и возвращаются как есть.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot(pub Vec<(String, String)>);

const HEADER: &str = "ochistka-proxy-backup v1";

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

impl Snapshot {
    /// Текст файла в папке данных: заголовок и строки `ключ<TAB>значение`.
    pub fn to_text(&self) -> String {
        let mut out = format!("{HEADER}\n");
        for (k, v) in &self.0 {
            out.push_str(&format!("{}\t{}\n", escape(k), escape(v)));
        }
        out
    }

    /// `None` — файл повреждён (нет заголовка или строка без значения).
    pub fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()? != HEADER {
            return None;
        }
        let mut pairs = Vec::new();
        for line in lines {
            if line.is_empty() {
                continue;
            }
            let (k, v) = line.split_once('\t')?;
            pairs.push((unescape(k), unescape(v)));
        }
        Some(Self(pairs))
    }
}

/// Системный прокси ОС. Реализации — в `crates/platform` (по одной на ОС), тестовые — в тестах.
pub trait SystemProxyPort: Send + Sync {
    /// Причина, по которой на этой системе обход недоступен; `None` — можно.
    fn unsupported(&self) -> Option<String>;
    /// Текущие значения (чтобы вернуть их при выключении).
    fn snapshot(&self) -> Result<Snapshot, String>;
    /// Направляет системный прокси на 127.0.0.1:port.
    fn set_local(&self, port: u16) -> Result<(), String>;
    /// Возвращает ранее сохранённые значения.
    fn restore(&self, snapshot: &Snapshot) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_survives_the_file_round_trip_with_odd_characters() {
        let s = Snapshot(vec![
            ("ProxyEnable".into(), "1".into()),
            ("ProxyServer".into(), "a:1;b\\2\tx\ny".into()),
            ("empty".into(), String::new()),
        ]);
        assert_eq!(Snapshot::from_text(&s.to_text()), Some(s));
    }

    #[test]
    fn a_damaged_file_is_not_a_snapshot() {
        assert_eq!(Snapshot::from_text(""), None);
        assert_eq!(Snapshot::from_text("other\nk\tv\n"), None);
        assert_eq!(
            Snapshot::from_text("ochistka-proxy-backup v1\nno-tab-here\n"),
            None
        );
    }
}
