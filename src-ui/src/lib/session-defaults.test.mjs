import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  SESSION_DEFAULTS_STORAGE_KEY,
  loadSessionDefaults,
  saveSessionDefaults,
  validateTestMetadata,
} from './session-defaults.js';

function memoryStorage(initial = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };
}

test('remembers only operator and location', () => {
  const storage = memoryStorage();
  saveSessionDefaults(storage, {
    purpose: 'must not persist',
    operator: ' Alice ',
    location: ' Lab ',
    initialBatteryVoltage: 8.2,
    notes: 'must not persist',
  });
  assert.deepEqual(loadSessionDefaults(storage), { operator: 'Alice', location: 'Lab' });
  assert.deepEqual(
    JSON.parse(storage.getItem(SESSION_DEFAULTS_STORAGE_KEY)),
    { operator: 'Alice', location: 'Lab' },
  );
});

test('validates every required field and positive finite voltage', () => {
  assert.deepEqual(Object.keys(validateTestMetadata({
    purpose: ' ', operator: '', location: '', initialBatteryVoltage: Number.NaN,
  })).sort(), ['initialBatteryVoltage', 'location', 'operator', 'purpose']);
  assert.deepEqual(validateTestMetadata({
    purpose: 'timer test', operator: 'Alice', location: 'Lab', initialBatteryVoltage: 8.2,
  }), {});
});

test('mandatory dialog cannot be dismissed through ESC or backdrop and gates unrecorded mode', () => {
  const source = readFileSync(
    new URL('../components/TestSessionDialog.svelte', import.meta.url),
    'utf8',
  );
  assert.match(source, /showModal\(\)/u);
  assert.match(source, /aria-modal="true"/u);
  assert.match(source, /oncancel=\{\(event\) => event\.preventDefault\(\)\}/u);
  assert.match(source, /storageFailed && !acknowledgedUnrecorded/u);
  assert.match(source, /開始監控並記錄/u);
  assert.match(source, /僅監控，不記錄/u);
  assert.doesNotMatch(source, /onclick=.*dialogElement\.close/u);
});

test('cancel closes the request without invoking the backend', () => {
  const source = readFileSync(
    new URL('../components/TestSessionDialog.svelte', import.meta.url),
    'utf8',
  );
  const cancelBody = source.match(/function cancel\(\) \{([\s\S]*?)\n  \}/u)?.[1] ?? '';
  assert.match(cancelBody, /store\.cancelTestStart\(\)/u);
  assert.doesNotMatch(cancelBody, /startTestMonitoring|invoke/u);
});
