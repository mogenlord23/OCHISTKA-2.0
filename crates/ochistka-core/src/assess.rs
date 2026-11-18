//! Итоговая оценка файла: инвентаризация + эвристика + ИИ-слой, с объяснением каждой причины.
//! Оценка только подсказывает: ни один слой не удаляет и не переносит файл.

use std::io::Read;

use serde::Serialize;
use ts_rs::TS;

use crate::fs::FsPort;
use crate::heuristic::{score_heuristic, FileFacts};
use crate::inventory::{inventory, HEADER_BYTES};
use crate::system::CoreError;

/// Сигнал одного слоя: добавка к оценке и причины, по которым она назначена.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Signal {
    pub score: u32,
    pub reasons: Vec<String>,
}

// Итог для интерфейса: оценка 0..=100 и причины с пометкой слоя.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "Suspicion.ts")]
pub struct Suspicion {
    pub score: u8,
    pub reasons: Vec<String>,
}

/// Вывод ИИ-слоя по файлу: оценка и одна причина.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelOpinion {
    pub score: u8,
    pub reason: String,
}

/// ИИ-слой. Реализация — локальная ONNX-модель (ADR 0004, пункт 4), пока не подключена.
/// Модель только повышает подозрение: `combine` не даёт ей понизить оценку.
pub trait SuspicionModel: Send + Sync {
    fn opinion(&self, facts: &FileFacts, header: &[u8]) -> Option<ModelOpinion>;
}

/// Заглушка: модели нет, мнения нет. Заменится локальной ONNX-моделью, когда будет выбрана
/// модель с понятной лицензией.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoModel;

impl SuspicionModel for NoModel {
    fn opinion(&self, _facts: &FileFacts, _header: &[u8]) -> Option<ModelOpinion> {
        None
    }
}

/// Складывает слои. Инвентаризация и эвристика суммируются; мнение модели добавляет половину
/// своей оценки сверху. Причины помечаются слоем, чтобы было видно, что подняло оценку.
pub fn combine(inventory: &Signal, heuristic: &Signal, model: Option<&ModelOpinion>) -> Suspicion {
    let mut reasons: Vec<String> = Vec::new();
    reasons.extend(inventory.reasons.iter().map(|r| format!("содержимое: {r}")));
    reasons.extend(
        heuristic
            .reasons
            .iter()
            .map(|r| format!("имя и место: {r}")),
    );

    let base = inventory.score + heuristic.score;
    let mut total = base;
    if let Some(opinion) = model {
        let bonus = u32::from(opinion.score) / 2;
        if bonus > 0 {
            total += bonus;
            reasons.push(format!("ИИ-подсказка (+{bonus}): {}", opinion.reason));
        }
    }
    Suspicion {
        score: total.min(100) as u8,
        reasons,
    }
}

/// Читает первые `HEADER_BYTES` байт файла. Файл не запускается и не читается целиком.
fn read_header(fs: &dyn FsPort, path: &str) -> Result<Vec<u8>, CoreError> {
    let mut reader = fs
        .open(path)
        .map_err(|err| CoreError::Probe(format!("файл недоступен: {err}")))?;
    let mut buffer = vec![0u8; HEADER_BYTES];
    let mut filled = 0;
    while filled < buffer.len() {
        let read = reader
            .read(&mut buffer[filled..])
            .map_err(|err| CoreError::Probe(format!("файл не читается: {err}")))?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    buffer.truncate(filled);
    Ok(buffer)
}

/// Полная оценка одного файла. `facts` собирает платформа (имя, размер, папки).
pub fn assess_path(
    fs: &dyn FsPort,
    path: &str,
    facts: &FileFacts,
    model: &dyn SuspicionModel,
) -> Result<Suspicion, CoreError> {
    let header = read_header(fs, path)?;
    let inv = inventory(&facts.file_name, &header);
    let heur = score_heuristic(facts);
    let opinion = model.opinion(facts, &header);
    Ok(combine(&inv, &heur, opinion.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake_fs::FakeFs;

    struct FixedModel(u8);

    impl SuspicionModel for FixedModel {
        fn opinion(&self, _facts: &FileFacts, _header: &[u8]) -> Option<ModelOpinion> {
            Some(ModelOpinion {
                score: self.0,
                reason: "тестовая модель".into(),
            })
        }
    }

    fn facts(name: &str, downloads: bool) -> FileFacts {
        FileFacts {
            file_name: name.into(),
            size_bytes: 4096,
            in_downloads: downloads,
            in_startup: false,
        }
    }

    #[test]
    fn no_model_adds_nothing() {
        let fs = FakeFs::new();
        let path = "C:\\Users\\Slava\\Downloads\\report.pdf";
        fs.file(path, b"%PDF-1.4 text", 0);
        let suspicion = assess_path(&fs, path, &facts("report.pdf", true), &NoModel).unwrap();
        assert_eq!(
            suspicion,
            Suspicion {
                score: 0,
                reasons: vec![]
            }
        );
    }

    #[test]
    fn disguised_program_gets_inventory_reason_and_heuristic_reason() {
        let fs = FakeFs::new();
        let path = "C:\\Users\\Slava\\Downloads\\invoice.pdf.exe";
        fs.file(path, b"MZ\x90\x00 payload", 0);
        let suspicion = assess_path(&fs, path, &facts("invoice.pdf.exe", true), &NoModel).unwrap();
        // Содержимое: программа под .exe — документа в расширении нет, поэтому только эвристика.
        assert_eq!(suspicion.score, 40 + 30);
        assert!(suspicion.reasons[0].starts_with("имя и место: двойное расширение"));
    }

    #[test]
    fn program_wearing_pdf_extension_adds_content_reason_first() {
        let fs = FakeFs::new();
        let path = "C:\\Users\\Slava\\Downloads\\invoice.pdf";
        fs.file(path, b"MZ\x90\x00 payload", 0);
        let suspicion = assess_path(&fs, path, &facts("invoice.pdf", false), &NoModel).unwrap();
        assert_eq!(suspicion.score, 60);
        assert_eq!(
            suspicion.reasons,
            vec!["содержимое: расширение .pdf, а содержимое — программа Windows (PE)".to_string()]
        );
    }

    #[test]
    fn model_opinion_only_raises_the_score() {
        let base = Signal {
            score: 30,
            reasons: vec!["x".into()],
        };
        let plain = combine(&Signal::default(), &base, None);
        let raised = combine(
            &Signal::default(),
            &base,
            Some(&ModelOpinion {
                score: 80,
                reason: "похоже".into(),
            }),
        );
        assert_eq!(plain.score, 30);
        assert_eq!(raised.score, 70);
        assert!(raised.score >= plain.score);
        assert!(raised
            .reasons
            .last()
            .unwrap()
            .starts_with("ИИ-подсказка (+40): похоже"));
    }

    #[test]
    fn model_cannot_lower_the_score() {
        let fs = FakeFs::new();
        let path = "C:\\Users\\Slava\\Downloads\\setup.exe";
        fs.file(path, b"MZ", 0);
        let with_zero = assess_path(&fs, path, &facts("setup.exe", true), &FixedModel(0)).unwrap();
        let without = assess_path(&fs, path, &facts("setup.exe", true), &NoModel).unwrap();
        assert_eq!(with_zero.score, without.score);
    }

    #[test]
    fn total_is_capped_at_one_hundred() {
        let inv = Signal {
            score: 90,
            reasons: vec![],
        };
        let heur = Signal {
            score: 40,
            reasons: vec![],
        };
        let capped = combine(
            &inv,
            &heur,
            Some(&ModelOpinion {
                score: 100,
                reason: "x".into(),
            }),
        );
        assert_eq!(capped.score, 100);
    }

    #[test]
    fn zero_byte_file_is_not_flagged_by_content() {
        let fs = FakeFs::new();
        let path = "C:\\Users\\Slava\\Downloads\\empty.pdf";
        fs.file(path, b"", 0);
        let suspicion = assess_path(&fs, path, &facts("empty.pdf", true), &NoModel).unwrap();
        assert_eq!(suspicion.score, 0);
    }

    #[test]
    fn missing_file_is_a_probe_error() {
        let fs = FakeFs::new();
        let err = assess_path(&fs, "C:\\nope.exe", &facts("nope.exe", true), &NoModel).unwrap_err();
        assert!(matches!(err, CoreError::Probe(_)));
    }
}
