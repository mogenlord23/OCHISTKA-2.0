import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { renderFrames } from '../render.mjs';

test('renderFrames writes fps*30 frames of 1080x1920', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'frames-'));
  const n = await renderFrames(dir, 2);
  assert.equal(n, 60);
  const files = fs.readdirSync(dir).sort();
  assert.equal(files.length, 60);
  assert.equal(files[0], 'frame_00000.png');
  const buf = fs.readFileSync(path.join(dir, files[0]));
  assert.equal(buf.readUInt32BE(16), 1080);
  assert.equal(buf.readUInt32BE(20), 1920);
  assert.notDeepEqual(fs.readFileSync(path.join(dir, files[7])), fs.readFileSync(path.join(dir, files[8])));
});
