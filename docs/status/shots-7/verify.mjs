import { chromium } from '/opt/node-tools/node_modules/playwright/index.mjs';
const OUT = process.argv[2];
const URL = process.env.URL || 'http://127.0.0.1:1433/';
const SIZES = [[1179, 803], [1920, 1080]];
const assert = (c, m) => { if (!c) { console.error('FAIL', m); process.exitCode = 1; } else console.log('ok  ', m); };
const svgIcon = (c, g) => 'data:image/svg+xml;utf8,' + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><circle cx="24" cy="24" r="20" fill="${c}"/><text x="24" y="31" font-size="22" text-anchor="middle" fill="white" font-family="Arial">${g}</text></svg>`);
const names = ['[System Process]','AbletonAudioCpl.exe','AggregatorHost.exe','ApplicationFrameHost.exe','audiodg.exe','cargo.exe','chrome.exe','chrome.exe','chrome.exe','Code.exe','dwm.exe','explorer.exe','firefox.exe','node.exe','Telegram.exe','svchost.exe','steam.exe','WINWORD.EXE'];
const procs = names.map((n, i) => ({ pid: [0,15012,4436,2628,18568,22340,1092,2612,3000,4112,1180,5240,9988,7424,6128,904,3312,8800][i], name: n, cpuPercent: [0,0,0,0,0.41,0,3.97,0,0.06,1.82,0.7,0.12,2.4,0.9,0.03,0,0.2,0][i], memoryBytes: i===0?0:[0,11.9,14.5,49.1,16.8,23.5,513.7,42,79.4,320.5,95.2,210.1,880.3,120.4,310.8,18.3,260.7,410.6][i]*1048576|0, startTimeUnix: 1 }));
const icons = { 'C:\\a\\chrome.exe': svgIcon('#e8453c','C'), 'C:\\a\\Code.exe': svgIcon('#2f7de1','{}'), 'C:\\a\\firefox.exe': svgIcon('#ff7a1a','F'), 'C:\\a\\Telegram.exe': svgIcon('#2aabee','T'), 'C:\\a\\steam.exe': svgIcon('#1b2838','S') };
const prog = (name, pub, ver, mb, loc, kind, extra={}) => ({ name, publisher: pub, version: ver, sizeBytes: mb==null?null:Math.round(mb*1048576), installLocation: loc, diskLetter: loc? loc.slice(0,2):null, kind, id: name.toLowerCase()+'|'+(ver||'')+'|'+(pub||'').toLowerCase(), iconPath: 'C:\\a\\'+name.length+'.exe', canUninstall: kind==='win32', uninstallReason: kind==='store'?'Приложения Microsoft Store удаляются через «Параметры» Windows':(kind==='other'?'В реестре нет команды удаления':null), ...extra });
const programs = [
  prog('1527c705-839a-4832-9118-54d4Bd6a0c89',null,'10.0.19640.1000',null,null,'store'),
  prog('Безопасное извлечение устройства','CN=Microsoft Windows, O=Microsoft Corporation','10.0.26100.1',null,null,'store'),
  prog('Безопасность Windows','CN=Microsoft Corporation, O=Microsoft Corporation','1000.29680.1000.0',null,null,'store'),
  prog('Блокнот Windows','CN=Microsoft Corporation, O=Microsoft Corporation','11.2609.11.0',null,null,'store'),
  prog('Google Chrome','Google LLC','131.0.6778.86',812.4,'C:\\Program Files\\Google\\Chrome','win32',{iconPath:'C:\\a\\chrome.exe'}),
  prog('Visual Studio Code','Microsoft Corporation','1.96.2',398.2,'C:\\Users\\u\\AppData\\Local\\Programs\\Microsoft VS Code','win32',{iconPath:'C:\\a\\Code.exe'}),
  prog('Telegram Desktop','Telegram FZ-LLC','5.9.0',1260,'D:\\Apps\\Telegram','win32',{iconPath:'C:\\a\\Telegram.exe'}),
  prog('Steam','Valve Corporation','2.10.91.91',2400,'D:\\Games\\Steam','win32',{iconPath:'C:\\a\\steam.exe'}),
  prog('7-Zip','Igor Pavlov','24.08',5.6,'C:\\Program Files\\7-Zip','win32'),
  prog('Старая утилита',null,null,null,null,'other'),
];
const startup = [
  { name:'Telegram', command:'"D:\\Apps\\Telegram\\Telegram.exe" -autostart', location:'Реестр: HKCU, Run', source:'hkcuRun', key:'Telegram', enabled:true, previousEnabled:null },
  { name:'Steam', command:'"D:\\Games\\Steam\\steam.exe" -silent', location:'Реестр: HKCU, Run', source:'hkcuRun', key:'Steam', enabled:false, previousEnabled:true },
  { name:'OneDrive', command:'C:\\Windows\\SysWOW64\\OneDriveSetup.exe', location:'Реестр: HKLM, Run', source:'hklmRun', key:'OneDrive', enabled:true, previousEnabled:null },
  { name:'Discord', command:null, location:'Папка «Автозагрузка» пользователя', source:'userFolder', key:'Discord.lnk', enabled:true, previousEnabled:null },
];
const mock = `(() => {
  const SVG = ${JSON.stringify(svgIcon('#888','?'))};
  const procs = ${JSON.stringify(procs)}; const programs = ${JSON.stringify(programs)}; const startup = ${JSON.stringify(startup)}; const icons = ${JSON.stringify(icons)};
  const exe = { 6128:'C:\\\\a\\\\Telegram.exe', 4112:'C:\\\\a\\\\Code.exe', 9988:'C:\\\\a\\\\firefox.exe', 3312:'C:\\\\a\\\\steam.exe', 1092:'C:\\\\a\\\\chrome.exe', 2612:'C:\\\\a\\\\chrome.exe', 3000:'C:\\\\a\\\\chrome.exe' };
  window.__calls = [];
  window.__TAURI_INTERNALS__ = { metadata:{currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}}, transformCallback(){return 1}, invoke: async (cmd, args) => {
    window.__calls.push([cmd, args]);
    const r = args && args.request;
    if (cmd === 'processes_request') { if (r.action==='list') return {type:'list', processes: procs}; if (r.action==='exePath') return {type:'exePath', path: exe[r.pid]||('C:\\\\a\\\\p'+r.pid+'.exe')}; if (r.action==='services') return {type:'services', services:[]}; }
    if (cmd === 'programs_request') {
      if (r.action==='listInstalled') return {type:'installed', programs};
      if (r.action==='listStartup') return {type:'startup', entries: startup};
      if (r.action==='icon') return {type:'icon', dataUrl: icons[r.path]||SVG};
      if (r.action==='measureFolder') return {type:'folder', report:{path:r.path, bytes: 5*1048576, files: 12, complete:true}};
      if (r.action==='uninstall') return {type:'uninstallStarted'};
      if (r.action==='setStartupEnabled') { const e = startup.find(x=>x.key===r.key); const prev=e.enabled; e.previousEnabled=prev; e.enabled=r.enabled; return {type:'startupChanged', previousEnabled:prev, enabled:r.enabled}; }
    }
    if (cmd === 'appearance_request') {
      const st = window.__st = window.__st || { accent:'#FF009D', transparency:true, darkApps:true, darkSystem:false, fillTaskbar:false, hasBackup:false, hasPreviousWallpaper:false };
      if (!r.confirmed && r.action !== 'status') throw { kind:'notConfirmed', message:'не подтверждено' };
      if (r.action==='setAccent') { st.accent=r.color; st.hasBackup=true; }
      if (r.action==='setTransparency') { st.transparency=r.enabled; st.hasBackup=true; }
      if (r.action==='setFillTaskbar') { st.fillTaskbar=r.enabled; st.hasBackup=true; }
      if (r.action==='setTheme') { st.darkApps=r.dark; st.darkSystem=r.dark; st.hasBackup=true; }
      if (r.action==='setSystemTheme') { st.darkSystem=r.dark; st.hasBackup=true; }
      if (r.action==='pickAndApplyWallpaper') { st.hasPreviousWallpaper=true; }
      return {type:'status', status:{...st}};
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
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  await page.addInitScript(mock);
  await page.goto(URL);
  await page.waitForTimeout(500);
  const tag = `${w}x${h}`;
  const noPageScroll = async (name) => {
    const r = await page.evaluate(() => [document.documentElement.scrollHeight - innerHeight, document.documentElement.scrollWidth - innerWidth]);
    assert(r[0] <= 1 && r[1] <= 1, `${tag} ${name}: no page-level scroll (${r})`);
  };

  // Процессы
  await page.getByRole('button', { name: 'Процессы', exact: true }).click();
  await page.waitForTimeout(1000);
  const procImgs = await page.evaluate(() => [...document.querySelectorAll('img')].map((i) => [i.complete, i.naturalWidth]));
  assert(procImgs.length >= 5 && procImgs.every(([c, n]) => c && n > 0), `${tag} processes: ${procImgs.length} <img> icons rendered (naturalWidth>0)`);
  const procFallbacks = await page.locator('[class*="fallback"], .letter').count();
  console.log(`${tag} processes fallback letter squares: ${procFallbacks}`);
  await noPageScroll('processes');
  await page.screenshot({ path: `${OUT}/processes-${tag}.png` });

  // Программы
  await page.getByRole('button', { name: 'Программы', exact: true }).click();
  await page.waitForTimeout(1000);
  const progImgs = await page.evaluate(() => [...document.querySelectorAll('img')].map((i) => [i.complete, i.naturalWidth]));
  assert(progImgs.length >= 4 && progImgs.every(([c, n]) => c && n > 0), `${tag} programs: ${progImgs.length} <img> icons rendered`);
  assert((await page.locator('input[type=checkbox]').count()) === 0, `${tag} programs: no checkboxes`);
  await noPageScroll('programs');
  await page.screenshot({ path: `${OUT}/programs-${tag}.png` });

  // Оформление
  await page.getByRole('button', { name: 'Оформление', exact: true }).click();
  await page.waitForTimeout(600);
  const tile = page.getByRole('button', { name: 'Обои 12', exact: true });
  await tile.scrollIntoViewIfNeeded();
  await tile.click();
  await page.mouse.move(0, 0);
  await page.waitForTimeout(300);
  assert((await tile.getAttribute('aria-pressed')) === 'true', `${tag} wallpaper tile selects`);
  const ring = await tile.evaluate((el) => getComputedStyle(el, '::after').borderTopWidth);
  assert(ring === '3px', `${tag} selected tile has white outline ring (${ring})`);
  assert((await page.locator('[title]').count()) === 0 || (await page.locator('.grid [title]').count()) === 0, `${tag} no title attributes on tiles`);
  const bodyText = await page.evaluate(() => document.body.innerText);
  assert(!/пока не подключено|Сплошной цвет|#[0-9A-Fa-f]{6}/.test(bodyText), `${tag} no hex / not-connected text`);
  const sizes = await page.evaluate(() => [...document.querySelectorAll('.grid-wallpaper .tile')].slice(0, 3).map((t) => Math.round(t.getBoundingClientRect().width)));
  console.log(`${tag} wallpaper tile widths`, sizes);
  await noPageScroll('wallpapers');
  await page.screenshot({ path: `${OUT}/wallpapers-${tag}.png` });
  // hover: only transform changes, neighbours do not move
  const before = await page.locator('.grid-wallpaper .tile').nth(1).boundingBox();
  await page.locator('.grid-wallpaper .tile').nth(0).hover();
  await page.waitForTimeout(300);
  const after = await page.locator('.grid-wallpaper .tile').nth(1).boundingBox();
  assert(Math.abs(before.x - after.x) < 0.5 && Math.abs(before.y - after.y) < 0.5, `${tag} hover does not shift neighbours`);
  await page.screenshot({ path: `${OUT}/wallpapers-hover-${tag}.png` });

  await page.getByRole('button', { name: 'Курсоры', exact: true }).click();
  await page.waitForTimeout(500);
  await page.getByRole('button', { name: 'Курсор 6', exact: true }).click();
  await page.mouse.move(0, 0);
  await page.waitForTimeout(300);
  assert(await page.evaluate(() => !!document.getElementById('ochistka-app-cursor')), `${tag} cursor applied to the app`);
  console.log(`${tag} cursor tile widths`, await page.evaluate(() => [...document.querySelectorAll('.grid-cursor .tile')].slice(0, 2).map((t) => Math.round(t.getBoundingClientRect().width))));
  await page.screenshot({ path: `${OUT}/cursors-${tag}.png` });

  await page.getByRole('button', { name: 'Цвета', exact: true }).click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: `${OUT}/colors-${tag}.png` });
  await page.getByRole('button', { name: 'Цвет 3', exact: true }).click();
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${OUT}/colors-dialog-${tag}.png` });
  await page.getByRole('button', { name: 'Сменить' }).click();
  await page.waitForTimeout(400);
  const accentCalls = await page.evaluate(() => window.__calls.filter((c) => c[1]?.request?.action === 'setAccent'));
  assert(accentCalls.length === 1 && accentCalls[0][1].request.color === '#087FFF' && accentCalls[0][1].request.confirmed === true, `${tag} accent request sent after confirm`);
  await page.mouse.move(0, 0);
  await page.screenshot({ path: `${OUT}/colors-after-${tag}.png` });
  console.log(`${tag} colour tile widths`, await page.evaluate(() => [...document.querySelectorAll('.grid-color .tile')].slice(0, 2).map((t) => Math.round(t.getBoundingClientRect().width))));
  await noPageScroll('colors');

  await page.getByRole('button', { name: 'Настройки приложения' }).click();
  await page.waitForTimeout(400);
  assert(!/Версия/.test(await page.evaluate(() => document.body.innerText)), `${tag} settings: no version text`);
  await page.screenshot({ path: `${OUT}/settings-${tag}.png` });
  assert(errors.filter((e) => !/Failed to load resource/.test(e)).length === 0, `${tag} no console errors ${JSON.stringify(errors)}`);
  await page.close();
}

// Иконка не загрузилась: буква.
{
  const page = await browser.newPage({ viewport: { width: 1179, height: 803 } });
  await page.addInitScript(mock.replace("if (r.action==='icon') return {type:'icon', dataUrl: icons[r.path]||SVG};", "if (r.action==='icon') throw {kind:'io', message:'x'};"));
  await page.goto(URL);
  await page.getByRole('button', { name: 'Программы', exact: true }).click();
  await page.waitForTimeout(1000);
  const imgs = await page.locator('img').count();
  assert(imgs === 0, `icon call fails -> no <img>, letter squares only (${imgs} imgs)`);
  await page.screenshot({ path: `${OUT}/programs-icons-fail.png` });
}
await browser.close();
