# Fix Record: FND-0026 (+ FND-0025, FND-0018, FND-0098)

**Date**: 2026-11-18
**Fix unit**: U05 (cascade, backend + frontend)
**Findings**: FND-0026, FND-0025, FND-0018, FND-0098
**Decision**: D18 (one scan at a time, visible cancel, per-run timeout, per-file size cap)

## Original Finding

- FND-0026: no concurrency guard, cancel or timeout for clamscan (`Command::output()` with no kill).
- FND-0025: `assess_file` was a sync command doing filesystem I/O (hung mapped drive could block IPC).
- FND-0018: VirusesPage state was per-instance, so switching sections lost a running scan/results and allowed a second scan.
- FND-0098: `scanning` stayed true with text "Проверяем файлы" during the assessFile phase.

Snippet (before), commands.rs / clamav.rs:
```rust
pub fn assess_file(path: String) -> Result<Suspicion, CommandError> { assess_path(&path) }
let output = command
    .output()
    .map_err(|err| CoreError::Probe(format!("не удалось запустить clamscan: {err}")))?;
```

## Change Plan

- Change: guard + cancel + timeout + size cap in backend; async assess_file; module-level scan store in the UI.
- Not changing: scan.rs/system.rs (trait `VirusScanPort` untouched, hence cancel handle is a process-wide static in the adapter), api.ts, types.ts, windows/mod.rs, Home.svelte, App.svelte (not needed: store is module-level, props unchanged).
- Risk: process-wide static for the running child assumes one scan at a time, which the AppState guard enforces.

## Edits Applied

- `crates/platform/src/windows/clamav.rs`: constants `MAX_SCAN_TIME_MS` (600000, 10 min) and `MAX_FILE_SIZE_BYTES` (200 MB) in one place; `scan_args` adds `--max-scantime=` and `--max-filesize=`. `scan` now spawns the child (absolute path, piped stdout/stderr drained by threads), stores it in a static `Mutex`, polls `try_wait` every 50 ms, kills it on wall-clock timeout (same 10 min constant; error "превышено время проверки") and on `cancel_running_scan()` (error "проверка отменена"). New tests: args limits, cancel without scan, cancel kills a fake engine (unix).
- `apps/desktop/src-tauri/src/commands.rs`: `AppState.scan_running: Arc<AtomicBool>`; `try_begin_scan` returns `CommandError{kind:"probe", message:"проверка уже идёт"}` for a second scan; RAII `ScanGuard` released when the blocking task ends (also on panic); new command `scan_cancel`; `assess_file` is async and runs `assess_path` in `spawn_blocking`. New test for the guard.
- `apps/desktop/src-tauri/src/lib.rs`: `scan_running` initialised; `scan_cancel` registered in `generate_handler!`.
- `apps/desktop/src/lib/scanState.svelte.ts` (new): module-level `$state` store (phase idle|scanning|assessing, target, report, error, notice, suspicions, runs, assessFailures), `startScan`, `cancelScan`, `resetScanState`. `phase` is `scanning` only during `scanStart`, `assessing` during assessFile calls (FND-0098).
- `apps/desktop/src/lib/scanControl.ts` (new): `scanCancel()` wrapper over `invoke('scan_cancel')`. **Orchestrator: api.ts was left unchanged; this wrapper may be moved into api.ts after U01 lands.**
- `apps/desktop/src/components/VirusesPage.svelte`: uses the store; "Проверяем файлы…" only while scanning, "Оцениваем находки…" while assessing; start button disabled while busy; "Остановить" button shown (enabled) only while scanning; cancel shows notice "Проверка отменена" instead of an error. Props unchanged.
- `apps/desktop/src/components/VirusesPage.test.ts`: minimal update because state is no longer per-instance: `resetScanState()` in `afterEach`, plus `vi.mock('../lib/scanControl')`. Added 3 tests (state survives remount, assessing text, cancel flow). Existing tests unchanged otherwise.

## Sanity Checks

- `cargo test --workspace`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo fmt --all --check`: PASS (I ran `cargo fmt --all` once; it may have touched formatting in files of parallel units if they were unformatted)
- `npm test -w apps/desktop`: PASS (88 tests; one earlier run showed a transient failure in a types/generated-keys test belonging to another parallel unit, gone on rerun)
- `npm run check -w apps/desktop`: PASS (0 errors, 0 warnings)

## Caveats

- UNVERIFIED on Windows: timeout, cancel (Child::kill on clamscan.exe), the `--max-scantime`/`--max-filesize` flag behaviour with the bundled ClamAV version, and that Tauri runs sync commands on the UI thread (FND-0025 premise). Unix-only tests used a fake shell engine.
- `--max-scantime` in clamscan is a per-file scan-time limit in practice; the whole-run limit is enforced by the adapter's wall-clock deadline using the same constant.
- Cancel while a scan is in "assessing" phase is not offered (assessFile calls are short and not cancellable).
- Children are not tied to a job object, so an app exit mid-scan may still orphan clamscan (UNVERIFIED).

## Next

Handing off to @test-engineer and @re-verifier.
