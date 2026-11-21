# Проверка SonicDPI v0.2.2 (Windows x64)

Дата: 2026-11-21. Файл скачан и проверен на GitHub Actions (windows-2022), не на рабочем компьютере. Файл не запускался и не распаковывался на диск (имена прочитаны из архива через `ZipFile.OpenRead`).

- Адрес: https://github.com/by-sonic/sonicdpi/releases/download/v0.2.2/SonicDPI-windows-x64.zip
- Размер: 1 363 584 байт
- Ожидаемый SHA-256 (из `SHA256SUMS.txt` релиза, по данным владельца): `ec1e9f32315bdab1989d8e78bb3d3ad6975700fe83546139eb6f21e132991fc0`
- Вычисленный SHA-256: `ec1e9f32315bdab1989d8e78bb3d3ad6975700fe83546139eb6f21e132991fc0`
- **Хеш совпал: да**
- Запуск: https://github.com/mogenlord23/OCHISTKA-2.0/actions/runs/44954088362

## Файлы в архиве (имя, размер в байтах)

| Файл | Размер |
|---|---|
| LICENSE-APACHE | 861 |
| LICENSE-MIT | 1099 |
| README.md | 16483 |
| sonicdpi-tray.exe | 1051648 |
| sonicdpi.exe | 1808896 |
| THIRD_PARTY_LICENSES.md | 1794 |
| WinDivert.dll | 47616 |
| WinDivert64.sys | 94144 |

## Не проверено

- Совпадение с `SHA256SUMS.txt` на странице релиза: сам файл мной не читался, эталон взят из сообщения владельца.
- Цифровая подпись exe и драйвера, отсутствие вредоносного кода (антивирусная проверка не делалась).
- Работа SonicDPI на Windows, у провайдера и в голосе Discord.
- Версия внутри exe (в `Cargo.toml` значится 0.1.0, тег v0.2.2).
- Запуск шёл из временного `push`-триггера: на ветке, не являющейся основной, ручной запуск (`workflow_dispatch`) через API вернул 404. Триггер убран, workflow снова только ручной.
