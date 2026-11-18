import { createRequire } from 'node:module';
const { chromium } = createRequire(import.meta.url)('playwright');
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
export const timeline = JSON.parse(fs.readFileSync(path.join(here, '..', 'timeline.json'), 'utf8'));
export async function openScene(browser, id, scale = 1) {
  const ctx = await browser.newContext({ viewport: { width: 1080, height: 1920 }, deviceScaleFactor: scale });
  const page = await ctx.newPage();
  const errors = [];
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.addInitScript((tl) => { window.TIMELINE = tl; }, timeline);
  await page.goto('file://' + path.join(here, 'scene.html') + '?scene=' + id);
  await page.evaluate(() => window.sceneReady);
  return { page, errors, ctx };
}
export { chromium };
