import assert from 'node:assert/strict';
import test from 'node:test';
import {
  appendTrackPoint,
  haversineDistanceMeters,
  isValidGpsPosition,
} from './gps-map.js';

test('validates finite coordinates and treats zero-zero as no fix', () => {
  assert.equal(isValidGpsPosition({ lat: 25.033, lng: 121.5654 }), true);
  assert.equal(isValidGpsPosition({ lat: 0, lng: 0 }), false);
  assert.equal(isValidGpsPosition({ lat: 91, lng: 121 }), false);
  assert.equal(isValidGpsPosition({ lat: Number.NaN, lng: 121 }), false);
});

test('computes useful Haversine distance', () => {
  const meters = haversineDistanceMeters(
    { lat: 25.033, lng: 121.5654 },
    { lat: 25.03309, lng: 121.5654 },
  );
  assert.ok(meters > 9 && meters < 11);
});

test('filters sub-two-meter jitter', () => {
  const first = { lat: 25.033, lng: 121.5654 };
  assert.equal(
    appendTrackPoint([first], { lat: 25.033001, lng: 121.5654 }).length,
    1,
  );
});

test('keeps at most 5000 track points', () => {
  const points = Array.from({ length: 5000 }, (_, index) => ({
    lat: 20 + index * 0.0001,
    lng: 121,
  }));
  const next = appendTrackPoint(points, { lat: 25.1, lng: 121 });
  assert.equal(next.length, 5000);
  assert.deepEqual(next.at(-1), { lat: 25.1, lng: 121 });
  assert.notDeepEqual(next[0], points[0]);
});

test('honors a one-point custom track limit', () => {
  const next = appendTrackPoint(
    [{ lat: 25, lng: 121 }],
    { lat: 25.1, lng: 121 },
    { maxPoints: 1 },
  );
  assert.deepEqual(next, [{ lat: 25.1, lng: 121 }]);
});

test('does not append invalid positions', () => {
  const points = [{ lat: 25.033, lng: 121.5654 }];
  assert.equal(appendTrackPoint(points, { lat: 0, lng: 0 }), points);
});
