import assert from 'node:assert/strict';
import test from 'node:test';
import {
  bearingDegrees,
  compassLabel,
  currentSessionRun,
  formatDistance,
  formatUptime,
  ladderFraction,
  niceAltitudeCeiling,
} from './flight-display.js';

test('altitude ceiling rounds up with headroom and a floor', () => {
  assert.equal(niceAltitudeCeiling(0), 60);
  assert.equal(niceAltitudeCeiling(612.4), 900);
  assert.equal(niceAltitudeCeiling(90), 120);
  assert.equal(niceAltitudeCeiling(2000) % 6, 0);
  assert.equal(niceAltitudeCeiling(Number.NaN), 60);
});

test('ladder fraction clamps to the scale', () => {
  assert.equal(ladderFraction(450, 900), 0.5);
  assert.equal(ladderFraction(-12, 900), 0);
  assert.equal(ladderFraction(1000, 900), 1);
  assert.equal(ladderFraction(Number.NaN, 800), 0);
});

test('bearing points the right way', () => {
  const origin = { lat: 22.3, lng: 120.9 };
  assert.ok(Math.abs(bearingDegrees(origin, { lat: 22.4, lng: 120.9 }) - 0) < 0.01);
  assert.ok(Math.abs(bearingDegrees(origin, { lat: 22.3, lng: 121.0 }) - 90) < 0.1);
  assert.equal(compassLabel(20), '北');
  assert.equal(compassLabel(24), '東北');
  assert.equal(compassLabel(46), '東北');
  assert.equal(compassLabel(359), '北');
});

test('distance and uptime formatting', () => {
  assert.equal(formatDistance(128.4), '128 m');
  assert.equal(formatDistance(2345), '2.35 km');
  assert.equal(formatUptime(74_250), '1:14.2');
  assert.equal(formatUptime(-1), '--');
});

test('profile keeps only the current, monotonic session run', () => {
  const samples = [
    { sessionId: 1, uptimeMs: 1000 },
    { sessionId: 1, uptimeMs: 2000 },
    { sessionId: 2, uptimeMs: 500 },
    { sessionId: 2, uptimeMs: 900 },
  ];
  assert.deepEqual(currentSessionRun(samples).map((sample) => sample.uptimeMs), [500, 900]);
  assert.deepEqual(currentSessionRun([]), []);
});
