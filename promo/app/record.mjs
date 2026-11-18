// Записывает настоящий интерфейс OCHISTKA (apps/desktop, vite) с виртуальным курсором.
// Результат: out/rec/frames/*.jpg + out/rec/frames.txt (список для ffmpeg concat с реальными длительностями).
import { createRequire } from 'node:module';
const { chromium } = createRequire(import.meta.url)('playwright');
import fs from 'node:fs';
import path from 'node:path';

const OUT = process.argv[2] || 'promo/out/rec';
const W = 760, H = 820, APP = process.env.APP_URL || 'http://127.0.0.1:1420/';
fs.rmSync(OUT, { recursive: true, force: true });
fs.mkdirSync(path.join(OUT, 'frames'), { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
await page.addInitScript(fs.readFileSync(new URL('./mock.js', import.meta.url), 'utf8'));
await page.addInitScript(() => { window.__scanMs = 3300; });
await page.goto(APP);
await page.waitForSelector('nav');

// курсор + круг клика
await page.evaluate(() => {
  const c = document.createElement('div');
  c.id = '__cur';
  c.style.cssText = 'position:fixed;left:0;top:0;width:44px;height:44px;z-index:99999;pointer-events:none;transform:translate(1000px,1300px);filter:drop-shadow(0 4px 6px rgba(0,0,0,.45))';
  c.innerHTML = '<svg viewBox="0 0 24 24" width="44" height="44"><path d="M4 2 L4 19 L8.6 14.9 L11.6 21.6 L14.2 20.4 L11.3 13.8 L17.6 13.8 Z" fill="#fff" stroke="#000" stroke-width="1.6" stroke-linejoin="round"/></svg>';
  document.body.appendChild(c);
  const r = document.createElement('div');
  r.id = '__rip';
  r.style.cssText = 'position:fixed;width:20px;height:20px;border-radius:50%;border:4px solid #fff;z-index:99998;pointer-events:none;opacity:0;box-shadow:0 0 0 2px #000';
  document.body.appendChild(r);
  window.__ripple = (x, y) => {
    r.style.left = x - 10 + 'px'; r.style.top = y - 10 + 'px';
    r.animate([{ opacity: 1, transform: 'scale(.4)' }, { opacity: 0, transform: 'scale(3.2)' }], { duration: 520, easing: 'ease-out' });
  };
  window.__cur = (x, y) => { c.style.transform = `translate(${x - 8}px,${y - 4}px)`; };
});

const client = await ctx.newCDPSession(page);
const frames = [];
let n = 0;
client.on('Page.screencastFrame', async (f) => {
  const name = `f${String(n++).padStart(5, '0')}.jpg`;
  fs.writeFileSync(path.join(OUT, 'frames', name), Buffer.from(f.data, 'base64'));
  frames.push({ name, ts: f.metadata.timestamp });
  client.send('Page.screencastFrameAck', { sessionId: f.sessionId }).catch(() => {});
});
await client.send('Page.startScreencast', { format: 'jpeg', quality: 92, maxWidth: W * 2, maxHeight: H * 2, everyNthFrame: 1 });

const T0 = Date.now();
const now = () => (Date.now() - T0) / 1000;
const sleepUntil = async (t) => { const d = (t - now()) * 1000; if (d > 0) await page.waitForTimeout(d); };
let cx = 1000, cy = 1300;
const ease = (x) => (x < 0.5 ? 4 * x ** 3 : 1 - (-2 * x + 2) ** 3 / 2);
async function moveTo(x, y, dur) {
  const sx = cx, sy = cy, t0 = now();
  for (;;) {
    const k = Math.min(1, (now() - t0) / dur);
    const e = ease(k);
    // лёгкая дуга, как у живой руки
    const nx = sx + (x - sx) * e + Math.sin(k * Math.PI) * 14, ny = sy + (y - sy) * e - Math.sin(k * Math.PI) * 10;
    cx = nx; cy = ny;
    await page.mouse.move(nx, ny);
    await page.evaluate(([a, b]) => window.__cur(a, b), [nx, ny]);
    if (k >= 1) break;
    await page.waitForTimeout(8);
  }
}
async function pos(sel) {
  const b = await page.locator(sel).first().boundingBox();
  return [b.x + b.width / 2, b.y + b.height / 2];
}
async function click(sel, at, dur = 0.7) {
  const [x, y] = await pos(sel);
  await sleepUntil(at - dur);
  await moveTo(x, y, dur);
  await page.evaluate(([a, b]) => window.__ripple(a, b), [x, y]);
  await page.mouse.down(); await page.waitForTimeout(70); await page.mouse.up();
}
const nav = (name) => `nav [aria-label="${name}"], nav button:has-text("${name}")`;

// ---- сценарий, секунды от начала записи ----
await sleepUntil(0.4);
await click('button:has-text("Вирусы")', 1.2);
await click('button:has-text("Начать проверку")', 2.4, 0.9);
await sleepUntil(5.6);
await click('button:has-text("Находки"), [role=tab]:has-text("Находки")', 6.3, 0.8);
await moveTo(cx - 40, cy + 260, 0.9);
await sleepUntil(7.5);
await click('button:has-text("Компьютер")', 8.3, 0.9);
await sleepUntil(11.0);
const [hx, hy] = [300, 380];
await moveTo(hx, hy, 1.0);
await sleepUntil(13.0);
for (const [name, at] of [['Процессы', 14.0], ['Диски', 15.0], ['Оформление', 16.0], ['VPN', 17.0], ['Вирусы', 18.0]]) {
  await click(`button:has-text("${name}")`, at, 0.55);
}
await sleepUntil(19.4);

await client.send('Page.stopScreencast');
await page.waitForTimeout(200);
// список кадров для ffmpeg concat (по реальным меткам времени)
let list = '';
for (let i = 0; i < frames.length; i++) {
  const dur = i < frames.length - 1 ? frames[i + 1].ts - frames[i].ts : 0.033;
  list += `file 'frames/${frames[i].name}'\nduration ${dur.toFixed(4)}\n`;
}
list += `file 'frames/${frames[frames.length - 1].name}'\n`;
fs.writeFileSync(path.join(OUT, 'frames.txt'), list);
console.log('frames', frames.length, 'span', (frames.at(-1).ts - frames[0].ts).toFixed(2), 's', 'errors', errors);
await browser.close();
