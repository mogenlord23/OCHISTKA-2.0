# Проверка ссылок каталога драйверов (2026-11-19)

Способ: `curl -L` (GET, до 5 переходов) из песочницы разработки, затем повтор со стандартным User-Agent curl там, где Chrome-подобный UA получил 403 от защиты сайта. Для установщиков запрашивались первые 1024 байта (`-r 0-1023`, ответ 206 = файл на месте). Это проверка из Linux-песочницы через общий прокси; на Windows у пользователя не запускалась.

Расшифровка: «исправлено» значит, что адрес в `crates/ochistka-core/src/drivers.rs` заменён; «не проверить» значит, что сайт отвечает 403 или обрывает соединение даже на главной странице (защита от ботов), то есть ошибка не в нашей ссылке.

## Страницы (кнопка «Открыть сайт»)

| Карточка | Ссылка | Код | Итог |
| --- | --- | --- | --- |
| nvidia-app | https://www.nvidia.com/en-us/software/nvidia-app/ | 200 | ок |
| nvidia-geforce | https://www.nvidia.com/en-us/geforce/drivers/ | 200 | ок |
| nvidia-physx | https://www.nvidia.com/en-us/drivers/physx/ | 404 | исправлено на https://www.nvidia.com/en-us/drivers/physx/physx-9-21-0713-driver/ (200, «NVIDIA PhysX System Software») |
| nvidia-cuda | https://developer.nvidia.com/cuda-downloads | 200 | ок |
| amd-software | https://www.amd.com/en/support/download/drivers.html | 200 | ок |
| amd-detect | https://www.amd.com/en/support/download/drivers.html | 200 | ок |
| amd-chipset | https://www.amd.com/en/support/download/drivers.html | 200 | ок |
| intel-dsa | https://www.intel.com/content/www/us/en/support/detect.html | 200 (с UA curl; 403 с Chrome-UA) | ок |
| intel-chipset | https://www.intel.com/content/www/us/en/download/19347/chipset-inf-utility.html | 200 (то же) | ок |
| intel-me | https://www.intel.com/content/www/us/en/download-center/home.html | 200 (то же) | ок (общий центр загрузки Intel: пакет зависит от платы). Страница 19392 оказалась «Version Detection Tool», а не драйвером, поэтому не взята |
| intel-wifi | https://www.intel.com/content/www/us/en/download/19351/intel-wireless-wi-fi-drivers-for-windows-10-and-windows-11.html | 200 (то же) | ок |
| intel-bluetooth | https://www.intel.com/content/www/us/en/download/18649/intel-wireless-bluetooth-for-windows-10-and-windows-11.html | 200 с переходом на новое имя | исправлено на .../18649/intel-wireless-bluetooth-drivers-for-windows-10-and-windows-11.html (200 без перехода) |
| realtek-audio | https://www.realtek.com/Download/List?cate_id=593&menu_id=369 | 200 | ок |
| realtek-network | https://www.realtek.com/Download/List?cate_id=584&menu_id=297 | 200 | ок |
| usb-controllers | https://support.microsoft.com/windows/get-updates-for-windows-drivers | 404 | исправлено на https://support.microsoft.com/en-us/windows/hardware/drivers/update-drivers-through-device-manager-in-windows (200) |
| logitech | https://www.logitech.com/en-us/software.html | 200 (переход на /software) | ок |
| asus | https://www.asus.com/support/download-center/ | 200 | ок |
| msi | https://www.msi.com/support/download | 403 | не проверить: главная msi.com тоже 403 (защита от ботов); страница официальная, оставлена |
| gigabyte | https://www.gigabyte.com/Support/Consumer/Download | 403 | не проверить: gigabyte.com тоже 403; оставлена |
| lenovo | https://pcsupport.lenovo.com/ | 200 | ок |
| dell | https://www.dell.com/support/home/drivers | 403 | не проверить: dell.com/support тоже 403; оставлена |
| hp | https://support.hp.com/drivers | 200 | ок |
| acer | https://www.acer.com/us-en/support/drivers-and-manuals | обрыв (HTTP/2 INTERNAL_ERROR, код 000) | не проверить: весь домен acer.com из песочницы обрывается (главная тоже); оставлена |
| surface | https://www.microsoft.com/en-us/download/details.aspx?id=100429 | 200 | ок |

## Прямые установщики (кнопка «Установить»)

| Карточка | Ссылка | Код | Итог |
| --- | --- | --- | --- |
| nvidia-app | https://us.download.nvidia.com/nvapp/client/11.0.9.565/NVIDIA_app_v11.0.9.565.exe | 206, 1024 байта | ок |
| intel-dsa | https://dsadata.intel.com/installer | 206, 1024 байта | ок |
| intel-wifi | https://downloadmirror.intel.com/929786/WiFi-24.80.0-Driver64-Win10-Win11.exe | 206, 1024 байта | ок |

Версии файлов зашиты в адресе: при выходе новых версий производитель может убрать старый файл, тогда кнопка покажет понятную ошибку «Не удалось скачать» и предложит открыть сайт. Подпись установщика проверяется всегда (издатель из каталога).

## Не проверено

- Ссылки MSI, Gigabyte, Dell, Acer: сайты недоступны из песочницы (см. выше).
- Реальная загрузка и запуск установщиков на Windows.
