# Fix Record: FND-0191 (+ FND-0196, FND-0060, FND-0091)

**Date**: 2026-11-18
**Fix unit**: U01 (cascade: Rust DTOs, generated types, api.ts, tests, ADR)
**Findings**: FND-0191, FND-0196, FND-0060, FND-0091
**Decision**: D5 (owner, binding): TS types are generated from Rust DTOs.

## Choice: ts-rs (not specta)

- ts-rs 12.0.1 builds offline-capable from the cargo registry available in this session (`cargo add` / `cargo fetch` succeeded; sources in `~/.cargo/registry/src/*/ts-rs-12.0.1`). No reason to fall back to specta.
- Context7 was not authorized, so the API was confirmed from the crate source and docs shipped in the crate: `ts-rs-12.0.1/src/lib.rs` (doc comments for `#[ts(export)]`, `#[ts(export_to = "..")]` relative to `TS_RS_EXPORT_DIR`, `#[ts(type = "..")]`, serde-compat enabled by default so `#[serde(rename_all)]` is honoured, `TS::export_all(&Config)`, `Config::new().with_out_dir(..)`, `TS_RS_EXPORT_DIR` env) and `ts-rs-12.0.1/README.md`. `cargo search` confirmed version 12.0.1. No WebSearch was needed.
- No API was invented; all calls compiled and the generated output was inspected (camelCase keys, `Array<DiskStatus>`, imports between files).

## Original Findings

Hand-written TS mirrors of Rust DTOs (FND-0191), no non-empty `findings` coverage (FND-0196), `invoke<T>` as an unchecked compile-time cast (FND-0060), and the related type-drift risk (FND-0091), as described in the finding JSONs.

## Change Plan

- Change: derive `TS` on all IPC DTOs, commit generated `.ts`, re-export from `types.ts`, add runtime guards and drift/fixture tests.
- Not changing: `CommandError`/`ErrorKind` (frontend-only; the Rust error DTO lives in `commands.rs`, owned by another unit), `SectionId`, `POLL_INTERVAL_MS`, `toCommandError`, components.
- Risk: u64 to `number` is exact only up to 2^53-1 bytes (about 8 PiB); documented in the ADR. A malformed backend answer now rejects with a `probe` CommandError instead of flowing through.

## Edits Applied

- `crates/ochistka-core/Cargo.toml`: `ts-rs = "12.0.1"` (Cargo.lock updated: ts-rs, ts-rs-macros, termcolor).
- `crates/ochistka-core/src/system.rs`, `scan.rs`: `#[derive(TS)]` + `#[ts(export, export_to = "<Name>.ts")]` on CpuStatus, MemoryStatus, DiskStatus, SystemOverview, Finding, ScanReport, EngineStatus, Suspicion; `#[ts(type = "number")]` on every `total_bytes`/`used_bytes` u64. `FileFacts` and `CoreError` are not serialized across IPC and are not exported.
- `.cargo/config.toml` (new): `TS_RS_EXPORT_DIR = target/ts-rs` so the automatic `#[ts(export)]` test output goes to a scratch dir, not into the committed folder (otherwise it would race the drift test and mask drift).
- `crates/ochistka-core/tests/generated_types.rs` (new): exports all types into a temp dir and compares with `apps/desktop/src/lib/generated/` (CRLF-normalized); fails on drift or file-set mismatch. Regenerate with `OCHISTKA_BLESS=1 cargo test -p ochistka-core --test generated_types`. Verified: appending a line to a generated file made the test fail; bless restored it.
- `apps/desktop/src/lib/generated/*.ts` (new, 8 files, committed): CpuStatus, MemoryStatus, DiskStatus, SystemOverview, Finding, ScanReport, EngineStatus, Suspicion.
- `apps/desktop/src/lib/types.ts`: `export type { ... } from './generated/...'` with the same names the app imports (checked by grep: components import CommandError, EngineStatus, ScanReport, Suspicion, SystemOverview); `ErrorKind` and `CommandError` kept.
- `apps/desktop/src/lib/api.ts`: guards `isSystemOverview`, `isScanReport`, `isEngineStatus` (including nested disks/findings) and `invokeChecked`; used by `getSystemOverview`, `scanStart`, `clamavStatus`. `assessFile`, `defaultScanTarget` unchanged.
- `apps/desktop/src/lib/api.test.ts`: guard tests (accept/reject, nested malformed items, probe error on wrong shape), a non-empty `findings` test, and a test reading the generated `.ts` files and asserting fixture keys equal the generated field names.
- `.prettierignore`: `apps/desktop/src/lib/generated` (generated, double-quoted output).
- `docs/adr/0001-stack.md`: item 5 and consequence updated, addendum dated 2026-11-18 (ts-rs, u64 as number, drift test, runtime guards).

## Sanity Checks

- `cargo test --workspace`: PASS (core 29 + drift test, others pass)
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo fmt --all --check`: PASS
- `npm test -w apps/desktop`: PASS (88 tests)
- `npm run check -w apps/desktop`: PASS (0 errors, 0 warnings)
- Prettier applied to changed TS (api.ts, api.test.ts, types.ts).

## Windows note

None needed beyond the normal build; ts-rs is pure Rust and the drift test normalizes line endings.

## Next

Handing off to @test-engineer and @re-verifier.
