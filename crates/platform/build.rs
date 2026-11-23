//! Сборка Windows-адаптеров.
//!
//! Когда рядом с репозиторием лежит официальная сборка ClamAV (`tools/clamav-win`, её кладёт
//! `tools/clamav-win/fetch.ps1`) и включена фича `libclamav`, добавляем путь к `clamav.lib` и
//! копируем `libclamav.dll` с её DLL-зависимостями рядом с исполняемым файлом. Без этой папки
//! сборка не меняется: движок просто не входит в неё.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if env::var_os("CARGO_FEATURE_LIBCLAMAV").is_none() {
        return;
    }

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .map(|workspace| workspace.join("tools/clamav-win"));
    let Some(root) = root else {
        return;
    };

    let dll = root.join("bin/libclamav.dll");
    let import_lib = root.join("lib/clamav.lib");
    if !dll.is_file() || !import_lib.is_file() {
        println!(
            "cargo:warning=libclamav включён, но сборки ClamAV нет: запустите tools/clamav-win/fetch.ps1"
        );
        return;
    }

    println!("cargo:rerun-if-changed={}", root.join("bin").display());
    println!(
        "cargo:rustc-link-search=native={}",
        root.join("lib").display()
    );

    // Окружение линковки и загрузки DLL: рядом с exe и в deps (там живут тестовые бинарники).
    // Каталог профиля (target/debug или target/release) — это родитель каталога `build`,
    // внутри которого cargo держит OUT_DIR.
    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let profile_dir = Path::new(&out_dir)
        .ancestors()
        .find(|dir| dir.file_name().is_some_and(|name| name == "build"))
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    let Some(profile_dir) = profile_dir else {
        return;
    };

    for target in [profile_dir.clone(), profile_dir.join("deps")] {
        if !target.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(root.join("bin")) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
            {
                continue;
            }
            let Some(name) = path.file_name() else {
                continue;
            };
            let destination = target.join(name);
            // Копия нужна только если её нет или она устарела: лишние записи на диск не нужны.
            let same = fs::metadata(&path)
                .ok()
                .zip(fs::metadata(&destination).ok())
                .is_some_and(|(source, copy)| {
                    source.len() == copy.len() && source.modified().ok() == copy.modified().ok()
                });
            if !same {
                let _ = fs::copy(&path, &destination);
            }
        }
    }
}
