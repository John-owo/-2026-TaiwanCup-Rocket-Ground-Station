import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DEFAULT_SETTINGS,
  SETTINGS_STORAGE_KEY,
  loadSettings,
  saveSettings,
  setAxisSign,
  swapAxisSource,
  validateSettings,
} from './settings.js';

function memoryStorage(initial = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
  };
}

test('loads safe defaults when storage is empty or corrupt', () => {
  assert.deepEqual(loadSettings(memoryStorage()), DEFAULT_SETTINGS);
  assert.deepEqual(
    loadSettings(memoryStorage({ [SETTINGS_STORAGE_KEY]: '{bad json' })),
    DEFAULT_SETTINGS,
  );
});

test('round trips valid COM, baud, and axis mapping', () => {
  const storage = memoryStorage();
  const settings = validateSettings({
    version: 1,
    portPath: 'COM7',
    baudRate: 57600,
    axisMapping: {
      x: { source: 'z', sign: -1 },
      y: { source: 'x', sign: 1 },
      z: { source: 'y', sign: 1 },
    },
  });
  saveSettings(storage, settings);
  assert.deepEqual(loadSettings(storage), settings);
});

test('replaces invalid settings with safe field defaults', () => {
  assert.deepEqual(validateSettings({
    version: 1,
    portPath: 123,
    baudRate: 12345,
    axisMapping: {
      x: { source: 'x', sign: 1 },
      y: { source: 'x', sign: -1 },
      z: { source: 'z', sign: 0 },
    },
  }), DEFAULT_SETTINGS);
});

test('falls back per field without discarding valid connection values', () => {
  const validated = validateSettings({
    version: 1,
    portPath: 'COM9',
    baudRate: 12345,
    axisMapping: DEFAULT_SETTINGS.axisMapping,
  });
  assert.equal(validated.portPath, 'COM9');
  assert.equal(validated.baudRate, 115200);
});

test('swaps occupied axis sources and changes signs without mutation', () => {
  const swapped = swapAxisSource(DEFAULT_SETTINGS.axisMapping, 'x', 'y');
  assert.deepEqual(swapped, {
    x: { source: 'y', sign: 1 },
    y: { source: 'x', sign: 1 },
    z: { source: 'z', sign: 1 },
  });
  assert.deepEqual(DEFAULT_SETTINGS.axisMapping.x, { source: 'x', sign: 1 });
  assert.equal(setAxisSign(swapped, 'z', -1).z.sign, -1);
});
