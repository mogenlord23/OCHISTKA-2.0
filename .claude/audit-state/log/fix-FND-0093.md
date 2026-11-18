# Fix Record: FND-0093, FND-0094, FND-0097

**Date**: 2026-11-18T05:52:58Z
**Fix unit**: U47 (single-file)
**Findings**: FND-0093, FND-0094, FND-0097
**File**: apps/desktop/src/components/VirusesPage.svelte

## Edits
- FND-0093: Run gets monotonic `id` from module-level counter `nextRunId`; journal each keyed on `run.id` (was startedAt + target).
- FND-0094: assessFile catch no longer swallows; counts failures and keeps first message (via toCommandError), shown in Находки as "Не удалось оценить файлов: N. Причина: ...". Scan continues for other files; reset on each start.
- FND-0097: derived `startBlockReason` ("Сначала проверьте движок" / "Укажите папку для проверки") shown as visible span.reason (var(--muted)) next to the start button.

Not changed: scan lifecycle, engine behaviour, existing strings, Остановить/Удалить title (visible reason for those not requested), test file.
Risk: low; the "engine unavailable" reason also shows before engine status loads.

## Sanity
- npm run check -w apps/desktop: PASS (0 errors, 0 warnings)
- npm test -w apps/desktop: PASS (58/58)
