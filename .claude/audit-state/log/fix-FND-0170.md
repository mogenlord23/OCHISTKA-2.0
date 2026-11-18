
## Tests (U12: FND-0170, FND-0171, FND-0175)

**Framework**: vitest 5.1 (jsdom, @testing-library/svelte)
**Test file**: apps/desktop/src/components/Home.test.ts (modified; Home.svelte untouched)
**test_status**: `passing`

### Tests
- FND-0170: `FND-0170: polls again on every interval while responses resolve` (counts 1,2,3,4 via advanceTimersByTimeAsync)
- FND-0170: `FND-0170: does not poll before the interval has elapsed` (INTERVAL-1 -> 1 call, +1 -> 2)
- FND-0170: `FND-0170: pending clears after a failed request and polling recovers`
- FND-0170: `FND-0170: resumes polling after a slow request settles`
- FND-0171: `FND-0171: an error that arrives after unmount is not processed` (toCommandError is now a vi.fn wrapper over the real one; must not be called after unmount)
- FND-0171: `FND-0171: an error from a mounted component is processed (guard control)`
- FND-0175: flush() = tick + `vi.advanceTimersByTimeAsync(0)` + tick (fake timers via beforeEach); no microtask count remains; console.error spy created once, restored/reset in afterEach.
- Existing tests kept, assertions unchanged (sync timer advances became async equivalents).

### Mutation reasoning (not executed; Home.svelte not editable)
- Delete `pending = false`: second tick never calls load -> count stays 1; fails "polls again", "does not poll before", "recovers", "resumes".
- Delete `setInterval`: same tests fail.
- Interval x100: "polls again" fails (count 1 after one interval).
- Shorter interval (e.g. /2): "does not poll before the interval has elapsed" fails.
- Remove `if (active)`: catch branch calls toCommandError after unmount -> "error that arrives after unmount" fails. The success-path guard (`overview = next` after unmount) writes only to detached state and is not observable through DOM or spies; it is covered only by the existing DOM/console assertion, which cannot fail (residual limitation).

### Commands / results
- `npx prettier --write apps/desktop/src/components/Home.test.ts`
- `npm test -w apps/desktop`: 11 files, 64 tests PASS
- `npm run check -w apps/desktop`: 0 errors, 0 warnings
