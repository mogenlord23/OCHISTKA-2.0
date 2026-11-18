// Проверка раскладки оболочки в настоящем Chromium (Playwright, вызовы Tauri подменены).
// Запуск: сервер `npx vite --port 1431 --strictPort`, затем `node scripts/layout-check.mjs [папка снимков]`.
// Нужен пакет playwright (глобально или рядом): PLAYWRIGHT_BROWSERS_PATH указывает на браузеры.
import fs from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = (() => {
  try {
    return require('playwright');
  } catch {
    return require('/opt/node22/lib/node_modules/playwright');
  }
})();

const OUT = process.argv[2];
if (OUT) fs.mkdirSync(OUT, { recursive: true });
const SIZES = [
  [1179, 803],
  [900, 520],
  [1920, 1080],
  [2560, 1440],
];
let failed = 0;
const check = (ok, text) => {
  if (!ok) failed++;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`);
};

const browser = await chromium.launch();
for (const [w, h] of SIZES) {
  for (const maximized of [false, true]) {
    if (maximized && w !== 1920) continue;
    const page = await browser.newPage({ viewport: { width: w, height: h } });
    await page.addInitScript((max) => {
      window.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: { label: 'main' },
          currentWebview: { label: 'main', windowLabel: 'main' },
        },
        transformCallback: () => 1,
        invoke: async (cmd) => {
          if (cmd === 'plugin:window|is_maximized') return max;
          if (cmd.startsWith('plugin:event|')) return 1;
          return null;
        },
      };
    }, maximized);
    await page.goto('http://127.0.0.1:1431/');
    await page.waitForTimeout(600);
    const tag = `${w}x${h}${maximized ? ' maximized' : ''}`;

    const m = await page.evaluate(() => {
      const box = (e) => {
        const r = e.getBoundingClientRect();
        return {
          top: r.top,
          bottom: r.bottom,
          left: r.left,
          right: r.right,
          cy: (r.top + r.bottom) / 2,
          h: r.height,
        };
      };
      return {
        rail: [...document.querySelectorAll('nav.rail button')].map(box),
        win: [...document.querySelectorAll('.titlebar .controls button')].map(box),
        brand: box(document.querySelector('.brand')),
        sw: document.documentElement.scrollWidth,
        sh: document.documentElement.scrollHeight,
        radius: getComputedStyle(document.querySelector('.shell')).borderTopLeftRadius,
        maxAttr: document.querySelector('.shell').dataset.maximized,
        vw: innerWidth,
        vh: innerHeight,
      };
    });
    check(m.rail.length === 7, `${tag}: 7 rail items`);
    check(
      m.rail.every((r) => r.top >= 0 && r.bottom <= m.vh),
      `${tag}: all rail items inside the viewport`,
    );
    check(m.sh <= m.vh && m.sw <= m.vw, `${tag}: no page scroll`);
    for (const wb of m.win)
      check(
        Math.abs(wb.cy - m.rail[0].cy) <= 0.5,
        `${tag}: window button centre ${wb.cy} == first rail item centre ${m.rail[0].cy}`,
      );
    check(
      Math.abs(m.brand.cy - m.rail[0].cy) <= 0.5,
      `${tag}: logo centre ${m.brand.cy} == rail centre`,
    );
    check(
      m.rail.every((r) => r.h % 8 === 0),
      `${tag}: rail item height on the 8px grid (${m.rail[0].h})`,
    );
    const pitches = m.rail.slice(1).map((r, i) => r.top - m.rail[i].bottom);
    check(
      pitches.every((p) => p % 8 === 0),
      `${tag}: rail gaps on the 8px grid (${pitches[0]})`,
    );
    check(
      m.win.every((r) => r.h === 40),
      `${tag}: window buttons 40px`,
    );
    check(m.maxAttr === String(maximized), `${tag}: data-maximized=${m.maxAttr}`);
    check(maximized ? m.radius === '0px' : m.radius !== '0px', `${tag}: corner radius ${m.radius}`);

    const names = await page.$$eval('nav.rail button', (b) => b.length);
    for (let i = 0; i < names; i++) {
      await page.locator('nav.rail button').nth(i).click();
      await page.waitForTimeout(350);
      const s = await page.evaluate(() => ({
        sh: document.documentElement.scrollHeight,
        vh: innerHeight,
        sw: document.documentElement.scrollWidth,
        vw: innerWidth,
      }));
      check(s.sh <= s.vh && s.sw <= s.vw, `${tag}: section ${i} has no page scroll`);
      if (OUT)
        await page.screenshot({ path: `${OUT}/${w}x${h}${maximized ? '-max' : ''}-${i}.png` });
    }
    await page.close();
  }
}
await browser.close();
console.log(failed ? `${failed} checks FAILED` : 'all checks passed');
process.exit(failed ? 1 : 0);
