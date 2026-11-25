import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import App from './App.svelte';
import TrayPanel from './components/TrayPanel.svelte';
import { applyReduceMotion } from './lib/appearanceStore';
import './geometry.css';

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

const tray = new URLSearchParams(window.location.search).get('view') === 'tray'
  || (isTauri() && getCurrentWindow().label === 'tray-panel');
mount(tray ? TrayPanel : App, { target });
