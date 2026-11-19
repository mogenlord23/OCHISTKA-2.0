# OCHISTKA

Утилита для обслуживания компьютера: очистка, программы, процессы, сеть, безопасность, виджеты, оптимизация, драйверы и настройки.

Текущий этап — скелет приложения для **Windows 10/11 x64**. macOS и Linux пока возвращают «недоступно на этой ОС» с причиной.

## Требования

- Node.js 22 и новее, Rust (stable).
- **Windows:** Visual Studio Build Tools с рабочей нагрузкой C++, WebView2 (есть в Windows 11).
- **Linux (только для разработки в контейнере):** пакеты GTK/WebKit:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
```

## VPN-ядро (один раз)

Раздел VPN запускает программу sing-box. Она не лежит в git, скачай её одной командой (из корня проекта, Windows):

```powershell
powershell -ExecutionPolicy Bypass -File tools\vpn-core\fetch.ps1
```

Скрипт берёт официальный выпуск с GitHub, проверяет SHA-256 и кладёт файлы в `apps/desktop/src-tauri/resources/vpn-core/`. Без этого шага VPN в интерфейсе выключен с пояснением. Вне PowerShell: `sh tools/vpn-core/fetch.sh`.

## Команды

```bash
npm install
npm run dev                      # приложение в режиме разработки
npm run build                    # сборка приложения
npm test                         # тесты интерфейса (Vitest)
npm run check                    # svelte-check
npm run format:check             # Prettier
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Структура

- `apps/desktop` — Tauri-оболочка (`src-tauri/`) и интерфейс на Svelte 5 (`src/`).
- `crates/ochistka-core` — DTO, расчёты и порт системных сведений; без кода ОС.
- `crates/platform` — адаптеры по ОС: `windows/` (реализация), `macos/` и `linux/` (заглушки).
- `docs/adr/` — архитектурные решения.
- `docs/platform-status.md` — статус функций по ОС.
- `CLAUDE.md` — правила проекта.

## Статус

Текущее состояние по разделам, заглушкам и проверкам на Windows — в `docs/status/итог.md`. Статус функций по ОС — в `docs/platform-status.md`.
