// Проверка разделов «Процессы», «Программы», «Вирусы» в браузере с имитацией invoke.
// Запуск: node docs/status/review-v2/proc-verify.mjs docs/status/review-v2  (dev-сервер на 1465)
import { chromium } from '/opt/node-tools/node_modules/playwright/index.mjs';
const OUT = process.argv[2];
const URL = process.env.URL || 'http://127.0.0.1:1465/';
const SIZES = [
  [1179, 803],
  [2560, 1440],
  [980, 640],
];
const assert = (c, m) => {
  if (!c) {
    console.error('FAIL', m);
    process.exitCode = 1;
  } else console.log('ok  ', m);
};
// Настоящий PNG 1x1 (красный) и крупный PNG 48x48 из SVG-рендера не нужен: проверяем путь data URL.
const PNG =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
const names = [
  'chrome.exe',
  'Code.exe',
  'explorer.exe',
  'firefox.exe',
  'svchost.exe',
  'Telegram.exe',
  'WINWORD.EXE',
];
const procs = Array.from({ length: 400 }, (_, i) => ({
  pid: 1000 + i * 4,
  name: i < names.length ? names[i] : `service${i}.exe`,
  cpuPercent: (i % 7) * 0.4,
  memoryBytes: (i + 1) * 3 * 1048576,
  startTimeUnix: 1,
}));
const mk = (name, pub, ver, mb, loc, kind) => ({
  name,
  publisher: pub,
  version: ver,
  sizeBytes: mb == null ? null : Math.round(mb * 1048576),
  installLocation: loc,
  diskLetter: loc ? loc.slice(0, 2) : null,
  kind,
  id: name.toLowerCase() + '|' + (ver || '') + '|' + (pub || '').toLowerCase(),
  iconPath: null,
  canUninstall: kind === 'win32',
  uninstallReason:
    kind === 'store' ? 'Приложения Microsoft Store удаляются через «Параметры» Windows' : null,
});
const programs = [
  mk(
    'Calculator',
    'Microsoft Corporation',
    '11.2.1',
    null,
    'C:\\Program Files\\WindowsApps\\Calc_11_x64__8wekyb3d8bbwe',
    'store',
  ),
  mk('Google Chrome', 'Google LLC', '131.0', 812, 'C:\\Program Files\\Google\\Chrome', 'win32'),
  mk(
    'Visual Studio Code',
    'Microsoft Corporation',
    '1.96',
    398,
    'C:\\Users\\u\\AppData\\Local\\Programs\\VS Code',
    'win32',
  ),
  mk('7-Zip', 'Igor Pavlov', '24.08', 5.6, 'C:\\Program Files\\7-Zip', 'win32'),
  mk('Old Tool', null, null, null, null, 'other'),
  ...Array.from({ length: 300 }, (_, i) =>
    mk(
      `Zeta ${i + 1}`,
      'Vendor',
      '1.' + i,
      i + 1,
      `D:\\Apps\\P${i}`,
      i % 5 === 0 ? 'store' : 'win32',
    ),
  ),
];
const withIcon = new Set([
  'google chrome|131.0|google llc',
  'calculator|11.2.1|microsoft corporation',
  'visual studio code|1.96|microsoft corporation',
]);
const mock = `(() => {
  const PNG = ${JSON.stringify(PNG)};
  const procs = ${JSON.stringify(procs)}; const programs = ${JSON.stringify(programs)}; const withIcon = new Set(${JSON.stringify([...withIcon])});
  window.__calls = [];
  window.__TAURI_INTERNALS__ = { metadata:{currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}}, transformCallback(){return 1}, invoke: async (cmd, args) => {
    window.__calls.push([cmd, args && args.request && args.request.action]);
    const r = args && args.request;
    if (cmd === 'processes_request') { if (r.action==='list') return {type:'list', processes: procs}; if (r.action==='exePath') return {type:'exePath', path: r.pid < 1030 ? 'C:\\\\a\\\\p.exe' : null}; if (r.action==='services') return {type:'services', services:[]}; }
    if (cmd === 'programs_request') {
      if (r.action==='listInstalled') return {type:'installed', programs};
      if (r.action==='listStartup') return {type:'startup', entries: []};
      if (r.action==='icon') return {type:'icon', dataUrl: PNG};
      if (r.action==='programIcon') return {type:'icon', dataUrl: withIcon.has(r.id) ? PNG : null};
    }
    if (cmd.startsWith('plugin:')) return null;
    return {};
  } };
})();`;
const browser = await chromium.launch();
for (const [w, h] of SIZES) {
  const page = await browser.newPage({ viewport: { width: w, height: h } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(m.text());
  });
  await page.addInitScript(mock);
  await page.goto(URL);
  await page.waitForTimeout(500);
  const tag = w === 980 ? 'min' : `${w}x${h}`;
  const titleBox = async () =>
    page.evaluate(() => {
      const r = document.querySelector('h1').getBoundingClientRect();
      return [Math.round(r.top), Math.round(r.left)];
    });
  const tops = {};
  for (const [id, label] of [
    ['proc', 'Процессы'],
    ['prog', 'Программы'],
    ['virus', 'Вирусы'],
  ]) {
    await page.getByRole('button', { name: label, exact: true }).click();
    await page.waitForTimeout(900);
    tops[id] = await titleBox();
    if (id === 'proc') {
      const info = await page.evaluate(() => ({
        rows: document.querySelectorAll('[role=table] [role=row]').length,
        imgs: [...document.querySelectorAll('.icon img')].filter(
          (i) => i.complete && i.naturalWidth > 0,
        ).length,
        glyphs: document.querySelectorAll('.icon svg.glyph').length,
        empties: [...document.querySelectorAll('.icon')].filter((e) => !e.querySelector('img,svg'))
          .length,
      }));
      assert(info.rows < 60, `${tag} proc: virtualised (${info.rows} rows for 400)`);
      assert(
        info.imgs > 0 && info.empties === 0,
        `${tag} proc: real icons ${info.imgs}, glyphs ${info.glyphs}, empty ${info.empties}`,
      );
      const t0 = Date.now();
      await page.evaluate(async () => {
        const v = document.querySelector('.vl');
        for (let i = 0; i < 20; i++) {
          v.scrollTop += 300;
          await new Promise((r) => requestAnimationFrame(r));
        }
      });
      assert(Date.now() - t0 < 2000, `${tag} proc: 20 scroll frames in ${Date.now() - t0} ms`);
      await page.evaluate(() => {
        document.querySelector('.vl').scrollTop = 0;
      });
    }
    if (id === 'prog') {
      const info = await page.evaluate(() => ({
        rows: document.querySelectorAll('[role=table] [role=row]').length,
        imgs: [...document.querySelectorAll('.icon img')].filter(
          (i) => i.complete && i.naturalWidth > 0,
        ).length,
        glyphs: document.querySelectorAll('.icon svg.glyph').length,
        empties: [...document.querySelectorAll('.icon')].filter((e) => !e.querySelector('img,svg'))
          .length,
      }));
      assert(info.rows < 60, `${tag} prog: virtualised (${info.rows} rows for 305)`);
      assert(
        info.imgs >= 2 && info.glyphs >= 1 && info.empties === 0,
        `${tag} prog: MSIX+Win32 icons ${info.imgs}, placeholders ${info.glyphs}, empty ${info.empties}`,
      );
    }
    await page.screenshot({ path: `${OUT}/proc-${id}-${tag}.png` });
  }
  assert(
    tops.proc.join() === tops.prog.join() && tops.prog.join() === tops.virus.join(),
    `${tag}: title position equal across three sections ${JSON.stringify(tops)}`,
  );
  const sc = await page.evaluate(() => [document.documentElement.scrollWidth - innerWidth]);
  assert(sc[0] <= 1, `${tag}: no horizontal page scroll`);
  assert(errors.length === 0, `${tag}: no console errors ${errors.slice(0, 2)}`);
  await page.close();
}
await browser.close();
