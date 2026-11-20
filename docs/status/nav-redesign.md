# Редизайн навигации: что изменено и как откатить (2026-11-20)

Предыдущее состояние: коммит `44c88f2dfe99c35c4415b4d759ba6dcb3de26c54` (ветка `claude/epic-knuth-0zhg19`).
Работа сделана в ветке `work/nav`, ничего не отправлялось на GitHub.

## Что изменилось (файлы)

- `apps/desktop/src/lib/dragSelect.ts` (новый) и `dragSelect.test.ts` (новый): переключение зажатой мышью для рядов пунктов.
- `apps/desktop/src/lib/sound.ts` и `sound.test.ts`: защита от дубля 60 мс стала общей (`playActionSound`).
- `apps/desktop/src/components/Subtabs.svelte` (переписан) и `Subtabs.test.ts` (новый): компактная панель вкладок с ползунком, `tablist`/`tab`, стрелки.
- `apps/desktop/src/components/Navigation.svelte` и `Navigation.test.ts`: ползунок за плитками, переключение зажатой мышью; вид плиток прежний.
- `apps/desktop/src/components/TitleBar.svelte` и `TitleBar.test.ts`: удалена строка «Create: Slava Lavs», слово OCHISTKA по оси кнопок.
- `apps/desktop/src/App.svelte` и `App.test.ts`: заголовок раздела скрыт визуально (остался для чтения с экрана), подпись раздела убрана, `body.dragging`; тесты ищут вкладки как `tab`.
- `docs/status/nav-redesign/` (новая папка): `geometry.mjs` (проверка геометрии в Chromium) и снимки VPN, Компьютер, Оформление на 900x712, 1179x803, 1920x1080.
- `docs/status/decisions.md`, `docs/platform-status.md`, этот файл: записи.

Не менялись: `VpnPage.svelte` (свой заголовок и свои вкладки), `sections.ts`, Rust-код.

## Коммиты

- `5652652` feat(ui): shared dragSelect action and shared action-sound guard
- `a88ac9b` fix(ui): do not move focus programmatically while dragging (no stray focus ring)
- `39b130f` feat(ui): compact sliding sub-tab bar with hold-and-drag switching; hide page title visually
- `beec3cc` feat(ui): sliding indicator and hold-and-drag switching on the right rail
- `4f8d5e1` feat(ui): remove author line, centre OCHISTKA on the window buttons axis

- `39d7b08` docs(nav): документы, проверка геометрии, снимки

## Как откатить

1. Целиком: `git revert --no-commit 44c88f2..HEAD`, затем `git commit` (откатывает все коммиты этой работы одним).
2. Отдельный файл: `git checkout 44c88f2dfe99c35c4415b4d759ba6dcb3de26c54 -- <путь к файлу>`. Для полного возврата старого вида нужны все файлы из списка выше (кроме новых; их достаточно удалить).
3. Только отключить переключение зажатой мышью: убрать `use:dragSelect` в `Subtabs.svelte` и `Navigation.svelte` и вернуть `onclick` на кнопки (старые версии файлов лежат в коммите `44c88f2`).

## Как проверить

В `apps/desktop`: `npx vite --port 1591 --strictPort`, затем `node ../../docs/status/nav-redesign/geometry.mjs <папка снимков>`.
Проверено: vitest, svelte-check, Prettier, геометрия в Chromium. Не проверено: окно Tauri на Windows (WebView2, DPI, перетаскивание окна за шапку, реальный звук).
