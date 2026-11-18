# Промо-ролик OCHISTKA Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Собрать рилс 1080×1920, 30 fps, 30 с, с озвучкой, музыкой и 3D-эмодзи.

**Architecture:** Каждая сцена — HTML-страница в `promo/scenes/`, управляемая функцией `seek(t)`. `promo/render.mjs` через Playwright делает кадры PNG по таймлайну `promo/timeline.json`. Аудио (музыка, голос) готовят отдельные скрипты, `promo/build.sh` склеивает всё ffmpeg.

**Tech Stack:** Node 22 + Playwright (уже в системе), Python 3 (синтез музыки, обёртка TTS), ffmpeg, Fluent Emoji 3D (MIT), Svelte-интерфейс `apps/desktop` для экранов.

**Spec:** `docs/superpowers/specs/2026-11-18-promo-video-design.md`

## Global Constraints

- Выход: `promo/out/ochistka-promo.mp4`, 1080×1920, 30 fps, 30.000 с, H.264 yuv420p + AAC. В git не коммитим.
- Таймкоды сцен: 0–3, 3–6, 6–11, 11–16, 16–21, 21–25, 25–30 с.
- Акцентное слово в заголовке оранжевое, цвета и шрифты берём из `docs/design/design-system.md`.
- Ключ ElevenLabs только из переменной окружения `ELEVENLABS_API_KEY`, в файлы не пишем.
- Музыка своя (синтез), без копирования примера. Текст интерфейса на русском.
- Не писать «работает на Windows»: экраны из веб-версии интерфейса.

## Review Focus

- Нет ключа ElevenLabs: сборка не падает, берётся временный голос и в логе явная пометка.
- Голос длиннее своей сцены: обрезать нельзя, скрипт падает с понятной ошибкой (или ускоряет в пределах 1.15×).
- Нет шрифта или эмодзи: сборка падает до рендера, а не рисует пустые квадраты.
- Итог не ровно 30.000 с: сборка падает.
- Конец музыки: плавное затухание последних 1.5 с, без щелчка.

---

### Task 1: Таймлайн и проверка длительности

**Files:**
- Create: `promo/timeline.json`, `promo/timeline.py`, `promo/tests/test_timeline.py`

**Interfaces:**
- Produces: `load_timeline(path: str) -> list[Scene]`, где `Scene = {id: str, start: float, end: float, title: str, accent: str, voice: str}`; `total_duration(scenes) -> float`.

- [ ] **Step 1: Тест** `test_timeline_covers_30s_without_gaps`: сцены идут подряд, `scenes[0].start == 0`, `scenes[-1].end == 30.0`, `start[i+1] == end[i]`, 7 сцен; `test_every_scene_has_voice_text`.
- [ ] **Step 2:** `python3 -m pytest promo/tests -q` → FAIL (нет модуля).
- [ ] **Step 3:** Заполнить `timeline.json` по таблице спека (id: hook, logo, viruses, processes, hardware, boost, cta), реализовать `load_timeline`, `total_duration` в `timeline.py`.
- [ ] **Step 4:** Тесты → PASS.
- [ ] **Step 5:** `git commit -m "feat(promo): add timeline"`.

### Task 2: Ресурсы (эмодзи, шрифт, мок-данные)

**Files:**
- Create: `promo/assets/emoji/*.png`, `promo/assets/fetch.sh`, `promo/assets/LICENSES.md`, `promo/tests/test_assets.py`

**Interfaces:**
- Produces: PNG-файлы `broom.png`, `wastebasket.png`, `shield.png`, `rocket.png`, `lightning.png`, `sparkles.png` (Fluent Emoji 3D, ≥256 px, с альфой).

- [ ] **Step 1: Тест** `test_required_emoji_exist_and_are_rgba`: файлы есть, Pillow открывает как RGBA, размер ≥256.
- [ ] **Step 2:** Запуск → FAIL.
- [ ] **Step 3:** `fetch.sh` скачивает файлы из репозитория `microsoft/fluentui-emoji` (MIT) по `curl` через прокси; если сеть закрыта, падает с понятной ошибкой. `LICENSES.md` — лицензия MIT и источник.
- [ ] **Step 4:** Тест → PASS.
- [ ] **Step 5:** commit `feat(promo): add emoji assets`.

### Task 3: Сцены (HTML + анимация)

**Files:**
- Create: `promo/scenes/scene.html`, `promo/scenes/scene.js`, `promo/scenes/mock/*.html` (экраны Вирусы, Процессы, Компьютер, Boost, взятые из `apps/desktop/src/components` как справка по виду), `promo/tests/scenes.test.mjs`

**Interfaces:**
- Consumes: `timeline.json` (Task 1), эмодзи (Task 2).
- Produces: страница `scene.html?scene=<id>` с глобальной функцией `window.seek(t: number): void` (t — секунды от начала сцены, детерминированная, без `requestAnimationFrame`) и `window.sceneReady: Promise<void>`.

- [ ] **Step 1: Тест** (Playwright): для каждой сцены открыть страницу 1080×1920, `await sceneReady`, `seek(0)` и `seek(длина)` дают разные скриншоты; текст заголовка из timeline присутствует; ни одной ошибки в консоли; все `img` загружены (naturalWidth > 0).
- [ ] **Step 2:** Запуск `node --test promo/tests/scenes.test.mjs` → FAIL.
- [ ] **Step 3:** Реализовать `scene.js`: заголовок с оранжевым акцентом, телефонная рамка с экраном сцены, плавающие эмодзи (синус по t), счётчики/полоса проверки на экране «Вирусы», падающая нагрузка в Boost, финал со ссылкой `ochistka.app` и QR (генерация `qrcode` из npm в dev-зависимости `promo/package.json`).
- [ ] **Step 4:** Тест → PASS.
- [ ] **Step 5:** commit `feat(promo): add scenes`.

### Task 4: Рендер кадров

**Files:**
- Create: `promo/render.mjs`, `promo/tests/render.test.mjs`

**Interfaces:**
- Consumes: `window.seek`, `timeline.json`.
- Produces: `renderFrames(outDir: string, fps = 30): Promise<number>` — пишет `frame_00000.png` … и возвращает число кадров (900).

- [ ] **Step 1: Тест:** `renderFrames` в временную папку с `fps=2` даёт 60 кадров размером 1080×1920; соседние кадры внутри сцены различаются.
- [ ] **Step 2:** → FAIL. **Step 3:** реализовать, один браузер, одна страница на сцену, `seek(t)` и `page.screenshot`. **Step 4:** → PASS. **Step 5:** commit `feat(promo): render frames`.

### Task 5: Музыка

**Files:**
- Create: `promo/audio/music.py`, `promo/tests/test_music.py`

**Interfaces:**
- Produces: `make_music(path: str, duration: float = 30.0, bpm: int = 110, sr: int = 44100) -> None` — стерео WAV.

- [ ] **Step 1: Тест:** длительность 30.0 ± 0.02 с, 2 канала, пик ≤ 0.95, последние 1.5 с тише 10 % максимума (затухание), не тишина (RMS > 0.02).
- [ ] **Step 2:** → FAIL. **Step 3:** синтез NumPy: бас, кик на долю, хэт, лёгкий пэд, подъём к 21 с и усиление к финалу. **Step 4:** → PASS. **Step 5:** commit `feat(promo): synth music`.

### Task 6: Голос

**Files:**
- Create: `promo/audio/voice.py`, `promo/tests/test_voice.py`

**Interfaces:**
- Consumes: `load_timeline`.
- Produces: `make_voice(scenes, out_path: str, api_key: str | None) -> VoiceReport`, где `VoiceReport = {engine: "elevenlabs" | "fallback", warnings: list[str]}`; на выходе WAV длиной ровно 30 с, фраза сцены стартует в её `start`.

- [ ] **Step 1: Тест:** без ключа `engine == "fallback"` и в `warnings` есть пометка; длительность 30.0 ± 0.02; фраза каждой сцены умещается в её интервал; с подменённым HTTP-клиентом (mock) и ключом `engine == "elevenlabs"` и ключ не попал в `warnings`, лог и файлы.
- [ ] **Step 2:** → FAIL. **Step 3:** ElevenLabs `POST /v1/text-to-speech/{voice_id}` с заголовком `xi-api-key`, модель `eleven_multilingual_v2`; без ключа — `espeak-ng -v ru` (установить, если нет). Вывод ранее сгенерированного звука складывается по таймкодам; слишком длинную фразу ускорять не больше 1.15×, иначе ошибка. **Step 4:** → PASS. **Step 5:** commit `feat(promo): voice track`.

### Task 7: Сборка и проверка

**Files:**
- Create: `promo/build.sh`, `promo/README.md`, `promo/tests/test_output.py`; Modify: `.gitignore` (`promo/out/`)

**Interfaces:**
- Consumes: `renderFrames`, `make_music`, `make_voice`.
- Produces: `promo/out/ochistka-promo.mp4`.

- [ ] **Step 1: Тест** `test_output_spec` (ffprobe): 1080×1920, 30 fps, длительность 30.000 ± 0.05, есть видео h264 и аудио aac, средняя громкость не «тишина» (volumedetect > −40 dB).
- [ ] **Step 2:** → FAIL. **Step 3:** `build.sh`: проверка шрифта, эмодзи, ключа (пометка «временный голос»), рендер, аудио, микс (голос над музыкой, дакинг музыки −12 dB под голосом через `sidechaincompress`), `ffmpeg -c:v libx264 -crf 16 -pix_fmt yuv420p -c:a aac -b:a 192k`. README: как запустить и подставить `ELEVENLABS_API_KEY`. **Step 4:** `bash promo/build.sh && python3 -m pytest promo/tests -q` → PASS. **Step 5:** просмотр контрольных кадров сцен глазами, commit `feat(promo): build pipeline`.
