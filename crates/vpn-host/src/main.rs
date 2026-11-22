//! Служба «OchistkaVpn» (Windows). sing-box сам не умеет работать как служба, поэтому эта оболочка
//! по запуску службы берёт конфигурацию из аргумента, проверяет её, запускает ядро из папки программы,
//! а по остановке завершает его. Служба запускается вручную (кнопкой в программе), автозапуска нет.
#![cfg_attr(not(windows), allow(dead_code))]

mod logic;
#[cfg(windows)]
mod service;

fn main() {
    #[cfg(windows)]
    service::run();
    #[cfg(not(windows))]
    eprintln!("ochistka-vpn-host работает только в Windows");
}
