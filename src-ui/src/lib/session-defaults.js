export const SESSION_DEFAULTS_STORAGE_KEY = 'rocket-ground-station.session-defaults.v1';

const EMPTY_DEFAULTS = Object.freeze({ operator: '', location: '' });

/** @param {{ getItem(key: string): string | null } | undefined} storage */
export function loadSessionDefaults(storage) {
  try {
    const raw = storage?.getItem(SESSION_DEFAULTS_STORAGE_KEY);
    if (!raw) return { ...EMPTY_DEFAULTS };
    const value = JSON.parse(raw);
    return {
      operator: typeof value?.operator === 'string' ? value.operator.trim() : '',
      location: typeof value?.location === 'string' ? value.location.trim() : '',
    };
  } catch {
    return { ...EMPTY_DEFAULTS };
  }
}

/**
 * @param {{ setItem(key: string, value: string): void } | undefined} storage
 * @param {{ operator?: unknown, location?: unknown } | undefined} defaults
 */
export function saveSessionDefaults(storage, defaults) {
  const value = {
    operator: typeof defaults?.operator === 'string' ? defaults.operator.trim() : '',
    location: typeof defaults?.location === 'string' ? defaults.location.trim() : '',
  };
  storage?.setItem(SESSION_DEFAULTS_STORAGE_KEY, JSON.stringify(value));
  return value;
}

/**
 * @param {{ purpose?: unknown, operator?: unknown, location?: unknown, initialBatteryVoltage?: unknown } | undefined} value
 * @returns {Record<string, string>}
 */
export function validateTestMetadata(value) {
  /** @type {Record<string, string>} */
  const errors = {};
  if (typeof value?.purpose !== 'string' || !value.purpose.trim()) {
    errors.purpose = '請填寫本次測試目的';
  }
  if (typeof value?.operator !== 'string' || !value.operator.trim()) {
    errors.operator = '請填寫操作者';
  }
  if (typeof value?.location !== 'string' || !value.location.trim()) {
    errors.location = '請填寫測試地點';
  }
  const voltage = value?.initialBatteryVoltage;
  if (typeof voltage !== 'number' || !Number.isFinite(voltage) || voltage <= 0) {
    errors.initialBatteryVoltage = '請輸入大於 0 的起始電池電壓';
  }
  return errors;
}
