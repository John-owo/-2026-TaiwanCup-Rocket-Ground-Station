import assert from 'node:assert/strict';
import test from 'node:test';
import {
  TELEMETRY_LINK_LOSS_THRESHOLD_MS,
  getTelemetryLinkState,
} from './telemetry-link.js';

test('distinguishes standby, waiting, live, and lost telemetry states', () => {
  assert.equal(getTelemetryLinkState(false, null, 10_000), 'standby');
  assert.equal(getTelemetryLinkState(true, null, 10_000), 'waiting');
  assert.equal(getTelemetryLinkState(true, 7_000, 10_000), 'live');
  assert.equal(getTelemetryLinkState(true, 5_000, 10_000), 'lost');
});

test('keeps telemetry live through exactly 4500 ms and loses it after', () => {
  assert.equal(TELEMETRY_LINK_LOSS_THRESHOLD_MS, 4_500);
  assert.equal(getTelemetryLinkState(true, 1_000, 5_500), 'live');
  assert.equal(getTelemetryLinkState(true, 1_000, 5_501), 'lost');
});
