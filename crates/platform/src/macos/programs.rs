//! Заглушка для macOS: реестр Windows и автозагрузка Windows недоступны.
//! Причина: «Недоступно на macOS в этой версии».

use ochistka_core::programs::{ProgramsPort, RawProgramRecord, RawStartup};
use ochistka_core::system::CoreError;

pub struct UnsupportedPrograms;

fn unsupported() -> CoreError {
    CoreError::Unsupported {
        feature: "Программы".into(),
        os: "macOS".into(),
    }
}

impl ProgramsPort for UnsupportedPrograms {
    fn installed_records(&self) -> Result<Vec<RawProgramRecord>, CoreError> {
        Err(unsupported())
    }

    fn startup_records(&self) -> Result<Vec<RawStartup>, CoreError> {
        Err(unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_programs_stub_reports_unsupported() {
        assert!(matches!(
            UnsupportedPrograms.installed_records(),
            Err(CoreError::Unsupported { .. })
        ));
        assert!(matches!(
            UnsupportedPrograms.startup_records(),
            Err(CoreError::Unsupported { .. })
        ));
    }
}
