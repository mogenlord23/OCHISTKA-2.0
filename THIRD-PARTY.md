# Сторонние материалы

## simple-icons 16.38.0 (CC0-1.0)

Контуры логотипов NVIDIA, AMD и Intel в `apps/desktop/src/lib/driverLogos.ts` взяты из npm-пакета `simple-icons` 16.38.0 (https://github.com/simple-icons/simple-icons), лицензия CC0 1.0 Universal (передача в общественное достояние, указание авторства не требуется). Сами названия и знаки принадлежат их владельцам (NVIDIA Corporation, Advanced Micro Devices, Intel Corporation) и показаны только для обозначения производителя драйвера. Знака USB в simple-icons нет: для него нарисована буквенная метка. Значки «Звук и связь» и «Производители ПК» нарисованы самостоятельно.

## reqwest (MIT или Apache-2.0)

Блокирующий HTTP-клиент для загрузки установщиков драйверов, на Windows с TLS системы (`native-tls`, Schannel). Только `crates/platform`.

## url (MIT или Apache-2.0)

Разбор и проверка адресов загрузки.

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

## Обход блокировок (ADR 0018)

- ochistkadpi — переименованный SonicDPI v0.2.2 (MIT/Apache-2.0), авторы by-sonic, код не изменён (`by-sonic/sonicdpi`; LICENSE-MIT, LICENSE-APACHE и THIRD_PARTY_LICENSES.md лежат рядом с файлом). Архив `SonicDPI-windows-x64.zip`, SHA-256 `ec1e9f32315bdab1989d8e78bb3d3ad6975700fe83546139eb6f21e132991fc0`; `sonicdpi.exe` из архива (SHA-256 `3d6451021f390f8302b3aa1d790b86e626d7f925f7e1a2ba9c51ec91afa35351`) в сборке сохраняется как `ochistkadpi.exe`. Журнал пишет сам движок: `%LOCALAPPDATA%\SonicDPI\sonicdpi.log`. Не входит в git: скачивается на сборке.
- WinDivert 2.2.2 (basil00/WinDivert, LGPL v3 / GPL v2) идёт внутри архива SonicDPI отдельными файлами DLL и драйвера, без статической линковки.
- Zapret 2 (winws2) и пресеты NetZapret удалены из репозитория (ADR 0018); история — в git.

