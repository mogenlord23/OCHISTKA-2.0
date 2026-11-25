import map from '../assets/vpn-globe.json';
import { SECTION_COLORS } from './tokens';

const RAD = Math.PI / 180;
const TURN = Math.PI * 2;
const CLOSE_ZOOM = 4.25;
const DETAIL_ARC = 34;
const countries: Readonly<Record<string, readonly number[]>> = map.countries;

export function globeCountry(code: string | null): readonly number[] | null {
  const key = code?.trim().toUpperCase();
  return key ? countries[key === 'UK' ? 'GB' : key] ?? null : null;
}

function vector(longitude: number, latitude: number) {
  const latitudeRadius = Math.cos(latitude * RAD);
  return [
    latitudeRadius * Math.sin(longitude * RAD),
    Math.sin(latitude * RAD),
    latitudeRadius * Math.cos(longitude * RAD),
  ] as const;
}

type Dot = readonly [x: number, y: number, z: number, border: boolean];
const land: Dot[] = map.land.map(([longitude, latitude]) => [...vector(longitude, latitude), false]);
let packedDetail: string | undefined;

function detailNear([longitude, latitude]: readonly number[]): Dot[] {
  const packed = packedDetail ??= atob(map.detail);
  const { gridWidth, gridHeight } = map;
  const step = 360 / gridWidth;
  const region: Dot[] = [];
  const minRow = Math.max(0, Math.floor((latitude + 90 - DETAIL_ARC) / step));
  const maxRow = Math.min(gridHeight - 1, Math.ceil((latitude + 90 + DETAIL_ARC) / step));
  const [targetX, targetY, targetZ] = vector(longitude, latitude);
  const threshold = Math.cos(DETAIL_ARC * RAD);

  for (let row = minRow; row <= maxRow; row++) {
    const y = (row + 0.5) * step - 90;
    for (let column = 0; column < gridWidth; column++) {
      const index = row * gridWidth + column;
      const kind = (packed.charCodeAt(index >> 2) >> (2 * (index & 3))) & 3;
      if (!kind) continue;
      const dot = vector((column + 0.5) * step - 180, y);
      if (dot[0] * targetX + dot[1] * targetY + dot[2] * targetZ >= threshold) {
        region.push([...dot, kind === 2]);
      }
    }
  }
  return region;
}

export function shortestTurn(from: number, to: number): number {
  return Math.atan2(Math.sin(to - from), Math.cos(to - from));
}

export function globeViewport(width: number, height: number, zoom: number) {
  const near = (zoom - 1) / (CLOSE_ZOOM - 1);
  const restRadius = Math.min(width * 0.47, height * 0.94);
  const closeRadius = Math.max(restRadius * CLOSE_ZOOM, width * 1.05);
  return {
    x: width / 2,
    y: height * (0.98 - 0.48 * near),
    radius: restRadius + (closeRadius - restRadius) * near,
  };
}

/** Крупные шаги размера позволяют не пересоздавать буфер на каждый пиксель при растягивании. */
export function globeBufferSize(width: number, height: number, deviceRatio: number) {
  const scale = Math.min(deviceRatio, 1.5, 1024 / width, 512 / height);
  const bucket = (size: number) => Math.ceil(size / 64) * 64;
  return [bucket(width * scale), bucket(height * scale)] as const;
}

const DOT_PIXELS = 12;
let dotSprites: HTMLCanvasElement | undefined;

function makeDots() {
  const sprite = document.createElement('canvas');
  sprite.width = DOT_PIXELS * 3;
  sprite.height = DOT_PIXELS;
  const ctx = sprite.getContext('2d');
  if (!ctx) return sprite;
  for (const [i, color] of ['#743955', '#ad557f', '#ff87bb'].entries()) {
    ctx.beginPath();
    ctx.roundRect(i * DOT_PIXELS, 0, DOT_PIXELS, DOT_PIXELS, 3);
    ctx.fillStyle = color;
    ctx.fill();
  }
  return sprite;
}

type GlobeState = { country: string | null; connected: boolean };

export function mountVpnGlobe(canvas: HTMLCanvasElement, state: () => GlobeState) {
  const context = canvas.getContext('2d', { alpha: false });
  if (!context) return;
  const ctx = context;
  const sprites = dotSprites ??= makeDots();
  const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
  let frame = 0;
  let previous = 0;
  let width = 0;
  let height = 0;
  let longitude = 0.26;
  let latitude = 0.38;
  let zoom = 1;
  let focus: readonly number[] | null = null;
  let detail: Dot[] = [];

  function draw(now: number) {
    frame = 0;
    const reduced = motion.matches || document.documentElement.dataset.reduceMotion === 'true';
    const elapsed = previous ? Math.min((now - previous) / 1000, 0.25) : 1 / 60;
    previous = now;
    const { country, connected } = state();
    const target = globeCountry(country);
    if (target && target !== focus) {
      focus = target;
      detail = detailNear(target);
    }
    const ease = reduced ? 1 : 1 - Math.exp(-6 * elapsed);
    longitude += target ? shortestTurn(longitude, target[0] * RAD) * ease : reduced ? 0 : elapsed * 0.16;
    longitude %= TURN;
    latitude += ((target ? target[1] * RAD : 0.38) - latitude) * ease;
    zoom += ((target ? CLOSE_ZOOM : 1) - zoom) * ease;

    const [bufferWidth, bufferHeight] = globeBufferSize(width, height, window.devicePixelRatio || 1);
    if (canvas.width !== bufferWidth) canvas.width = bufferWidth;
    if (canvas.height !== bufferHeight) canvas.height = bufferHeight;
    ctx.setTransform(bufferWidth / width, 0, 0, bufferHeight / height, 0, 0);

    const { x: centerX, y: centerY, radius } = globeViewport(width, height, zoom);
    const sx = Math.sin(longitude);
    const cx = Math.cos(longitude);
    const sy = Math.sin(latitude);
    const cy = Math.cos(latitude);
    let marker: { x: number; y: number } | null = null;
    if (target && connected) {
      const [x, y, z] = vector(target[0], target[1]);
      const meridian = x * sx + z * cx;
      if (y * sy + meridian * cy > 0) {
        marker = {
          x: centerX + (x * cx - z * sx) * radius,
          y: centerY - (y * cy - meridian * sy) * radius,
        };
      }
    }

    ctx.fillStyle = '#000';
    ctx.fillRect(0, 0, width, height);
    const light = ctx.createLinearGradient(0, centerY - radius, 0, centerY + radius);
    light.addColorStop(0, '#2b0018');
    light.addColorStop(0.4, '#0c0308');
    light.addColorStop(1, '#000');
    ctx.fillStyle = light;
    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, 0, TURN);
    ctx.fill();
    ctx.strokeStyle = '#542038';
    ctx.lineWidth = 1;
    ctx.stroke();

    const targetOffset = focus
      ? Math.max(Math.abs(latitude - focus[1] * RAD), Math.abs(shortestTurn(longitude, focus[0] * RAD)))
      : Infinity;
    const detailMix = Math.max(0, Math.min(1, (zoom - 1.8) / 1.25, (0.54 - targetOffset) / 0.22));
    const paint = (dots: readonly Dot[], size: number, alpha: number) => {
      if (alpha <= 0) return;
      ctx.globalAlpha = alpha;
      const half = size / 2;
      for (const [x, y, z, border] of dots) {
        const meridian = x * sx + z * cx;
        const front = y * sy + meridian * cy;
        if (front <= 0) continue;
        const left = centerX + (x * cx - z * sx) * radius;
        const top = centerY - (y * cy - meridian * sy) * radius;
        if (left < -half || left > width + half || top < -half || top > height + half) continue;
        if (marker && Math.abs(left - marker.x) < 10 + half && Math.abs(top - marker.y) < 10 + half) continue;
        const rank = border ? 2 : front < 0.46 ? 0 : 1;
        ctx.drawImage(sprites, rank * DOT_PIXELS, 0, DOT_PIXELS, DOT_PIXELS, left - half, top - half, size, size);
      }
    };
    paint(land, Math.max(1.2, Math.min(3.5, radius * 0.011)), 1 - detailMix);
    paint(detail, Math.max(1.4, Math.min(4, radius * 0.0028)), detailMix);
    ctx.globalAlpha = 1;

    if (marker) {
      const { x, y } = marker;
      ctx.fillStyle = SECTION_COLORS.vpn;
      ctx.strokeStyle = SECTION_COLORS.vpn;
      const phase = (now % 2050) / 2050;
      for (let ring = 0; !reduced && ring < 3; ring++) {
        const pulse = (phase + ring / 3) % 1;
        const size = 16 + pulse * 42;
        ctx.globalAlpha = Math.sin(pulse * Math.PI) * (1 - pulse) * 0.7;
        ctx.beginPath();
        ctx.roundRect(x - size / 2, y - size / 2, size, size, size / 4);
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
      ctx.beginPath();
      ctx.roundRect(x - 5, y - 5, 10, 10, 2.5);
      ctx.fill();
    }

    if (!reduced) invalidate();
  }

  function invalidate() {
    if (!frame && width && height && document.visibilityState !== 'hidden') {
      frame = requestAnimationFrame(draw);
    }
  }

  function pause() {
    cancelAnimationFrame(frame);
    frame = 0;
    previous = 0;
  }

  function syncVisibility() {
    if (document.visibilityState === 'hidden') pause();
    else invalidate();
  }

  const observer = new ResizeObserver(([entry]) => {
    ({ width, height } = entry.contentRect);
    invalidate();
  });
  observer.observe(canvas);
  motion.addEventListener('change', invalidate);
  document.addEventListener('visibilitychange', syncVisibility);
  return {
    invalidate,
    destroy() {
      pause();
      observer.disconnect();
      motion.removeEventListener('change', invalidate);
      document.removeEventListener('visibilitychange', syncVisibility);
    },
  };
}
