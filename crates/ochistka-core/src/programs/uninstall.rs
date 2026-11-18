//! Удаление программы её штатным деинсталлятором: разбор строки `UninstallString` из реестра.
//! Чистый код. Запуск делает адаптер платформы, и только после подтверждения пользователя.

use thiserror::Error;

/// Папка Windows по умолчанию, когда адаптер не знает настоящую.
pub const DEFAULT_SYSTEM_ROOT: &str = r"C:\Windows";

/// Готовая к запуску команда: абсолютный путь к exe и строка аргументов «как в реестре».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallCommand {
    pub exe: String,
    pub raw_args: String,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum UninstallParseError {
    #[error("в реестре нет команды удаления")]
    Empty,
    #[error("не удалось разобрать команду удаления")]
    Unparsable,
    #[error("команда удаления использует переменные окружения или относительный путь")]
    NotAbsolute,
    #[error("в команде удаления есть недопустимые символы")]
    BadCharacters,
}

fn is_absolute_exe(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() > 7
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'\\'
        && path.to_ascii_lowercase().ends_with(".exe")
        && !path.split('\\').any(|part| part == "..")
}

/// Разбирает `UninstallString`. Без оболочки и без поиска по PATH: единственное исключение —
/// `msiexec.exe`, которому подставляется `<SystemRoot>\System32`. `/I{GUID}` заменяется на `/X{GUID}`.
pub fn parse_uninstall_string(
    raw: &str,
    system_root: &str,
) -> Result<UninstallCommand, UninstallParseError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(UninstallParseError::Empty);
    }
    if raw.chars().any(|c| c.is_control()) {
        return Err(UninstallParseError::BadCharacters);
    }
    let (exe, args) = if let Some(rest) = raw.strip_prefix('"') {
        let end = rest.find('"').ok_or(UninstallParseError::Unparsable)?;
        (rest[..end].trim().to_string(), rest[end + 1..].trim())
    } else if raw.len() >= 7
        && raw[..7].eq_ignore_ascii_case("msiexec")
        && raw[7..].chars().next().is_none_or(|c| c == ' ')
    {
        ("msiexec".to_string(), raw[7..].trim())
    } else {
        // Без кавычек: берём самое короткое начало, заканчивающееся на «.exe» перед пробелом или концом.
        let lower = raw.to_ascii_lowercase();
        let mut cut = None;
        let mut from = 0;
        while let Some(pos) = lower[from..].find(".exe") {
            let end = from + pos + 4;
            if end == raw.len() || raw.as_bytes()[end] == b' ' {
                cut = Some(end);
                break;
            }
            from = end;
        }
        let end = cut.ok_or(UninstallParseError::Unparsable)?;
        (raw[..end].to_string(), raw[end..].trim())
    };
    let mut exe = exe;
    let mut args = args.to_string();
    if exe.eq_ignore_ascii_case("msiexec.exe") || exe.eq_ignore_ascii_case("msiexec") {
        exe = format!(
            r"{}\System32\msiexec.exe",
            system_root.trim_end_matches('\\')
        );
    }
    if exe.contains('%') || !is_absolute_exe(&exe) {
        return Err(UninstallParseError::NotAbsolute);
    }
    if exe.to_ascii_lowercase().ends_with(r"\msiexec.exe") && args.len() >= 2 {
        let head = args[..2].to_ascii_lowercase();
        if head == "/i" {
            args = format!("/X{}", &args[2..]);
        }
    }
    Ok(UninstallCommand {
        exe,
        raw_args: args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = r"C:\Windows";

    #[test]
    fn quoted_path_with_args() {
        let cmd =
            parse_uninstall_string(r#""C:\Program Files\A\unins000.exe" /SILENT"#, ROOT).unwrap();
        assert_eq!(cmd.exe, r"C:\Program Files\A\unins000.exe");
        assert_eq!(cmd.raw_args, "/SILENT");
    }

    #[test]
    fn unquoted_path_with_spaces_ends_at_exe() {
        let cmd =
            parse_uninstall_string(r"C:\Program Files\A B\u.exe -uninstall -x", ROOT).unwrap();
        assert_eq!(cmd.exe, r"C:\Program Files\A B\u.exe");
        assert_eq!(cmd.raw_args, "-uninstall -x");
    }

    #[test]
    fn unquoted_path_without_args() {
        let cmd = parse_uninstall_string(r"C:\A\u.exe", ROOT).unwrap();
        assert_eq!(cmd.raw_args, "");
    }

    #[test]
    fn msiexec_gets_absolute_path_and_modify_becomes_remove() {
        let cmd = parse_uninstall_string("MsiExec.exe /I{ABC-1}", ROOT).unwrap();
        assert_eq!(cmd.exe, r"C:\Windows\System32\msiexec.exe");
        assert_eq!(cmd.raw_args, "/X{ABC-1}");
        let cmd = parse_uninstall_string("msiexec /x{ABC-1} /qn", r"D:\Win\").unwrap();
        assert_eq!(cmd.exe, r"D:\Win\System32\msiexec.exe");
        assert_eq!(cmd.raw_args, "/x{ABC-1} /qn");
    }

    #[test]
    fn other_bare_or_relative_names_are_refused() {
        assert_eq!(
            parse_uninstall_string("uninstall.exe /S", ROOT),
            Err(UninstallParseError::NotAbsolute)
        );
        assert_eq!(
            parse_uninstall_string(r"%ProgramFiles%\A\u.exe", ROOT),
            Err(UninstallParseError::NotAbsolute)
        );
        assert_eq!(
            parse_uninstall_string(r"C:\A\..\u.exe", ROOT),
            Err(UninstallParseError::NotAbsolute)
        );
    }

    #[test]
    fn empty_broken_and_control_characters_are_refused() {
        assert_eq!(
            parse_uninstall_string("  ", ROOT),
            Err(UninstallParseError::Empty)
        );
        assert_eq!(
            parse_uninstall_string(r#""C:\A\u.exe /S"#, ROOT),
            Err(UninstallParseError::Unparsable)
        );
        assert_eq!(
            parse_uninstall_string("C:\\A\\u.exe\n& calc", ROOT),
            Err(UninstallParseError::BadCharacters)
        );
        assert_eq!(
            parse_uninstall_string(r"C:\A\setup.cmd /x", ROOT),
            Err(UninstallParseError::Unparsable)
        );
    }
}
