# Аудит скруглений (круг 3, блок A, 2026-11-19)

Единые токены (`apps/desktop/src/lib/tokens.ts`, `RADIUS`; в CSS как `var(--radius-*)`, задаются в корне `App.svelte`):

| Токен | Значение | Где |
| --- | --- | --- |
| `--radius-xs` | 4px | флажок, полоски графиков, мелкие метки |
| `--radius-sm` | 8px | кнопки, поля, кнопки окна, пункты меню |
| `--radius-md` | 12px | плитки, вложенные блоки |
| `--radius-lg` | 14px | карточки, панели, корень окна |
| `--radius-pill` | 999px | таблетки, бегунки |

Круги: `50%` или `9999px`. Рисунки иконок (`rx` в SVG) — геометрия, не стиль, в аудит не входят.
Защита от возврата: `apps/desktop/src/radius-tokens.test.ts` (падает на любом своём числе в `<style>` компонентов).

## Заменено в этом блоке

| Файл | Было | Стало |
| --- | --- | --- |
| TitleBar.svelte (кнопки окна) | 10px | `--radius-sm` |
| App.svelte (стрелка дока) | 10px | `--radius-sm` |
| LineGraph.svelte | 8px | `--radius-sm` |
| ContextMenu.svelte | 8px | `--radius-sm` |
| AppIcon.svelte | 8px | `--radius-sm` |
| Check.svelte | 6px | `--radius-xs` |
| DisksPage.svelte | 12px | `--radius-md` |
| DisksPage.svelte | 4px, 3px, 3px | `--radius-xs` |
| VirusesPage.svelte | 999px | `--radius-pill` |
| VirusesPage.svelte | 4px | `--radius-xs` |
| Home.svelte | 4px | `--radius-xs` |

## Не тронуто (файлы других блоков; владельцы заменяют по этой таблице)

Эти файлы перечислены в `PENDING` теста. Когда файл исправлен, его убирают из списка в тесте.

| Файл (строка) | Сейчас | Нужный токен |
| --- | --- | --- |
| ProcessesPage.svelte (434, 537) | 4px, 6px | `--radius-xs` |
| ProcessesPage.svelte (499) | 16px | `--radius-lg` |
| ProcessesPage.svelte (577) | 12px | `--radius-md` |
| ProcessesPage.svelte (613) | 10px | `--radius-sm` |
| ProgramsPage.svelte (710) | 16px | `--radius-lg` |
| ProgramsPage.svelte (750, 837) | 6px, 4px | `--radius-xs` |
| ProgramsPage.svelte (792) | 12px | `--radius-md` |
| ProgramsPage.svelte (866) | 999px | `--radius-pill` |
| VpnPage.svelte (808, 1009) | 999px | `--radius-pill` |
| VpnPage.svelte (906) | 6px | `--radius-xs` |
| VpnPage.svelte (966) | 16px | `--radius-lg` |
| VpnConnectionsTab.svelte (192) | 999px | `--radius-pill` |
| AppearancePage.svelte (447, 529) | 14px | `--radius-lg` |
| AppearancePage.svelte (477) | 8px | `--radius-sm` |
| AppearancePage.svelte (544) | 12px | `--radius-md` |
| AppearancePage.svelte (638, 656) | 999px | `--radius-pill` |
| DriversPage.svelte | проверить при правке | токены |
