# Сторонние материалы

## Natural Earth 5.1.2 (общественное достояние)

Точки суши и центры меток стран для локального глобуса VPN получены из
[`ne_50m_admin_0_countries.geojson`](https://github.com/nvkelso/natural-earth-vector/blob/f1890d9f152c896d250a77557a5751a93d494776/geojson/ne_50m_admin_0_countries.geojson).
SHA-256 исходника — `3e458fc036ad0a66411f2c1e6cac49c5d7bfb81cb1123bc513b22511a2b7fdeb`.
Условия — [Natural Earth Terms of Use](https://www.naturalearthdata.com/about/terms-of-use/).
Преобразование в `apps/desktop/src/assets/vpn-globe.json` выполняет
`apps/desktop/scripts/generate-vpn-globe.py` с исходным GeoJSON в качестве аргумента.

## simple-icons 16.38.0 (CC0-1.0)

Контуры логотипов NVIDIA, AMD и Intel в `apps/desktop/src/lib/driverLogos.ts` взяты из npm-пакета `simple-icons` 16.38.0 (https://github.com/simple-icons/simple-icons), лицензия CC0 1.0 Universal (передача в общественное достояние, указание авторства не требуется). Сами названия и знаки принадлежат их владельцам (NVIDIA Corporation, Advanced Micro Devices, Intel Corporation) и показаны только для обозначения производителя драйвера. Знака USB в simple-icons нет: для него нарисована буквенная метка. Значки «Звук и связь» и «Производители ПК» нарисованы самостоятельно.

## reqwest (MIT или Apache-2.0)

Блокирующий HTTP-клиент для загрузки установщиков драйверов, на Windows с TLS системы (`native-tls`, Schannel). Только `crates/platform`.

## url (MIT или Apache-2.0)

Разбор и проверка адресов загрузки.

## ClamAV 1.4.6 для Windows (GPL-2.0)

- Источник: официальный выпуск Cisco Talos, GitHub Releases `Cisco-Talos/clamav`, тег `clamav-1.4.6`,
  файл `clamav-1.4.6.win.x64.zip`
  (https://github.com/Cisco-Talos/clamav/releases/download/clamav-1.4.6/clamav-1.4.6.win.x64.zip).
- Размер архива: 191 947 200 байт. SHA-256 архива (совпадает с полем `digest` в GitHub API):
  `57b6fd1d60cd87bafe800f97407ecdef0576d36b3900b8b7abcfbbabe88295fd`.
- Лицензия: GNU General Public License v2.0 (`COPYING.txt` из архива; в поставку кладётся
  `resources/licenses/ClamAV-GPL-2.0.txt`).
- Поставляются **только** файлы движка из архива: `libclamav.dll` и его DLL-зависимости
  (OpenSSL, libxml2, pcre2, curl, json-c, bz2, ssh2, nghttp2, unrar, mspack, runtime MSVC),
  а также import-библиотека и заголовки для сборки. Программы `clamscan.exe`, `freshclam.exe`,
  `clamd.exe`, `clamdscan.exe`, `sigtool.exe`, `clambc.exe`, `clamconf.exe`, `clamsubmit.exe`,
  `clamdtop.exe` **не поставляются и не запускаются**.
- Библиотека линкуется с OCHISTKA как разделяемая (`clamav.lib` + `libclamav.dll`), поэтому
  действуют требования GPL-2.0 к распространению: текст лицензии прилагается, исходный код
  ClamAV доступен по ссылке выше. Лицензия самой OCHISTKA: TBD (см. ADR 0004).
- Скачивание и распаковка: `tools/clamav-win/fetch.ps1` (проверка SHA-256 и размера, повторный
  запуск безопасен). В git бинарники не попадают, папка `tools/clamav-win` игнорируется.
- Базы сигнатур в поставку не входят: их кладёт владелец в `%LOCALAPPDATA%\OCHISTKA\clamdb`.

## EMBER2018 (MIT) и tract-onnx 0.23.10 (MIT или Apache-2.0)

- Локальная модель OCHISTKA обучена на числовых признаках из архива `ember_dataset_2018_2.tar.bz2` набора [Elastic EMBER2018](https://github.com/elastic/ember/tree/d97a0b523de02f3fe5ea6089d080abacab6ee931). SHA-256 архива и модели указаны в `crates/platform/resources/pe-header-v1.json`. Исходники EMBER и его предобученные веса не используются. Лицензия набора находится в `apps/desktop/src-tauri/resources/licenses/EMBER-MIT.txt`.
- Выполнение модели обеспечивает [tract-onnx](https://crates.io/crates/tract-onnx/0.23.10). Программа использует встроенный ONNX-файл, а инструменты обучения и набор данных в приложение не входят.

## Прочие компоненты

## sing-box (VPN-ядро)

- Версия 1.14.4, исходный код: https://github.com/SagerNet/sing-box/tree/v1.14.4
- Лицензия: GPL-3.0-or-later с дополнительным условием о названии (полный текст лежит рядом с `sing-box.exe` в файле `LICENSE`; SHA-256 `bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c`).
- Официальный архив `sing-box-1.14.4-windows-amd64.zip`, SHA-256 `99185d75d314a07dbba4a130c8ad986cbeb629a07927975636b341ca61f991df`. Внутри `sing-box.exe` (`8f30682e…93bbe7c8b9`) и `libcronet.dll` (сетевой стек Chromium, лицензия BSD, `3217c626…ba54a2928`).
- Программа не изменяется и не линкуется с OCHISTKA: приложение запускает её как отдельный процесс. Скачивается скриптом `tools/vpn-core/fetch.ps1` с проверкой хэшей и упаковывается как ресурс установщика.
- Последствия GPL для распространения установщика (приложить текст лицензии и ссылку на исходный код выпуска) надо проверить перед публичным выпуском. Лицензия самой OCHISTKA: TBD.

## React Bits: Variable Proximity и Decrypted Text (MIT + Commons Clause)

`apps/desktop/src/components/VariableProximity.svelte` и `DecryptedText.svelte` — порты на Svelte 5 оригинальных компонентов React Bits (https://reactbits.dev, автор David Haz, репозиторий DavidHDev/react-bits, файлы `VariableProximity.jsx` и `DecryptedText.jsx`). Лицензия «MIT + Commons Clause»: использование разрешено как части приложения; нельзя продавать сам компонент. Уведомление об авторских правах сохранено в заголовках файлов. Не перенесены эффект `flip` и `hoverRadius` у Decrypted Text, перенос слова на несколько строк у Variable Proximity.

## Roboto Flex (SIL OFL 1.1)

`apps/desktop/src/assets/fonts/RobotoFlex-OCHISTKA.woff2` — переменный шрифт Roboto Flex (Google Fonts, https://github.com/googlefonts/roboto-flex), урезанный Google Fonts до букв O, C, H, I, S, T, K, A (оси wght 100–1000 и opsz). Нужен для эффекта Variable Proximity на слове OCHISTKA. Лицензия SIL Open Font License 1.1: встраивание разрешено.

## Uiverse.io — «smart-emu-83» (vinodjangid07)

Принцип движения кнопок «Программы → Установленные» (`ExpandButton.svelte`) и контур корзины взяты из https://uiverse.io/vinodjangid07/smart-emu-83 (исходник: github.com/uiverse-io/galaxy, `Buttons/vinodjangid07_smart-emu-83.html`). Лицензия MIT (© vinodjangid07, Uiverse.io): разрешено использовать, изменять и распространять при сохранении этого уведомления; программа поставляется «как есть», без гарантий.

## Флаги стран — flag-icons (lipis)

Папка `apps/desktop/src/assets/flags`: SVG-флаги 4×3 из проекта flag-icons (https://github.com/lipis/flag-icons), лицензия MIT, © 2013 Panayiotis Lipiridis. Файлы rs, mx, es, hr, md заменены упрощёнными трёхполосными версиями (без гербов) ради размера.
