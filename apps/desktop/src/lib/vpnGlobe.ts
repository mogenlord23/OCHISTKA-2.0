import map from '../assets/vpn-globe.json';
import { NEUTRAL, SECTION_COLORS } from './tokens';

const RAD = Math.PI / 180;
const TURN = Math.PI * 2;
const CLOSE_ZOOM = 4.25;
const DETAIL_ARC = 34;
const regions: Readonly<Record<string, readonly number[]>> = {
  EU: [13, 50], ASIA: [105, 35], AFRICA: [20, 3],
  NORTH_AMERICA: [-100, 40], SOUTH_AMERICA: [-58, -15],
  OCEANIA: [135, -25], ANTARCTICA: [30, -78], GLOBAL: [0, 15],
};
const countries: Readonly<Record<string, readonly number[]>> = map.countries;

export function globeCountry(code: string | null): readonly number[] | null {
  const key = code?.trim().toUpperCase();
  if (!key) return null;
  const country = countries[key === 'UK' ? 'GB' : key];
  if (country) return country;
  if (regions[key]) return regions[key];
  if (/EUROPE|ЕВРОП/.test(key)) return regions.EU;
  if (/ASIA|АЗИ/.test(key)) return regions.ASIA;
  if (/AFRICA|АФРИ/.test(key)) return regions.AFRICA;
  if (/NORTH.*AMERICA|СЕВЕРН.*АМЕРИК/.test(key)) return regions.NORTH_AMERICA;
  if (/SOUTH.*AMERICA|ЮЖН.*АМЕРИК/.test(key)) return regions.SOUTH_AMERICA;
  if (/OCEAN|ОКЕАНИ|AUSTRALIA|АВСТРАЛ/.test(key)) return regions.OCEANIA;
  if (/ANTARCTIC|АНТАРКТ/.test(key)) return regions.ANTARCTICA;
  return regions.GLOBAL;
}

function vector(longitude: number, latitude: number) {
  const latitudeRadius = Math.cos(latitude * RAD);
  return [
    latitudeRadius * Math.sin(longitude * RAD),
    Math.sin(latitude * RAD),
    latitudeRadius * Math.cos(longitude * RAD),
  ] as const;
}

type Dot = readonly [x: number, y: number, z: number, border: boolean, lime: boolean];
const palette = atob(map.lime);
const isLime = (index: number) => Boolean(palette.charCodeAt(index >> 3) & (1 << (index % 8)));
const packedDetail = atob(map.detail);
const land: Dot[] = [];
const gridStep = 360 / map.gridWidth;
// Все уровни используют одну сетку. При приближении добавляются точки между существующими.
for (let row = 0; row < map.gridHeight; row += 8) {
  for (let column = 0; column < map.gridWidth; column += 8) {
    const index = row * map.gridWidth + column;
    const kind = (packedDetail.charCodeAt(index >> 2) >> (2 * (index & 3))) & 3;
    if (kind) land.push([...vector((column + 0.5) * gridStep - 180, (row + 0.5) * gridStep - 90), kind === 2, isLime(index)]);
  }
}

export function globeMarkerColor(target: readonly number[]) {
  const column = Math.min(map.gridWidth - 1, Math.max(0, Math.floor((target[0] + 180) / gridStep)));
  const row = Math.min(map.gridHeight - 1, Math.max(0, Math.floor((target[1] + 90) / gridStep)));
  return isLime(row * map.gridWidth + column) ? SECTION_COLORS.vpn : SECTION_COLORS.viruses;
}

export function globeDotSize(width: number, height: number) {
  return Math.max(width * 0.515, height * 1.08) * 0.0115;
}

const meridians = Array.from({ length: map.gridWidth }, (_, column) => {
  const radians = ((column + 0.5) * 360 / map.gridWidth - 180) * RAD;
  return [Math.sin(radians), Math.cos(radians)] as const;
});

function detailNear([longitude, latitude]: readonly number[]): [Dot[], Dot[]] {
  const packed = packedDetail;
  const { gridWidth, gridHeight } = map;
  const step = 360 / gridWidth;
  const region: [Dot[], Dot[]] = [[], []];
  const minRow = Math.max(0, Math.floor((latitude + 90 - DETAIL_ARC) / step));
  const maxRow = Math.min(gridHeight - 1, Math.ceil((latitude + 90 + DETAIL_ARC) / step));
  const [targetX, targetY, targetZ] = vector(longitude, latitude);
  const threshold = Math.cos(DETAIL_ARC * RAD);

  for (let row = minRow; row <= maxRow; row++) {
    const y = (row + 0.5) * step - 90;
    const parallel = Math.cos(y * RAD);
    const north = Math.sin(y * RAD);
    for (let column = 0; column < gridWidth; column++) {
      const index = row * gridWidth + column;
      const kind = (packed.charCodeAt(index >> 2) >> (2 * (index & 3))) & 3;
      if (!kind || (row % 8 === 0 && column % 8 === 0)) continue;
      const [sin, cos] = meridians[column];
      const x = parallel * sin;
      const z = parallel * cos;
      if (x * targetX + north * targetY + z * targetZ >= threshold) {
        region[row % 2 === 0 && column % 2 === 0 ? 0 : 1].push([x, north, z, kind === 2, isLime(index)]);
      }
    }
  }
  return region;
}

const detailCache = new Map<string, [Dot[], Dot[]]>();
function cachedDetail(code: string, target: readonly number[]): [Dot[], Dot[]] {
  let detail = detailCache.get(code);
  if (!detail) {
    detail = detailNear(target);
    detailCache.set(code, detail);
  }
  return detail;
}

export function shortestTurn(from: number, to: number): number {
  return Math.atan2(Math.sin(to - from), Math.cos(to - from));
}

export function globeViewport(width: number, height: number, zoom: number) {
  const near = (zoom - 1) / (CLOSE_ZOOM - 1);
  const restRadius = Math.max(width * 0.515, height * 1.08);
  const closeRadius = Math.max(restRadius * CLOSE_ZOOM, width * 1.05);
  return {
    x: width / 2,
    y: (restRadius + height * 0.035) * (1 - near) + height * 0.5 * near,
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
  sprite.width = DOT_PIXELS * 6;
  sprite.height = DOT_PIXELS;
  const ctx = sprite.getContext('2d');
  if (!ctx) return sprite;
  for (const [i, color] of [
    '#87305e', '#d6198d', SECTION_COLORS.vpn,
    '#586d26', '#99c72c', SECTION_COLORS.viruses,
  ].entries()) {
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
  let frameTimer = 0;
  let previous = 0;
  let width = 0;
  let height = 0;
  let longitude = 0.26;
  let latitude = 0.38;
  let zoom = 1;
  let focus: readonly number[] | null = null;
  let destination: readonly number[] | null = null;
  let departure = { longitude, latitude, zoom };
  let progress = 1;
  let detail: [Dot[], Dot[]] = [[], []];
  let cachedMap: HTMLCanvasElement | undefined;
  let cachedView = '';

  function draw(now: number) {
    frame = 0;
    if (width <= 0 || height <= 0) {
      previous = 0;
      return;
    }
    const reduced = motion.matches || document.documentElement.dataset.reduceMotion === 'true';
    const elapsed = previous ? Math.min((now - previous) / 1000, 0.25) : 1 / 60;
    previous = now;
    const { country, connected } = state();
    const target = globeCountry(country);
    if (target && target !== focus) {
      focus = target;
      detail = cachedDetail(country!, target);
    }
    if (destination !== target) {
      destination = target;
      departure = { longitude, latitude, zoom };
      progress = 0;
    }
    progress = reduced ? 1 : Math.min(1, progress + Math.min(elapsed, 0.05) / 1.85);
    const t = progress * progress * (3 - 2 * progress);
    if (target) {
      longitude = departure.longitude + shortestTurn(departure.longitude, target[0] * RAD) * t;
    } else if (!reduced) {
      longitude = (longitude + elapsed * 0.16 * t) % TURN;
    }
    latitude = departure.latitude + ((target ? target[1] * RAD : 0.38) - departure.latitude) * t;
    zoom = departure.zoom + ((target ? CLOSE_ZOOM : 1) - departure.zoom) * t;

    const [bufferWidth, bufferHeight] = globeBufferSize(width, height, window.devicePixelRatio || 1);
    if (canvas.width !== bufferWidth) canvas.width = bufferWidth;
    if (canvas.height !== bufferHeight) canvas.height = bufferHeight;
    ctx.setTransform(bufferWidth / width, 0, 0, bufferHeight / height, 0, 0);

    const markerScale = Math.max(1, width / 534);
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

    const settled = target && Math.abs(CLOSE_ZOOM - zoom) < 0.0001 &&
      Math.abs(latitude - target[1] * RAD) < 0.00001 &&
      Math.abs(shortestTurn(longitude, target[0] * RAD)) < 0.00001;
    const view = settled ? [country, connected, width, height, bufferWidth, bufferHeight].join('/') : '';
    if (view && view === cachedView && cachedMap) {
      ctx.drawImage(cachedMap, 0, 0, width, height);
    } else {
      ctx.fillStyle = NEUTRAL.surfaceAlt;
      ctx.fillRect(0, 0, width, height);
      ctx.fillStyle = '#000000';
      ctx.beginPath();
      ctx.arc(centerX, centerY, radius, 0, TURN);
      ctx.fill();
      ctx.strokeStyle = '#303030';
      ctx.lineWidth = 1;
      ctx.stroke();

      const smooth = (value: number) => {
        const t = Math.max(0, Math.min(1, value));
        return t * t * (3 - 2 * t);
      };
      const paint = (dots: readonly Dot[], size: number, alpha: number) => {
        if (alpha <= 0) return;
        ctx.globalAlpha = alpha;
        const half = size / 2;
        for (const [x, y, z, border, lime] of dots) {
          const meridian = x * sx + z * cx;
          const front = y * sy + meridian * cy;
          if (front <= 0) continue;
          const left = centerX + (x * cx - z * sx) * radius;
          const top = centerY - (y * cy - meridian * sy) * radius;
          if (left < -half || left > width + half || top < -half || top > height + half) continue;
          if (marker && Math.abs(left - marker.x) < 10 * markerScale + half && Math.abs(top - marker.y) < 10 * markerScale + half) continue;
          const rank = (lime ? 3 : 0) + (border || front > 0.85 ? 2 : front < 0.46 ? 0 : 1);
          ctx.drawImage(sprites, rank * DOT_PIXELS, 0, DOT_PIXELS, DOT_PIXELS, left - half, top - half, size, size);
        }
      };
      const size = globeDotSize(width, height);
      paint(land, size, 1);
      paint(detail[0], size, smooth((zoom - 1.7) / 1.6));
      paint(detail[1], size, smooth((zoom - 3.5) / 0.75));
      ctx.globalAlpha = 1;

      cachedView = '';
      if (view) {
        cachedMap ??= document.createElement('canvas');
        cachedMap.width = bufferWidth;
        cachedMap.height = bufferHeight;
        const cache = cachedMap.getContext('2d');
        if (cache) {
          cache.drawImage(canvas, 0, 0);
          cachedView = view;
        }
      }
    }

    if (marker) {
      const { x, y } = marker;
      ctx.fillStyle = globeMarkerColor(target!);
      ctx.strokeStyle = ctx.fillStyle;
      const phase = (now % 2050) / 2050;
      for (let ring = 0; !reduced && ring < 3; ring++) {
        const pulse = (phase + ring / 3) % 1;
        const size = (16 + pulse * 42) * markerScale;
        ctx.globalAlpha = Math.sin(pulse * Math.PI) * (1 - pulse) * 0.7;
        ctx.beginPath();
        ctx.roundRect(x - size / 2, y - size / 2, size, size, size / 4);
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
      ctx.beginPath();
      ctx.roundRect(x - 5 * markerScale, y - 5 * markerScale, 10 * markerScale, 10 * markerScale, 2.5 * markerScale);
      ctx.fill();
    }

    if (!reduced) invalidate();
  }

  function invalidate() {
    if (!frame && !frameTimer && width > 0 && height > 0 &&
      document.visibilityState !== 'hidden' && document.hasFocus()) {
      frameTimer = window.setTimeout(() => {
        frameTimer = 0;
        frame = requestAnimationFrame(draw);
      }, destination ? 40 : 80);
    }
  }

  function pause() {
    clearTimeout(frameTimer);
    frameTimer = 0;
    cancelAnimationFrame(frame);
    frame = 0;
    previous = 0;
  }

  function syncVisibility() {
    if (document.visibilityState === 'hidden' || !document.hasFocus()) pause();
    else invalidate();
  }

  const observer = new ResizeObserver(([entry]) => {
    ({ width, height } = entry.contentRect);
    invalidate();
  });
  observer.observe(canvas);
  motion.addEventListener('change', invalidate);
  document.addEventListener('visibilitychange', syncVisibility);
  window.addEventListener('blur', syncVisibility);
  window.addEventListener('focus', syncVisibility);
  return {
    invalidate,
    destroy() {
      pause();
      observer.disconnect();
      motion.removeEventListener('change', invalidate);
      document.removeEventListener('visibilitychange', syncVisibility);
      window.removeEventListener('blur', syncVisibility);
      window.removeEventListener('focus', syncVisibility);
    },
  };
}
