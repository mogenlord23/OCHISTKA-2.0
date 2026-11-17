# Статус функций по ОС

Статусы:

- `implemented` — код написан и покрыт тестами, но не проверен на этой ОС;
- `verified` — проверено на реальной ОС;
- `planned` — запланировано; в интерфейсе выключено с причиной.

| Функция                                              | Windows 10/11 x64 | macOS                   | Linux                   |
| ---------------------------------------------------- | ----------------- | ----------------------- | ----------------------- |
| Компьютер: загрузка CPU, RAM, список дисков          | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Вирусы: проверка через ClamAV                        | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Вирусы: эвристическая подсказка по файлу             | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Вирусы: карантин и удаление                          | `planned`         | `planned`               | `planned`               |
| VPN, Процессы, Программы, Диски, Оформление (каркас) | `planned`         | `planned`               | `planned`               |

## Примечания

- Код Windows собирается и проходит `clippy -D warnings` под целевую платформу `x86_64-pc-windows-msvc` (проверено с Linux, 2026-11-17). Это проверка компиляции.
- `cargo test --workspace` прошёл на Windows 10/11 на этапе скелета (до добавления ClamAV и эвристики). Тесты ClamAV и эвристики на Windows ещё не запускались. Сверка значений с Диспетчером задач не сделана, поэтому статус остаётся `implemented`.
- ClamAV на Linux проверен фейковым `clamscan` (shell-скрипт, код 1); настоящий `clamscan.exe` на Windows ещё не запускался.
- На macOS и Linux команды возвращают `kind: "unsupported"` или `"engineMissing"` с причиной.
