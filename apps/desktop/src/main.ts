import { mount } from 'svelte';
import App from './App.svelte';
import { applyReduceMotion, loadSettings } from './lib/appearanceStore';

applyReduceMotion(loadSettings().reduceMotion);

const target = document.getElementById('app');
if (!target) {
  throw new Error('В index.html нет контейнера #app');
}

mount(App, { target });
