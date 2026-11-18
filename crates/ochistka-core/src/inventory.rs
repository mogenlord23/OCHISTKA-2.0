//! Инвентаризация: тип содержимого по первым байтам файла (магические числа) и сравнение с
//! расширением. Файл не запускается и не открывается целиком: читается заголовок до 512 байт.

use crate::assess::Signal;

/// Сколько байт заголовка читаем для определения типа.
pub const HEADER_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// Пустой файл: ничего не определено.
    Empty,
    /// Исполняемый файл Windows (PE, заголовок `MZ`).
    Pe,
    /// Исполняемый файл Linux (ELF).
    Elf,
    /// Исполняемый файл macOS (Mach-O, 32 и 64 бит, little-endian).
    MachO,
    Pdf,
    Zip,
    Ole,
    Png,
    Jpeg,
    /// Нет известной сигнатуры (текст, данные, повреждённый заголовок).
    Other,
}

impl FileKind {
    fn is_program(self) -> bool {
        matches!(self, Self::Pe | Self::Elf | Self::MachO)
    }

    fn describe(self) -> &'static str {
        match self {
            Self::Pe => "программа Windows (PE)",
            Self::Elf => "программа Linux (ELF)",
            Self::MachO => "программа macOS (Mach-O)",
            _ => "не программа",
        }
    }
}

/// Тип по магическим числам. Пустой заголовок — `Empty`, неизвестный — `Other`.
pub fn detect_kind(header: &[u8]) -> FileKind {
    let head = &header[..header.len().min(HEADER_BYTES)];
    if head.is_empty() {
        FileKind::Empty
    } else if head.starts_with(b"MZ") {
        FileKind::Pe
    } else if head.starts_with(b"\x7fELF") {
        FileKind::Elf
    } else if head.starts_with(&[0xCF, 0xFA, 0xED, 0xFE])
        || head.starts_with(&[0xCE, 0xFA, 0xED, 0xFE])
        || head.starts_with(&[0xFE, 0xED, 0xFA, 0xCE])
        || head.starts_with(&[0xFE, 0xED, 0xFA, 0xCF])
    {
        FileKind::MachO
    } else if head.starts_with(b"%PDF-") {
        FileKind::Pdf
    } else if head.starts_with(b"PK\x03\x04") {
        FileKind::Zip
    } else if head.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        FileKind::Ole
    } else if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        FileKind::Png
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        FileKind::Jpeg
    } else {
        FileKind::Other
    }
}

/// Расширение файла в нижнем регистре; пустая строка, если его нет.
fn extension(file_name: &str) -> String {
    let lower = file_name.to_lowercase();
    match lower.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_string(),
        _ => String::new(),
    }
}

/// Расширения, которые объявляют документ или картинку (не программу).
const DOCUMENT_EXTENSIONS: [&str; 9] = [
    "pdf", "doc", "docx", "xls", "xlsx", "txt", "jpg", "jpeg", "png",
];

/// Инвентаризация файла: тип по заголовку и несоответствие расширению.
/// Документ, у которого заголовок программы, — сильный признак маскировки (+60).
pub fn inventory(file_name: &str, header: &[u8]) -> Signal {
    let kind = detect_kind(header);
    let ext = extension(file_name);
    let mut score = 0;
    let mut reasons = Vec::new();
    if kind.is_program() && DOCUMENT_EXTENSIONS.contains(&ext.as_str()) {
        score += 60;
        reasons.push(format!(
            "расширение .{ext}, а содержимое — {}",
            kind.describe()
        ));
    }
    Signal { score, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_headers() {
        assert_eq!(detect_kind(b"MZ\x90\x00"), FileKind::Pe);
        assert_eq!(detect_kind(b"\x7fELF\x02\x01"), FileKind::Elf);
        assert_eq!(
            detect_kind(&[0xCF, 0xFA, 0xED, 0xFE, 0, 0]),
            FileKind::MachO
        );
        assert_eq!(detect_kind(b"%PDF-1.7\n"), FileKind::Pdf);
        assert_eq!(detect_kind(b"PK\x03\x04rest"), FileKind::Zip);
        assert_eq!(detect_kind(b"\x89PNG\r\n\x1a\n"), FileKind::Png);
        assert_eq!(detect_kind(&[0xFF, 0xD8, 0xFF, 0xE0]), FileKind::Jpeg);
    }

    #[test]
    fn empty_and_unknown_headers_are_not_guessed() {
        assert_eq!(detect_kind(b""), FileKind::Empty);
        assert_eq!(
            detect_kind(b"M"),
            FileKind::Other,
            "one byte is not a PE header"
        );
        assert_eq!(detect_kind(b"hello world"), FileKind::Other);
    }

    #[test]
    fn only_the_first_512_bytes_are_considered() {
        let mut data = vec![b'a'; 600];
        data[..2].copy_from_slice(b"MZ");
        assert_eq!(detect_kind(&data[..HEADER_BYTES]), FileKind::Pe);
        let mut late = vec![b'a'; 600];
        late[550..552].copy_from_slice(b"MZ");
        assert_eq!(detect_kind(&late), FileKind::Other);
    }

    #[test]
    fn program_with_document_extension_is_flagged() {
        let signal = inventory("invoice.pdf", b"MZ\x90\x00");
        assert_eq!(signal.score, 60);
        assert_eq!(
            signal.reasons,
            vec!["расширение .pdf, а содержимое — программа Windows (PE)".to_string()]
        );
    }

    #[test]
    fn real_pdf_and_empty_file_are_not_flagged() {
        assert_eq!(inventory("report.pdf", b"%PDF-1.4").score, 0);
        assert_eq!(inventory("empty.pdf", b"").score, 0);
    }

    #[test]
    fn real_program_with_program_extension_is_not_flagged_here() {
        let signal = inventory("setup.exe", b"MZ\x90\x00");
        assert_eq!(signal.score, 0);
    }

    #[test]
    fn extension_is_case_insensitive_and_needs_a_stem() {
        assert_eq!(inventory("SCAN.PDF", b"MZ").score, 60);
        assert_eq!(inventory(".pdf", b"MZ").score, 0);
    }
}
