# Удаление Telegram, Steam и Riot / Valorant из «Обхода блокировок»

Источник данных: RixyPow/netzapret@11f073c2fd05505a46e29622a52b6715cfdeb2de. Решение владельца; создано `tools/bypass-merge/merge_presets.py`, вручную не правится.

Правило: секция убирается, только если весь её охват (списки адресов и домены) — это удаляемый сервис. Если секция нужна ещё и другим сервисам, она остаётся и записывается ниже.

## Telegram

Удалено секций пресетов: 14.

Из основы (Universal V13 Lite, в самом исходном файле остаётся без изменений): №27 «Telegram».
Из других пресетов: 13 секций (подробно в `bypass-merge-coverage.json`, статус `removed`).
Маршруты и списки: `ipset-telegram.txt`, `telegram.txt`.

## Steam

Удалено секций пресетов: 11.

Из основы (Universal V13 Lite, в самом исходном файле остаётся без изменений): №50 «Steam».
Из других пресетов: 10 секций (подробно в `bypass-merge-coverage.json`, статус `removed`).
Образцы, объявленные, но не используемые ни одной оставшейся секцией: 20 (например `--blob=tls_steam_chat:@bin/nz-tls_clienthello_steam_chat_com.bin`).
Маршруты и списки: `steam-web.txt`, `steam.txt`.

## Riot / Valorant

Удалено секций пресетов: 10.

Из основы (Universal V13 Lite, в самом исходном файле остаётся без изменений): №10 «Riot / Valorant TCP».
Из других пресетов: 9 секций (подробно в `bypass-merge-coverage.json`, статус `removed`).
Маршруты и списки: `riot-network.txt`, `riot-valorant.txt`.

## Общее использование (проверка перед удалением)

Секций, которые одновременно нужны другим сервисам, не найдено: удалены только секции, целиком относящиеся к удаляемым сервисам.

## Что НЕ менялось

- 135 закреплённых файлов NetZapret (в том числе `lists/ipset-telegram.txt`, `lists/steam.txt`, `lists/riot-valorant.txt`, `lists/russia-blacklist.txt`) и их SHA-256: файлы остаются встроенными, но объединённый пресет на них больше не ссылается.
- `assets/desync/presets/Universal V13 Lite.txt` (SHA-256 `e679901475d9…`).
- Маршруты Claude и других сервисов «только через VPN» (`vpn-only`) оставлены как есть; отдельного обхода Claude нет.
