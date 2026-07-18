const EARTH_RADIUS_METERS = 6_371_000;
const DEFAULT_MIN_DISTANCE_METERS = 2;
const DEFAULT_MAX_POINTS = 5_000;

/** @typedef {{ lat: number, lng: number }} GpsPosition */

/** @param {unknown} position @returns {position is GpsPosition} */
export function isValidGpsPosition(position) {
  if (!position || typeof position !== 'object') return false;
  const candidate = /** @type {Record<string, unknown>} */ (position);
  if (!Number.isFinite(candidate.lat) || !Number.isFinite(candidate.lng)) return false;
  const lat = /** @type {number} */ (candidate.lat);
  const lng = /** @type {number} */ (candidate.lng);
  if (lat === 0 && lng === 0) return false;
  return lat >= -90 && lat <= 90 && lng >= -180 && lng <= 180;
}

/** @param {number} degrees */
function toRadians(degrees) {
  return degrees * Math.PI / 180;
}

/** @param {GpsPosition} from @param {GpsPosition} to */
export function haversineDistanceMeters(from, to) {
  const latitudeDelta = toRadians(to.lat - from.lat);
  const longitudeDelta = toRadians(to.lng - from.lng);
  const fromLatitude = toRadians(from.lat);
  const toLatitude = toRadians(to.lat);
  const a = Math.sin(latitudeDelta / 2) ** 2
    + Math.cos(fromLatitude) * Math.cos(toLatitude) * Math.sin(longitudeDelta / 2) ** 2;
  return EARTH_RADIUS_METERS * 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
}

/**
 * @param {GpsPosition[]} points
 * @param {GpsPosition} position
 * @param {{ minDistanceMeters?: number, maxPoints?: number }} [options]
 * @returns {GpsPosition[]}
 */
export function appendTrackPoint(points, position, options = {}) {
  if (!isValidGpsPosition(position)) return points;
  const minimumDistance = options.minDistanceMeters ?? DEFAULT_MIN_DISTANCE_METERS;
  const maximumPoints = Math.max(1, options.maxPoints ?? DEFAULT_MAX_POINTS);
  const previous = points.at(-1);
  if (previous && haversineDistanceMeters(previous, position) < minimumDistance) return points;
  const retained = maximumPoints === 1 ? [] : points.slice(-(maximumPoints - 1));
  return [...retained, { ...position }];
}
