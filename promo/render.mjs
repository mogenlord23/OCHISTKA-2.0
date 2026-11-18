import fs from 'node:fs';
import path from 'node:path';
import { chromium, openScene, timeline } from './scenes/harness.mjs';

// Рендерит кадры PNG по таймлайну. Возвращает число кадров.
export async function renderFrames(outDir, fps = 30) {
  fs.mkdirSync(outDir, { recursive: true });
  const browser = await chromium.launch();
  let n = 0;
  try {
    for (const sc of timeline) {
      const { page, ctx } = await openScene(browser, sc.id);
      const count = Math.round((sc.end - sc.start) * fps);
      for (let i = 0; i < count; i++) {
        await page.evaluate((t) => window.seek(t), i / fps);
        await page.screenshot({ path: path.join(outDir, `frame_${String(n).padStart(5, '0')}.png`) });
        n++;
      }
      await ctx.close();
    }
  } finally {
    await browser.close();
  }
  return n;
}

if (process.argv[1] && import.meta.url.endsWith(path.basename(process.argv[1]))) {
  const n = await renderFrames(process.argv[2], 30);
  console.log(`frames: ${n}`);
}
