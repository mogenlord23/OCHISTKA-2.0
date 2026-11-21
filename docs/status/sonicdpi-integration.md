# SonicDPI v0.2.2 как второй движок: что известно

Источник: README SonicDPI на теге `v0.2.2`. Сам exe не запускался.

## Команды из README (дословно)

- Windows, запуск из админ-PowerShell: `.\sonicdpi.exe run --profile youtube-discord -vv`
- Служба (автозапуск при логине): `.\sonicdpi.exe install --profile youtube-discord`
- Список профилей: `sonicdpi profiles`
- Дамп профиля для правки: `sonicdpi show youtube-discord -o my.toml`
- Свой профиль: `sonicdpi run --profile my.toml -v`
- Отладка: `sonicdpi run --profile youtube-discord -vv`; журнал Windows: `%LOCALAPPDATA%\SonicDPI\sonicdpi.log`
- Трей: «запустить `sonicdpi-tray.exe` — UAC → значок в трее → правый клик»; «UAC обрабатывается `sonicdpi-tray.exe`»; Windows «требует админ».

Профили (таблица README): `youtube-discord` («По умолчанию»), `youtube-discord-aggressive`, `youtube-discord-multidisorder`, `youtube-discord-seqovl` (алиас по умолчанию), `youtube-discord-hostfakesplit`.

Голос Discord: «перехват UDP к `*.discord.media` на портах 19294–19344 и 50000–50100» (README, раздел о возможностях); на macOS в v0.2 голос «не поддерживается».

## Что README НЕ говорит

- Команды остановки процесса нет (кроме трея и службы); в OCHISTKA остановка — по PID, как у winws2.
- Параметров для пользовательских доменов в командной строке нет: схема TOML-профиля содержит `targets` (имена групп вроде `YouTube`, `DiscordGateway`, `DiscordCloudflare`) и `decoy_host`/`fake_host`. Можно ли добавить произвольный домен в TOML — **не подтверждено**.
- Параметра «без трея» нет: `sonicdpi.exe run` — консольный процесс; нужно ли ему окно или ввод — не подтверждено.

## Что добавлено (код, не подключён к кнопке)

`crates/platform/src/sonicdpi.rs`: `SonicDpiEngine`.
- Запуск: абсолютный путь `<папка>/sonicdpi.exe`, без PATH и оболочки, аргументы `run --profile <профиль>` (без `-vv`), рабочая папка — папка движка (рядом `WinDivert.dll`/`WinDivert64.sys`), без окна, вывод в журнал, привязка к заданию «завершить при закрытии».
- Профиль: только пять встроенных имён или абсолютный `.toml` внутри папки движка; всё прочее отклоняется.
- Остановка: по PID с проверкой имени `sonicdpi.exe` и пути файла (`process_guard::terminate_named`); чужой процесс и одноимённый из другой папки не трогаются.
- Константы: тег `v0.2.2`, архив и его SHA-256 (проверен на этапе 3).
- Не изменены: кнопка, winws2-путь, пресеты, 135 файлов NetZapret.

Тесты (`cargo test -p ochistka-platform --lib sonicdpi`, 5 штук, прошли на Linux с фейковым процессом): аргументы по умолчанию; чужие профили отклоняются; запуск без файла и с относительным путём отклоняется; фейковый `sonicdpi.exe` получает `run --profile youtube-discord`, и остановка проходит; чужой процесс не трогается.

## Не проверено

- Запуск настоящего `sonicdpi.exe` на Windows (в том числе консольное поведение, права администратора, UAC).
- Драйвер WinDivert 2.2.2 из архива: загрузка, совместимость с уже установленным WinDivert winws2 (конфликт двух драйверов не исследован).
- Сеть: работа у провайдера, YouTube, Discord текст, видео и голос (UDP).
- Скачивание, проверка SHA-256 отдельных файлов внутри архива и их размещение в папке ресурсов (адаптер пока не проверяет exe по хешу).
- Совместное существование с VPN и взаимное исключение (`NetCoordinator`) — не подключено.
- Свои домены в TOML-профиле.
