//! Короткое название модели диска и связь «буква тома → модель». Чистые функции, без кода ОС.

use std::collections::HashMap;

/// Служебные хвосты, которые Windows дописывает к модели диска.
const NOISE_WORDS: [&str; 6] = ["ATA", "SCSI", "NVME", "USB", "DISK", "DEVICE"];

/// Максимум слов в коротком названии.
pub const MAX_WORDS: usize = 3;

/// Токен похож на серийный номер или длинный код ревизии: ≥ 15 знаков с цифрами, или ≥ 8 цифр,
/// или шестнадцатеричная строка из 8+ знаков.
fn looks_like_serial(token: &str) -> bool {
    let len = token.chars().count();
    let digits = token.chars().filter(char::is_ascii_digit).count();
    let hex_only = token.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    (len >= 15 && digits > 0) || digits >= 8 || (len >= 8 && hex_only && digits > 0)
}

/// «KINGSTON» → «Kingston»; короткие (≤ 4) и содержащие цифры слова остаются как есть.
fn soften_case(word: &str) -> String {
    let letters_only = word.chars().all(|c| c.is_ascii_alphabetic());
    if letters_only && word.chars().count() > 4 && word.chars().all(|c| c.is_ascii_uppercase()) {
        let mut chars = word.chars();
        let first = chars.next().map(|c| c.to_string()).unwrap_or_default();
        format!("{first}{}", chars.as_str().to_ascii_lowercase())
    } else {
        word.to_string()
    }
}

/// Короткая модель: без служебных слов и серийных токенов, не более трёх слов.
/// Пусто или «ничего не осталось» — `None`.
pub fn short_model(raw: &str) -> Option<String> {
    let words: Vec<String> = raw
        .split_whitespace()
        .filter(|word| !NOISE_WORDS.contains(&word.to_ascii_uppercase().as_str()))
        .filter(|word| !looks_like_serial(word))
        .take(MAX_WORDS)
        .map(soften_case)
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// Значение в кавычках из WMI-ссылки вида `\\HOST\root\cimv2:Win32_LogicalDisk.DeviceID="C:"`;
/// двойные обратные косые черты разворачиваются.
pub fn wmi_ref_value(reference: &str) -> Option<String> {
    let start = reference.find('"')? + 1;
    let end = reference.rfind('"')?;
    (end > start).then(|| reference[start..end].replace("\\\\", "\\"))
}

/// Связывает буквы томов с моделями. Вход: диски `(DeviceID, Model)`, связи диск→раздел и
/// раздел→том как пары ссылок `(Antecedent, Dependent)`. Ключ результата — буква вида `C:`.
pub fn link_volumes_to_models(
    drives: &[(String, String)],
    drive_to_partition: &[(String, String)],
    partition_to_volume: &[(String, String)],
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for (drive_ref, partition_ref) in drive_to_partition {
        let (Some(drive_id), Some(partition_id)) =
            (wmi_ref_value(drive_ref), wmi_ref_value(partition_ref))
        else {
            continue;
        };
        let Some(model) = drives
            .iter()
            .find(|(id, _)| *id == drive_id)
            .and_then(|(_, model)| short_model(model))
        else {
            continue;
        };
        for (part_ref, volume_ref) in partition_to_volume {
            if wmi_ref_value(part_ref).as_deref() == Some(partition_id.as_str()) {
                if let Some(letter) = wmi_ref_value(volume_ref) {
                    out.insert(letter.to_ascii_uppercase(), model.clone());
                }
            }
        }
    }
    out
}

/// Буква тома из точки монтирования: `C:\` → `C:`.
pub fn volume_letter(mount_point: &str) -> Option<String> {
    let mut chars = mount_point.chars();
    let letter = chars.next()?;
    (letter.is_ascii_alphabetic() && chars.next() == Some(':'))
        .then(|| format!("{}:", letter.to_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_noise_and_serials_and_caps_at_three_words() {
        assert_eq!(
            short_model("KINGSTON SNV2S1000G ATA Device").as_deref(),
            Some("Kingston SNV2S1000G")
        );
        assert_eq!(short_model("WDC WD10EZEX-21WN4A0").as_deref(), Some("WDC"));
        assert_eq!(
            short_model("Samsung SSD 970 EVO Plus 1TB").as_deref(),
            Some("Samsung SSD 970")
        );
        assert_eq!(
            short_model("Crucial CT500MX500SSD1 2012E3E1A4F9").as_deref(),
            Some("Crucial CT500MX500SSD1")
        );
    }

    #[test]
    fn empty_or_only_noise_is_unknown() {
        assert_eq!(short_model("   "), None);
        assert_eq!(short_model("ATA Device"), None);
        assert_eq!(short_model("0123456789ABCDEF"), None);
    }

    #[test]
    fn extracts_quoted_wmi_values() {
        assert_eq!(
            wmi_ref_value(r#"\\PC\root\cimv2:Win32_LogicalDisk.DeviceID="C:""#).as_deref(),
            Some("C:")
        );
        assert_eq!(
            wmi_ref_value(r#"x.DeviceID="\\\\.\\PHYSICALDRIVE0""#).as_deref(),
            Some(r"\\.\PHYSICALDRIVE0")
        );
        assert_eq!(wmi_ref_value("no quotes"), None);
    }

    #[test]
    fn links_letters_to_models_through_partitions() {
        let drives = vec![
            (
                r"\\.\PHYSICALDRIVE0".to_string(),
                "KINGSTON NV2 ATA Device".to_string(),
            ),
            (r"\\.\PHYSICALDRIVE1".to_string(), String::new()),
        ];
        let d2p = vec![
            (
                r#"Win32_DiskDrive.DeviceID="\\\\.\\PHYSICALDRIVE0""#.to_string(),
                r#"Win32_DiskPartition.DeviceID="Disk #0, Partition #1""#.to_string(),
            ),
            (
                r#"Win32_DiskDrive.DeviceID="\\\\.\\PHYSICALDRIVE1""#.to_string(),
                r#"Win32_DiskPartition.DeviceID="Disk #1, Partition #0""#.to_string(),
            ),
        ];
        let p2v = vec![
            (
                r#"Win32_DiskPartition.DeviceID="Disk #0, Partition #1""#.to_string(),
                r#"Win32_LogicalDisk.DeviceID="C:""#.to_string(),
            ),
            (
                r#"Win32_DiskPartition.DeviceID="Disk #1, Partition #0""#.to_string(),
                r#"Win32_LogicalDisk.DeviceID="D:""#.to_string(),
            ),
        ];
        let map = link_volumes_to_models(&drives, &d2p, &p2v);
        assert_eq!(map.get("C:").map(String::as_str), Some("Kingston NV2"));
        assert_eq!(map.get("D:"), None);
    }

    #[test]
    fn volume_letter_reads_windows_mount_points() {
        assert_eq!(volume_letter(r"c:\").as_deref(), Some("C:"));
        assert_eq!(volume_letter("/"), None);
    }
}
