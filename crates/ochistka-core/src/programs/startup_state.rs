//! Состояние записи автозагрузки по флагу `StartupApproved` (то же, что «Диспетчер задач»).
//! Значение — 12 байт: первый байт чётный (2, 6) — включено, нечётный (3, 7) — отключено;
//! байты 4..12 — время отключения (FILETIME, little-endian). Чистая логика без реестра.

use super::StartupSource;

/// Размер значения `StartupApproved`.
pub const APPROVED_LEN: usize = 12;

/// Включена ли запись по значению. Пустое значение — `None` (флаг не задан, запись включена по умолчанию).
pub fn is_enabled(bytes: &[u8]) -> Option<bool> {
    bytes.first().map(|first| first & 1 == 0)
}

/// Новое значение флага. Прежние байты (если их 12 и более) сохраняются, меняются только состояние и время.
pub fn toggled_bytes(current: Option<&[u8]>, enable: bool, filetime: u64) -> Vec<u8> {
    let mut bytes = match current {
        Some(existing) if existing.len() >= APPROVED_LEN => existing[..APPROVED_LEN].to_vec(),
        _ => vec![0u8; APPROVED_LEN],
    };
    if enable {
        bytes[0] = 0x02;
        bytes[4..APPROVED_LEN].fill(0);
    } else {
        bytes[0] = 0x03;
        bytes[4..APPROVED_LEN].copy_from_slice(&filetime.to_le_bytes());
    }
    bytes
}

/// Подключ `StartupApproved`, где Windows хранит флаг для источника.
pub fn approved_subkey(source: StartupSource) -> &'static str {
    match source {
        StartupSource::HkcuRun | StartupSource::HklmRun => "Run",
        StartupSource::UserStartupFolder | StartupSource::CommonStartupFolder => "StartupFolder",
    }
}

/// Куст: `true` — HKLM (нужны права администратора), `false` — HKCU.
pub fn is_machine_wide(source: StartupSource) -> bool {
    matches!(
        source,
        StartupSource::HklmRun | StartupSource::CommonStartupFolder
    )
}

/// Имя значения допустимо: не пустое, без разделителей пути и управляющих символов.
pub fn valid_value_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= 260
        && !name
            .chars()
            .any(|c| c.is_control() || c == '\\' || c == '/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_comes_from_the_first_byte() {
        assert_eq!(is_enabled(&[2, 0, 0, 0]), Some(true));
        assert_eq!(is_enabled(&[6, 0, 0, 0]), Some(true));
        assert_eq!(is_enabled(&[3, 0, 0, 0]), Some(false));
        assert_eq!(is_enabled(&[7]), Some(false));
        assert_eq!(is_enabled(&[]), None);
    }

    #[test]
    fn disabling_writes_state_and_time_and_keeps_the_rest() {
        let mut current = vec![6u8, 1, 2, 3];
        current.extend_from_slice(&[0; 8]);
        let bytes = toggled_bytes(Some(&current), false, 0x0102030405060708);
        assert_eq!(bytes.len(), APPROVED_LEN);
        assert_eq!(bytes[0], 3);
        assert_eq!(&bytes[1..4], &[1, 2, 3]);
        assert_eq!(&bytes[4..], &0x0102030405060708u64.to_le_bytes());
        assert_eq!(is_enabled(&bytes), Some(false));
    }

    #[test]
    fn enabling_clears_the_time_and_roundtrips() {
        let off = toggled_bytes(None, false, 99);
        let on = toggled_bytes(Some(&off), true, 0);
        assert_eq!(on[0], 2);
        assert!(on[4..].iter().all(|b| *b == 0));
        assert_eq!(is_enabled(&on), Some(true));
    }

    #[test]
    fn short_current_value_is_replaced_by_a_full_one() {
        assert_eq!(toggled_bytes(Some(&[2]), true, 0).len(), APPROVED_LEN);
    }

    #[test]
    fn subkeys_and_scope_by_source() {
        assert_eq!(approved_subkey(StartupSource::HkcuRun), "Run");
        assert_eq!(
            approved_subkey(StartupSource::UserStartupFolder),
            "StartupFolder"
        );
        assert!(is_machine_wide(StartupSource::HklmRun));
        assert!(!is_machine_wide(StartupSource::HkcuRun));
    }

    #[test]
    fn value_names_are_checked() {
        assert!(valid_value_name("OneDrive"));
        assert!(!valid_value_name(""));
        assert!(!valid_value_name(r"a\b"));
        assert!(!valid_value_name("a\nb"));
    }
}
