# Fix Record: FND-0037 through FND-0103 (U08)

**Fix unit**: cascade (2 files + test)
**Findings**: FND-0037, FND-0039, FND-0102, FND-0103

## Original Findings
- FND-0037: panic under the System mutex poisons it; later overviews fail with a misleading lock message.
- FND-0039: disk enumeration ran under the System mutex; a hung removable drive blocked CPU/memory and serialized calls.
- FND-0102: a `getSystemOverview` call that never settles kept `pending` true forever.
- FND-0103: teardown does not cancel the in-flight job; remounts queue on the mutex.

## Change Plan
- FND-0037: `lock().unwrap_or_else(PoisonError::into_inner)`; comment explains that the state is a refresh-only snapshot overwritten by every call. The clear error remains only for the `None` (uninitialized) state.
- FND-0039: CPU and memory are read in a block scope that releases the guard; `Disks::new_with_refreshed_list()` runs after the guard is dropped. Return type unchanged.
- FND-0102: `REQUEST_TIMEOUT_MS = 10_000` in Home.svelte; `Promise.race` against a timeout rejecting with a `CommandError` (`kind: 'probe'`, Russian message), so the existing error heading applies. Timer cleared and `pending` reset in `finally`. `POLL_INTERVAL_MS` untouched.
- FND-0103: no backend cancel added. Per-instance `pending` guard already prevents stacking within one component. ACCEPTED RESIDUAL: on remount the old backend call keeps running and new calls wait on the mutex; the narrowed lock (disk enumeration outside it) bounds that wait to CPU/memory refresh (plus first-time init sleep).
- Not changing: backend API, commands.rs, api.ts, core crate.
- Risk: a timed-out request still occupies the backend until it returns; the UI shows the error, then the next poll starts a new call.

## Edits Applied
- `crates/platform/src/windows/mod.rs`: overview() restructured as above; new test `overview_recovers_from_poisoned_mutex` (poisons the mutex in a thread via panic, asserts the next `overview()` is Ok).
- `apps/desktop/src/components/Home.svelte`: timeout constant, race, clearTimeout.
- `apps/desktop/src/components/Home.test.ts`: test "FND-0102: a request that never settles ..." (fake timers: no alert at 9.999 s, timeout alert at 10 s, a new request starts, success then clears the alert).

## Sanity Checks
- cargo test --workspace: PASS (platform 14 tests incl. new one)
- cargo clippy --workspace --all-targets -- -D warnings: PASS
- cargo fmt --all --check: PASS
- npm test -w apps/desktop: PASS (79/79)
- npm run check -w apps/desktop: PASS (0 errors, 0 warnings)
- prettier on changed TS/Svelte: applied

## Verification limits
UNVERIFIED on Windows: real timing with a slow or hung USB/removable drive, and the actual `Disks` behaviour there. Tests ran on Linux only. Needs Windows verification.

## Next
Handing off to @test-engineer and @re-verifier.
