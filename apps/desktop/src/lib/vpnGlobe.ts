import map from '../assets/vpn-globe.json';
import { SECTION_COLORS } from './tokens';

const RAD = Math.PI / 180;
const TURN = Math.PI * 2;
const countries: Readonly<Record<string, readonly number[]>> = map.countries;

export function globeCountry(code: string | null): readonly number[] | null {
  const key = code?.trim().toUpperCase();
  return key ? countries[key === 'UK' ? 'GB' : key] ?? null : null;
}

function vector(longitude: number, latitude: number) {
  const latitudeRadius = Math.cos(latitude * RAD);
  return [latitudeRadius * Math.sin(longitude * RAD), Math.sin(latitude * RAD),
    latitudeRadius * Math.cos(longitude * RAD)] as const;
}

const land = map.land.map(([longitude, latitude]) => vector(longitude, latitude));

export function shortestTurn(from: number, to: number): number {
  return Math.atan2(Math.sin(to - from), Math.cos(to - from));
}

type GlobeState = { country: string | null; connected: boolean };

export function mountVpnGlobe(canvas: HTMLCanvasElement, state: () => GlobeState) {
  const context = canvas.getContext('2d');
  if (!context) return;
  const ctx = context;
  const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
  let frame = 0;
  let previous = 0;
  let width = 0;
  let height = 0;
  let ratio = 1;
  let longitude = 0.26;
  let latitude = 0.38;
  let zoom = 1;

  function draw(now: number) {
    frame = 0;
    const reduced = motion.matches || document.documentElement.dataset.reduceMotion === 'true';
    const elapsed = previous ? Math.min((now - previous) / 1000, 0.05) : 1 / 60;
    previous = now;
    const { country, connected } = state();
    const target = globeCountry(country);
    const ease = reduced ? 1 : 1 - Math.exp(-7 * elapsed);
    longitude += target ? shortestTurn(longitude, target[0] * RAD) * ease : reduced ? 0 : elapsed * 0.11;
    longitude %= TURN;
    latitude += ((target ? target[1] * RAD : 0.38) - latitude) * ease;
    zoom += ((target ? 3.6 : 1) - zoom) * ease;

    const centerX = width / 2;
    const centerY = height / 2;
    const radius = Math.min(width, height) * 0.44 * zoom;
    const sx = Math.sin(longitude);
    const cx = Math.cos(longitude);
    const sy = Math.sin(latitude);
    const cy = Math.cos(latitude);
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    ctx.clearRect(0, 0, width, height);
    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, 0, TURN);
    ctx.fillStyle = '#181820';
    ctx.fill();
    ctx.strokeStyle = '#37313e';
    ctx.lineWidth = 1;
    ctx.stroke();

    const dots = [new Path2D(), new Path2D(), new Path2D()];
    const pointSize = Math.max(1.15, radius * 0.013);
    for (const [x, y, z] of land) {
      const meridian = x * sx + z * cx;
      const front = y * sy + meridian * cy;
      if (front <= 0) continue;
      const left = centerX + (x * cx - z * sx) * radius;
      const top = centerY - (y * cy - meridian * sy) * radius;
      if (left < -pointSize || left > width + pointSize || top < -pointSize || top > height + pointSize) continue;
      const size = pointSize * (0.65 + 0.35 * front);
      dots[front < 0.36 ? 0 : front < 0.72 ? 1 : 2].roundRect(left - size / 2, top - size / 2, size, size, size / 4);
    }
    for (const [index, color] of ['#443c48', '#716574', '#a395a2'].entries()) {
      ctx.fillStyle = color;
      ctx.fill(dots[index]);
    }

    if (target && connected) {
      const [x, y, z] = vector(target[0], target[1]);
      const meridian = x * sx + z * cx;
      if (y * sy + meridian * cy > 0) {
        const left = centerX + (x * cx - z * sx) * radius;
        const top = centerY - (y * cy - meridian * sy) * radius;
        ctx.fillStyle = SECTION_COLORS.vpn;
        ctx.strokeStyle = SECTION_COLORS.vpn;
        const phase = (now % 1600) / 1600;
        for (let ring = 0; !reduced && ring < 3; ring++) {
          const pulse = (phase + ring / 3) % 1;
          const size = 11 + pulse * 53;
          ctx.globalAlpha = (1 - pulse) * 0.64;
          ctx.beginPath();
          ctx.roundRect(left - size / 2, top - size / 2, size, size, size / 4);
          ctx.stroke();
        }
        ctx.globalAlpha = 1;
        ctx.beginPath();
        ctx.roundRect(left - 5, top - 5, 10, 10, 2.5);
        ctx.fill();
      }
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
    ratio = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
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
