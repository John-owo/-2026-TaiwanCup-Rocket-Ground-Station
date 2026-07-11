import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

test('monitoring UI uses Traditional Chinese labels', () => {
  const text = [
    read('../App.svelte'),
    read('../components/TelemetryGrid.svelte'),
    read('../components/TelemetryCharts.svelte'),
    read('../components/StatusBar.svelte'),
    read('../components/ConnectionPanel.svelte'),
  ].join('\n');

  for (const oldLabel of [
    'Ground Speed',
    'Vertical Velocity',
    'Waiting for telemetry',
    'Start Monitoring',
    'Disconnect',
    'Relative Altitude',
  ]) {
    assert.equal(text.includes(oldLabel), false, oldLabel);
  }

  for (const label of [
    '地面速度',
    '垂直速度',
    '等待遙測資料',
    '開始監控',
    '中斷連線',
    '相對高度',
  ]) {
    assert.equal(text.includes(label), true, label);
  }
});

test('app renders GPS map before attitude in the right sidebar', () => {
  const app = read('../App.svelte');
  assert.match(app, /<aside class="sidebar-right">[\s\S]*<GpsMap \/>[\s\S]*<AttitudeIndicator \/>/u);
});
