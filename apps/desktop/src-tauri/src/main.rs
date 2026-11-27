#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--check-embedded-zapret"))
    {
        if let Err(err) = ochistka_desktop_lib::check_embedded_zapret() {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }
    ochistka_desktop_lib::run()
}
