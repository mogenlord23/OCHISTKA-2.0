//! Служба «OchistkaBypass» (Windows). Движок SonicDPI сам не умеет работать как служба (в нём нет
//! обмена с диспетчером служб), поэтому эта небольшая оболочка делает это за него: по запуску службы
//! стартует `ochistkadpi.exe run --profile youtube-discord`, по остановке завершает его и, если
//! драйвер WinDivert не работал до запуска, останавливает и его.

mod logic;
#[cfg(windows)]
mod service;

fn main() {
    #[cfg(windows)]
    service::run();
    #[cfg(not(windows))]
    eprintln!("ochistka-bypass-host работает только в Windows");
}
