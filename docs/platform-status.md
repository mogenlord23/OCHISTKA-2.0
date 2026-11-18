# Статус функций по ОС

Статусы:

- `implemented` — код написан и покрыт тестами, но не проверен на этой ОС;
- `verified` — проверено на реальной ОС;
- `planned` — запланировано; в интерфейсе выключено с причиной.

| Функция                                                          | Windows 10/11 x64 | macOS                   | Linux                   |
| ---------------------------------------------------------------- | ----------------- | ----------------------- | ----------------------- |
| Компьютер: загрузка CPU, RAM, список дисков                      | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Компьютер: плитки CPU и RAM, графики за 90 с (клиент)            | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Компьютер: тип носителя SSD/HDD                                  | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Компьютер: название и версия ОС                                  | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Компьютер: GPU (загрузка, память)                                | `planned`         | `planned`               | `planned`               |
| Компьютер: материнская плата                                     | `planned`         | `planned`               | `planned`               |
| Компьютер: Настройки и Boost (заглушки с причиной)               | `planned`         | `planned`               | `planned`               |
| Компьютер: Драйверы (список Windows)                             | `planned`         | `planned`               | `planned`               |
| Вирусы: проверка через ClamAV                                    | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Вирусы: эвристическая подсказка по файлу                         | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Вирусы: карантин и удаление                                      | `planned`         | `planned`               | `planned`               |
| Диски: глубокая очистка (анализ, «Стоп», карантин)               | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Диски: очистка файлов, анализ пространства, приватность (чтение) | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Диски: карантин (манифест, восстановление, удаление по токену)   | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| VPN, Процессы, Программы, Оформление (каркас)                    | `planned`         | `planned`               | `planned`               |

## Примечания

- Код Windows собирается и проходит `clippy -D warnings` под целевую платформу `x86_64-pc-windows-msvc` (проверено с Linux, 2026-11-17). Это проверка компиляции.
- `cargo test --workspace` прошёл на Windows 10/11 на этапе скелета (до добавления ClamAV и эвристики). Тесты ClamAV и эвристики на Windows ещё не запускались. Сверка значений с Диспетчером задач не сделана, поэтому статус остаётся `implemented`.
- ClamAV на Linux проверен фейковым `clamscan` (shell-скрипт, код 1); настоящий `clamscan.exe` на Windows ещё не запускался.
- На macOS и Linux команды возвращают `kind: "unsupported"` или `"engineMissing"` с причиной.
