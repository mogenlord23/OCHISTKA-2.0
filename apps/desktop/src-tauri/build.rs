use std::{env, fs, io, path::PathBuf};

const ZAPRET_FILES: &[&str] = &[
    "general.bat",
    "bin/winws.exe",
    "bin/cygwin1.dll",
    "bin/WinDivert.dll",
    "bin/WinDivert64.sys",
    "bin/quic_initial_www_google_com.bin",
    "bin/tls_clienthello_4pda_to.bin",
    "bin/tls_clienthello_www_google_com.bin",
    "lists/ipset-all.txt",
    "lists/ipset-exclude.txt",
    "lists/ipset-telegram.txt",
    "lists/list-exclude.txt",
    "lists/list-general.txt",
    "lists/list-google.txt",
];

fn build_zapret() -> io::Result<()> {
    println!("cargo:rerun-if-env-changed=OCHISTKA_ZAPRET_EMBED_DIR");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    let mut code = String::from("pub const FILES: &[(&str, &[u8])] = &[\n");
    if let Some(root) = env::var_os("OCHISTKA_ZAPRET_EMBED_DIR") {
        let root = PathBuf::from(root);
        for file in ZAPRET_FILES {
            let source = root.join(file);
            let destination = out.join("zapret-embedded").join(file);
            println!("cargo:rerun-if-changed={}", source.display());
            fs::create_dir_all(destination.parent().expect("embedded file parent"))?;
            fs::copy(&source, destination)
                .map_err(|err| io::Error::new(err.kind(), format!("Zapret {file}: {err}")))?;
            code.push_str(&format!(
                "    ({file:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/zapret-embedded/\", {file:?}))),\n"
            ));
        }
    }
    code.push_str("];\n");
    fs::write(out.join("zapret_embedded.rs"), code)
}

fn main() {
    build_zapret().expect("prepare built-in Zapret");
    tauri_build::build()
}
