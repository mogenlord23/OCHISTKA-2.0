# OCHISTKA audit: final report

_Generated from `coverage.json` and `findings/FND-*.json` on 2026-11-18T06:04:01Z._

## A. Coverage

| File | Declared Lines | Inspected Lines | Functions Analyzed | critical | high | medium | low | info |
|---|---|---|---|---|---|---|---|---|
| `apps/desktop/src-tauri/src/main.rs` | 5 | 5 | 1 | 0 | 0 | 0 | 0 | 0 |
| `apps/desktop/src-tauri/src/lib.rs` | 26 | 26 | 1 | 0 | 0 | 0 | 2 | 2 |
| `crates/ochistka-core/src/lib.rs` | 2 | 2 | 2 | 0 | 0 | 0 | 0 | 0 |
| `crates/platform/src/lib.rs` | 97 | 97 | 8 | 0 | 0 | 0 | 1 | 3 |
| `apps/desktop/src/main.ts` | 9 | 9 | 1 | 0 | 0 | 0 | 0 | 1 |
| `apps/desktop/src/App.svelte` | 208 | 208 | 4 | 0 | 0 | 2 | 2 | 3 |
| `apps/desktop/src-tauri/src/commands.rs` | 230 | 230 | 21 | 0 | 0 | 2 | 2 | 3 |
| `apps/desktop/src-tauri/capabilities/default.json` | 13 | 13 | 1 | 0 | 0 | 0 | 1 | 1 |
| `apps/desktop/src-tauri/tauri.conf.json` | 32 | 32 | 0 | 0 | 0 | 0 | 2 | 1 |
| `crates/platform/src/windows/mod.rs` | 80 | 80 | 5 | 0 | 0 | 2 | 1 | 4 |
| `crates/platform/src/windows/files.rs` | 99 | 99 | 4 | 0 | 0 | 0 | 1 | 2 |
| `crates/platform/src/windows/clamav.rs` | 152 | 152 | 11 | 0 | 0 | 1 | 3 | 1 |
| `crates/ochistka-core/src/scan.rs` | 307 | 307 | 4 | 0 | 0 | 0 | 2 | 3 |
| `apps/desktop/src/lib/api.ts` | 60 | 60 | 7 | 0 | 0 | 0 | 1 | 3 |
| `apps/desktop/src-tauri/build.rs` | 3 | 3 | 1 | 0 | 0 | 0 | 2 | 3 |
| `crates/ochistka-core/src/system.rs` | 180 | 180 | 15 | 0 | 0 | 0 | 0 | 5 |
| `crates/platform/src/linux/mod.rs` | 14 | 14 | 3 | 0 | 0 | 0 | 0 | 2 |
| `crates/platform/src/macos/mod.rs` | 14 | 14 | 3 | 0 | 0 | 0 | 0 | 0 |
| `apps/desktop/src/lib/types.ts` | 57 | 57 | 0 | 0 | 0 | 0 | 0 | 2 |
| `apps/desktop/src/components/ComputerPage.svelte` | 12 | 12 | 1 | 0 | 0 | 0 | 1 | 1 |
| `apps/desktop/src/components/Home.svelte` | 165 | 165 | 2 | 0 | 0 | 0 | 3 | 2 |
| `apps/desktop/src/components/Navigation.svelte` | 98 | 98 | 2 | 0 | 0 | 0 | 2 | 3 |
| `apps/desktop/src/components/Placeholder.svelte` | 12 | 12 | 1 | 0 | 0 | 0 | 1 | 1 |
| `apps/desktop/src/components/PlaceholderPage.svelte` | 24 | 24 | 1 | 0 | 0 | 0 | 2 | 3 |
| `apps/desktop/src/components/Subtabs.svelte` | 54 | 54 | 2 | 0 | 0 | 0 | 1 | 2 |
| `apps/desktop/src/components/TitleBar.svelte` | 71 | 71 | 4 | 0 | 0 | 0 | 3 | 3 |
| `apps/desktop/src/components/VirusesPage.svelte` | 311 | 311 | 2 | 0 | 0 | 1 | 6 | 3 |
| `apps/desktop/src/components/VpnPage.svelte` | 66 | 66 | 1 | 0 | 0 | 0 | 2 | 3 |
| `apps/desktop/src/lib/format.ts` | 20 | 20 | 2 | 0 | 0 | 0 | 1 | 5 |
| `apps/desktop/src/lib/sections.ts` | 76 | 76 | 0 | 0 | 0 | 0 | 1 | 2 |
| `apps/desktop/src/lib/tokens.ts` | 69 | 69 | 4 | 0 | 0 | 0 | 2 | 4 |
| `apps/desktop/src/vite-env.d.ts` | 7 | 7 | 0 | 0 | 0 | 0 | 1 | 1 |
| `Cargo.toml` | 8 | 8 | 0 | 0 | 0 | 0 | 1 | 3 |
| `crates/ochistka-core/Cargo.toml` | 12 | 12 | 0 | 0 | 0 | 0 | 0 | 0 |
| `crates/platform/Cargo.toml` | 9 | 9 | 0 | 0 | 0 | 0 | 0 | 1 |
| `apps/desktop/src-tauri/Cargo.toml` | 21 | 21 | 0 | 0 | 0 | 0 | 1 | 2 |
| `package.json` | 20 | 20 | 0 | 0 | 0 | 0 | 1 | 5 |
| `apps/desktop/package.json` | 28 | 28 | 0 | 0 | 0 | 0 | 0 | 1 |
| `apps/desktop/vite.config.ts` | 24 | 24 | 0 | 0 | 0 | 0 | 3 | 2 |
| `apps/desktop/svelte.config.js` | 5 | 5 | 1 | 0 | 0 | 0 | 0 | 4 |
| `apps/desktop/tsconfig.json` | 15 | 15 | 0 | 0 | 0 | 0 | 2 | 1 |
| `apps/desktop/src/App.test.ts` | 62 | 62 | 5 | 0 | 0 | 0 | 2 | 5 |
| `apps/desktop/src/components/Home.test.ts` | 116 | 116 | 9 | 0 | 0 | 2 | 1 | 3 |
| `apps/desktop/src/components/TitleBar.test.ts` | 38 | 38 | 2 | 0 | 0 | 0 | 2 | 2 |
| `apps/desktop/src/components/VirusesPage.test.ts` | 75 | 75 | 3 | 0 | 0 | 2 | 5 | 2 |
| `apps/desktop/src/components/VpnPage.test.ts` | 18 | 18 | 2 | 0 | 0 | 0 | 1 | 1 |
| `apps/desktop/src/lib/api.test.ts` | 85 | 85 | 8 | 0 | 0 | 1 | 2 | 3 |
| `apps/desktop/src/lib/format.test.ts` | 20 | 20 | 2 | 0 | 0 | 0 | 0 | 3 |
| `apps/desktop/src/lib/sections.test.ts` | 55 | 55 | 6 | 0 | 0 | 0 | 1 | 3 |
| `apps/desktop/src/lib/tokens.test.ts` | 93 | 93 | 10 | 0 | 0 | 0 | 2 | 4 |
| **Total** | **3277** | **3277** | | **0** | **0** | **13** | **70** | **117** |

Coverage: 50/50 files, 3277/3277 lines.

## B. Top findings (unresolved state inherited by future audits)

### #1 -- [sev:medium] [conf:high] FND-0026

File: `apps/desktop/src-tauri/src/commands.rs`:65-77 (blocking call in clamav.rs:75-77)
Status: needs-human

Title: No scan concurrency guard, cancellation or timeout; duplicate scans run in parallel and a hung clamscan is never stopped
Impact: scan_start("C:\\") invoked twice (e.g. after section switch) runs two full-disk clamscan processes. On app exit the child may be orphaned (UNVERIFIED: job object).
Resolution: Re-verifier CONFIRM-PARTIAL: single-scan guard, cancel, timeout and limits verified by trace and tests on Linux. Windows-only items UNVERIFIED: clamscan --max-scantime/--max-filesize behaviour, Child::kill on clamscan.exe, orphaned clamscan if the app exits mid-scan (no job object), possible spin if kill() fails. Windows check required.

### #2 -- [sev:low] [conf:medium] FND-0107

File: `apps/desktop/src/components/Navigation.svelte`:45-73 (window limit tauri.conf.json:19)
Status: needs-human

Title: At the minimum window height (600px) the rail, bottom controls and title bar total about 733px, so the page scrolls and the frameless title bar leaves the screen
Impact: Resize to 960x600: window controls and drag area scroll out of view until the user scrolls back.
Resolution: Code verified by re-trace; the 600px height check at 100/125/150 percent scaling on real Windows is pending. Estimate is about 591px against 600px, so label wrapping could break it.

### #3 -- [sev:low] [conf:medium] FND-0001

File: `apps/desktop/src-tauri/src/lib.rs`:24-25
Status: needs-human

Title: Startup failure in a release build is invisible: .expect panics and the GUI subsystem has no console
Impact: Release build on a machine where the webview cannot be created: the process exits with no dialog and no log.
Resolution: Code verified by re-trace; the Windows check is pending: compile and clippy on x86_64-pc-windows-msvc, then confirm the dialog text and the stderr line and exit code 1 on a real release build.

Verified fixes are listed in section D2 below; full resolution is in `log/fix-*.md`.

## C. Cross-cutting themes

| Pattern | Instances handled | Status |
|---|---|---|
| Types and DTO drift (TS against Rust) | U01: FND-0191, FND-0196, FND-0060, FND-0091 fixed via ts-rs generated types with a drift test; residuals FND-0212, FND-0213. | verified (residuals accepted) |
| Tests that cannot fail (false greens) | U12, U13, U22, U25, U47: each fixed test set was mutation-checked (handler swap, removed guard, removed attribute, removed reason). Residual: unobservable success-path guard in Home.svelte. | verified (one residual accepted) |
| Scan lifecycle and state | U05: single-scan guard, cancel, timeout, size limit, state kept across sections; Windows items pending. | needs-human (Windows) |
| Clamscan and freshclam integration | D7: 14 findings closed as wontfix by ADR 0004 (YARA-X replacement, proposed). | wontfix (engine replaced by decision) |
| Windows-only code running on all OSes | U04 (FND-0003 et al.) not fixed in this round: low severity accepted; the platform split remains. | accepted (low) |
| Accessibility of pale and dark UI states | U06 focus ring fixed; FND-0210 accepted (pill merge). | verified (one low accepted) |

## D. Unverified / deferred inventory

No findings are left `unverified` or `deferred`. Items that depend on Windows are listed in section E.


## E. Needs human (Windows checks and their exact steps)

- **FND-0001** (startup error box): build the release exe on Windows, force a startup failure (corrupt the Tauri config or occupy the port), run from Explorer and expect the OCHISTKA error box; run from cmd with `2> err.txt`, expect the stderr line and exit code 1; `cargo check --target x86_64-pc-windows-msvc`.
- **FND-0026** (scan lifecycle): run a scan of a large folder, press Остановить, confirm clamscan.exe is killed; a long file hits the time limit; close the app during a scan and check for an orphaned clamscan.exe; check that a second Проверить is refused while one runs.
- **FND-0107** (600px rail): set the window to 600px height at 100%, 125% and 150% scaling; the title bar and bottom controls must stay on screen and the rail must not clip; check one navigation landmark with a screen reader.

## F. Residual risk

- Out of scope by `scope.json`: see the scope file for the exclusion list.
- ClamAV: all related findings are closed because the engine is replaced (ADR 0004, proposed).
- Windows behaviour (timing on slow USB, real Disks, clamscan flags, message box) was not checked on real Windows; the audit ran on Linux.
- jsdom does not apply Svelte component CSS, so visual results (contrast with the pattern, label fit in the 124px rail) are reasoned, not rendered.
- Cargo MSRV and dependency minimum versions are not declared (FND-0155 accepted).

## G. Metadata

- Audit started: 2026-11-17T22:09:08.887139+00:00
- Audit completed: 2026-11-18T06:04:01Z
- Agent set: audit plugin agents (planner, code-auditor, triage-analyst, fix-implementer, test-engineer, re-verifier) and the audit skills (init, run, triage, fix, summary).
- Total findings: 213; verified: 30; wontfix: 180 (engine replacement 14, accepted low severity 165, accepted by owner 1); needs-human: 3; unresolved: 0.
