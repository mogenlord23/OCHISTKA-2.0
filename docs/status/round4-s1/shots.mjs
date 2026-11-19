// Снимки Настройки / Boost с подставным invoke. Запуск: node shots.mjs <папка> [width height]
import { chromium } from '/opt/node22/lib/node_modules/playwright/index.mjs';
const OUT = process.argv[2];
const W = Number(process.argv[3] || 1179), H = Number(process.argv[4] || 803);
const URL = process.env.URL || 'http://127.0.0.1:1571/';
const ids = ['transparency','spotlight','login_blur','widgets','fewer_offers','extra_tips','ads','suggestions','new_outlook','web_search','search_highlights','cloud_search','diagnostics','ad_id','location','error_reports','feedback','recall','camera','microphone','contacts','edge_background','edge_telemetry','edge_ads','edge_suggest','edge_passwords','edge_sidebar','cortana','copilot','onedrive_sync','settings_sync','update_ask','no_auto_reboot','game_dvr','game_dvr_policy','game_mode','power_throttling','ultimate','hags','vbs_off'];
const mock = `(() => {
  const ids = ${JSON.stringify(ids)};
  const admin = new Set(['login_blur','diagnostics','onedrive_sync','settings_sync','update_ask','no_auto_reboot','game_dvr_policy','power_throttling','hags','vbs_off']);
  const win10only = new Set(['cortana']);
  const tw = window.__tw = { on:{ transparency:true, web_search:true, camera:true }, profile:'basic' };
  const ov = () => ({ tweaks: ids.map(id => ({ id, applied: !!tw.on[id], hasBackup: !!tw.on[id],
    availability: admin.has(id) ? 'needsAdmin' : (win10only.has(id) ? 'unsupportedVersion' : 'available'),
    reason: admin.has(id) ? 'Нужны права администратора: запусти OCHISTKA от имени администратора.' : (win10only.has(id) ? 'Есть только в Windows 10.' : null),
    restart: id==='hags'||id==='vbs_off' ? 'reboot' : id==='widgets'||id==='web_search' ? 'explorer' : 'none', dangerous: id==='vbs_off' })),
    boost:{ profile: tw.profile, hasBackup: tw.profile!==null }, build: 22631, elevated:false });
  window.__TAURI_INTERNALS__ = { metadata:{currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}}, transformCallback(){return 1}, invoke: async (cmd, a) => {
    if (cmd === 'tweaks_overview') return ov();
    if (cmd === 'tweak_set') { await new Promise(r=>setTimeout(r,300)); tw.on[a.id]=a.enabled; return ov(); }
    if (cmd === 'boost_set_profile') { tw.profile=a.profile; return ov(); }
    if (cmd === 'windows_activation') return { status:'licensed', graceDays:null };
    if (cmd === 'get_system_overview') return { cpu:{ usagePercent: 30, logicalCores: 8 }, memory:{ totalBytes: 1e10, usedBytes: 5e9, usagePercent: 50 }, disks:[], osName:'Windows 11', osVersion:'10.0.22631' };
    if (cmd === 'get_hardware') return { gpus:[], motherboard:null, cpuBrand:'CPU', driveModels:[] };
    if (cmd.startsWith('plugin:')) return null;
    return {};
  } };
})();`;
const browser = await chromium.launch({ executablePath: '/opt/pw-browsers/chromium', args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: W, height: H } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
await page.addInitScript(mock);
await page.goto(URL);
await page.waitForTimeout(500);
await page.getByRole('button', { name: 'Компьютер', exact: true }).click();
await page.waitForTimeout(800);
await page.getByRole('button', { name: 'Настройки', exact: true }).click();
await page.waitForTimeout(700);
await page.screenshot({ path: `${OUT}/settings-${W}.png` });
await page.getByRole('button', { name: /Поиск Windows/, expanded: false }).first().click();
await page.getByRole('button', { name: /Конфиденциальность и диагностика/, expanded: false }).first().click();
await page.waitForTimeout(300);
await page.getByRole('switch', { name: /Убрать прозрачность/ }).click();
await page.waitForTimeout(120);
await page.screenshot({ path: `${OUT}/settings-open-${W}.png` });
await page.getByRole('button', { name: 'Boost', exact: true }).click();
await page.waitForTimeout(700);
await page.screenshot({ path: `${OUT}/boost-${W}.png` });
await page.getByRole('switch', { name: /VBS/ }).click({ force: true }).catch(()=>{});
await page.getByRole('switch', { name: /Убрать фоновую запись/ }).click();
await page.waitForTimeout(500);
await page.screenshot({ path: `${OUT}/boost-on-${W}.png` });
const text = await page.evaluate(() => document.body.innerText);
console.log('errors:', errors.length ? errors : 'none');
console.log('jargon:', /sysinfo|WMI|DTO|invoke|undefined|NaN|HKLM|HKCU|реестр/i.test(text) ? 'FOUND' : 'none');
await browser.close();
