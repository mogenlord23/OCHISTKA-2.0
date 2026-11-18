import { mount } from 'svelte';
import App from './App.svelte';
import {
  applyAppCursor,
  applyReduceMotion,
  loadSettings,
  storedCursorId,
} from './lib/appearanceStore';

applyReduceMotion(loadSettings().reduceMotion);
applyAppCursor(storedCursorId());

const target = document.getElementById('app');
if (!target) {
  throw new Error('В index.html нет контейнера #app');
}

mount(App, { target });
