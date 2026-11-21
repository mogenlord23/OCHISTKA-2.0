# Инвентаризация пресетов десинка (RixyPow/netzapret)

Источник: RixyPow/netzapret@11f073c2fd05505a46e29622a52b6715cfdeb2de. Лицензия данных: MIT (NetZapret © 2026 RixyPow; Zapret GUI © censorliber/loop-uh). Код не копируется, разобраны только файлы настроек. Файл создан инструментом `tools/bypass-merge/merge_presets.py`, вручную не правится.

## Сводка

| Файл | Общих аргументов | Секций |
|---|---:|---:|
| ALL TCP & UDP multisplit_stun.txt | 79 | 7 |
| Default + YouTube no QUIC + wssize.txt | 81 | 28 |
| Default v1 (game filter).txt | 80 | 69 |
| Default v2 (game filter).txt | 81 | 48 |
| Default v3 (game filter).txt | 81 | 34 |
| Default v4 (game filter).txt | 81 | 43 |
| Default v5 (game filter).txt | 81 | 38 |
| Universal V10.txt | 80 | 69 |
| Universal V11 Lite.txt | 80 | 71 |
| Universal V11.txt | 80 | 71 |
| Universal V12 Lite.txt | 87 | 72 |
| Universal V12.txt | 87 | 72 |
| Universal V13 Lite.txt | 87 | 73 |
| Universal V13 YT FIX.txt | 90 | 75 |
| Universal V13.txt | 87 | 73 |
| Universal V8.txt | 80 | 61 |
| Universal V9.txt | 80 | 67 |
| shadow probes (circular).txt | 83 | 4 |
| white sni (circular).txt | 84 | 4 |

## ALL TCP & UDP multisplit_stun.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=1`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=80,443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=Голосовые звонки/чаты`
- `--skip`
- `--filter-l7=stun,discord`
- `--lua-desync=fake:blob=quic_google:ip_autottl=-2,3-20:ip6_autottl=-2,3-20:payload=all:repeats=10`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Discord UDP |  | 443-65535 |  | discord |  | send, fake |
| 2 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | multidisorder |
| 3 | discord (images) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-images |  | multidisorder |
| 4 | updates.discord.com | 443 |  |  | discord-updates |  | multidisorder |
| 5 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 6 | Все сайты (айпи) | 80,443-65535 |  |  | (без списка) |  | send, syndata, multisplit |
| 7 | Все сайты UDP (исключение) |  | 443-65535 |  | (без списка) |  | fake |

## Default + YouTube no QUIC + wssize.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=1`
- `--wf-tcp-out=80,443,1080,2053,2083,2087,2096,8443,8080`
- `--wf-udp-out=80,443`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_sber_v2:@bin/tls_clienthello_sberbank_ru_v2.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Signal | 443 |  |  | signal |  | hostfakesplit |
| 2 | Matrix | 443 |  |  | matrix |  | hostfakesplit |
| 3 | SimpleX | 443 |  |  | simplex |  | hostfakesplit |
| 4 | YouTube · сайт и приложение | 80,443 |  |  | youtube |  | multisplit |
| 5 | YouTube · быстрый протокол QUIC |  | 443 |  | youtube |  | drop |
| 6 | YouTube · быстрый протокол QUIC |  | 443 |  | googlevideo |  | drop |
| 7 | YouTube · видео (googlevideo.com) | 80,443 |  |  | googlevideo |  | wssize, multidisorder |
| 8 | Бэкенд Google | 80,443 |  |  | google-backend |  | multisplit |
| 9 | Исключения домены (RU сайты) | 80,443-65535 |  |  | netrogat |  | pass |
| 10 | Исключения айпи (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 11 | Discord · обновления | 443 |  |  | discord-updates |  | tls_multisplit_sni |
| 12 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata, hostfakesplit_multi |
| 13 | Discord · сайт и приложение | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | tls_multisplit_sni |
| 14 | Мои сайты | 80,443-65535 |  |  | all |  | tls_multisplit_sni |
| 15 | SoundCloud | 80,443 |  |  | soundcloud |  | tls_multisplit_sni |
| 16 | rutracker.org | 80,443 |  |  | rutracker |  | tls_multisplit_sni |
| 17 | rutor.info (.is) | 80,443 |  |  | rutor |  | tls_multisplit_sni |
| 18 | Roblox · игра | 80,443-65535 |  |  | roblox |  | tls_multisplit_sni |
| 19 | itch.io | 80,443 |  |  | itch |  | tls_multisplit_sni |
| 20 | Facebook | 80,443 |  |  | facebook |  | tls_multisplit_sni |
| 21 | Instagram | 80,443 |  |  | instagram |  | tls_multisplit_sni |
| 22 | WhatsApp · сообщения | 80,443 |  |  | whatsapp |  | tls_multisplit_sni |
| 23 | Discord · голос и видео (discord.media) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | tls_multisplit_sni |
| 24 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 25 | Twitter/X | 80,443 |  |  | twitter |  | tls_multisplit_sni |
| 26 | ntc.party | 80,443 |  |  | ntcparty |  | multidisorder |
| 27 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | multidisorder |
| 28 | Tanki X | 80,443-65535 |  |  | tankix |  | fake, multidisorder |

## Default v1 (game filter).txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443,1080,2053,2083,2087,2096,5050,8443,8080,45631`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_sber_v2:@bin/tls_clienthello_sberbank_ru_v2.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | chatgpt | 80-65535 |  |  | chatgpt |  | pass |
| 2 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 3 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 4 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | hostfakesplit_multi |
| 5 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multisplit |
| 6 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 7 | Исключения домены (RU сайты) | 80,443-65535 |  |  | netrogat |  | pass |
| 8 | Исключения айпи (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 9 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 10 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit |
| 11 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | hostfakesplit |
| 12 | discord (images) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-images |  | hostfakesplit |
| 13 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 14 | Telegram | 80,443-65535 |  |  | telegram |  | tls_aggressive |
| 15 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 16 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 17 | Steam | 80,443 |  |  | steam |  | fake, fakeddisorder |
| 18 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 19 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | hostfakesplit |
| 20 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 21 | Roblox TCP | 80,443-65535 |  |  | roblox |  | pass |
| 22 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 23 | Twitch | 443 |  |  | twitch |  | hostfakesplit |
| 24 | Google TCP | 80,443-65535 |  |  | google |  | pass |
| 25 | TikTok | 80,443 |  |  | tiktok |  | pass |
| 26 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 27 | rutor.info (.is) | 80,443 |  |  | rutor |  | hostfakesplit_multi |
| 28 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 29 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | send, syndata, hostfakesplit_multi |
| 30 | WhatsApp | 80,443 |  |  | whatsapp |  | send, syndata, hostfakesplit_multi |
| 31 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 32 | Мои сайты | 80,443-65535 |  |  | other |  | hostfakesplit |
| 33 | Facebook | 80,443 |  |  | facebook |  | hostfakesplit_multi |
| 34 | githubusercontent.com | 443 |  |  | githubusercontent |  | send, syndata, tls_multisplit_sni |
| 35 | GitHub | 443 |  |  | github |  | send, syndata, tls_multisplit_sni |
| 36 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | hostfakesplit |
| 37 | Hetzner TCP | 80,443-65535 |  |  | hetzner |  | send, syndata, tls_multisplit_sni |
| 38 | WorldStream TCP | 80,443-65535 |  |  | worldstream |  | hostfakesplit |
| 39 | OVH TCP | 80,443-65535 |  |  | ovh |  | send, syndata, hostfakesplit_multi |
| 40 | DigitalOcean TCP | 80,443-65535 |  |  | digitalocean |  | hostfakesplit |
| 41 | Datacamp TCP | 80,443-65535 |  |  | datacamp |  | hostfakesplit |
| 42 | NovoServe TCP | 80,443-65535 |  |  | novoserve |  | hostfakesplit |
| 43 | FranTech Solutions TCP | 80,443-65535 |  |  | frantech |  | send, syndata, hostfakesplit_multi |
| 44 | Fastly TCP | 80,443-65535 |  |  | fastly |  | hostfakesplit |
| 45 | Akamai TCP | 80,443-65535 |  |  | akamai |  | hostfakesplit |
| 46 | Railway TCP | 80,443-65535 |  |  | railway |  | hostfakesplit |
| 47 | Google Cloud TCP | 80,443-65535 |  |  | usa-google |  | hostfakesplit |
| 48 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 49 | League of Legends RU TCP | 80,443-65535 |  |  | lol-ru |  | fake, fakedsplit |
| 50 | League of Legends EUW TCP | 80,443-65535 |  |  | lol-euw |  | fake, fakedsplit |
| 51 | WARP TCP | 80,443-65535 |  |  | warp |  | fake, fakedsplit |
| 52 | ZapretKVN TCP | 80,443-65535 |  |  | zapretkvn |  | fake, fakedsplit |
| 53 | Timeweb TCP | 80,443-65535 |  |  | timeweb |  | fake, fakedsplit |
| 54 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | fake |
| 55 | DigitalOcean UDP |  | 443-65535 |  | digitalocean |  | fake |
| 56 | Datacamp UDP |  | 443-65535 |  | datacamp |  | fake |
| 57 | NovoServe UDP |  | 443-65535 |  | novoserve |  | fake |
| 58 | FranTech Solutions UDP |  | 443-65535 |  | frantech |  | fake |
| 59 | Fastly UDP |  | 443-65535 |  | fastly |  | fake |
| 60 | Akamai UDP |  | 443-65535 |  | akamai |  | fake |
| 61 | Railway UDP |  | 443-65535 |  | railway |  | fake |
| 62 | Google Cloud UDP |  | 443-65535 |  | usa-google |  | fake |
| 63 | Hetzner UDP |  | 443-65535 |  | hetzner |  | fake |
| 64 | Cloudflare UDP |  | 443-65535 |  | cloudflare |  | fake |
| 65 | Cloudflare legacy UDP |  | 443-65535 |  | cloudflare-ipset |  | fake |
| 66 | Cloudflare IPv6 legacy UDP |  | 443-65535 |  | cloudflare-ipset |  | fake |
| 67 | Cloudflare alt UDP |  | 443-65535 |  | cloudflare1 |  | fake |
| 68 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 69 | Все сайты UDP (айпи) |  | 443-65535 |  | (без списка) |  | fake |

## Default v2 (game filter).txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=1`
- `--wf-tcp-out=80,443,1080,2053,2083,2087,2096,8443,8080`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_sber_v2:@bin/tls_clienthello_sberbank_ru_v2.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multisplit |
| 2 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 3 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 4 | Исключения домены (RU сайты) | 80,443-65535 |  |  | netrogat |  | pass |
| 5 | Исключения айпи (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 6 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 7 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata, hostfakesplit_multi |
| 8 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 9 | Мои сайты | 80,443-65535 |  |  | all |  | fake, multidisorder |
| 10 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 11 | rutracker.org | 80,443 |  |  | rutracker |  | hostfakesplit_multi |
| 12 | rutor.info (.is) | 80,443 |  |  | rutor |  | hostfakesplit_multi |
| 13 | Roblox TCP | 80,443-65535 |  |  | roblox |  | hostfakesplit_multi |
| 14 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 15 | Facebook | 80,443 |  |  | facebook |  | hostfakesplit_multi |
| 16 | Instagram | 80,443 |  |  | instagram |  | hostfakesplit_multi |
| 17 | WhatsApp | 80,443 |  |  | whatsapp |  | hostfakesplit_multi |
| 18 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | hostfakesplit_multi |
| 19 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 20 | Twitter/X | 80,443 |  |  | twitter |  | hostfakesplit_multi |
| 21 | ntc.party | 80,443 |  |  | ntcparty |  | multidisorder |
| 22 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | multidisorder |
| 23 | discord.com | 80,443-65535 |  |  | discord |  | fake, multidisorder |
| 24 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | fake, multidisorder |
| 25 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | fake, multidisorder |
| 26 | Amazon TCP | 80,443-65535 |  |  | amazon |  | fake, multidisorder |
| 27 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 28 | League of Legends RU TCP | 80,443-65535 |  |  | lol-ru |  | fake, multidisorder |
| 29 | League of Legends EUW TCP | 80,443-65535 |  |  | lol-euw |  | fake, multidisorder |
| 30 | OVH TCP | 80,443-65535 |  |  | ovh |  | fake, multidisorder |
| 31 | WhatsApp | 80,443-65535 |  |  | whatsapp |  | fake, multidisorder |
| 32 | WARP TCP | 80,443-65535 |  |  | warp |  | fake, multidisorder |
| 33 | ZapretKVN TCP | 80,443-65535 |  |  | zapretkvn |  | fake, multidisorder |
| 34 | Timeweb TCP | 80,443-65535 |  |  | timeweb |  | fake, multidisorder |
| 35 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | fake |
| 36 | Cloudflare UDP |  | 443-65535 |  | cloudflare |  | fake |
| 37 | Cloudflare legacy UDP |  | 443-65535 |  | cloudflare-ipset |  | fake |
| 38 | Cloudflare IPv6 legacy UDP |  | 443-65535 |  | cloudflare-ipset |  | fake |
| 39 | Cloudflare alt UDP |  | 443-65535 |  | cloudflare1 |  | fake |
| 40 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 41 | Roblox UDP |  | 443-65535 |  | roblox |  | fake |
| 42 | League of Legends RU UDP |  | 443-65535 |  | lol-ru |  | fake |
| 43 | League of Legends EUW UDP |  | 443-65535 |  | lol-euw |  | fake |
| 44 | OVH UDP |  | 443-65535 |  | ovh |  | fake |
| 45 | WhatsApp UDP wide |  | 443-65535 |  | whatsapp |  | fake |
| 46 | WARP UDP |  | 443-65535 |  | warp |  | fake |
| 47 | ZapretKVN UDP |  | 443-65535 |  | zapretkvn |  | fake |
| 48 | Timeweb UDP |  | 443-65535 |  | timeweb |  | fake |

## Default v3 (game filter).txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=1`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_sber_v2:@bin/tls_clienthello_sberbank_ru_v2.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | send, multidisorder |
| 2 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multisplit |
| 3 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 4 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | send, syndata, hostfakesplit_multi |
| 5 | Исключения домены (RU сайты) | 80,443-65535 |  |  | netrogat |  | pass |
| 6 | Исключения айпи (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 7 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 8 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | hostfakesplit_multi |
| 9 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 10 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata, hostfakesplit_multi |
| 11 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 12 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | fake |
| 13 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata, hostfakesplit_multi |
| 14 | ntc.party | 80,443 |  |  | ntcparty |  | multidisorder |
| 15 | itch.io | 80,443 |  |  | itch |  | send, syndata, hostfakesplit_multi |
| 16 | Google (сервисы) | 80,443 |  |  | google |  | send, syndata, hostfakesplit_multi |
| 17 | Amazon TCP | 80,443-65535 |  |  | amazon |  | send, syndata, hostfakesplit_multi |
| 18 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | send, syndata, hostfakesplit_multi |
| 19 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 20 | Игровые UDP порты |  | 5056,27002 |  | (без списка) |  | fake |
| 21 | Мои сайты | 80,443-65535 |  |  | all |  | send, syndata, hostfakesplit_multi |
| 22 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | multidisorder |
| 23 | Tanki X | 80,443-65535 |  |  | tankix |  | fake, multidisorder |
| 24 | discord.com | 80,443-65535 |  |  | discord |  | send, syndata, hostfakesplit_multi |
| 25 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | send, syndata, hostfakesplit_multi |
| 26 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | send, syndata, hostfakesplit_multi |
| 27 | ZapretKVN TCP | 80,443-65535 |  |  | zapretkvn |  | send, syndata, hostfakesplit_multi |
| 28 | Cloudflare UDP |  | 443-65535 |  | cloudflare |  | fake |
| 29 | Cloudflare alt UDP |  | 443-65535 |  | cloudflare1 |  | fake |
| 30 | Roblox UDP |  | 443-65535 |  | roblox |  | fake |
| 31 | League of Legends RU UDP |  | 443-65535 |  | lol-ru |  | fake |
| 32 | League of Legends EUW UDP |  | 443-65535 |  | lol-euw |  | fake |
| 33 | OVH UDP |  | 443-65535 |  | ovh |  | fake |
| 34 | WhatsApp UDP wide |  | 443-65535 |  | whatsapp |  | fake |

## Default v4 (game filter).txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=1`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=80,443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_sber_v2:@bin/tls_clienthello_sberbank_ru_v2.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | send, syndata, hostfakesplit_multi |
| 2 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | send, syndata, hostfakesplit_multi |
| 3 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 4 | Исключения домены (RU сайты) | 80,443-65535 |  |  | netrogat |  | pass |
| 5 | Исключения айпи (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 6 | DeepSeek | 80,443-65535 |  |  | deepseek |  |  |
| 7 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | pass |
| 8 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 9 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | hostfakesplit_multi |
| 10 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 11 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata, hostfakesplit_multi |
| 12 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 13 | ntc.party | 80,443 |  |  | ntcparty |  | multidisorder |
| 14 | Roblox TCP | 80,443-65535 |  |  | roblox |  | send, syndata, hostfakesplit_multi |
| 15 | Roblox UDP |  | 443-65535 |  | roblox |  | fake |
| 16 | Мои сайты | 80,443-65535 |  |  | all |  | send, syndata, multisplit |
| 17 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | multidisorder |
| 18 | ZapretKVN UDP |  | 443-65535 |  | zapretkvn |  | send, syndata, hostfakesplit_multi |
| 19 | Timeweb UDP |  | 443-65535 |  | timeweb |  | send, syndata, hostfakesplit_multi |
| 20 | discord.com | 80,443-65535 |  |  | discord |  | send, syndata, multisplit |
| 21 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | send, syndata, multisplit |
| 22 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | send, syndata, multisplit |
| 23 | Amazon TCP | 80,443-65535 |  |  | amazon |  | send, syndata, multisplit |
| 24 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | send, syndata, multisplit |
| 25 | League of Legends RU TCP | 80,443-65535 |  |  | lol-ru |  | send, syndata, multisplit |
| 26 | League of Legends EUW TCP | 80,443-65535 |  |  | lol-euw |  | send, syndata, multisplit |
| 27 | OVH TCP | 80,443-65535 |  |  | ovh |  | send, syndata, multisplit |
| 28 | WhatsApp | 80,443-65535 |  |  | whatsapp |  | send, syndata, multisplit |
| 29 | WARP TCP | 80,443-65535 |  |  | warp |  | send, syndata, multisplit |
| 30 | ZapretKVN TCP | 80,443-65535 |  |  | zapretkvn |  | send, syndata, multisplit |
| 31 | Timeweb TCP | 80,443-65535 |  |  | timeweb |  | send, syndata, multisplit |
| 32 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | fake |
| 33 | Cloudflare UDP |  | 443-65535 |  | cloudflare |  | fake |
| 34 | Cloudflare legacy UDP |  | 443-65535 |  | cloudflare-ipset |  | fake |
| 35 | Cloudflare IPv6 legacy UDP |  | 443-65535 |  | cloudflare-ipset |  | fake |
| 36 | Cloudflare alt UDP |  | 443-65535 |  | cloudflare1 |  | fake |
| 37 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 38 | League of Legends RU UDP |  | 443-65535 |  | lol-ru |  | fake |
| 39 | League of Legends EUW UDP |  | 443-65535 |  | lol-euw |  | fake |
| 40 | OVH UDP |  | 443-65535 |  | ovh |  | fake |
| 41 | WhatsApp UDP wide |  | 443-65535 |  | whatsapp |  | fake |
| 42 | WARP UDP |  | 443-65535 |  | warp |  | fake |
| 43 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata, pass |

## Default v5 (game filter).txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=1`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=80,444-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_sber_v2:@bin/tls_clienthello_sberbank_ru_v2.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multisplit |
| 2 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | fake, multidisorder |
| 3 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 4 | Исключения домены (RU сайты) | 80,443-65535 |  |  | netrogat |  | pass |
| 5 | Исключения айпи (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 6 | updates.discord.com | 443 |  |  | discord-updates |  | send, syndata |
| 7 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 8 | discord.com | 80,443-65535 |  |  | discord |  | send, syndata |
| 9 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 10 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | fake, udplen |
| 11 | Telegram | 80,443,5222 |  |  | telegram |  | send, multisplit |
| 12 | rutracker.org | 80,443 |  |  | rutracker |  | send, syndata, multisplit |
| 13 | Мои сайты | 80,443-65535 |  |  | all |  | send, syndata, multisplit |
| 14 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | send, syndata, multisplit |
| 15 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | send, syndata, multisplit |
| 16 | Tanki X | 80,443-65535 |  |  | tankix |  | send, syndata, hostfakesplit_multi |
| 17 | Amazon TCP | 80,443-65535 |  |  | amazon |  | send, syndata, multisplit |
| 18 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | send, syndata, multisplit |
| 19 | Roblox TCP | 80,443-65535 |  |  | roblox |  | send, syndata, multisplit |
| 20 | League of Legends RU TCP | 80,443-65535 |  |  | lol-ru |  | send, syndata, multisplit |
| 21 | League of Legends EUW TCP | 80,443-65535 |  |  | lol-euw |  | send, syndata, multisplit |
| 22 | OVH TCP | 80,443-65535 |  |  | ovh |  | send, syndata, multisplit |
| 23 | WhatsApp | 80,443-65535 |  |  | whatsapp |  | send, syndata, multisplit |
| 24 | WARP TCP | 80,443-65535 |  |  | warp |  | send, syndata, multisplit |
| 25 | ZapretKVN TCP | 80,443-65535 |  |  | zapretkvn |  | send, syndata, multisplit |
| 26 | Timeweb TCP | 80,443-65535 |  |  | timeweb |  | send, syndata, multisplit |
| 27 | Cloudflare UDP |  | 443-65535 |  | cloudflare |  | fake, udplen |
| 28 | Cloudflare legacy UDP |  | 443-65535 |  | cloudflare-ipset |  | fake, udplen |
| 29 | Cloudflare IPv6 legacy UDP |  | 443-65535 |  | cloudflare-ipset |  | fake, udplen |
| 30 | Cloudflare alt UDP |  | 443-65535 |  | cloudflare1 |  | fake, udplen |
| 31 | Amazon UDP |  | 443-65535 |  | amazon |  | fake, udplen |
| 32 | Roblox UDP |  | 443-65535 |  | roblox |  | fake, udplen |
| 33 | League of Legends RU UDP |  | 443-65535 |  | lol-ru |  | fake, udplen |
| 34 | League of Legends EUW UDP |  | 443-65535 |  | lol-euw |  | fake, udplen |
| 35 | OVH UDP |  | 443-65535 |  | ovh |  | fake, udplen |
| 36 | WARP UDP |  | 443-65535 |  | warp |  | fake, udplen |
| 37 | ZapretKVN UDP |  | 443-65535 |  | zapretkvn |  | fake, udplen |
| 38 | Timeweb UDP |  | 443-65535 |  | timeweb |  | fake, udplen |

## Universal V10.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  | valheim |  | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  | facebook |  | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  | speedtest |  | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  | medium |  | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  | virustotal |  | hostfakesplit |
| 7 | Выбрано вручную: github | 80,443 |  |  | github |  | hostfakesplit_multi |
| 8 | Выбрано вручную: bluesky | 80,443 |  |  | bsky-app |  | hostfakesplit_multi |
| 9 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 10 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 11 | GitHub | 443 |  |  | github |  | pass |
| 12 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 13 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 14 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 15 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 16 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 17 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 18 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 19 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 20 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 21 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 22 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 23 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 24 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |
| 25 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 26 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | udplen |
| 27 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 28 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 29 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 30 | Twitter/X | 80,443 |  |  | twitter |  | multidisorder |
| 31 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 32 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 33 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 34 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 35 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 36 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 37 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 38 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 39 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 40 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 41 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 42 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 43 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 44 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 45 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 46 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 47 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 48 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 49 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 50 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 51 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 52 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 53 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 54 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 55 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 56 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 57 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 58 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 59 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 60 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 61 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 62 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 63 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 64 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 65 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 66 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 67 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 68 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 69 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |

## Universal V11 Lite.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443,6568,19294-19344,50000-51000`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  | valheim |  | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  | facebook |  | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  | speedtest |  | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  | medium |  | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  | virustotal |  | hostfakesplit |
| 7 | Выбрано вручную: github | 80,443 |  |  | github |  | hostfakesplit_multi |
| 8 | Выбрано вручную: bluesky | 80,443 |  |  | bsky-app |  | hostfakesplit_multi |
| 9 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 10 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 11 | GitHub | 443 |  |  | github |  | pass |
| 12 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 13 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 14 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 15 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 16 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 17 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 18 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 19 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 20 | youtube.com (QUIC по имени) |  | 443 | quic | googlevideo, youtube |  | fake |
| 21 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 22 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 23 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 24 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 25 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 26 | Discord UDP (обычно не нужно) |  | 443,19294-19344,50000-50100 |  | discord |  | udplen |
| 27 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 28 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 29 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 30 | Twitter/X | 80,443 |  |  | twitter |  | multidisorder |
| 31 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 32 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 33 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 34 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 35 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 36 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 37 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 38 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 39 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 40 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 41 | Amazon UDP |  | 443 |  | amazon |  | fake |
| 42 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 43 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 44 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 45 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 46 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 47 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 48 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 49 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 50 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 51 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 52 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 53 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 54 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 55 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 56 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 57 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 58 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 59 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 60 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 61 | Cloudflare 8.6 (рецепт AnyDesk TCP) | 80,443 |  |  | cloudflare-8 |  | hostfakesplit |
| 62 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 63 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 64 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 65 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 66 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 67 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 68 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 69 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 70 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |
| 71 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |

## Universal V11.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  | valheim |  | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  | facebook |  | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  | speedtest |  | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  | medium |  | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  | virustotal |  | hostfakesplit |
| 7 | Выбрано вручную: github | 80,443 |  |  | github |  | hostfakesplit_multi |
| 8 | Выбрано вручную: bluesky | 80,443 |  |  | bsky-app |  | hostfakesplit_multi |
| 9 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 10 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 11 | GitHub | 443 |  |  | github |  | pass |
| 12 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 13 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 14 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 15 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 16 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 17 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 18 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 19 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 20 | youtube.com (QUIC по имени) |  | 443 | quic | googlevideo, youtube |  | fake |
| 21 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 22 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 23 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 24 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 25 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 26 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | udplen |
| 27 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 28 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 29 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 30 | Twitter/X | 80,443 |  |  | twitter |  | multidisorder |
| 31 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 32 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 33 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 34 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 35 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 36 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 37 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 38 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 39 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 40 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 41 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 42 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 43 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 44 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 45 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 46 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 47 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 48 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 49 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 50 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 51 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 52 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 53 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 54 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 55 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 56 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 57 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 58 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 59 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 60 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 61 | Cloudflare 8.6 (рецепт AnyDesk TCP) | 80,443 |  |  | cloudflare-8 |  | hostfakesplit |
| 62 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 63 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 64 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 65 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 66 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 67 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 68 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 69 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 70 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |
| 71 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |

## Universal V12 Lite.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443,6568,19294-19344,50000-51000`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_awswaf:@bin/nz-tls_clienthello_awswaf_com.bin`
- `--blob=tls_steam_chat:@bin/nz-tls_clienthello_steam_chat_com.bin`
- `--blob=tls_steamstatic:@bin/nz-tls_clienthello_steamstatic_com.bin`
- `--blob=tls_teamspeak:@bin/nz-tls_clienthello_teamspeak_com.bin`
- `--blob=quic_lemana:@bin/nz-quic_initial_lemana_pro.bin`
- `--blob=quic_steam_chat:@bin/nz-quic_initial_steam_chat_com.bin`
- `--blob=quic_steamstatic:@bin/nz-quic_initial_steamstatic_com.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  | valheim |  | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  | facebook |  | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  | speedtest |  | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  | medium |  | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  | virustotal |  | hostfakesplit |
| 7 | Выбрано вручную: github | 80,443 |  |  | github |  | hostfakesplit_multi |
| 8 | Выбрано вручную: bluesky | 80,443 |  |  | bsky-app |  | hostfakesplit_multi |
| 9 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 10 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 11 | GitHub | 443 |  |  | github |  | pass |
| 12 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 13 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 14 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 15 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 16 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 17 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 18 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 19 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 20 | youtube.com (QUIC по имени) |  | 443 | quic | googlevideo, youtube |  | fake |
| 21 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 22 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 23 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 24 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 25 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 26 | Discord UDP (обычно не нужно) |  | 443,19294-19344,50000-50100 |  | discord |  | udplen |
| 27 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 28 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 29 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 30 | Twitter/X | 80,443 |  |  | twitter |  | multidisorder |
| 31 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 32 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 33 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 34 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 35 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 36 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 37 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 38 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 39 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 40 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 41 | Amazon UDP |  | 443 |  | amazon |  | fake |
| 42 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 43 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 44 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 45 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 46 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 47 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 48 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 49 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 50 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 51 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 52 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 53 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 54 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 55 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 56 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 57 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 58 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 59 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 60 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 61 | Cloudflare 8.6 (рецепт AnyDesk TCP) | 80,443 |  |  | cloudflare-8 |  | hostfakesplit |
| 62 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 63 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 64 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 65 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 66 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 67 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 68 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 69 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 70 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |
| 71 | Hetzner TCP | 80,443-65535 |  |  | hetzner |  | multisplit |
| 72 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |

## Universal V12.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_awswaf:@bin/nz-tls_clienthello_awswaf_com.bin`
- `--blob=tls_steam_chat:@bin/nz-tls_clienthello_steam_chat_com.bin`
- `--blob=tls_steamstatic:@bin/nz-tls_clienthello_steamstatic_com.bin`
- `--blob=tls_teamspeak:@bin/nz-tls_clienthello_teamspeak_com.bin`
- `--blob=quic_lemana:@bin/nz-quic_initial_lemana_pro.bin`
- `--blob=quic_steam_chat:@bin/nz-quic_initial_steam_chat_com.bin`
- `--blob=quic_steamstatic:@bin/nz-quic_initial_steamstatic_com.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  | valheim |  | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  | facebook |  | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  | speedtest |  | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  | medium |  | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  | virustotal |  | hostfakesplit |
| 7 | Выбрано вручную: github | 80,443 |  |  | github |  | hostfakesplit_multi |
| 8 | Выбрано вручную: bluesky | 80,443 |  |  | bsky-app |  | hostfakesplit_multi |
| 9 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 10 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 11 | GitHub | 443 |  |  | github |  | pass |
| 12 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 13 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 14 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 15 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 16 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 17 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 18 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 19 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 20 | youtube.com (QUIC по имени) |  | 443 | quic | googlevideo, youtube |  | fake |
| 21 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 22 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 23 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 24 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 25 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 26 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | udplen |
| 27 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 28 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 29 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 30 | Twitter/X | 80,443 |  |  | twitter |  | multidisorder |
| 31 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 32 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 33 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 34 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 35 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 36 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 37 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 38 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 39 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 40 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 41 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 42 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 43 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 44 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 45 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 46 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 47 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 48 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 49 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 50 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 51 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 52 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 53 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 54 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 55 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 56 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 57 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 58 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 59 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 60 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 61 | Cloudflare 8.6 (рецепт AnyDesk TCP) | 80,443 |  |  | cloudflare-8 |  | hostfakesplit |
| 62 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 63 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 64 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 65 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 66 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 67 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 68 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 69 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 70 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |
| 71 | Hetzner TCP | 80,443-65535 |  |  | hetzner |  | multisplit |
| 72 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |

## Universal V13 Lite.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443,6568,19294-19344,50000-51000`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_awswaf:@bin/nz-tls_clienthello_awswaf_com.bin`
- `--blob=tls_steam_chat:@bin/nz-tls_clienthello_steam_chat_com.bin`
- `--blob=tls_steamstatic:@bin/nz-tls_clienthello_steamstatic_com.bin`
- `--blob=tls_teamspeak:@bin/nz-tls_clienthello_teamspeak_com.bin`
- `--blob=quic_lemana:@bin/nz-quic_initial_lemana_pro.bin`
- `--blob=quic_steam_chat:@bin/nz-quic_initial_steam_chat_com.bin`
- `--blob=quic_steamstatic:@bin/nz-quic_initial_steamstatic_com.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  | valheim |  | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  | facebook |  | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  | speedtest |  | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  | medium |  | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  | virustotal |  | hostfakesplit |
| 7 | Выбрано вручную: github | 80,443 |  |  | github |  | hostfakesplit_multi |
| 8 | Выбрано вручную: bluesky | 80,443 |  |  | bsky-app |  | hostfakesplit_multi |
| 9 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 10 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 11 | GitHub | 443 |  |  | github |  | pass |
| 12 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 13 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 14 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 15 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 16 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 17 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 18 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 19 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 20 | youtube.com (QUIC по имени) |  | 443 | quic | googlevideo, youtube |  | fake |
| 21 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 22 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 23 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 24 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 25 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 26 | Discord UDP (обычно не нужно) |  | 443,19294-19344,50000-50100 |  | discord |  | udplen |
| 27 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 28 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 29 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 30 | Twitter/X (по имени) | 80,443 |  |  | twitter |  | multisplit |
| 31 | Twitter/X | 80,443 |  |  | twitter |  | multisplit |
| 32 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 33 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 34 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 35 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 36 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 37 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 38 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 39 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 40 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 41 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 42 | Amazon UDP |  | 443 |  | amazon |  | fake |
| 43 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 44 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 45 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 46 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 47 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 48 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 49 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 50 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 51 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 52 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 53 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 54 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 55 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 56 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 57 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 58 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 59 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 60 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 61 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 62 | Cloudflare 8.6 (рецепт AnyDesk TCP) | 80,443 |  |  | cloudflare-8 |  | hostfakesplit |
| 63 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 64 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 65 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 66 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 67 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 68 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 69 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 70 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 71 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |
| 72 | Hetzner TCP | 80,443-65535 |  |  | hetzner |  | multisplit |
| 73 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |

## Universal V13 YT FIX.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--lua-init=@lua/nz-video.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-tcp-empty=1`
- `--wf-raw-filter=(!tcp or tcp.Syn or tcp.Rst or tcp.Fin or tcp.PayloadLength>0 or (outbound and tcp.DstPort==443 and ((ip.DstAddr>=74.125.0.0 and ip.DstAddr<=74.125.255.255) or (ip.DstAddr>=173.194.0.0 and ip.DstAddr<=173.194.255.255))))`
- `--wf-udp-out=443,6568,19294-19344,50000-51000`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_awswaf:@bin/nz-tls_clienthello_awswaf_com.bin`
- `--blob=tls_steam_chat:@bin/nz-tls_clienthello_steam_chat_com.bin`
- `--blob=tls_steamstatic:@bin/nz-tls_clienthello_steamstatic_com.bin`
- `--blob=tls_teamspeak:@bin/nz-tls_clienthello_teamspeak_com.bin`
- `--blob=quic_lemana:@bin/nz-quic_initial_lemana_pro.bin`
- `--blob=quic_steam_chat:@bin/nz-quic_initial_steam_chat_com.bin`
- `--blob=quic_steamstatic:@bin/nz-quic_initial_steamstatic_com.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  | valheim |  | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  | facebook |  | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  | speedtest |  | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  | medium |  | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  | virustotal |  | hostfakesplit |
| 7 | Выбрано вручную: github | 80,443 |  |  | github |  | hostfakesplit_multi |
| 8 | Выбрано вручную: bluesky | 80,443 |  |  | bsky-app |  | hostfakesplit_multi |
| 9 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 10 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 11 | GitHub | 443 |  |  | github |  | pass |
| 12 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 13 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 14 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 15 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 16 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 17 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | nz_first2, multidisorder |
| 18 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | nz_first2, multidisorder |
| 19 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 20 | youtube.com (QUIC по имени) |  | 443 | quic | googlevideo, youtube |  | fake |
| 21 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 22 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 23 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 24 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 25 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 26 | Discord UDP (обычно не нужно) |  | 443,19294-19344,50000-50100 |  | discord |  | udplen |
| 27 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 28 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 29 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 30 | Twitter/X (по имени) | 80,443 |  |  | twitter |  | multisplit |
| 31 | Twitter/X | 80,443 |  |  | twitter |  | multisplit |
| 32 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 33 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 34 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 35 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 36 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 37 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 38 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 39 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 40 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 41 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 42 | Amazon UDP |  | 443 |  | amazon |  | fake |
| 43 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 44 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 45 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 46 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 47 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 48 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 49 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 50 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 51 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 52 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 53 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 54 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 55 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 56 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 57 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 58 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 59 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 60 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 61 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 62 | Cloudflare 8.6 (рецепт AnyDesk TCP) | 80,443 |  |  | cloudflare-8 |  | hostfakesplit |
| 63 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 64 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 65 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 66 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 67 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 68 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 69 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 70 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 71 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |
| 72 | Hetzner TCP | 80,443-65535 |  |  | hetzner |  | multisplit |
| 73 | Узлы видео Google: малое окно по надобности | 443 |  |  | (без списка) |  | nz_first2 |
| 74 | Узлы видео Google: малое окно по надобности, запасные сети | 443 |  |  | (без списка) |  | nz_first2 |
| 75 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |

## Universal V13.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_awswaf:@bin/nz-tls_clienthello_awswaf_com.bin`
- `--blob=tls_steam_chat:@bin/nz-tls_clienthello_steam_chat_com.bin`
- `--blob=tls_steamstatic:@bin/nz-tls_clienthello_steamstatic_com.bin`
- `--blob=tls_teamspeak:@bin/nz-tls_clienthello_teamspeak_com.bin`
- `--blob=quic_lemana:@bin/nz-quic_initial_lemana_pro.bin`
- `--blob=quic_steam_chat:@bin/nz-quic_initial_steam_chat_com.bin`
- `--blob=quic_steamstatic:@bin/nz-quic_initial_steamstatic_com.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  | valheim |  | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  | facebook |  | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  | soundcloud |  | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  | speedtest |  | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  | medium |  | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  | virustotal |  | hostfakesplit |
| 7 | Выбрано вручную: github | 80,443 |  |  | github |  | hostfakesplit_multi |
| 8 | Выбрано вручную: bluesky | 80,443 |  |  | bsky-app |  | hostfakesplit_multi |
| 9 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 10 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 11 | GitHub | 443 |  |  | github |  | pass |
| 12 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 13 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 14 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 15 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 16 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 17 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 18 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 19 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 20 | youtube.com (QUIC по имени) |  | 443 | quic | googlevideo, youtube |  | fake |
| 21 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 22 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 23 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 24 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 25 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 26 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | udplen |
| 27 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 28 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 29 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 30 | Twitter/X (по имени) | 80,443 |  |  | twitter |  | multisplit |
| 31 | Twitter/X | 80,443 |  |  | twitter |  | multisplit |
| 32 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 33 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 34 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 35 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 36 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 37 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 38 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 39 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 40 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 41 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 42 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 43 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 44 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 45 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 46 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 47 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 48 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 49 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 50 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 51 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 52 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 53 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 54 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 55 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 56 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 57 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 58 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 59 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 60 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 61 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 62 | Cloudflare 8.6 (рецепт AnyDesk TCP) | 80,443 |  |  | cloudflare-8 |  | hostfakesplit |
| 63 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 64 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 65 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 66 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 67 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 68 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 69 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 70 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 71 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |
| 72 | Hetzner TCP | 80,443-65535 |  |  | hetzner |  | multisplit |
| 73 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |

## Universal V8.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 2 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 3 | GitHub | 443 |  |  | github |  | pass |
| 4 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 5 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 6 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 7 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 8 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 9 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 10 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 11 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 12 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 13 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 14 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 15 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 16 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |
| 17 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 18 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | udplen |
| 19 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 20 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 21 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 22 | Twitter/X | 80,443 |  |  | twitter |  | multidisorder |
| 23 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 24 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 25 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 26 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 27 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 28 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 29 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 30 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 31 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 32 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 33 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 34 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 35 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 36 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 37 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 38 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 39 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 40 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 41 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 42 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 43 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 44 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 45 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 46 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 47 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 48 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 49 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 50 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 51 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 52 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 53 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 54 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 55 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 56 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 57 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 58 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 59 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 60 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 61 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |

## Universal V9.txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=0`
- `--wf-tcp-out=80,443-65535`
- `--wf-udp-out=443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Выбрано вручную: valheim | 80,443 |  |  |  | valheim.com | hostfakesplit_multi |
| 2 | Выбрано вручную: facebook | 80,443 |  |  |  | facebook.com, fbcdn.net, fbsbx.com, fburl.com | multidisorder |
| 3 | Выбрано вручную: soundcloud | 80,443 |  |  |  | soundcloud.com, sndcdn.com, soundcloudstatus.com, soundcloudforartists.com, soundclouddesign.com, soundcloudpress.com | hostfakesplit_multi |
| 4 | Выбрано вручную: speedtest | 443,8080 |  |  |  | speedtest.net, ooklaserver.net, cdnst.net | multidisorder |
| 5 | Выбрано вручную: medium | 80,443-65535,443 |  |  |  | medium.com | hostfakesplit |
| 6 | Выбрано вручную: *.virustotal | 80,443-65535 |  |  |  | virustotal.com | hostfakesplit |
| 7 | Исключения (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 8 | Riot / Valorant TCP | 80,443-65535 |  |  | riot-valorant |  | pass |
| 9 | GitHub | 443 |  |  | github |  | pass |
| 10 | githubusercontent.com | 443 |  |  | githubusercontent |  | pass |
| 11 | Apple | 80,443-65535 |  |  | apple |  | pass |
| 12 | Cloudflare WARP API | 443 |  |  |  | api.cloudflareclient.com | multidisorder |
| 13 | Cloudflare TCP | 80,443-65535 |  |  | cloudflare |  | pass |
| 14 | cloudfront.net | 80,443-65535 |  |  | cloudfront |  | pass |
| 15 | googlevideo.com (CDN сервера) | 80,443 |  |  | googlevideo |  | multidisorder |
| 16 | youtube.com (интерфейс) | 80,443 |  |  | youtube |  | multidisorder |
| 17 | youtube.com (QUIC) |  | 443 |  | youtube |  | fake |
| 18 | updates.discord.com | 443 |  |  | discord-updates |  | hostfakesplit_multi |
| 19 | discord.media (voice RTC) | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord-media |  | send, syndata |
| 20 | disboard.org | 80,443 |  |  |  | disboard.org | hostfakesplit_multi |
| 21 | discord.com | 80,443,1080,2053,2083,2087,2096,8443 |  |  | discord |  | hostfakesplit_multi |
| 22 | discord.com (IP fallback) | 80,443-65535 |  |  | discord |  | send, syndata |
| 23 | Голосовые звонки/чаты |  |  | stun,discord | (без списка) |  | fake |
| 24 | Discord UDP (обычно не нужно) |  | 443-65535 |  | discord |  | udplen |
| 25 | Telegram | 80,443,5222 |  |  | telegram |  | send, syndata |
| 26 | Facebook | 80,443 |  |  | facebook |  | multidisorder |
| 27 | Instagram | 80,443 |  |  | instagram |  | multidisorder |
| 28 | Twitter/X | 80,443 |  |  | twitter |  | multidisorder |
| 29 | Google (сервисы) | 80,443 |  |  | google |  | fake, multidisorder |
| 30 | chatgpt | 80-65535 |  |  | chatgpt |  | fake, multidisorder |
| 31 | Gemini | 80,443 |  |  | gemini |  | fake, multidisorder |
| 32 | Notion | 80,443 |  |  | notion |  | fake, multidisorder |
| 33 | Claude | 80,443 |  |  | claude |  | fake, multidisorder |
| 34 | DeepSeek | 80,443-65535 |  |  | deepseek |  | fake, multidisorder |
| 35 | Twitch | 443 |  |  | twitch |  | fake, multidisorder |
| 36 | EpicGames & Fortnite | 80,443-65535 |  |  | epicgames-fortnite |  | fake, multidisorder |
| 37 | awsapps.com (AWS SSO / WorkSpaces) | 80,443-65535 |  |  |  | awsapps.com | fake, multidisorder |
| 38 | Amazon TCP | 80,443-65535 |  |  | amazon |  | hostfakesplit |
| 39 | Amazon UDP |  | 443-65535 |  | amazon |  | fake |
| 40 | youtube.com (RTMPS Россия) | 80,443-65535 |  |  | russia-youtube-rtmps |  | fake, multidisorder |
| 41 | rutracker.org | 443 |  |  | rutracker |  | hostfakesplit |
| 42 | LinkedIn | 80,443 |  |  | linkedin |  | multidisorder |
| 43 | virustotal.com | 80,443-65535 |  |  |  | virustotal.com | fake, multidisorder |
| 44 | modrinth.com | 80,443-65535 |  |  |  | modrinth.com | fake, multidisorder |
| 45 | skinsrestorer.net | 80,443-65535 |  |  |  | skinsrestorer.net | hostfakesplit |
| 46 | i.scdn.co (обложки Spotify) | 443 |  |  |  | i.scdn.co, mosaic.scdn.co, lineup-images.scdn.co, t.scdn.co, i-lo.scdn.co, seektables.scdn.co … | hostfakesplit |
| 47 | Steam | 80,443 |  |  | steam |  | hostfakesplit |
| 48 | Ubisoft | 80,443-65535 |  |  | ubisoft |  | hostfakesplit |
| 49 | SoundCloud | 80,443 |  |  | soundcloud |  | hostfakesplit |
| 50 | Speedtest | 443,8080 |  |  | speedtest |  | hostfakesplit |
| 51 | itch.io | 80,443 |  |  | itch |  | hostfakesplit_multi |
| 52 | Fandom | 443 |  |  | fandom |  | hostfakesplit |
| 53 | ntc.party | 80,443 |  |  | ntcparty |  | hostfakesplit |
| 54 | obsidian.md | 80,443 |  |  | obsidian |  | send, syndata |
| 55 | Microsoft Store | 443 |  |  | microsoft-store |  | send, syndata, tls_multisplit_sni |
| 56 | WhatsApp | 80,443 |  |  | whatsapp |  | pass |
| 57 | static.whatsapp.net | 80-65535 |  |  | static-whatsapp-net |  | pass |
| 58 | AnyDesk TCP | 80,443,6568 |  |  | anydesk |  | hostfakesplit |
| 59 | AnyDesk UDP |  | 443,6568,50000-51000 |  | anydesk |  | fake |
| 60 | Proton | 80,443 |  |  | proton |  | hostfakesplit |
| 61 | Signal | 80,443 |  |  | signal |  | hostfakesplit |
| 62 | Viber | 80,443 |  |  | viber |  | hostfakesplit |
| 63 | archive.org | 80,443 |  |  | archive |  | hostfakesplit |
| 64 | Bluesky | 80,443 |  |  | bluesky |  | hostfakesplit |
| 65 | Мои сайты | 80,443-65535 |  |  | other |  | fake, multidisorder |
| 66 | rutor.info (.is) | 80,443 |  |  | rutor |  | fake, multidisorder |
| 67 | Russia blacklist TCP 443 | 443 |  |  | russia-blacklist |  | fake, multidisorder |

## shadow probes (circular).txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=1`
- `--wf-tcp-out=80,443-65535`
- `--wf-tcp-in=80,443-65535`
- `--wf-udp-out=80,443-65535`
- `--wf-udp-in=80,443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_sber_v2:@bin/tls_clienthello_sberbank_ru_v2.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Исключения домены (RU сайты) | 80,443-65535 |  |  | netrogat |  | pass |
| 2 | Исключения айпи (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 3 | Все сайты (айпи) | 80,443-65535 |  |  | (без списка) |  | shadow_probe, fake, multidisorder, multisplit, fakeddisorder, fakedsplit |
| 4 | Все сайты UDP (айпи) |  | 443-65535 |  | (без списка) |  | circular, fake |

## white sni (circular).txt

Общие аргументы (порты перехвата, образцы, модули):

- `--lua-init=@lua/zapret-lib.lua`
- `--lua-init=@lua/zapret-antidpi.lua`
- `--lua-init=@lua/zapret-auto.lua`
- `--lua-init=@lua/custom_funcs.lua`
- `--lua-init=@lua/custom_diag.lua`
- `--lua-init=@lua/zapret-multishake.lua`
- `--lua-init=@lua/fakemultisplit.lua`
- `--lua-init=@lua/fakemultidisorder.lua`
- `--ctrack-disable=0`
- `--writable=user/winws2`
- `--ipcache-lifetime=8400`
- `--ipcache-hostname=1`
- `--wf-tcp-out=80,443-65535`
- `--wf-tcp-in=80,443-65535`
- `--wf-udp-out=80,443-65535`
- `--wf-udp-in=80,443-65535`
- `--wf-raw-part=@windivert.filter/windivert_part.discord_media.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.stun.txt`
- `--wf-raw-part=@windivert.filter/windivert_part.wireguard.txt`
- `--blob=tls_google:@bin/tls_clienthello_www_google_com.bin`
- `--blob=tls1:@bin/tls_clienthello_1.bin`
- `--blob=tls2:@bin/tls_clienthello_2.bin`
- `--blob=tls2n:@bin/tls_clienthello_2n.bin`
- `--blob=tls3:@bin/tls_clienthello_3.bin`
- `--blob=tls4:@bin/tls_clienthello_4.bin`
- `--blob=tls5:@bin/tls_clienthello_5.bin`
- `--blob=tls6:@bin/tls_clienthello_6.bin`
- `--blob=tls7:@bin/tls_clienthello_7.bin`
- `--blob=tls8:@bin/tls_clienthello_8.bin`
- `--blob=tls9:@bin/tls_clienthello_9.bin`
- `--blob=tls10:@bin/tls_clienthello_10.bin`
- `--blob=tls11:@bin/tls_clienthello_11.bin`
- `--blob=tls12:@bin/tls_clienthello_12.bin`
- `--blob=tls13:@bin/tls_clienthello_13.bin`
- `--blob=tls14:@bin/tls_clienthello_14.bin`
- `--blob=tls17:@bin/tls_clienthello_17.bin`
- `--blob=tls18:@bin/tls_clienthello_18.bin`
- `--blob=tls_sber:@bin/tls_clienthello_sberbank_ru.bin`
- `--blob=tls_sber_v2:@bin/tls_clienthello_sberbank_ru_v2.bin`
- `--blob=tls_vk:@bin/tls_clienthello_vk_com.bin`
- `--blob=tls_vk_kyber:@bin/tls_clienthello_vk_com_kyber.bin`
- `--blob=tls_deepseek:@bin/tls_clienthello_chat_deepseek_com.bin`
- `--blob=tls_max:@bin/tls_clienthello_max_ru.bin`
- `--blob=tls_iana:@bin/tls_clienthello_iana_org.bin`
- `--blob=tls_4pda:@bin/tls_clienthello_4pda_to.bin`
- `--blob=tls_gosuslugi:@bin/tls_clienthello_gosuslugi_ru.bin`
- `--blob=syndata3:@bin/tls_clienthello_3.bin`
- `--blob=syn_packet:@bin/syn_packet.bin`
- `--blob=dtls_w3:@bin/dtls_clienthello_w3_org.bin`
- `--blob=quic_google:@bin/quic_initial_www_google_com.bin`
- `--blob=quic_vk:@bin/quic_initial_vk_com.bin`
- `--blob=quic1:@bin/quic_1.bin`
- `--blob=quic2:@bin/quic_2.bin`
- `--blob=quic3:@bin/quic_3.bin`
- `--blob=quic4:@bin/quic_4.bin`
- `--blob=quic5:@bin/quic_5.bin`
- `--blob=quic6:@bin/quic_6.bin`
- `--blob=quic7:@bin/quic_7.bin`
- `--blob=stun_pat:@bin/stun.bin`
- `--blob=quic_test:@bin/quic_test_00.bin`
- `--blob=fake_tls:@bin/fake_tls_1.bin`
- `--blob=fake_tls_1:@bin/fake_tls_1.bin`
- `--blob=fake_tls_2:@bin/fake_tls_2.bin`
- `--blob=fake_tls_3:@bin/fake_tls_3.bin`
- `--blob=fake_tls_4:@bin/fake_tls_4.bin`
- `--blob=fake_tls_5:@bin/fake_tls_5.bin`
- `--blob=fake_tls_6:@bin/fake_tls_6.bin`
- `--blob=fake_tls_7:@bin/fake_tls_7.bin`
- `--blob=fake_tls_8:@bin/fake_tls_8.bin`
- `--blob=fake_quic:@bin/fake_quic.bin`
- `--blob=fake_quic_1:@bin/fake_quic_1.bin`
- `--blob=fake_quic_2:@bin/fake_quic_2.bin`
- `--blob=fake_quic_3:@bin/fake_quic_3.bin`
- `--blob=fake_default_udp:0x00000000000000000000000000000000`
- `--blob=http_req:@bin/http_iana_org.bin`
- `--blob=hex_0e0e0f0e:0x0E0E0F0E`
- `--blob=hex_0f0e0e0f:0x0F0E0E0F`
- `--blob=hex_0f0f0f0f:0x0F0F0F0F`
- `--blob=hex_00:0x00`
- `--name=git.zapret.moe`
- `--filter-tcp=443`
- `--hostlist=lists/git-zapret-moe.txt`
- `--out-range=-d8`
- `--lua-desync=hostfakesplit:host=ozon.ru:tcp_ts=-1000:tcp_md5:repeats=4`

| № | Секция | TCP | UDP | L7 | Сервисы (списки) | Домены | Функции |
|---:|---|---|---|---|---|---|---|
| 1 | Исключения домены (RU сайты) | 80,443-65535 |  |  | netrogat |  | pass |
| 2 | Исключения айпи (RU сайты) | 80,443-65535 |  |  | ru, dns, exclude |  | pass |
| 3 | Все сайты (айпи) | 80,443-65535 |  |  | (без списка) |  | circular, white_seqovl, fake, multisplit, multidisorder, fakeddisorder, fakedsplit |
| 4 | Все сайты UDP (айпи) |  | 443-65535 |  | (без списка) |  | circular, fake |
