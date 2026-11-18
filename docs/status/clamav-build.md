# Сборка libclamav из исходников — статус (2026-11-18)

- Попытка: скачать исходники ClamAV с GitHub (`Cisco-Talos/clamav`, архив `main`) из среды разработки.
- Результат: **не собрано**. Запрос к GitHub вернул HTTP 403 через прокси среды; сайт clamav.net тоже 403. Исходники недоступны, поэтому CMake-сборка и MSVC-сборка не выполнялись.
- Что сделано вместо этого: порт `VirusScanPort` и FFI-граница описаны в коде; движок на этом этапе — заглушка, которая честно отвечает «движок ClamAV не собран». Приложение не падает, раздел «Вирусы» показывает причину.
- Что нужно для продолжения: доступ к GitHub из среды сборки (или локальная копия исходников в `tools/clamav-build/`), затем Windows-сборка по `tools/clamav-build/README.md`.
- Статус по ОС: Windows — `planned` (сборка не выполнена); остальные ОС — не поддерживаются.

## Обновление: источник найден (2026-11-18)

- Исходники ClamAV 1.4.6 доступны через зеркало Debian (`deb.debian.org`), не через GitHub. Скачивание и проверка SHA-256 — `tools/clamav-build/fetch.sh`.
- Конфигурация CMake на Linux остановилась на обязательных зависимостях `libxml2` и `pcre2`. Установка системных пакетов в этой среде заблокирована политикой; сборка не выполнена.
- Чтобы закрыть пункт: установить `libxml2-dev` и `libpcre2-dev` (Linux) или подготовить vcpkg (Windows), затем по `tools/clamav-build/README.md`.

## Итог сборки на Linux (2026-11-18)

- libclamav 1.4.6 собран статически (`clamav_static`, `libclamav_static.a`) на Linux. Для этого по разрешению владельца установлены системные пакеты: libxml2, pcre2, json-c, libcurl, libmspack.
- Из исходников Debian удалены компоненты unrar и mspack (лицензии); конфигурация: `ENABLE_UNRAR=OFF`, `ENABLE_EXTERNAL_MSPACK=ON`.
- Экспорт: `cl_init`, `cl_engine_new`, `cl_load`, `cl_engine_compile`, `cl_scanfile`, `cl_engine_free`, `cl_retver`, `cl_strerror` найдены в библиотеке. Сигнатуры в `crates/platform/src/windows/clamav_ffi.rs` совпадают с `libclamav/clamav.h` (сверено по исходникам).
- Не проверено: сборка под Windows (MSVC), линковка Rust-модуля с библиотекой, загрузка сигнатур (нужны базы: freshclam), сканирование EICAR с реальной базой.
- Фича `libclamav` в `crates/platform` остаётся выключенной по умолчанию.

## Проверка библиотеки на Linux (2026-11-18)

- Небольшая программа на C, связанная с `libclamav_static.a` и `libclamav_rust.a`, выполняет: `cl_init` (0), `cl_retver` (1.4.6), `cl_engine_new`, `cl_load` из пустой папки (код 8 — баз нет, ожидаемо), `cl_engine_compile` (0), `cl_scanfile` (0, без сигнатур).
- Вывод: API вызывается корректно, библиотека работает. Поиск EICAR требует баз; базы не загружались.
- Линковка: нужны `libclamav_rust.a` и системные пакеты (см. выше).
