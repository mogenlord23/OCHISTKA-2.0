//! Эвристика по имени и месту файла: двойное расширение, исполняемый файл в загрузках или в
//! автозагрузке, очень маленькая программа, слова взломщиков. Только подсказка с причинами.

use crate::assess::Signal;

/// Признаки файла для эвристики. Заполняет адаптер платформы; содержимое не читается.
#[derive(Clone, Debug, PartialEq)]
pub struct FileFacts {
    pub file_name: String,
    pub size_bytes: u64,
    pub in_downloads: bool,
    pub in_startup: bool,
}

const EXECUTABLE_EXTENSIONS: [&str; 10] = [
    "exe", "scr", "bat", "cmd", "ps1", "vbs", "js", "msi", "com", "pif",
];
const DOCUMENT_EXTENSIONS: [&str; 9] = [
    "pdf", "doc", "docx", "xls", "xlsx", "txt", "jpg", "jpeg", "png",
];
const CRACK_WORDS: [&str; 3] = ["crack", "keygen", "patch"];

/// Оценка по имени и месту. Не вердикт: файл решает пользователь.
pub fn score_heuristic(facts: &FileFacts) -> Signal {
    let name = facts.file_name.to_lowercase();
    let parts: Vec<&str> = name.split('.').collect();
    let extension = if parts.len() >= 2 {
        parts[parts.len() - 1]
    } else {
        ""
    };
    let is_executable = EXECUTABLE_EXTENSIONS.contains(&extension);

    let mut score: u32 = 0;
    let mut reasons = Vec::new();
    if is_executable && parts.len() >= 3 && DOCUMENT_EXTENSIONS.contains(&parts[parts.len() - 2]) {
        score += 40;
        reasons.push("двойное расширение: документ, замаскированный под программу".to_string());
    }
    if is_executable && facts.in_startup {
        score += 30;
        reasons.push("исполняемый файл в автозагрузке".to_string());
    } else if is_executable && facts.in_downloads {
        score += 30;
        reasons.push("исполняемый файл в загрузках".to_string());
    }
    if is_executable && facts.size_bytes < 1024 {
        score += 10;
        reasons.push("очень маленький исполняемый файл".to_string());
    }
    if is_executable && CRACK_WORDS.iter().any(|word| name.contains(word)) {
        score += 20;
        reasons.push("имя похоже на взломщик".to_string());
    }
    Signal { score, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(name: &str, size: u64, downloads: bool, startup: bool) -> FileFacts {
        FileFacts {
            file_name: name.into(),
            size_bytes: size,
            in_downloads: downloads,
            in_startup: startup,
        }
    }

    #[test]
    fn plain_document_is_not_flagged() {
        let signal = score_heuristic(&facts("report.pdf", 50_000, false, false));
        assert_eq!(signal.score, 0);
        assert!(signal.reasons.is_empty());
    }

    #[test]
    fn double_extension_is_flagged_with_reason() {
        let signal = score_heuristic(&facts("invoice.pdf.exe", 5_000, false, false));
        assert!(signal.score >= 40);
        assert!(signal.reasons[0].starts_with("двойное расширение"));
    }

    #[test]
    fn executable_in_downloads_is_flagged() {
        let signal = score_heuristic(&facts("setup.exe", 5_000, true, false));
        assert_eq!(signal.score, 30);
        assert_eq!(
            signal.reasons,
            vec!["исполняемый файл в загрузках".to_string()]
        );
    }

    #[test]
    fn executable_in_startup_is_flagged_once() {
        let signal = score_heuristic(&facts("tool.exe", 5_000, true, true));
        assert_eq!(signal.score, 30);
        assert_eq!(
            signal.reasons,
            vec!["исполняемый файл в автозагрузке".to_string()]
        );
    }

    #[test]
    fn all_signals_together_reach_one_hundred() {
        let signal = score_heuristic(&facts("crack.doc.exe", 10, false, true));
        assert_eq!(signal.score, 100);
    }
}
