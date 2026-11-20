import { mount } from 'svelte';
import App from './App.svelte';
import { applyReduceMotion } from './lib/appearanceStore';

// Уменьшенное движение берём только из настроек системы (prefers-reduced-motion).
try {
  applyReduceMotion(window.matchMedia('(prefers-reduced-motion: reduce)').matches);
} catch {
  /* нет matchMedia: анимации остаются включёнными */
}

const target = document.getElementById('app');
if (!target) {
  throw new Error('В index.html нет контейнера #app');
}

mount(App, { target });
