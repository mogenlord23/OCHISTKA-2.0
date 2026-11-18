# Статус функций по ОС

Статусы:

- `implemented` — код написан и покрыт тестами, но не проверен на этой ОС;
- `verified` — проверено на реальной ОС;
- `planned` — запланировано; в интерфейсе выключено с причиной.

| Функция                                                                                          | Windows 10/11 x64 | macOS                   | Linux                   |
| ------------------------------------------------------------------------------------------------ | ----------------- | ----------------------- | ----------------------- |
| Компьютер: загрузка CPU, RAM, список дисков                                                      | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Компьютер: плитки CPU и RAM, графики за 90 с (клиент)                                            | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Компьютер: тип носителя SSD/HDD                                                                  | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Компьютер: название и версия ОС                                                                  | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Компьютер: GPU (загрузка, память)                                                                | `planned`         | `planned`               | `planned`               |
| Компьютер: материнская плата                                                                     | `planned`         | `planned`               | `planned`               |
| Компьютер: Настройки и Boost (заглушки с причиной)                                               | `planned`         | `planned`               | `planned`               |
| Компьютер: Драйверы (список Windows)                                                             | `planned`         | `planned`               | `planned`               |
| Вирусы: проверка через libclamav (движок не собран, заглушка с причиной)                         | `planned`         | `planned` (Unsupported) | `planned` (Unsupported) |
| Вирусы: инвентаризация, эвристика, ИИ-заглушка и итоговая оценка                                 | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Вирусы: находки в карантин, восстановление, окончательное удаление по токену                     | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Диски: глубокая очистка (анализ, «Стоп», карантин)                                               | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Диски: очистка файлов, анализ пространства, приватность (чтение)                                 | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Диски: карантин (манифест, восстановление, удаление по токену)                                   | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Программы: установленные из реестра Uninstall (HKLM 64/32, HKCU), чтение                         | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Программы: поиск по названию и издателю (клиент)                                                 | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Программы: размер папки установки (бюджет записей, отмена)                                       | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Программы: автозагрузка (Run HKCU/HKLM, папки «Автозагрузка»), чтение                            | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Программы: реестр Microsoft Store (пакеты)                                                       | `planned`         | `planned`               | `planned`               |
| Программы: удаление (заглушка с причиной)                                                        | `planned`         | `planned`               | `planned`               |
| Процессы: список (имя, PID, ЦП, память), обновление раз в 3 с, поиск по имени и PID              | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Процессы: завершение задачи (подтверждение, отказ для PID 0, 4 и своего, сверка времени запуска) | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Процессы: расположение файла (путь к exe, только текст)                                          | `implemented`     | `planned` (Unsupported) | `planned` (Unsupported) |
| Процессы: службы (заглушка «Список служб пока не подключён»)                                     | `planned`         | `planned`               | `planned`               |
| VPN, Оформление (каркас)                                                                         | `planned`         | `planned`               | `planned`               |

## Примечания

- Код Windows собирается и проходит `clippy -D warnings` под целевую платформу `x86_64-pc-windows-msvc` (проверено с Linux, 2026-11-17). Это проверка компиляции.
- `cargo test --workspace` прошёл на Windows 10/11 на этапе скелета (до добавления ClamAV и эвристики). Тесты ClamAV и эвристики на Windows ещё не запускались. Сверка значений с Диспетчером задач не сделана, поэтому статус остаётся `implemented`.
- Вирусы (2026-11-18): libclamav не собран (см. `docs/status/clamav-build.md`), движок — заглушка «движок не собран». Проверка по сигнатурам идёт через `VirusScanPort` и проверена фейковым движком в тестах на Linux; на Windows ещё не запускалась. Статус по Windows — `planned` до сборки и проверки.
- Оценка файла (инвентаризация, эвристика, заглушка ИИ) и карантин находок проверены юнит-тестами на Linux; на Windows тесты не запускались (карантин в тестах с реальным диском — только под `cfg(windows)`).
- На macOS и Linux команды возвращают `kind: "unsupported"` или `"engineMissing"` с причиной.
- Процессы на Windows: sysinfo, проценты CPU делятся на число ядер. Завершение и путь к exe не проверены на реальной Windows (с Linux проверена только компиляция и тесты чтения своего процесса), поэтому статус `implemented`. Службы не читаются: нужен Service Control Manager и FFI.
