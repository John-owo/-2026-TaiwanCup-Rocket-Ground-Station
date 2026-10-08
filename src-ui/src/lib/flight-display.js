// Pure display helpers for the chart-style readouts. No Svelte, no DOM.

/** Six hypsometric bands, low to high. */
export const TINT_BANDS = 6;

// Ceilings divisible by the six bands so every band boundary is a round number.
const NICE_STEPS = [1.2, 1.8, 2.4, 3, 6, 9, 12];

/**
 * Round an altitude ceiling up to a value that divides cleanly into the tint bands.
 * @param {number} value
 * @param {number} [minimum]
 */
export function niceAltitudeCeiling(value, minimum = 60) {
  // minimum stays a multiple of the band count.
  const target = Math.max(minimum, Number.isFinite(value) ? value * 1.1 : minimum);
  const magnitude = 10 ** Math.floor(Math.log10(target));
  for (const step of NICE_STEPS) {
    const candidate = step * magnitude;
    if (candidate >= target) return candidate;
  }
  return 12 * magnitude;
}

/**
 * Position of an altitude on the ladder, 0 (ground) to 1 (ceiling), clamped.
 * @param {number} altitude
 * @param {number} ceiling
 */
export function ladderFraction(altitude, ceiling) {
  if (!Number.isFinite(altitude) || !(ceiling > 0)) return 0;
  return Math.min(1, Math.max(0, altitude / ceiling));
}

/**
 * Initial great-circle bearing in degrees (0 = north, clockwise).
 * @param {{ lat: number, lng: number }} from
 * @param {{ lat: number, lng: number }} to
 */
export function bearingDegrees(from, to) {
  const rad = Math.PI / 180;
  const fromLat = from.lat * rad;
  const toLat = to.lat * rad;
  const deltaLng = (to.lng - from.lng) * rad;
  const y = Math.sin(deltaLng) * Math.cos(toLat);
  const x = Math.cos(fromLat) * Math.sin(toLat) - Math.sin(fromLat) * Math.cos(toLat) * Math.cos(deltaLng);
  return ((Math.atan2(y, x) / rad) + 360) % 360;
}

const COMPASS_POINTS = ['北', '東北', '東', '東南', '南', '西南', '西', '西北'];

/** @param {number} degrees */
export function compassLabel(degrees) {
  if (!Number.isFinite(degrees)) return '--';
  return COMPASS_POINTS[Math.round((((degrees % 360) + 360) % 360) / 45) % 8];
}

/** @param {number} meters */
export function formatDistance(meters) {
  if (!Number.isFinite(meters)) return '--';
  return meters >= 1000 ? `${(meters / 1000).toFixed(2)} km` : `${Math.round(meters)} m`;
}

/**
 * Airborne uptime as m:ss.t
 * @param {number} uptimeMs
 */
export function formatUptime(uptimeMs) {
  if (!Number.isFinite(uptimeMs) || uptimeMs < 0) return '--';
  const totalTenths = Math.floor(uptimeMs / 100);
  const minutes = Math.floor(totalTenths / 600);
  const seconds = Math.floor((totalTenths % 600) / 10);
  const tenths = totalTenths % 10;
  return `${minutes}:${String(seconds).padStart(2, '0')}.${tenths}`;
}

/**
 * Keep the trailing run of samples that belongs to the latest airborne session and
 * whose uptime only moves forward, so a restart never folds the time axis back.
 * @template {{ sessionId: number, uptimeMs: number }} T
 * @param {T[]} samples
 * @returns {T[]}
 */
export function currentSessionRun(samples) {
  const latest = samples.at(-1);
  if (!latest) return [];
  let start = samples.length - 1;
  while (start > 0) {
    const previous = samples[start - 1];
    if (previous.sessionId !== latest.sessionId || previous.uptimeMs > samples[start].uptimeMs) break;
    start -= 1;
  }
  return samples.slice(start);
}
