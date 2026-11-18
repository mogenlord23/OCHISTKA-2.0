import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium, openScene, timeline } from '../scenes/harness.mjs';

test('every scene renders, animates, shows title, loads images, no console errors', async () => {
  const browser = await chromium.launch();
  try {
    for (const sc of timeline) {
      const { page, errors, ctx } = await openScene(browser, sc.id);
      const dur = sc.end - sc.start;
      await page.evaluate(() => window.seek(0));
      const a = await page.screenshot();
      await page.evaluate((d) => window.seek(d * 0.6), dur);
      const b = await page.screenshot();
      assert.notDeepEqual(a, b, `${sc.id}: seek changes the picture`);
      const text = await page.evaluate(() => document.getElementById('title').textContent);
      assert.equal(text, sc.title, `${sc.id}: title`);
      const broken = await page.evaluate(() => [...document.images].filter((i) => !i.naturalWidth).length);
      assert.equal(broken, 0, `${sc.id}: broken images`);
      assert.deepEqual(errors, [], `${sc.id}: console errors`);
      await ctx.close();
    }
  } finally { await browser.close(); }
});
