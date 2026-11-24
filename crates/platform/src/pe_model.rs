//! Компактная локальная ONNX-модель. Читает только признаки заголовка PE.

use std::sync::{Arc, OnceLock};

use ochistka_core::assess::{ModelOpinion, SuspicionModel};
use ochistka_core::fs::sha256_hex;
use ochistka_core::heuristic::FileFacts;
use tract_onnx::prelude::*;

const FEATURE_COUNT: usize = 16;
const MODEL: &[u8] = include_bytes!("../resources/pe-header-v1.onnx");
const MANIFEST: &str = include_str!("../resources/pe-header-v1.json");

struct Detector {
    network: Arc<TypedRunnableModel>,
    threshold: f32,
}

static DETECTOR: OnceLock<Option<Detector>> = OnceLock::new();

/// Повышает подозрение для PE-файлов при высоком результате модели.
pub struct PeHeaderModel;

impl SuspicionModel for PeHeaderModel {
    fn opinion(&self, facts: &FileFacts, header: &[u8]) -> Option<ModelOpinion> {
        let features = pe_header_features(header, facts.size_bytes)?;
        let detector = DETECTOR.get_or_init(load_detector).as_ref()?;
        let risk = detector.predict(&features)?;
        (risk >= detector.threshold).then(|| ModelOpinion {
            score: (risk * 100.0).round() as u8,
            reason: "локальная модель отметила сочетание признаков заголовка PE".into(),
        })
    }
}

fn load_detector() -> Option<Detector> {
    let metadata: serde_json::Value = serde_json::from_str(MANIFEST).ok()?;
    let expected = metadata["model_sha256"].as_str()?;
    if sha256_hex(MODEL) != expected {
        return None;
    }
    let threshold = metadata["threshold"].as_f64()? as f32;
    if !(threshold > 0.0 && threshold < 1.0) {
        return None;
    }
    let network = tract_onnx::onnx()
        .model_for_read(&mut &MODEL[..])
        .ok()?
        .into_optimized()
        .ok()?
        .into_runnable()
        .ok()?;
    Some(Detector { network, threshold })
}

impl Detector {
    fn predict(&self, features: &[f32; FEATURE_COUNT]) -> Option<f32> {
        let input = Tensor::from_shape(&[1, FEATURE_COUNT], features).ok()?;
        let output = self.network.run(tvec!(input.into_tvalue())).ok()?;
        let view = output.first()?.try_as_plain_ram().ok()?;
        let risk = *view.as_slice::<f32>().ok()?.first()?;
        (0.0..=1.0).contains(&risk).then_some(risk)
    }
}

fn read_bytes<const N: usize>(bytes: &[u8], start: usize) -> Option<[u8; N]> {
    bytes.get(start..start.checked_add(N)?)?.try_into().ok()
}

fn pe_header_features(bytes: &[u8], file_size: u64) -> Option<[f32; FEATURE_COUNT]> {
    if bytes.get(..2)? != b"MZ" {
        return None;
    }
    let pe = u32::from_le_bytes(read_bytes(bytes, 60)?) as usize;
    if pe < 64 || &read_bytes::<4>(bytes, pe)? != b"PE\0\0" {
        return None;
    }
    let optional_start = pe.checked_add(24)?;
    let optional_len = usize::from(u16::from_le_bytes(read_bytes(bytes, pe.checked_add(20)?)?));
    let coff = u16::from_le_bytes(read_bytes(bytes, pe.checked_add(22)?)?);
    let optional_end = bytes.len().min(optional_start.checked_add(optional_len)?);
    let optional = bytes.get(optional_start..optional_end)?;

    let word = |offset| Some(u16::from_le_bytes(read_bytes(optional, offset)?));
    let dword = |offset| Some(u32::from_le_bytes(read_bytes(optional, offset)?));
    let (is_64, heap_commit) = match word(0)? {
        0x10b => (false, u64::from(dword(84)?)),
        0x20b => (true, u64::from_le_bytes(read_bytes(optional, 96)?)),
        _ => return None,
    };
    let subsystem = word(68)?;
    let ln = |n: u64| (n as f64).ln_1p() as f32;
    let flag = |value: bool| f32::from(u8::from(value));
    Some([
        ln(file_size),
        flag(is_64),
        flag(subsystem == 2),
        flag(subsystem == 3),
        flag(coff & 0x2000 != 0),
        ln(u64::from(*optional.get(2)?)),
        ln(u64::from(*optional.get(3)?)),
        ln(u64::from(word(44)?)),
        ln(u64::from(word(46)?)),
        ln(u64::from(word(40)?)),
        ln(u64::from(word(42)?)),
        ln(u64::from(word(48)?)),
        ln(u64::from(word(50)?)),
        ln(u64::from(dword(4)?)),
        ln(u64::from(dword(60)?)),
        ln(heap_commit),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_match_the_training_layout_and_invalid_headers_are_ignored() {
        let mut pe = [0u8; 512];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64u32.to_le_bytes());
        pe[64..68].copy_from_slice(b"PE\0\0");
        pe[84..86].copy_from_slice(&112u16.to_le_bytes());
        pe[86..88].copy_from_slice(&0x2000u16.to_le_bytes());
        let optional = &mut pe[88..];
        optional[..2].copy_from_slice(&0x20bu16.to_le_bytes());
        optional[2] = 14;
        optional[68..70].copy_from_slice(&3u16.to_le_bytes());
        optional[96..104].copy_from_slice(&4096u64.to_le_bytes());

        let features = pe_header_features(&pe, 1024).unwrap();
        assert_eq!(&features[1..5], &[1.0, 0.0, 1.0, 1.0]);
        assert!((features[5] - 15.0f32.ln()).abs() < 1e-6);
        assert!((features[15] - 4097.0f32.ln()).abs() < 1e-6);
        assert!(pe_header_features(&pe[..120], 1024).is_none());
        assert!(pe_header_features(b"ordinary text", 13).is_none());
    }

    #[test]
    fn embedded_onnx_model_loads_and_predicts() {
        let detector = load_detector().expect("valid embedded model and digest");
        let risk = detector.predict(&[0.0; FEATURE_COUNT]).unwrap();
        // Значение для этого вектора даёт эталонный интерпретатор ONNX.
        assert!((risk - 0.954_910_2).abs() < 1e-5);
    }
}
