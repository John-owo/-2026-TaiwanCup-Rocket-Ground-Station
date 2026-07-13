import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
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

test('uses COM3 as the first-run connection default', () => {
  assert.equal(loadSettings(memoryStorage()).portPath, 'COM3');
});

test('restores COM3 when a saved connection port is blank', () => {
  assert.equal(validateSettings({
    version: 1,
    portPath: '',
    baudRate: 115200,
    axisMapping: DEFAULT_SETTINGS.axisMapping,
  }).portPath, 'COM3');
});

test('drops non-serializable extras from an otherwise valid axis mapping', () => {
  const mapping = {
    x: { source: 'x', sign: 1, extra: () => {} },
    y: { source: 'y', sign: 1 },
    z: { source: 'z', sign: 1 },
  };
  const settings = validateSettings({
    version: 1,
    portPath: 'COM3',
    baudRate: 115200,
    axisMapping: mapping,
  });

  assert.deepEqual(settings.axisMapping, DEFAULT_SETTINGS.axisMapping);
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

test('increments telemetry revision for every store update', () => {
  const source = readFileSync(new URL('./stores.svelte.ts', import.meta.url), 'utf8');
  const updateBody = source.match(
    /updateTelemetry\(payload: TelemetryPayload\) \{([\s\S]*?)\n    \},/u,
  )?.[1] ?? '';
  assert.match(updateBody, /telemetryRevision \+= 1/u);
});

test('monitoring button uses a direct click handler without a window pointer workaround', () => {
  const source = readFileSync(
    new URL('../components/ConnectionPanel.svelte', import.meta.url),
    'utf8',
  );
  assert.match(source, /<button[^>]*class="connect-btn"[^>]*onclick=\{handleConnect\}[^>]*>/u);
  assert.doesNotMatch(source, /handleWindowPointerDown/u);
  assert.doesNotMatch(source, /window\.addEventListener\('pointerdown'/u);
});

test('connection panel displays and clears background serial errors', () => {
  const source = readFileSync(
    new URL('../components/ConnectionPanel.svelte', import.meta.url),
    'utf8',
  );

  assert.match(source, /store\.errors\.at\(-1\)\?\.detail/u);
  assert.match(source, /store\.clearErrors\(\)/u);
});

test('monitoring starts without persisting settings inside the click handler', () => {
  const source = readFileSync(
    new URL('../components/ConnectionPanel.svelte', import.meta.url),
    'utf8',
  );
  const body = source.match(/async function handleConnect\(\) \{([\s\S]*?)\n  \}/u)?.[1] ?? '';

  assert.match(body, /const selectedPort = portPath\.trim\(\);/u);
  assert.doesNotMatch(body, /persistPort\(\);\s*persistBaudRate\(\);/u);
});

test('left connection panel is stacked above central telemetry content', () => {
  const source = readFileSync(new URL('../App.svelte', import.meta.url), 'utf8');
  assert.match(source, /\.sidebar-left\s*\{[\s\S]*?z-index:\s*2;/u);
});
