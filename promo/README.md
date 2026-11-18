# Промо-ролик OCHISTKA

Рилс 1080×1920, 30 с. Сцены — HTML (`scenes/`), кадры снимает Playwright, звук синтезируется, склейка через ffmpeg.

```bash
promo/assets/fetch.sh                      # 3D-эмодзи (один раз)
ELEVENLABS_API_KEY=... bash promo/build.sh # без ключа голос временный (espeak-ng)
python3 -m pytest promo/tests -q && node --test promo/tests/*.mjs
```

Экраны — макеты в стиле интерфейса с демо-данными, не запись настоящего Windows-приложения. Адрес `ochistka.app` — заглушка. Ключ в репозиторий не записывается.
