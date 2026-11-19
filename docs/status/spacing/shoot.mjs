// Скриншоты всех разделов и вкладок. node shoot.mjs <папка> [ширина] [высота]
import { chromium } from '/opt/node-tools/node_modules/playwright/index.mjs';
const OUT = process.argv[2];
const W = +(process.argv[3] || 1179),
  H = +(process.argv[4] || 803);
const SUF = W === 1179 ? '' : `-${W}x${H}`;
const URL = 'http://127.0.0.1:1501/';
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

    if (cmd === 'default_scan_target') return 'C:/Users/user/Downloads';
    if (cmd === 'processes_request') { if (a.request.action==='services') return {type:'services',services:Array.from({length:30},(_,i)=>({name:'svc'+i,displayName:'Служба Windows '+i,startType:i%3?'Автоматически':'Вручную'}))}; return {type:'list',processes:Array.from({length:40},(_,i)=>({pid:100+i*8,name:['chrome.exe','explorer.exe','Code.exe','svchost.exe'][i%4],cpuPercent:(i*7%23)/3,memoryBytes:(30+i*13)*1048576,startTimeUnix:1700000000+i}))}; }
    if (cmd === 'programs_request') { const ac=a.request.action; if(ac==='listStartup') return {type:'startup',entries:Array.from({length:8},(_,i)=>({name:'Автозапуск '+i,command:'C:\\\\Program Files\\\\App'+i+'\\\\app.exe',location:'HKCU Run',source:'hkcu',key:'k'+i,enabled:i%2===0,previousEnabled:null}))}; if(ac==='listInstalled') return {type:'installed',programs:Array.from({length:30},(_,i)=>({name:'Программа '+i,publisher:'Издатель '+i,version:'1.'+i,sizeBytes:(i+1)*5e7,installLocation:null,diskLetter:'C:',kind:i%3?'win32':'store',id:'p'+i,iconPath:null,canUninstall:true,uninstallReason:null}))}; return {type:'stopped'}; }
    if (cmd === 'disks_request') { const ac=a.request.action; if(ac==='analyzeCleanup') return {type:'cleanup',categories:[{id:'temp',title:'Временные файлы',found:true,fileCount:1200,bytes:3e9,complete:true},{id:'cache',title:'Кэш браузеров',found:true,fileCount:800,bytes:1.2e9,complete:true},{id:'bin',title:'Корзина',found:false,fileCount:0,bytes:0,complete:true}]}; if(ac==='listQuarantine') return {type:'quarantine',entries:[]}; if(ac==='listDrives') return {type:'drives',drives:['C:','D:']}; if(ac==='privacyReport') return {type:'privacy',items:[]}; return {type:'stopped'}; }
    if (cmd === 'get_network_adapters') return [];
    if (cmd === 'get_tcp_connections') return [];
    if (cmd.startsWith('plugin:')) return null;
    return {};
  } };
})();`;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: W, height: H } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
await page.addInitScript(mock);
await page.goto(URL);
await page.waitForTimeout(600);
const secs = ['VPN', 'Вирусы', 'Процессы', 'Программы', 'Диски', 'Компьютер', 'Оформление'];
for (let i = 0; i < secs.length; i++) {
  const rail = page.locator('nav').filter({ hasText: secs[i] }).first();
  await page
    .getByRole('button', { name: new RegExp(secs[i]) })
    .first()
    .click()
    .catch(() => {});
  await page.waitForTimeout(900);
  const tabs = await page.locator('nav.subtabs:visible button').allInnerTexts();
  for (let j = 0; j < Math.max(tabs.length, 1); j++) {
    if (tabs.length) await page.locator('nav.subtabs:visible button').nth(j).click();
    await page.waitForTimeout(900);
    const over = await page.evaluate(() => ({
      sx: document.documentElement.scrollWidth > innerWidth,
      sy: document.documentElement.scrollHeight > innerHeight,
    }));
    await page.screenshot({ path: `${OUT}/${i + 1}-${secs[i]}-${j + 1}${SUF}.png` });
    if (over.sx || over.sy) console.log('overflow', secs[i], tabs[j], over);
  }
}
console.log('errors', errors.slice(0, 3));
await browser.close();
