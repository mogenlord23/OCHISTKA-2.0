# Какие курсоры заменяет схема OCHISTKA

Схема пишет все 17 значений `HKCU\Control Panel\Cursors` в одном стиле (цвета, пропорции, hotspots проверены тестами в ядре): Arrow, Help, AppStarting, Wait, Crosshair, IBeam, NWPen, No, SizeNS, SizeWE, SizeNWSE, SizeNESW, SizeAll, UpArrow, Hand, Pin, Person. Wait и AppStarting — настоящие `.ani` (24 кадра, «хвост кометы»).

Таблица состояний браузерного теста (https://abacktools.com/ru/tools/design/utilities/css-cursor-tester) лежит в `apps/desktop/src/lib/cursorSupport.ts` и проверяется тестом.

| Состояние CSS                                                              | Что происходит                                                         |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| default, pointer, text, wait, progress, move, crosshair, help, not-allowed | системная роль, схема заменяет                                         |
| n/s/e/w-resize, ne/nw/se/sw-resize, ns/ew/nesw/nwse-resize                 | системные роли SizeNS/SizeWE/SizeNESW/SizeNWSE, схема заменяет         |
| col-resize, row-resize                                                     | зависит от версии браузера (системные SizeWE/SizeNS или своя картинка) |
| vertical-text, cell, zoom-in, zoom-out, grab, grabbing, copy, alias        | рисует сам браузер: в Windows-схеме для них нет роли                   |

Автопрокрутка по колёсику (якорь) и некоторые окна сторонних программ тоже рисуют свои курсоры.
OCHISTKA не внедряет код в чужие процессы и не подменяет курсоры сторонних браузеров. Не проверено на настоящей Windows и в разных версиях браузеров.
