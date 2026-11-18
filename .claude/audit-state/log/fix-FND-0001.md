# Fix Record: FND-0001

**Date**: 2026-11-18
**Fix unit**: single-file (U45)
**Findings**: FND-0001

## Original Finding

Startup failure in a release build is invisible: `.expect` panics and the GUI subsystem (`windows_subsystem = "windows"` in release) has no console.

**Snippet (before):**
```rust
pub fn run() {
    tauri::Builder::default()
        .manage(AppState { ... })
        .invoke_handler(tauri::generate_handler![...])
        .run(tauri::generate_context!())
        .expect("не удалось запустить OCHISTKA");
}
```

## Change Plan

- Change: capture the `Result` of `.run(...)`; on error print to stderr, and on Windows show a native MessageBoxW (fixed Russian text), then exit with code 1.
- Not changing: Cargo.toml (no new crate; MessageBoxW declared via `extern "system"` from user32), no Tauri plugin/capability, no telemetry/network, `run()` signature.
- Risk: small `unsafe` FFI block, Windows-only. The box text is constant and contains no error detail, paths or secrets (detail goes to stderr only).

## Edits Applied

### apps/desktop/src-tauri/src/lib.rs

**Before:** `.run(...).expect("не удалось запустить OCHISTKA");`

**After:** `let result = ....run(...); if let Err(err) = result { eprintln!(...); #[cfg(windows)] show_startup_error(); std::process::exit(1); }` plus `#[cfg(windows)] fn show_startup_error()` declaring `MessageBoxW` from `user32`.

Rationale: the user now sees a dialog on Windows release builds instead of a silent exit.

## Sanity Checks

- cargo fmt --all --check: PASS
- cargo clippy --workspace --all-targets -- -D warnings: PASS (Linux target; the cfg(windows) code is NOT compiled here)
- cargo test --workspace: PASS
- Message box behaviour and Windows compilation: UNVERIFIED (Linux environment).

## Next

Handing off to @test-engineer / @re-verifier.

## Tests

**Framework**: cargo test (Rust workspace)
**Test file**: none added (not unit-testable on Linux)

### Why no automated test

`MessageBoxW` is a native Windows dialog behind `#[cfg(windows)]`. It cannot be run or asserted on Linux, and the failure path needs a real `tauri::Builder::run` error. Production code and test files were not changed.

Proposal (not applied): extract a pure helper, e.g. `fn startup_error_text() -> &'static str` (and the stderr line builder), so the text can be unit-tested without behaviour change. Needs a separate refactor unit.

### Confirmed by reading `apps/desktop/src-tauri/src/lib.rs`

- `show_startup_error` (lines 13-39) and its call (lines 57-58) are each gated by `#[cfg(windows)]`, so they are not compiled off-Windows.
- The non-Windows path still runs `eprintln!("не удалось запустить OCHISTKA: {err}")` and `std::process::exit(1)` (lines 56, 59), which are not gated.
- The box text is constant and has no error detail; detail goes to stderr only.

### Commands run

- `cargo test --workspace`: PASS (21 + 8 + 13 tests across crates, 0 failed)
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS (no warnings)
- The Windows-only code is NOT compiled or linted on Linux.

### Manual Windows check

1. On Windows 10/11 x64, build release: `npm run tauri build` (or `cargo build --release -p` the desktop crate).
2. Case A (corrupt config): copy the release exe aside, then make startup fail, e.g. run a build whose `tauri.conf.json` frontend/dist assets are missing or invalid. Alternative: occupy the dev/asset port with another process (`python -m http.server <port>`) where the app needs it.
3. Run the release exe from Explorer (no console).
4. Expect a native error dialog titled "OCHISTKA" with the text "Не удалось запустить OCHISTKA. Попробуйте перезапустить приложение или переустановить его." and the error icon; the text has no paths or error detail.
5. Run the same exe from `cmd.exe` with `2> err.txt`; expect a line "не удалось запустить OCHISTKA: ..." in err.txt and exit code 1 (`echo %errorlevel%`).
6. Also check that the Windows build compiles (`cargo check --target x86_64-pc-windows-msvc`) and clippy passes there.

**test_status**: `manual`
