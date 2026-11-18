//! Заглушка для Linux: сведения о видеокартах и плате пока не читаются.

use ochistka_core::hardware::{HardwareOverview, HardwarePort};
use ochistka_core::system::CoreError;

pub struct UnsupportedHardware;

impl HardwarePort for UnsupportedHardware {
    fn overview(&self) -> Result<HardwareOverview, CoreError> {
        Err(CoreError::Unsupported {
            feature: "Чтение комплектующих".into(),
            os: "Linux".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_reports_unsupported_with_linux_reason() {
        match UnsupportedHardware.overview() {
            Err(CoreError::Unsupported { feature, os }) => {
                assert_eq!(os, "Linux");
                assert!(feature.contains("комплектующих"));
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }
}
