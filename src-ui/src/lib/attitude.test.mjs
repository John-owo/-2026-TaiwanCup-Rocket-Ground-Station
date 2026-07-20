import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  createAttitudeEstimator,
  integrateGyroDegrees,
  mapSensorVector,
} from './attitude.js';

const identity = {
  x: { source: 'x', sign: 1 },
  y: { source: 'y', sign: 1 },
  z: { source: 'z', sign: 1 },
};

const sample = (gyro, accel = { x: 0, y: 0, z: 30 }) => ({ gyro, accel });

test('maps axes and signs for sensor vectors', () => {
  const mapping = {
    x: { source: 'z', sign: -1 },
    y: { source: 'x', sign: 1 },
    z: { source: 'y', sign: -1 },
  };
  assert.deepEqual(
    mapSensorVector({ x: 1, y: 2, z: 3 }, mapping),
    { x: -3, y: 1, z: -2 },
  );
});

test('integrates degrees per second without radian conversion', () => {
  assert.deepEqual(
    integrateGyroDegrees(
      { roll: 0, pitch: 0, yaw: 0 },
      { x: 90, y: 45, z: 180 },
      1,
    ),
    { roll: 90, pitch: 45, yaw: 180 },
  );
});

test('updates the estimator once per packet interval', () => {
  const estimator = createAttitudeEstimator();
  estimator.update(sample({ x: 90, y: 45, z: 180 }), 1000, identity);
  assert.deepEqual(
    estimator.update(sample({ x: 90, y: 45, z: 180 }), 1100, identity),
    { roll: 9, pitch: 4.5, yaw: 18 },
  );
});

test('integrates normal low-frequency telemetry at 500 ms and 1000 ms', () => {
  const estimator = createAttitudeEstimator();
  const actualImuSample = sample(
    { x: 31.5, y: 1.4, z: -0.06 },
    { x: -0.5, y: -0.35, z: 9.3 },
  );

  estimator.update(actualImuSample, 1000, identity);
  const after500Ms = estimator.update(actualImuSample, 1500, identity);
  const after1000Ms = estimator.update(actualImuSample, 2500, identity);

  assert.ok(Math.abs(after500Ms.roll) + Math.abs(after500Ms.pitch) > 0.1);
  assert.notDeepEqual(after1000Ms, after500Ms);
});

test('integrates the formal 1800 ms telemetry interval', () => {
  const estimator = createAttitudeEstimator();
  const rotating = sample({ x: 0, y: 0, z: 10 });

  estimator.update(rotating, 1_000, identity);
  const afterFormalInterval = estimator.update(rotating, 2_800, identity);

  assert.equal(afterFormalInterval.yaw, 18);
});

test('uses gravity correction near 1g but rejects powered-flight acceleration', () => {
  const estimator = createAttitudeEstimator({
    initialAttitude: { roll: 30, pitch: 20, yaw: 0 },
  });
  estimator.update(
    sample({ x: 0, y: 0, z: 0 }, { x: 0, y: 0, z: 9.80665 }),
    1000,
    identity,
  );
  const corrected = estimator.update(
    sample({ x: 0, y: 0, z: 0 }, { x: 0, y: 0, z: 9.80665 }),
    1100,
    identity,
  );
  assert.ok(Math.abs(corrected.roll) < 30);
  assert.ok(Math.abs(corrected.pitch) < 20);

  const beforeBoost = estimator.getAttitude();
  const boosted = estimator.update(
    sample({ x: 0, y: 0, z: 0 }, { x: 30, y: 0, z: 30 }),
    1200,
    identity,
  );
  assert.deepEqual(boosted, beforeBoost);
});

test('rebuilds the time baseline after a multi-second telemetry interruption', () => {
  const estimator = createAttitudeEstimator();
  const rotating = sample({ x: 0, y: 0, z: 100 });

  estimator.update(rotating, 1000, identity);
  const beforeGap = estimator.update(rotating, 1500, identity);
  const afterGap = estimator.update(rotating, 6001, identity);
  const afterResume = estimator.update(rotating, 6501, identity);

  assert.equal(beforeGap.yaw, 50);
  assert.deepEqual(afterGap, beforeGap);
  assert.equal(afterResume.yaw, 100);
});

test('rebuilds the time baseline after the 4500 ms link-loss threshold', () => {
  const estimator = createAttitudeEstimator();
  const rotating = sample({ x: 0, y: 0, z: 10 });

  estimator.update(rotating, 1_000, identity);
  const beforeGap = estimator.update(rotating, 2_800, identity);
  const afterGap = estimator.update(rotating, 7_301, identity);
  const afterResume = estimator.update(rotating, 9_101, identity);

  assert.equal(beforeGap.yaw, 18);
  assert.deepEqual(afterGap, beforeGap);
  assert.equal(afterResume.yaw, 36);
});

test('rebuilds the time baseline when airborne uptime moves backwards', () => {
  const estimator = createAttitudeEstimator();
  const rotating = sample({ x: 0, y: 0, z: 10 });

  estimator.update(rotating, 10_000, identity);
  const beforeRestart = estimator.update(rotating, 11_800, identity);
  const afterRestart = estimator.update(rotating, 200, identity);
  const afterResume = estimator.update(rotating, 2_000, identity);

  assert.equal(beforeRestart.yaw, 18);
  assert.deepEqual(afterRestart, beforeRestart);
  assert.equal(afterResume.yaw, 36);
});

test('attitude component uses airborne uptime and explains relative yaw', () => {
  const source = readFileSync(
    new URL('../components/AttitudeIndicator.svelte', import.meta.url),
    'utf8',
  );

  assert.match(source, /snapshot\.telemetry\.uptimeMs/u);
  assert.doesNotMatch(source, /performance\.now\(\)/u);
  assert.match(source, /估算姿態/u);
  assert.match(source, /YAW 為相對角度，非絕對航向/u);
});

test('ignores non-finite gyro updates', () => {
  const estimator = createAttitudeEstimator();
  estimator.update(sample({ x: 0, y: 0, z: 0 }), 1000, identity);
  assert.deepEqual(
    estimator.update(sample({ x: Number.NaN, y: 0, z: 0 }), 1100, identity),
    { roll: 0, pitch: 0, yaw: 0 },
  );
});

test('reset clears attitude and establishes a new time baseline', () => {
  const estimator = createAttitudeEstimator();
  estimator.update(sample({ x: 0, y: 0, z: 100 }), 1000, identity);
  estimator.update(sample({ x: 0, y: 0, z: 100 }), 1100, identity);
  assert.equal(estimator.getAttitude().yaw, 10);
  assert.deepEqual(estimator.reset(), { roll: 0, pitch: 0, yaw: 0 });
  assert.deepEqual(
    estimator.update(sample({ x: 0, y: 0, z: 100 }), 5000, identity),
    { roll: 0, pitch: 0, yaw: 0 },
  );
});
