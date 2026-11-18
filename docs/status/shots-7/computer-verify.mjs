// Проверка раздела «Компьютер» в браузере с подставным invoke. Запуск: node computer-verify.mjs <папка> <iter>
import { chromium } from '/opt/node-tools/node_modules/playwright/index.mjs';
const OUT = process.argv[2];
const IT = process.argv[3] || '1';
const URL = process.env.URL || 'http://127.0.0.1:1432/';
const GB = 1024 ** 3;
const mock = `(() => {
  let n = 0; const tw = window.__tw = { transparency:false, web_search:false, spotlight:false, suggestions:false, game_dvr:false, game_mode:false, backup:{}, profile:null };
  const ov = () => ({ tweaks: Object.keys(tw).filter(k=>!['backup','profile'].includes(k)).map(id=>({id, applied: tw[id], hasBackup: !!tw.backup[id]})), boost:{ profile: tw.profile, hasBackup: tw.profile!==null } });
  window.__calls = [];
  window.__TAURI_INTERNALS__ = { metadata:{currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}}, transformCallback(){return 1}, invoke: async (cmd, a) => {
    window.__calls.push([cmd, a]);
    if (cmd === 'get_system_overview') { n++; const w = Math.sin(n/4); return { cpu:{ usagePercent: 30 + 25*w + (n%7), logicalCores: 8 }, memory:{ totalBytes: ${24 * GB}, usedBytes: ${15.6 * GB}, usagePercent: 65 + (n%3)*0.2 }, disks:[{displayName:'Windows (C:)', mountPoint:'C:\\\\', totalBytes:${111 * GB}, usedBytes:${60 * GB}, usagePercent:54, kind:'ssd'},{displayName:'Данные (D:)', mountPoint:'D:\\\\', totalBytes:${931 * GB}, usedBytes:${400 * GB}, usagePercent:43, kind:'hdd'}], osName:'Майкрософт Windows 11 Домашняя', osVersion:'10.0.26200' }; }
    if (cmd === 'get_hardware') return { gpus:[{ vendor:'NVIDIA', model:'GeForce GTX 1060 3GB', usagePercent: Math.abs(Math.sin(n/3))*12, totalMemoryBytes:${3 * GB}, usedMemoryBytes:${0.9 * GB} }], motherboard:{ vendor:'ASRock', name:'Z170 Extreme4', version:null }, cpuBrand:'Intel(R) Core(TM) i7-6700 CPU @ 3.40GHz', driveModels:['KINGSTON SHFS37A120G','WDC WD10EZEX-21WN4A0'] };
    if (cmd === 'tweaks_overview') return ov();
    if (cmd === 'tweak_set') { tw[a.id]=a.enabled; tw.backup[a.id]=a.enabled; return ov(); }
    if (cmd === 'tweaks_revert') { for (const k of Object.keys(tw)) if (typeof tw[k]==='boolean') tw[k]=false; tw.backup={}; tw.profile=null; return ov(); }
    if (cmd === 'boost_set_profile') { tw.profile=a.profile; tw.game_dvr = a.profile!==null; tw.game_mode = a.profile!==null; return ov(); }
    if (cmd.startsWith('plugin:')) return null;
    return {};
  } };
})();`;
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1179, height: 803 } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
page.on('console', (m) => {
  if (m.type() === 'error') errors.push(m.text());
});
await page.addInitScript(mock);
await page.goto(URL);
await page.waitForTimeout(400);
await page.getByRole('button', { name: 'Компьютер', exact: true }).click();
await page.waitForTimeout(6500);
await page.screenshot({ path: `${OUT}/computer-${IT}-components.png` });
await page.getByRole('button', { name: 'Показать графики загрузки' }).click();
await page.waitForTimeout(800);
await page.screenshot({ path: `${OUT}/computer-${IT}-graphs.png` });
await page.getByRole('button', { name: /GPU \/ Видеокарта/ }).click();
await page.waitForTimeout(900);
await page.screenshot({ path: `${OUT}/computer-${IT}-gpu.png` });
await page.getByRole('button', { name: 'Настройки', exact: true }).click();
await page.waitForTimeout(500);
await page.screenshot({ path: `${OUT}/computer-${IT}-settings.png` });
await page.getByRole('checkbox', { name: /Убрать прозрачность/ }).click();
await page.waitForTimeout(400);
await page.screenshot({ path: `${OUT}/computer-${IT}-settings-dialog.png` });
await page.getByRole('button', { name: 'Применить', exact: true }).click();
await page.waitForTimeout(500);
await page.getByRole('button', { name: 'Boost', exact: true }).click();
await page.waitForTimeout(500);
await page.screenshot({ path: `${OUT}/computer-${IT}-boost.png` });
await page.getByRole('button', { name: 'Драйверы', exact: true }).click();
await page.waitForTimeout(600);
await page.screenshot({ path: `${OUT}/computer-${IT}-drivers.png` });
const text = await page.evaluate(() => document.body.innerText);
console.log('errors:', errors.length ? errors : 'none');
console.log('jargon:', /sysinfo|WMI|DTO|invoke|undefined|NaN/i.test(text) ? 'FOUND' : 'none');
await browser.close();
