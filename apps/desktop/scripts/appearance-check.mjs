// Проверка раздела «Оформление» в настоящем Chromium (Playwright, вызовы Tauri подменены).
// Запуск: `npx vite --port 1463 --strictPort`, затем `node scripts/appearance-check.mjs <папка снимков>`.
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
  [900, 544, 'min'],
  [1179, 803, '1179x803'],
  [2560, 1440, '2560x1440'],
];
const TABS = ['wallpapers', 'cursors', 'colors'];
const TAB_NAME = { wallpapers: 'Обои', cursors: 'Курсоры', colors: 'Цвета' };
let failed = 0;
const check = (ok, text) => {
  if (!ok) failed++;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`);
};

const browser = await chromium.launch();
for (const [w, h, label] of SIZES) {
  const page = await browser.newPage({ viewport: { width: w, height: h } });
  await page.addInitScript(() => {
    const win = {
      accent: '#FF009D',
      transparency: true,
      darkApps: true,
      darkSystem: false,
      fillTaskbar: false,
      hasBackup: false,
      hasPreviousWallpaper: false,
      wallpaperTile: null,
      accentTile: null,
      cursorScheme: null,
      originalsMissing: false,
    };
    window.__calls = [];
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { label: 'main', windowLabel: 'main' },
      },
      transformCallback: () => 1,
      invoke: async (cmd, args) => {
        if (cmd === 'plugin:window|is_maximized') return false;
        if (cmd.startsWith('plugin:event|')) return 1;
        if (cmd === 'appearance_request') {
          const r = args.request;
          window.__calls.push(r.action);
          if (r.action === 'applyWallpaperTile')
            Object.assign(win, { wallpaperTile: r.id, hasPreviousWallpaper: true });
          if (r.action === 'restoreWallpaper')
            Object.assign(win, { wallpaperTile: null, hasPreviousWallpaper: false });
          if (r.action === 'applyCursorScheme') win.cursorScheme = r.id;
          if (r.action === 'restoreCursorScheme') win.cursorScheme = null;
          if (r.action === 'setAccent')
            Object.assign(win, { accentTile: r.color, hasBackup: true });
          return { type: 'status', status: { ...win } };
        }
        return null;
      },
    };
  });
  await page.goto('http://127.0.0.1:1463/');
  await page.waitForTimeout(500);
  await page.locator('nav.rail button').nth(6).click();
  for (const tab of TABS) {
    await page.getByRole('button', { name: TAB_NAME[tab], exact: true }).first().click();
    await page.mouse.move(1, 1);
    await page.waitForTimeout(350);
    const tag = `${label} ${tab}`;
    const grid = `.grid-${tab === 'wallpapers' ? 'wallpaper' : tab === 'cursors' ? 'cursor' : 'color'}`;
    const tilesBefore = await page.evaluate((sel) => {
      const g = document.querySelector(sel);
      const gr = g.getBoundingClientRect();
      const body = document.querySelector('.page-body').getBoundingClientRect();
      const sizes = [...g.querySelectorAll('.tile')].map((t) => {
        const r = t.getBoundingClientRect();
        return [Math.round(r.width * 10) / 10, Math.round(r.height * 10) / 10];
      });
      const first = g.querySelector('.tile').getBoundingClientRect();
      const lastInRow = [...g.querySelectorAll('.tile')]
        .map((t) => t.getBoundingClientRect())
        .filter((r) => Math.abs(r.top - first.top) < 1)
        .pop();
      return {
        left: gr.left - body.left,
        right: body.right - lastInRow.right,
        sizes,
        sw: document.documentElement.scrollWidth,
        vw: innerWidth,
      };
    }, grid);
    check(
      tilesBefore.left <= 1 && tilesBefore.right <= 1.5,
      `${tag}: no side gutters (left ${tilesBefore.left.toFixed(1)}, right ${tilesBefore.right.toFixed(1)})`,
    );
    const widths = tilesBefore.sizes.map((s) => s[0]);
    check(
      Math.max(...widths) - Math.min(...widths) < 0.5,
      `${tag}: all tiles one size within 0.5 px (${Math.min(...widths)}..${Math.max(...widths)})`,
    );
    if (tab !== 'colors')
      check(tilesBefore.sizes[0][0] === tilesBefore.sizes[0][1], `${tag}: tiles are square`);
    check(tilesBefore.sw <= tilesBefore.vw, `${tag}: no horizontal scroll`);
    if (tab === 'cursors') {
      const svg = await page.evaluate(() => {
        const r = document.querySelector('.grid-cursor svg').getBoundingClientRect();
        return [r.width, r.height];
      });
      check(svg[0] === 32 && svg[1] === 32, `${tag}: cursor preview ${svg} px`);
    }
    if (tab === 'wallpapers') {
      const fit = await page.evaluate(
        () => getComputedStyle(document.querySelector('.grid-wallpaper img')).objectFit,
      );
      check(fit === 'cover', `${tag}: image object-fit ${fit}`);
    }
    // Первый клик применяет, второй ничего не делает, кнопка возвращает.
    const idx = tab === 'colors' ? 2 : tab === 'cursors' ? 5 : 11;
    const tile = page.locator(`${grid} .tile`).nth(idx);
    await page.evaluate(() => (window.__calls.length = 0));
    await tile.click();
    await page.waitForTimeout(150);
    await tile.click();
    await page.waitForTimeout(150);
    const calls = await page.evaluate(() => window.__calls.filter((c) => c !== 'status'));
    check(calls.length === 1, `${tag}: second click did nothing (${calls.join(',')})`);
    const outline = await page.evaluate(
      ({ sel, idx }) => {
        const t = document.querySelectorAll(`${sel} .tile`)[idx];
        const cs = getComputedStyle(t, '::after');
        const tr = getComputedStyle(t);
        return {
          border: cs.borderTopWidth,
          radius: cs.borderTopLeftRadius,
          tileRadius: tr.borderTopLeftRadius,
          inset: cs.top,
          pressed: t.getAttribute('aria-pressed'),
        };
      },
      { sel: grid, idx },
    );
    check(
      outline.pressed === 'true' &&
        outline.border === '3px' &&
        outline.radius === outline.tileRadius &&
        outline.inset === '0px',
      `${tag}: selected outline ${JSON.stringify(outline)}`,
    );
    await tile.hover();
    await page.mouse.move(1, 1);
    if (OUT) await page.screenshot({ path: `${OUT}/appearance-${tab}-${label}.png` });
    if (tab !== 'colors') {
      await page.getByRole('button', { name: 'Вернуть как было' }).click();
      await page.waitForTimeout(150);
      const after = await page.locator(`${grid} .tile`).nth(idx).getAttribute('aria-pressed');
      check(after === 'false', `${tag}: «Вернуть как было» clears the outline`);
    }
  }
  await page.close();
}
await browser.close();
console.log(failed ? `${failed} check(s) failed` : 'all checks passed');
process.exit(failed ? 1 : 0);
