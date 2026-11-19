// Скриншоты этапа S2 с подставным бэкендом. Запуск: node shots.mjs <папка> [ширина] [высота]
import { chromium } from '/opt/node22/lib/node_modules/playwright/index.mjs';
const OUT = process.argv[2];
const W = Number(process.argv[3] || 1179);
const H = Number(process.argv[4] || 803);
const URL = process.env.URL || 'http://127.0.0.1:1572/';
const mock = `(() => {
  window.__calls = [];
  const procs = Array.from({length: 40}, (_, i) => ({ pid: 100 + i * 8, name: ['chrome.exe','explorer.exe','Code.exe','svchost.exe','System','steam.exe','node.exe'][i % 7] + '', cpuPercent: Math.max(0, 9 - i * 0.2), memoryBytes: i % 7 === 4 ? 0 : (900 - i * 12) * 1048576, startTimeUnix: 1000 + i }));
  procs[4].pid = 4;
  const svcs = [
    ['Spooler','Диспетчер печати','running','auto','careful','Перестанет работать печать на всех принтерах.'],
    ['RpcSs','Удалённый вызов процедур (RPC)','running','auto','locked','Критичная служба Windows: менять её нельзя, система перестанет работать правильно.'],
    ['Fax','Факс','stopped','manual','free',null],
    ['wuauserv','Центр обновления Windows','running','manual','careful','Центр обновления Windows не сможет скачивать и ставить обновления.'],
    ['WSearch','Windows Search','running','autoDelayed','careful','Поиск в Windows станет медленным: индекс не будет обновляться.'],
    ['XblAuthManager','Диспетчер проверки подлинности Xbox Live','stopped','manual','free',null],
  ];
  for (let i = 0; i < 40; i++) svcs.push(['Svc' + i, 'Служба номер ' + i, i % 3 ? 'stopped' : 'running', i % 4 ? 'manual' : 'auto', 'free', null]);
  let elevated = ${process.env.ELEV === '0' ? 'false' : 'true'};
  const list = svcs.map(([name, displayName, state, startType, guard, note]) => ({ name, displayName, state, startType, previousStartType: null, guard, note }));
  const programs = Array.from({length: 30}, (_, i) => ({ name: 'Программа ' + i, publisher: 'Издатель', version: '1.' + i, sizeBytes: (500 - i * 10) * 1048576, installLocation: i === 3 ? null : 'C:\\\\Program Files\\\\Programma' + i, diskLetter: 'C:', kind: 'win32', id: 'p' + i, iconPath: null, canUninstall: true, uninstallReason: null }));
  const startup = Array.from({length: 25}, (_, i) => ({ name: 'Запись ' + i, command: '"C:\\\\Tools\\\\tool' + i + '.exe" /min', location: 'HKCU Run', source: 'hkcuRun', key: 'k' + i, enabled: i % 3 !== 0, previousEnabled: null }));
  window.__TAURI_INTERNALS__ = { metadata:{currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}}, transformCallback(){return 1}, invoke: async (cmd, a) => {
    window.__calls.push([cmd, a]);
    if (cmd === 'processes_request') {
      const r = a.request;
      if (r.action === 'list') return { type: 'list', processes: procs };
      if (r.action === 'exePath') return { type: 'exePath', path: null };
      if (r.action === 'revealExe') {
        if (r.pid === 4) throw { kind: 'noPath', message: 'Это системный процесс Windows, у него нет файла на диске.' };
        return { type: 'revealed', path: 'C:\\\\Program Files\\\\Google\\\\Chrome\\\\chrome.exe' };
      }
    }
    if (cmd === 'services_request') {
      const r = a.request;
      if (r.action === 'list') return { type: 'list', snapshot: { elevated, services: list } };
      const s = list.find((x) => x.name === r.name);
      await new Promise((res) => setTimeout(res, 700));
      if (r.action === 'start') s.state = 'running';
      if (r.action === 'stop') s.state = 'stopped';
      if (r.action === 'setStart') { s.previousStartType = s.previousStartType || s.startType; s.startType = r.start === 'auto' ? 'auto' : r.start === 'manual' ? 'manual' : 'disabled'; }
      return { type: 'changed', service: s };
    }
    if (cmd === 'programs_request') {
      const r = a.request;
      if (r.action === 'listInstalled') return { type: 'installed', programs };
      if (r.action === 'listStartup') return { type: 'startup', entries: startup };
      if (r.action === 'reveal') return { type: 'revealed' };
      if (r.action === 'programIcons' || r.action === 'pathIcons') return { type: 'iconBatch', items: [] };
      if (r.action === 'icon') return { type: 'icon', dataUrl: null };
    }
    if (cmd.startsWith('plugin:')) return null;
    return {};
  } };
})();`;
const browser = await chromium.launch({ executablePath: '/opt/pw-browsers/chromium', args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: W, height: H } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e) + (e.stack || '').slice(0, 300)));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
await page.addInitScript(mock);
await page.goto(URL);
await page.waitForTimeout(500);
const tag = `${W}x${H}`;
await page.getByRole('button', { name: 'Процессы', exact: true }).click();
await page.waitForTimeout(800);
await page.getByRole('button', { name: 'chrome.exe' }).first().click();
await page.getByRole('button', { name: 'Расположение файла' }).click();
await page.waitForTimeout(400);
await page.screenshot({ path: `${OUT}/processes-${tag}.png` });
await page.getByRole('button', { name: 'System' }).first().click();
await page.getByRole('button', { name: 'Расположение файла' }).click();
await page.waitForTimeout(400);
await page.screenshot({ path: `${OUT}/processes-noreveal-${tag}.png` });
await page.getByRole('tab', { name: 'Службы' }).click().catch(async () => page.getByText('Службы', { exact: true }).first().click());
await page.waitForTimeout(800);
await page.screenshot({ path: `${OUT}/services-${tag}.png` });
if (process.env.ELEV !== '0') {
  await page.getByRole('switch', { name: /Диспетчер печати/ }).click();
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${OUT}/services-confirm-${tag}.png` });
  await page.getByRole('button', { name: 'Остановить', exact: true }).click();
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/services-after-${tag}.png` });
}
await page.getByRole('button', { name: 'Программы', exact: true }).click();
await page.waitForTimeout(800);
await page.getByRole('button', { name: 'Программа 1', exact: true }).click();
await page.waitForTimeout(300);
await page.screenshot({ path: `${OUT}/programs-${tag}.png` });
await page.getByText('Автозагрузка', { exact: true }).first().click();
await page.waitForTimeout(800);
await page.screenshot({ path: `${OUT}/startup-${tag}.png` });
const bottoms = await page.evaluate(() => [...document.querySelectorAll('.grid')].map((g) => { const r = g.getBoundingClientRect(); const f = document.querySelector('footer').getBoundingClientRect(); return { gridBottom: Math.round(r.bottom), footerTop: Math.round(f.top), h: Math.round(r.height) }; }));
console.log(tag, JSON.stringify(bottoms), 'errors:', errors.length ? errors : 'none');
await browser.close();
