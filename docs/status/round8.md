# Круг 8: точные порты анимаций (2026-11-21)

Точка отката: 153f44c.

Использованы оригинальные исходники (скачаны с raw.githubusercontent.com/DavidHDev/react-bits/main):

- Variable Proximity: `VariableProximity.jsx` → `components/VariableProximity.svelte` (алгоритм, параметры radius/falloff/smoothing, цикл кадров, idleWave, color-mix, scale/lift перенесены; многострочный перенос не перенесён). Шрифт: Roboto Flex, как в оригинале, урезанный Google Fonts до букв слова (5 КБ), в `assets/fonts`. Слово OCHISTKA теперь набрано этим шрифтом: форма букв немного отличается от прежнего Arial Black; положение и размер слова зарезервированы по самому жирному состоянию (проверено замером: слово 148×28 в покое, у курсора и после ухода).
- Decrypted Text: `DecryptedText.jsx` → `components/DecryptedText.svelte` (движок кадров, sequential, revealDirection, fit-сжатие, hover/view/click, loop, scramble/fade, reduced-motion; не перенесены flip и hoverRadius). Применяется только к маске ссылки подписки: при вставке и при повторном скрытии. Секретная часть в компонент не передаётся; «Показать» раскрывает ссылку сразу, без анимации. Проверено по кадрам в браузере и в тестах: секрет ни разу не попал в разметку анимации.

НЕ сделано: Uiverse (slimy-chipmunk-97 — верхняя панель, slippery-snail-18 — поле подписки). Сайт uiverse.io закрыт защитой Cloudflare (ответ 403 и страница проверки), исходный HTML/CSS получить из облака нельзя, копий этих элементов в открытых репозиториях не найдено. Нынешняя панель и поле остаются собственными реализациями и не являются точным переносом. Нужен код из блока «Get code» на страницах элементов.

Лицензии: THIRD-PARTY.md (React Bits: MIT + Commons Clause; Roboto Flex: OFL 1.1).
Не проверено на Windows: вид шрифта и плавность в WebView2.
