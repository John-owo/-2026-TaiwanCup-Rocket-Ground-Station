/** @typedef {'x' | 'y' | 'z'} SensorAxis */
/** @typedef {1 | -1} AxisSign */
/** @typedef {{ source: SensorAxis, sign: AxisSign }} AxisRule */
/** @typedef {{ x: AxisRule, y: AxisRule, z: AxisRule }} AxisMapping */
/** @typedef {{ version: 1, portPath: string, baudRate: number, axisMapping: AxisMapping }} AppSettings */
/** @typedef {{ getItem(key: string): string | null, setItem(key: string, value: string): void, removeItem?(key: string): void }} StorageLike */

export const SETTINGS_STORAGE_KEY = 'rocket-ground-station.settings.v1';
export const BAUD_RATES = Object.freeze([9600, 19200, 38400, 57600, 115200]);
/** @type {readonly SensorAxis[]} */
export const BODY_AXES = Object.freeze(['x', 'y', 'z']);

/** @type {AppSettings} */
export const DEFAULT_SETTINGS = Object.freeze({
  version: 1,
  portPath: 'COM3',
  baudRate: 115200,
  axisMapping: Object.freeze({
    x: Object.freeze({ source: 'x', sign: 1 }),
    y: Object.freeze({ source: 'y', sign: 1 }),
    z: Object.freeze({ source: 'z', sign: 1 }),
  }),
});

/** @returns {AppSettings} */
function cloneDefaults() {
  return structuredClone(DEFAULT_SETTINGS);
}

/** @param {unknown} value @returns {value is SensorAxis} */
function isSensorAxis(value) {
  return value === 'x' || value === 'y' || value === 'z';
}

/** @param {unknown} value @returns {AppSettings} */
export function validateSettings(value) {
  if (!value || typeof value !== 'object' || !('version' in value) || value.version !== 1) {
    return cloneDefaults();
  }

  const candidate = /** @type {Record<string, unknown>} */ (value);
  const portPath = typeof candidate.portPath === 'string' && candidate.portPath.trim()
    ? candidate.portPath.trim()
    : DEFAULT_SETTINGS.portPath;
  const baudRate = typeof candidate.baudRate === 'number' && BAUD_RATES.includes(candidate.baudRate)
    ? candidate.baudRate
    : 115200;
  const mapping = /** @type {Record<string, unknown> | undefined} */ (candidate.axisMapping);
  const rules = BODY_AXES.map((axis) => /** @type {Record<string, unknown> | undefined} */ (mapping?.[axis]));
  const sources = rules.map((rule) => rule?.source);
  const mappingValid = new Set(sources).size === 3
    && sources.every(isSensorAxis)
    && rules.every((rule) => rule?.sign === 1 || rule?.sign === -1);
  const axisMapping = mappingValid
    ? /** @type {AxisMapping} */ ({
      x: {
        source: /** @type {SensorAxis} */ (rules[0]?.source),
        sign: /** @type {AxisSign} */ (rules[0]?.sign),
      },
      y: {
        source: /** @type {SensorAxis} */ (rules[1]?.source),
        sign: /** @type {AxisSign} */ (rules[1]?.sign),
      },
      z: {
        source: /** @type {SensorAxis} */ (rules[2]?.source),
        sign: /** @type {AxisSign} */ (rules[2]?.sign),
      },
    })
    : structuredClone(DEFAULT_SETTINGS.axisMapping);

  return {
    version: 1,
    portPath,
    baudRate,
    axisMapping,
  };
}

/** @param {StorageLike | undefined} storage @returns {AppSettings} */
export function loadSettings(storage) {
  try {
    const raw = storage?.getItem(SETTINGS_STORAGE_KEY);
    return raw ? validateSettings(JSON.parse(raw)) : cloneDefaults();
  } catch {
    return cloneDefaults();
  }
}

/** @param {StorageLike | undefined} storage @param {unknown} settings @returns {AppSettings} */
export function saveSettings(storage, settings) {
  const validated = validateSettings(settings);
  storage?.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(validated));
  return validated;
}

/**
 * @param {AxisMapping} mapping
 * @param {SensorAxis} bodyAxis
 * @param {SensorAxis} source
 * @returns {AxisMapping}
 */
export function swapAxisSource(mapping, bodyAxis, source) {
  const next = structuredClone(mapping);
  const occupiedAxis = BODY_AXES.find((axis) => next[axis].source === source);
  const previousSource = next[bodyAxis].source;
  if (occupiedAxis && occupiedAxis !== bodyAxis) next[occupiedAxis].source = previousSource;
  next[bodyAxis].source = source;
  return next;
}

/**
 * @param {AxisMapping} mapping
 * @param {SensorAxis} bodyAxis
 * @param {number} sign
 * @returns {AxisMapping}
 */
export function setAxisSign(mapping, bodyAxis, sign) {
  const next = structuredClone(mapping);
  next[bodyAxis].sign = sign === -1 ? -1 : 1;
  return next;
}
