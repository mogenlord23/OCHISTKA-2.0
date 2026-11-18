//! Видеокарты и материнская плата через sysinfo (только чтение). Код собирается на всех ОС,
//! потому что sysinfo переносим; на других ОС `current_hardware_port` отдаёт заглушку.

use ochistka_core::hardware::{build_gpu, build_motherboard, HardwareOverview, HardwarePort};
use ochistka_core::system::CoreError;
use sysinfo::{Gpus, Motherboard};

pub struct WindowsHardware;

impl WindowsHardware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsHardware {
    fn default() -> Self {
        Self::new()
    }
}

fn probe(err: sysinfo::Error) -> CoreError {
    CoreError::Probe(err.to_string())
}

impl HardwarePort for WindowsHardware {
    fn overview(&self) -> Result<HardwareOverview, CoreError> {
        let gpus = Gpus::new_with_refreshed_list().map_err(probe)?;
        let gpus = gpus
            .iter()
            .map(|gpu| {
                build_gpu(
                    gpu.vendor(),
                    gpu.model(),
                    gpu.usage(),
                    gpu.total_memory(),
                    gpu.used_memory(),
                )
            })
            .collect();
        let board = Motherboard::new().map_err(probe)?;
        Ok(HardwareOverview {
            gpus,
            motherboard: build_motherboard(
                board.vendor_name().as_deref(),
                board.name().as_deref(),
                board.version().as_deref(),
            ),
        })
    }
}
