// Геометрия новой навигации в настоящем Chromium (Playwright, вызовы Tauri подменены).
// Запуск: в apps/desktop `npx vite --port 1591 --strictPort`, затем
//   node docs/status/nav-redesign/geometry.mjs [папка снимков] [адрес]
// Проверяет на трёх размерах для всех 7 разделов: положение ряда вкладок, ползунок = выбранная вкладка,
// ползунок панели = выбранная плитка, ось слова OCHISTKA = ось кнопок окна, нет горизонтального
// переполнения, подписи вкладок не обрезаны, перетаскивание зажатой мышью и наведение.
import fs from 'node:fs';
import { chromium } from '/opt/node22/lib/node_modules/playwright/index.mjs';

const OUT = process.argv[2];
const URL = process.argv[3] || 'http://127.0.0.1:1591/';
if (OUT) fs.mkdirSync(OUT, { recursive: true });
const SIZES = [
  [900, 712],
  [1179, 803],
  [1920, 1080],
];
const SECTIONS = ['VPN', 'Вирусы', 'Процессы', 'Программы', 'Диски', 'Компьютер', 'Оформление'];
const SHOT = new Set(['VPN', 'Компьютер', 'Оформление']);
let failed = 0;
const check = (ok, text) => {
  if (!ok) failed++;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`);
};
const near = (a, b, d = 1) => Math.abs(a - b) <= d;

const mock = `try { localStorage.setItem('dockCollapsed', '1'); } catch {} window.__TAURI_INTERNALS__ = { metadata:{currentWindow:{label:'main'},currentWebview:{label:'main',windowLabel:'main'}}, transformCallback(){return 1}, invoke: async (cmd) => (cmd.startsWith('plugin:') ? (cmd === 'plugin:window|is_maximized' ? false : 1) : null) };`;

const browser = await chromium.launch({
  executablePath: '/opt/pw-browsers/chromium',
  args: ['--no-sandbox'],
});
for (const [w, h] of SIZES) {
  const page = await browser.newPage({ viewport: { width: w, height: h } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.addInitScript(mock);
  await page.goto(URL);
  await page.waitForTimeout(500);
  for (const name of SECTIONS) {
    await page.locator('.rail .item', { has: page.locator(`.label:text-is("${name}")`) }).click();
    await page.waitForTimeout(450);
    const m = await page.evaluate(() => {
      const r = (e) => {
        if (!e) return null;
        const b = e.getBoundingClientRect();
        return {
          l: b.left,
          t: b.top,
          w: b.width,
          h: b.height,
          cx: b.left + b.width / 2,
          cy: b.top + b.height / 2,
          b: b.bottom,
        };
      };
      const track = document.querySelector('.subtabs .track');
      const shared = track && getComputedStyle(track.closest('.subtabs')).display !== 'none';
      const tab = track?.querySelector('[aria-selected="true"]');
      const tile = document.querySelector('.rail .item[aria-current="page"]');
      const main = document.querySelector('main');
      const tabs = track ? [...track.querySelectorAll('[role="tab"]')] : [];
      return {
        shared,
        track: shared ? r(track) : null,
        tab: shared ? r(tab) : null,
        ind: shared ? r(track.querySelector('.ind')) : null,
        tabCount: tabs.length,
        truncated: shared
          ? tabs.filter((b) => b.scrollWidth > b.clientWidth + 1).map((b) => b.textContent.trim())
          : [],
        tile: r(tile),
        railInd: r(document.querySelector('.rail .ind')),
        brand: r(document.querySelector('.titlebar .brand strong')),
        buttons: r(document.querySelector('.titlebar .controls')),
        header: r(document.querySelector('.titlebar')),
        docOverflow: document.documentElement.scrollWidth - window.innerWidth,
        mainOverflow: main.scrollWidth - main.clientWidth,
        mainScrolls: main.scrollHeight > main.clientHeight + 1,
        footer: r(document.querySelector('footer')),
        h1: document.querySelector('main h1')?.textContent,
        created: /Create|Slava Lavs/.test(document.querySelector('.titlebar').textContent),
      };
    });
    const tag = `${w}x${h} ${name}`;
    check(!m.created, `${tag}: строки «Create» нет`);
    check(
      near(m.brand.cy, m.buttons.cy),
      `${tag}: ось OCHISTKA ${m.brand.cy.toFixed(1)} = ось кнопок ${m.buttons.cy.toFixed(1)}`,
    );
    check(
      m.docOverflow <= 0 && m.mainOverflow <= 0,
      `${tag}: нет горизонтального переполнения (${m.docOverflow}/${m.mainOverflow})`,
    );
    check(
      m.mainScrolls || m.footer.b <= h + 0.5,
      `${tag}: подвал внутри окна или страница прокручивается (${m.footer.b.toFixed(0)} / ${h}, прокрутка ${m.mainScrolls})`,
    );
    check(
      near(m.railInd.l, m.tile.l) &&
        near(m.railInd.t, m.tile.t) &&
        near(m.railInd.w, 88) &&
        near(m.railInd.h, 88),
      `${tag}: ползунок панели = плитка (${m.railInd.t.toFixed(1)} / ${m.tile.t.toFixed(1)})`,
    );
    if (m.shared) {
      check(
        m.track.t >= m.header.b && m.track.t - m.header.b <= 12,
        `${tag}: ряд вкладок сразу под шапкой (отступ ${(m.track.t - m.header.b).toFixed(1)})`,
      );
      check(
        near(m.ind.l, m.tab.l) &&
          near(m.ind.t, m.tab.t) &&
          near(m.ind.w, m.tab.w) &&
          near(m.ind.h, m.tab.h),
        `${tag}: ползунок = вкладка (${m.ind.l.toFixed(1)}+${m.ind.w.toFixed(1)} / ${m.tab.l.toFixed(1)}+${m.tab.w.toFixed(1)})`,
      );
      check(
        m.truncated.length === 0,
        `${tag}: подписи вкладок не обрезаны ${m.truncated.join(',')}`,
      );
    } else {
      console.log(`info ${tag}: общая панель вкладок скрыта, у VPN своя`);
    }
    if (OUT && SHOT.has(name)) await page.screenshot({ path: `${OUT}/${name}-${w}x${h}.png` });
  }

  // Перетаскивание и наведение мышью (общая панель вкладок раздела «Компьютер», правая панель).
  await page.locator('.rail .item', { has: page.locator('.label:text-is("Компьютер")') }).click();
  await page.waitForTimeout(400);
  const tabs = page.locator('.subtabs [role="tab"]');
  const n = await tabs.count();
  const boxes = [];
  for (let i = 0; i < n; i++) boxes.push(await tabs.nth(i).boundingBox());
  const sel = () =>
    page.evaluate(() =>
      [...document.querySelectorAll('.subtabs [role="tab"]')].findIndex(
        (b) => b.getAttribute('aria-selected') === 'true',
      ),
    );
  const cy = boxes[0].y + boxes[0].height / 2;
  await page.mouse.move(boxes[0].x + 10, cy);
  for (let i = 1; i < n; i++) await page.mouse.move(boxes[i].x + 10, cy, { steps: 4 });
  check((await sel()) === 0, `${w}x${h}: наведение без кнопки ничего не переключает`);
  await page.mouse.move(boxes[0].x + 10, cy);
  await page.mouse.down();
  check((await sel()) === 0, `${w}x${h}: нажатие на первую вкладку`);
  for (let i = 1; i < n; i++)
    await page.mouse.move(boxes[i].x + boxes[i].width / 2, cy, { steps: 6 });
  check((await sel()) === n - 1, `${w}x${h}: перетаскивание по вкладкам доводит до последней`);
  await page.mouse.move(boxes[0].x + 10, cy - 200, { steps: 4 });
  check((await sel()) === n - 1, `${w}x${h}: выход за ряд вкладок не переключает`);
  await page.mouse.up();
  check(
    (await page.evaluate(() => document.body.classList.contains('dragging'))) === false,
    `${w}x${h}: после отпускания выделение текста снова разрешено`,
  );

  const tiles = page.locator('.rail .item');
  const tb = [];
  for (let i = 0; i < 7; i++) tb.push(await tiles.nth(i).boundingBox());
  const sec = () =>
    page.evaluate(() =>
      [...document.querySelectorAll('.rail .item')].findIndex(
        (b) => b.getAttribute('aria-current') === 'page',
      ),
    );
  const x = tb[0].x + 44;
  await page.mouse.move(x, tb[0].y + 44);
  await page.mouse.down();
  for (let i = 1; i < 7; i++) await page.mouse.move(x, tb[i].y + 44, { steps: 5 });
  check((await sec()) === 6, `${w}x${h}: перетаскивание по правой панели вниз до «Оформление»`);
  for (let i = 5; i >= 3; i--) await page.mouse.move(x, tb[i].y + 44, { steps: 5 });
  check((await sec()) === 3, `${w}x${h}: и обратно вверх до «Программы»`);
  await page.mouse.up();
  await page.mouse.move(x, tb[0].y + 44, { steps: 5 });
  check((await sec()) === 3, `${w}x${h}: наведение на правую панель без кнопки не переключает`);
  check(errors.length === 0, `${w}x${h}: ошибок страницы нет ${errors.slice(0, 2).join(' | ')}`);
  await page.close();
}
await browser.close();
console.log(failed ? `FAILED: ${failed}` : 'ALL OK');
process.exit(failed ? 1 : 0);
