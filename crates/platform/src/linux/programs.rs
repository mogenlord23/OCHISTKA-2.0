//! Заглушка для Linux: реестр Windows и автозагрузка Windows недоступны.

use ochistka_core::programs::{ProgramsPort, RawProgramRecord, RawStartup};
use ochistka_core::system::CoreError;

pub struct UnsupportedPrograms;

fn unsupported() -> CoreError {
    CoreError::Unsupported {
        feature: "Программы".into(),
        os: "Linux".into(),
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
