> **Отменён решением владельца, см. ADR 0019.**

# ADR 0018: обход блокировок — один движок SonicDPI вместо winws2

- **Дата:** 2026-11-21. **Статус:** решение владельца. Заменяет ADR 0014 и 0017 в части движка и пресета (winws2, объединённый пресет, основа YT FIX); остальное из них (кнопка, взаимное исключение с VPN, остановка по PID, службы WinDivert) остаётся.

## Решение

1. Кнопка «Обход блокировок» запускает `sonicdpi.exe run --profile youtube-discord` (SonicDPI v0.2.2). Один профиль, без выбора в интерфейсе.
2. Причина: SonicDPI несёт свой WinDivert 2.2.2, winws2 — свой. Два движка с разными копиями драйвера одновременно работать не могут, поэтому старый путь убран, а не оставлен «на всякий случай».
3. Выбор движка: аудит `docs/status/dpi-engine-audit.md` (лицензия MIT/Apache-2.0, три ОС и голос Discord по UDP заявлены в README). Архив `SonicDPI-windows-x64.zip` проверен по SHA-256 `ec1e9f32315bdab1989d8e78bb3d3ad6975700fe83546139eb6f21e132991fc0` (`docs/status/sonicdpi-verify.md`).
4. Пользовательские домены пока НЕ применяются: список в панели «Сайты» хранится, но движку не передаётся (в README SonicDPI нет подтверждённого способа добавить произвольный домен). Панель честно пишет: «Домены пока не применяются».

## Что удалено

- winws2: закреплённый выпуск, загрузка, проверка 135 файлов NetZapret и Lua, запись ресурсов (`crates/platform/src/desync/merged_assets.rs`, `preset_assets.rs`), разбор пресета (`crates/ochistka-core/src/desync/preset.rs`), проверка длины командной строки, секция пользовательских сайтов для winws2.
- Данные: `assets/desync/presets/` (включая `OCHISTKA Merged.txt`, `Universal V13 Lite.txt`, лицензии NetZapret), `assets/desync/runtime/`, `assets/desync/runtime-extra/`, Lua-скрипты Zapret 2, `.gitattributes` под них.
- Инструменты: `tools/bypass-merge/`, winws2-скрипты в `tools/desync-core/`.
- Workflow: все, проверявшие winws2 и пресеты (`validate-desync-*`, `build-zapret2-windows`, `publish-zapret2-prebuilt`, `import-desync-preset-resources`, `build-desync-diagnostic-package`).

Осталось: остановка процесса по PID с проверкой имени и пути (`process_guard.rs`), служба WinDivert (`driver_service.rs`), задание Windows «завершить при закрытии», панель «Сайты» (хранение), взаимное исключение с VPN (`NetCoordinator`), проверки сервисов (`probe`). Файл `assets/desync/routes/OCHISTKA Routes.json` не тронут (на него ничего не ссылается).

## Открытое

- `sonicdpi.exe`, `WinDivert.dll`, `WinDivert64.sys` пока не входят в установщик: на собранной версии кнопка сообщает, что не хватает файлов. Включение файлов и проверка их SHA-256 — следующий этап.
- Не проверено: запуск на Windows, права администратора, конфликт с уже установленной службой WinDivert, работа у провайдера, YouTube и Discord (текст, видео, голос).
- Пользовательские домены: нужен способ передать список (TOML-профиль, `targets`) — не подтверждён.
- Linux и macOS: адаптеры не написаны; SonicDPI заявляет их, но не проверено.
- История ADR 0014/0017 и отчётов `docs/status/bypass-*` не переписывалась: это архив прежнего пути.
