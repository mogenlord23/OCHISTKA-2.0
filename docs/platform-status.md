# Статус функций по ОС

Статусы:

- `implemented` — код написан и покрыт тестами, но не проверен на этой ОС;
- `verified` — проверено на реальной ОС;
- `planned` — запланировано; в интерфейсе выключено с причиной.

| Функция                                | Windows 10/11 x64 | macOS                   | Linux                   |
| -------------------------------------- | ----------------- | ----------------------- | ----------------------- |
| Главная: загрузка CPU                  | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Главная: использование RAM             | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Главная: список дисков                 | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Разделы 2–10 (заглушки «в разработке») | `planned`         | `planned`               | `planned`               |

## Примечания

- Адаптер Windows проверен только тестом `windows_adapter_reads_live_system_overview`, который запускался на Linux: sysinfo кроссплатформенна, поэтому тест проверяет отображение данных, но не поведение Windows. Статус Windows остаётся `implemented`, пока не будет проверен на Windows 10/11.
- Код Windows собирается и проходит `clippy -D warnings` под целевую платформу `x86_64-pc-windows-msvc` (проверено с Linux через `cargo check` и `cargo clippy --target`, 2026-11-17). Это проверка компиляции, а не работы; статус остаётся `implemented`.
- На macOS и Linux команда возвращает `kind: "unsupported"` с причиной; раздел «Главная» показывает эту причину.
