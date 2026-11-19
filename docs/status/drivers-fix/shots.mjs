// Снимки страницы «Драйверы» в браузере с подставным invoke. Запуск: node shots.mjs <папка>
// (vite dev на порту 1544). Установка подменена: приходят события загрузки и ошибка подписи.
import { chromium } from '/opt/node22/lib/node_modules/playwright/index.mjs';
const OUT = process.argv[2];
const URL = process.env.URL || 'http://127.0.0.1:1544/';
const mock = `(() => {
  const cbs = {}; let seq = 1;
  const items = [
    ['nvidia-app','nvidia','NVIDIA App · драйверы и библиотека игр','Официальное приложение NVIDIA: драйверы Game Ready и Studio, настройки игр и запись игрового процесса.','install',false],
    ['nvidia-geforce','nvidia','GeForce Game Ready / Studio','Выберите точную модель видеокарты и ветку драйвера на странице NVIDIA.','openSite',true],
    ['nvidia-physx','nvidia','PhysX System Software','Дополнительная библиотека для старых игр, которым нужен NVIDIA PhysX.','openSite',false],
    ['nvidia-cuda','nvidia','CUDA Toolkit','Для приложений и игр, которым нужны вычисления на видеокарте.','openSite',false],
    ['amd-software','amd','AMD Software · Adrenalin','Официальный установщик драйвера Radeon. Версию и совместимость проверяет сам производитель.','openSite',true],
    ['intel-dsa','intel','Intel · автоопределение','Официальный помощник Intel подбирает поддерживаемые драйверы для устройств Intel.','install',false],
    ['intel-wifi','bluetooth','Intel · Wi-Fi','Пакет для поддерживаемых беспроводных адаптеров Intel под Windows 10 и 11.','install',true],
    ['realtek-audio','realtek','Realtek · звук','Каталог звуковых драйверов Realtek.','openSite',true],
    ['usb-controllers','usb','USB / контроллеры','Стандартные USB-драйверы входят в Windows.','openSite',true],
    ['asus','oem','ASUS · ноутбук или плата','Поиск по точной модели: Wi-Fi, USB, звук и чипсет.','openSite',true],
  ].map(([id,vendor,title,description,action,windowsUpdate]) => ({id,vendor,title,description,action,windowsUpdate}));
  window.__calls = [];
  window.__TAURI_INTERNALS__ = { metadata:{currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}},
    transformCallback(cb){ const id = seq++; cbs[id] = cb; return id; }, unregisterCallback(id){ delete cbs[id]; },
    invoke: async (cmd, a) => {
      window.__calls.push([cmd, a && a.id]);
      if (cmd === 'get_hardware') return { gpus:[{ vendor:'NVIDIA', model:'GeForce GTX 1060 3GB', usagePercent:3, totalMemoryBytes:3221225472, usedMemoryBytes:900000000 }], motherboard:{ vendor:'ASRock', name:'Z170 Extreme4', version:null }, cpuBrand:'Intel(R) Core(TM) i7-6700 CPU @ 3.40GHz', driveModels:[] };
      if (cmd === 'drivers_catalog') return items;
      if (cmd === 'driver_install') {
        const send = (i, message) => cbs[a.onProgress.id]({ index: i, message });
        const ev = (stage, percent=null, message=null) => ({ id:a.id, stage, percent, message });
        if (a.id === 'nvidia-app') { send(0, ev('downloading', 42)); await new Promise(() => {}); }
        send(0, ev('downloading', 100)); send(1, ev('verifying'));
        send(2, ev('error', null, 'Подпись принадлежит другому издателю: Somebody Else'));
        throw { kind:'driver', message:'Подпись принадлежит другому издателю: Somebody Else' };
      }
      if (cmd.startsWith('plugin:')) return null;
      return {};
    } };
})();`;
const browser = await chromium.launch({ executablePath: '/opt/pw-browsers/chromium', args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 1179, height: 803 } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
await page.addInitScript(mock);
await page.goto(URL);
await page.waitForTimeout(400);
await page.getByRole('button', { name: 'Компьютер', exact: true }).click();
await page.getByRole('button', { name: 'Драйверы', exact: true }).click();
await page.waitForTimeout(1500);
await page.evaluate(() => { for (const e of document.querySelectorAll('.stage')) e.scrollLeft = 0; });
await page.screenshot({ path: `${OUT}/drivers-all.png` });
await page.getByRole('button', { name: /^NVIDIA/ }).click();
await page.getByRole('button', { name: 'Установить', exact: true }).click();
await page.waitForTimeout(500);
await page.evaluate(() => { for (const e of document.querySelectorAll('.stage')) e.scrollLeft = 0; });
await page.screenshot({ path: `${OUT}/drivers-nvidia-downloading.png` });
await page.reload();
await page.waitForTimeout(600);
await page.getByRole('button', { name: 'Компьютер', exact: true }).click();
await page.getByRole('button', { name: 'Драйверы', exact: true }).click();
await page.waitForTimeout(800);
await page.getByRole('button', { name: /^Intel/ }).click();
await page.getByRole('button', { name: 'Установить', exact: true }).click();
await page.waitForTimeout(500);
await page.evaluate(() => { for (const e of document.querySelectorAll('.stage')) e.scrollLeft = 0; });
await page.screenshot({ path: `${OUT}/drivers-intel-error.png` });
console.log('errors:', JSON.stringify(errors), 'calls:', JSON.stringify(await page.evaluate(() => window.__calls)));
await browser.close();
